//! # messaging-stream-boundary-kernel
//!
//! The owned ORDERED-STREAM surface — second of the three single-concern
//! messaging surfaces (ADR-0536 D-13 queue/stream/bus trichotomy;
//! ADR-0132 no-grouping) composed over the ONE substrate port
//! (`messaging-substrate-kernel`).
//!
//! Stream semantics (precedent: Google Pub/Sub seek/replay; Pulsar
//! cursors over segmented BookKeeper storage):
//! - an append-only log with totally-ordered positions and per-key
//!   publish-order delivery;
//! - named readers, each owning an independent committed cursor;
//! - replay: a reader seeks any retained position and re-observes the
//!   log from there (the audit/metering re-derivation primitive).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;
use std::num::NonZeroU32;
use std::time::Duration;

use messaging_substrate_kernel::{
    AckToken, Delivery, LossClass, MessageConsumer, MessageEnvelope, MessageId, MessageProducer,
    MessagingAdmin, MessagingError, MessagingSubstrate, StreamPosition, SubscriptionName,
    TopicName, TopicSpec,
};

/// Stream-surface errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// Stream name violates the canonical slug shape.
    InvalidStreamName { value: String },
    /// Reader name violates the canonical slug shape.
    InvalidReaderName { value: String },
    /// The underlying substrate raised a contract error.
    Substrate(MessagingError),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStreamName { value } => write!(f, "invalid stream name: {value:?}"),
            Self::InvalidReaderName { value } => write!(f, "invalid reader name: {value:?}"),
            Self::Substrate(error) => write!(f, "substrate: {error}"),
        }
    }
}

impl std::error::Error for StreamError {}

impl From<MessagingError> for StreamError {
    fn from(error: MessagingError) -> Self {
        Self::Substrate(error)
    }
}

/// Canonical stream name (a topic-safe slug).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct StreamName(String);

impl StreamName {
    /// Parses and validates a stream name.
    ///
    /// # Errors
    /// Returns [`StreamError::InvalidStreamName`] when the derived topic
    /// name would not be a canonical slug.
    pub fn parse(value: &str) -> Result<Self, StreamError> {
        // `stream.` is a WIRE identifier, not a crate name: a rename here is a
        // behavior change and must not ride along inside a relocation.
        TopicName::parse(&format!("stream.{value}")).map_err(|_| {
            StreamError::InvalidStreamName {
                value: value.to_owned(),
            }
        })?;
        Ok(Self(value.to_owned()))
    }

    /// The canonical string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The owned ordered-stream surface over the ONE messaging substrate.
pub struct EventStream<'a, S: MessagingSubstrate> {
    substrate: &'a S,
    topic: TopicName,
}

impl<'a, S: MessagingSubstrate> EventStream<'a, S> {
    /// Binds (and idempotently provisions) the stream on the substrate.
    /// Streams are always [`LossClass::NeverLose`] with a bounded replay
    /// retention: a lossy "stream" is a contradiction — lossy traffic
    /// belongs on the bus surface with [`LossClass::Expendable`] topics.
    ///
    /// # Errors
    /// Propagates substrate provisioning failures.
    pub fn bind(
        substrate: &'a S,
        name: &StreamName,
        retention: Duration,
    ) -> Result<Self, StreamError> {
        let topic = TopicName::parse(&format!("stream.{}", name.as_str()))?;
        substrate.ensure_topic(
            &topic,
            &TopicSpec {
                loss_class: LossClass::NeverLose,
                retention: Some(retention),
            },
        )?;
        Ok(Self { substrate, topic })
    }

    /// Appends one record to the stream.
    ///
    /// # Errors
    /// Propagates substrate publish failures.
    pub fn append(&self, envelope: MessageEnvelope) -> Result<MessageId, StreamError> {
        Ok(self.substrate.publish(&self.topic, envelope)?)
    }

    /// Binds (and idempotently provisions) a named reader with its own
    /// committed cursor. New readers begin at the stream head.
    ///
    /// # Errors
    /// Propagates substrate provisioning failures.
    pub fn reader(&self, name: &str) -> Result<StreamReader<'a, S>, StreamError> {
        let subscription =
            SubscriptionName::parse(name).map_err(|_| StreamError::InvalidReaderName {
                value: name.to_owned(),
            })?;
        self.substrate
            .ensure_subscription(&self.topic, &subscription)?;
        Ok(StreamReader {
            substrate: self.substrate,
            topic: self.topic.clone(),
            subscription,
        })
    }
}

