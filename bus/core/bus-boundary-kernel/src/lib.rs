//! # messaging-bus-boundary-kernel
//!
//! The owned EVENT-BUS surface — third of the three single-concern
//! messaging surfaces (ADR-0536 D-13 queue/stream/bus trichotomy;
//! ADR-0132 no-grouping) composed over the ONE substrate port
//! (`messaging-substrate-kernel`).
//!
//! Bus semantics (precedent: AWS EventBridge; Google Pub/Sub fan-out):
//! - publish/subscribe fan-out — every subscriber group independently
//!   observes every event on the channel;
//! - typed events: each event names its type in the envelope headers so
//!   subscribers can route without parsing payloads;
//! - within one subscriber group, deliveries compete (a group scales
//!   horizontally without double-processing).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU32;

use messaging_substrate_kernel::{
    AckToken, Delivery, LossClass, MessageConsumer, MessageEnvelope, MessageId, MessageKey,
    MessageProducer, MessagingAdmin, MessagingError, MessagingSubstrate, SubscriptionName,
    TopicName, TopicSpec,
};

/// Envelope header carrying the event type.
pub const EVENT_TYPE_HEADER: &str = "event-type";

/// Bus-surface errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusError {
    /// Channel name violates the canonical slug shape.
    InvalidChannelName { value: String },
    /// Subscriber-group name violates the canonical slug shape.
    InvalidSubscriberGroup { value: String },
    /// Event type violates the canonical slug shape.
    InvalidEventType { value: String },
    /// The underlying substrate raised a contract error.
    Substrate(MessagingError),
}

impl fmt::Display for BusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChannelName { value } => write!(f, "invalid channel name: {value:?}"),
            Self::InvalidSubscriberGroup { value } => {
                write!(f, "invalid subscriber group: {value:?}")
            }
            Self::InvalidEventType { value } => write!(f, "invalid event type: {value:?}"),
            Self::Substrate(error) => write!(f, "substrate: {error}"),
        }
    }
}

impl std::error::Error for BusError {}

impl From<MessagingError> for BusError {
    fn from(error: MessagingError) -> Self {
        Self::Substrate(error)
    }
}

/// Canonical bus channel name (a topic-safe slug).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ChannelName(String);

impl ChannelName {
    /// Parses and validates a channel name.
    ///
    /// # Errors
    /// Returns [`BusError::InvalidChannelName`] when the derived topic
    /// name would not be a canonical slug.
    pub fn parse(value: &str) -> Result<Self, BusError> {
        // `bus.` is a WIRE identifier, not a crate name: a rename here is a
        // behavior change and must not ride along inside a relocation.
        TopicName::parse(&format!("bus.{value}")).map_err(|_| BusError::InvalidChannelName {
            value: value.to_owned(),
        })?;
        Ok(Self(value.to_owned()))
    }

    /// The canonical string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A typed event: type slug + optional ordering key + payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    /// Event type slug (e.g. `tenant.suspended`).
    pub event_type: String,
    /// Optional per-key ordering unit (e.g. the tenant id).
    pub key: Option<MessageKey>,
    /// Extra headers merged under the typed header.
    pub headers: BTreeMap<String, String>,
    /// Opaque payload bytes (schema ownership belongs to the publisher's
    /// contract crate, never to the bus).
    pub payload: Vec<u8>,
}

/// The owned event-bus surface over the ONE messaging substrate.
pub struct EventBus<'a, S: MessagingSubstrate> {
    substrate: &'a S,
    topic: TopicName,
}

impl<'a, S: MessagingSubstrate> EventBus<'a, S> {
    /// Binds (and idempotently provisions) the channel on the substrate.
    ///
    /// # Errors
    /// Propagates substrate provisioning failures.
    pub fn bind(
        substrate: &'a S,
        channel: &ChannelName,
        loss_class: LossClass,
    ) -> Result<Self, BusError> {
        let topic = TopicName::parse(&format!("bus.{}", channel.as_str()))?;
        substrate.ensure_topic(
            &topic,
            &TopicSpec {
                loss_class,
                retention: None,
            },
        )?;
        Ok(Self { substrate, topic })
    }

    /// Publishes one typed event to every subscriber group.
    ///
    /// # Errors
    /// Returns [`BusError::InvalidEventType`] for a malformed type slug;
    /// propagates substrate publish failures.
    pub fn publish(&self, event: Event) -> Result<MessageId, BusError> {
        TopicName::parse(&event.event_type).map_err(|_| BusError::InvalidEventType {
            value: event.event_type.clone(),
        })?;
        let mut headers = event.headers;
        headers.insert(EVENT_TYPE_HEADER.to_owned(), event.event_type);
        Ok(self.substrate.publish(
            &self.topic,
            MessageEnvelope {
                key: event.key,
                headers,
                payload: event.payload,
            },
        )?)
    }

