//! # messaging-queue-boundary-kernel
//!
//! The owned WORK-QUEUE surface — first of the three single-concern
//! messaging surfaces (ADR-0536 D-13 queue/stream/bus trichotomy;
//! ADR-0132 no-grouping) composed over the ONE substrate port
//! (`messaging-substrate-kernel`). Services enqueue and work jobs
//! through THIS boundary; they never speak a broker protocol.
//!
//! Queue semantics (precedent: AWS SQS; Meta FOQS):
//! - competing consumers over one subscription — each job is worked by
//!   at most one in-flight worker;
//! - at-least-once with explicit complete/fail settlement;
//! - poison-message containment: a job failed past its
//!   [`DeadLetterPolicy`] budget moves to the queue's dead-letter topic
//!   instead of redelivering forever (SQS redrive policy).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;
use std::num::NonZeroU32;

use messaging_substrate_kernel::{
    AckToken, Delivery, LossClass, MessageConsumer, MessageEnvelope, MessageId, MessageProducer,
    MessagingAdmin, MessagingError, MessagingSubstrate, SubscriptionName, TopicName, TopicSpec,
};

/// Queue-surface errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    /// Queue name violates the canonical slug shape.
    InvalidQueueName { value: String },
    /// The underlying substrate raised a contract error.
    Substrate(MessagingError),
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQueueName { value } => write!(f, "invalid queue name: {value:?}"),
            Self::Substrate(error) => write!(f, "substrate: {error}"),
        }
    }
}

impl std::error::Error for QueueError {}

impl From<MessagingError> for QueueError {
    fn from(error: MessagingError) -> Self {
        Self::Substrate(error)
    }
}

/// Canonical queue name (a topic-safe slug).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct QueueName(String);