/// A named reader with an independent committed cursor.
pub struct StreamReader<'a, S: MessagingSubstrate> {
    substrate: &'a S,
    topic: TopicName,
    subscription: SubscriptionName,
}

impl<S: MessagingSubstrate> StreamReader<'_, S> {
    /// Reads up to `max` records from the cursor.
    ///
    /// # Errors
    /// Propagates substrate receive failures.
    pub fn read(&self, max: NonZeroU32) -> Result<Vec<Delivery>, StreamError> {
        Ok(self
            .substrate
            .receive(&self.topic, &self.subscription, max)?)
    }

    /// Commits one record as consumed; the cursor never returns it.
    ///
    /// # Errors
    /// Propagates substrate settle failures.
    pub fn commit(&self, token: &AckToken) -> Result<(), StreamError> {
        Ok(self.substrate.ack(token)?)
    }

    /// Releases one record back to the cursor for redelivery.
    ///
    /// # Errors
    /// Propagates substrate settle failures.
    pub fn release(&self, token: &AckToken) -> Result<(), StreamError> {
        Ok(self.substrate.nack(token)?)
    }

    /// Rewinds the cursor to `position`; retained records at or after it
    /// are re-observed (replay).
    ///
    /// # Errors
    /// Propagates substrate seek failures.
    pub fn seek(&self, position: StreamPosition) -> Result<(), StreamError> {
        Ok(self
            .substrate
            .seek(&self.topic, &self.subscription, position)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use messaging_substrate_kernel::MessageKey;
    use messaging_substrate_kernel::reference::InMemorySubstrate;

    use super::*;

    fn record(key: &str, ordinal: u32) -> MessageEnvelope {
        MessageEnvelope {
            key: Some(MessageKey::parse(key).unwrap()),
            headers: BTreeMap::new(),
            payload: format!("record-{ordinal}").into_bytes(),
        }
    }

    fn batch(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn day() -> Duration {
        Duration::from_secs(24 * 60 * 60)
    }

    #[test]
    fn append_read_commit_preserves_order() {
        let substrate = InMemorySubstrate::new();
        let stream =
            EventStream::bind(&substrate, &StreamName::parse("usage").unwrap(), day()).unwrap();
        let reader = stream.reader("rater").unwrap();
        for ordinal in 0..3 {
            stream.append(record("tenant-a", ordinal)).unwrap();
        }
        let mut seen = Vec::new();
        loop {
            let records = reader.read(batch(1)).unwrap();
            let Some(delivery) = records.first() else {
                break;
            };
            seen.push(delivery.position);
            reader.commit(&delivery.ack_token).unwrap();
        }
        let mut sorted = seen.clone();
        sorted.sort_unstable();
        assert_eq!(seen.len(), 3);
        assert_eq!(seen, sorted);
    }

    #[test]
    fn independent_readers_have_independent_cursors() {
        let substrate = InMemorySubstrate::new();
        let stream =
            EventStream::bind(&substrate, &StreamName::parse("usage").unwrap(), day()).unwrap();
        let rater = stream.reader("rater").unwrap();
        let auditor = stream.reader("auditor").unwrap();
        stream.append(record("tenant-a", 1)).unwrap();
        let first = rater.read(batch(16)).unwrap();
        assert_eq!(first.len(), 1);
        rater.commit(&first[0].ack_token).unwrap();
        // The auditor's cursor is untouched by the rater's commit.
        let audit_view = auditor.read(batch(16)).unwrap();
        assert_eq!(audit_view.len(), 1);
    }

    #[test]
    fn seek_replays_committed_records() {
        let substrate = InMemorySubstrate::new();
        let stream =
            EventStream::bind(&substrate, &StreamName::parse("usage").unwrap(), day()).unwrap();
        let reader = stream.reader("rater").unwrap();
        for ordinal in 0..2 {
            stream.append(record("tenant-a", ordinal)).unwrap();
        }
        // Same-key records are head-of-line blocked, so drain one at a
        // time, committing as we go.
        let drain = |reader: &StreamReader<'_, InMemorySubstrate>| {
            let mut count = 0;
            loop {
                let records = reader.read(batch(16)).unwrap();
                if records.is_empty() {
                    break;
                }
                for delivery in records {
                    reader.commit(&delivery.ack_token).unwrap();
                    count += 1;
                }
            }
            count
        };
        assert_eq!(drain(&reader), 2);
        assert!(reader.read(batch(16)).unwrap().is_empty());
        reader.seek(StreamPosition::EARLIEST).unwrap();
        assert_eq!(drain(&reader), 2);
    }
}