    /// Binds (and idempotently provisions) a subscriber group. Every
    /// group independently observes every event published after it.
    ///
    /// # Errors
    /// Propagates substrate provisioning failures.
    pub fn subscriber_group(&self, name: &str) -> Result<SubscriberGroup<'a, S>, BusError> {
        let subscription =
            SubscriptionName::parse(name).map_err(|_| BusError::InvalidSubscriberGroup {
                value: name.to_owned(),
            })?;
        self.substrate
            .ensure_subscription(&self.topic, &subscription)?;
        Ok(SubscriberGroup {
            substrate: self.substrate,
            topic: self.topic.clone(),
            subscription,
        })
    }
}

/// One subscriber group: competing consumers within, fan-out across.
pub struct SubscriberGroup<'a, S: MessagingSubstrate> {
    substrate: &'a S,
    topic: TopicName,
    subscription: SubscriptionName,
}

impl<S: MessagingSubstrate> SubscriberGroup<'_, S> {
    /// Pulls up to `max` events for this group.
    ///
    /// # Errors
    /// Propagates substrate receive failures.
    pub fn poll(&self, max: NonZeroU32) -> Result<Vec<Delivery>, BusError> {
        Ok(self
            .substrate
            .receive(&self.topic, &self.subscription, max)?)
    }

    /// Settles an event as handled by this group.
    ///
    /// # Errors
    /// Propagates substrate settle failures.
    pub fn ack(&self, token: &AckToken) -> Result<(), BusError> {
        Ok(self.substrate.ack(token)?)
    }

    /// Returns an event to this group's backlog.
    ///
    /// # Errors
    /// Propagates substrate settle failures.
    pub fn nack(&self, token: &AckToken) -> Result<(), BusError> {
        Ok(self.substrate.nack(token)?)
    }
}

/// Reads the event type recorded on a delivery, if present.
#[must_use]
pub fn event_type_of(delivery: &Delivery) -> Option<&str> {
    delivery
        .envelope
        .headers
        .get(EVENT_TYPE_HEADER)
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use messaging_substrate_kernel::reference::InMemorySubstrate;

    use super::*;

    fn batch(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn event(event_type: &str, ordinal: u32) -> Event {
        Event {
            event_type: event_type.to_owned(),
            key: None,
            headers: BTreeMap::new(),
            payload: format!("event-{ordinal}").into_bytes(),
        }
    }

    #[test]
    fn every_subscriber_group_observes_every_event() {
        let substrate = InMemorySubstrate::new();
        let bus = EventBus::bind(
            &substrate,
            &ChannelName::parse("tenancy").unwrap(),
            LossClass::NeverLose,
        )
        .unwrap();
        let billing = bus.subscriber_group("billing").unwrap();
        let audit = bus.subscriber_group("audit").unwrap();
        bus.publish(event("tenant.suspended", 1)).unwrap();
        for group in [&billing, &audit] {
            let events = group.poll(batch(16)).unwrap();
            assert_eq!(events.len(), 1);
            assert_eq!(event_type_of(&events[0]), Some("tenant.suspended"));
            group.ack(&events[0].ack_token).unwrap();
        }
    }

    #[test]
    fn within_a_group_events_compete() {
        let substrate = InMemorySubstrate::new();
        let bus = EventBus::bind(
            &substrate,
            &ChannelName::parse("tenancy").unwrap(),
            LossClass::Expendable,
        )
        .unwrap();
        let group = bus.subscriber_group("billing").unwrap();
        bus.publish(event("tenant.created", 1)).unwrap();
        assert_eq!(group.poll(batch(16)).unwrap().len(), 1);
        // The same group polling again while in flight gets nothing.
        assert!(group.poll(batch(16)).unwrap().is_empty());
    }

    #[test]
    fn malformed_event_types_are_rejected() {
        let substrate = InMemorySubstrate::new();
        let bus = EventBus::bind(
            &substrate,
            &ChannelName::parse("tenancy").unwrap(),
            LossClass::NeverLose,
        )
        .unwrap();
        let result = bus.publish(event("Tenant Suspended!", 1));
        assert!(matches!(result, Err(BusError::InvalidEventType { .. })));
    }
}