impl QueueName {
    /// Parses and validates a queue name.
    ///
    /// # Errors
    /// Returns [`QueueError::InvalidQueueName`] when the derived topic
    /// names would not be canonical slugs.
    pub fn parse(value: &str) -> Result<Self, QueueError> {
        // `queue.` is a WIRE identifier, not a crate name: a rename here is a
        // behavior change and must not ride along inside a relocation.
        TopicName::parse(&format!("queue.{value}")).map_err(|_| QueueError::InvalidQueueName {
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

/// Poison-message containment budget (SQS redrive doctrine).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeadLetterPolicy {
    /// A delivery observed with `delivery_count` ABOVE this budget is
    /// moved to the dead-letter topic instead of being handed to workers.
    pub max_delivery_count: NonZeroU32,
}

/// The owned work-queue surface over the ONE messaging substrate.
pub struct WorkQueue<'a, S: MessagingSubstrate> {
    substrate: &'a S,
    topic: TopicName,
    dead_letter_topic: TopicName,
    subscription: SubscriptionName,
    dead_letter_subscription: SubscriptionName,
    policy: DeadLetterPolicy,
}

impl<'a, S: MessagingSubstrate> WorkQueue<'a, S> {
    /// Binds (and idempotently provisions) the queue on the substrate.
    ///
    /// # Errors
    /// Propagates substrate provisioning failures.
    pub fn bind(
        substrate: &'a S,
        name: &QueueName,
        loss_class: LossClass,
        policy: DeadLetterPolicy,
    ) -> Result<Self, QueueError> {
        let topic = TopicName::parse(&format!("queue.{}", name.as_str()))?;
        let dead_letter_topic = TopicName::parse(&format!("queue.{}.dead-letter", name.as_str()))?;
        let subscription = SubscriptionName::parse("workers")?;
        let dead_letter_subscription = SubscriptionName::parse("dead-letter-review")?;
        let spec = TopicSpec {
            loss_class,
            retention: None,
        };
        substrate.ensure_topic(&topic, &spec)?;
        substrate.ensure_topic(&dead_letter_topic, &spec)?;
        substrate.ensure_subscription(&topic, &subscription)?;
        substrate.ensure_subscription(&dead_letter_topic, &dead_letter_subscription)?;
        Ok(Self {
            substrate,
            topic,
            dead_letter_topic,
            subscription,
            dead_letter_subscription,
            policy,
        })
    }

    /// Enqueues one job.
    ///
    /// # Errors
    /// Propagates substrate publish failures.
    pub fn enqueue(&self, envelope: MessageEnvelope) -> Result<MessageId, QueueError> {
        Ok(self.substrate.publish(&self.topic, envelope)?)
    }

    /// Pulls up to `max` workable jobs. Deliveries past the dead-letter
    /// budget are moved to the dead-letter topic and never returned.
    ///
    /// # Errors
    /// Propagates substrate receive/publish/settle failures.
    pub fn dequeue(&self, max: NonZeroU32) -> Result<Vec<Delivery>, QueueError> {
        let deliveries = self
            .substrate
            .receive(&self.topic, &self.subscription, max)?;
        let mut workable = Vec::with_capacity(deliveries.len());
        for delivery in deliveries {
            if delivery.delivery_count > self.policy.max_delivery_count.get() {
                // Containment first, settle second: if the dead-letter
                // publish fails we leave the delivery in flight so the
                // substrate redelivers it (at-least-once, never silent loss).
                self.substrate
                    .publish(&self.dead_letter_topic, delivery.envelope.clone())?;
                self.substrate.ack(&delivery.ack_token)?;
            } else {
                workable.push(delivery);
            }
        }
        Ok(workable)
    }

    /// Settles a job as done.
    ///
    /// # Errors
    /// Propagates substrate settle failures.
    pub fn complete(&self, token: &AckToken) -> Result<(), QueueError> {
        Ok(self.substrate.ack(token)?)
    }

    /// Settles a job as failed; it returns to the backlog (or to the
    /// dead-letter topic once past budget).
    ///
    /// # Errors
    /// Propagates substrate settle failures.
    pub fn fail(&self, token: &AckToken) -> Result<(), QueueError> {
        Ok(self.substrate.nack(token)?)
    }

    /// Pulls up to `max` dead-lettered jobs for operator review/redrive.
    ///
    /// # Errors
    /// Propagates substrate receive failures.
    pub fn dequeue_dead_letters(&self, max: NonZeroU32) -> Result<Vec<Delivery>, QueueError> {
        Ok(self
            .substrate
            .receive(&self.dead_letter_topic, &self.dead_letter_subscription, max)?)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use messaging_substrate_kernel::reference::InMemorySubstrate;

    use super::*;

    fn job(ordinal: u32) -> MessageEnvelope {
        MessageEnvelope {
            key: None,
            headers: BTreeMap::new(),
            payload: format!("job-{ordinal}").into_bytes(),
        }
    }

    fn batch(value: u32) -> NonZeroU32 {
        NonZeroU32::new(value).unwrap()
    }

    fn queue(substrate: &InMemorySubstrate) -> WorkQueue<'_, InMemorySubstrate> {
        WorkQueue::bind(
            substrate,
            &QueueName::parse("metering-ingest").unwrap(),
            LossClass::NeverLose,
            DeadLetterPolicy {
                max_delivery_count: batch(2),
            },
        )
        .unwrap()
    }

    #[test]
    fn enqueue_dequeue_complete_roundtrip() {
        let substrate = InMemorySubstrate::new();
        let queue = queue(&substrate);
        queue.enqueue(job(1)).unwrap();
        let jobs = queue.dequeue(batch(16)).unwrap();
        assert_eq!(jobs.len(), 1);
        queue.complete(&jobs[0].ack_token).unwrap();
        assert!(queue.dequeue(batch(16)).unwrap().is_empty());
    }

    #[test]
    fn competing_consumers_never_share_an_in_flight_job() {
        let substrate = InMemorySubstrate::new();
        let queue = queue(&substrate);
        queue.enqueue(job(1)).unwrap();
        assert_eq!(queue.dequeue(batch(16)).unwrap().len(), 1);
        // Second worker polls while the job is in flight: nothing to work.
        assert!(queue.dequeue(batch(16)).unwrap().is_empty());
    }

    #[test]
    fn poison_job_moves_to_dead_letter_after_budget() {
        let substrate = InMemorySubstrate::new();
        let queue = queue(&substrate);
        queue.enqueue(job(7)).unwrap();
        // Budget is 2 deliveries; fail both.
        for _ in 0..2 {
            let jobs = queue.dequeue(batch(16)).unwrap();
            assert_eq!(jobs.len(), 1);
            queue.fail(&jobs[0].ack_token).unwrap();
        }
        // Third delivery exceeds the budget: contained, not workable.
        assert!(queue.dequeue(batch(16)).unwrap().is_empty());
        let dead = queue.dequeue_dead_letters(batch(16)).unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].envelope.payload, b"job-7");
        // And the main queue stays empty afterwards.
        assert!(queue.dequeue(batch(16)).unwrap().is_empty());
    }

    #[test]
    fn queue_names_are_validated() {
        assert!(QueueName::parse("UPPER").is_err());
        assert!(QueueName::parse("").is_err());
        assert!(QueueName::parse("metering-ingest").is_ok());
    }
}
