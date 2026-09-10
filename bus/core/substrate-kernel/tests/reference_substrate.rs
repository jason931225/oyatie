use std::num::NonZeroU32;
use std::time::Duration;

use messaging_substrate_kernel::conformance::{self, SubstrateFixture};
use messaging_substrate_kernel::reference::InMemorySubstrate;
use messaging_substrate_kernel::{
    AckToken, Delivery, LossClass, MessageConsumer, MessageEnvelope, MessageId, MessageProducer,
    MessagingAdmin, MessagingError, StreamPosition, SubscriptionName, TopicName, TopicSpec,
};

struct ReferenceFixture {
    loss_class: LossClass,
}

impl SubstrateFixture for ReferenceFixture {
    type Substrate = InMemorySubstrate;

    fn fresh_substrate(&self) -> Self::Substrate {
        InMemorySubstrate::new()
    }

    fn topic_spec(&self) -> TopicSpec {
        TopicSpec {
            loss_class: self.loss_class,
            retention: Some(Duration::from_secs(24 * 60 * 60)),
        }
    }
}

#[test]
fn reference_substrate_is_fully_conformant_for_both_loss_classes() {
    for loss_class in [LossClass::NeverLose, LossClass::Expendable] {
        let fixture = ReferenceFixture { loss_class };
        let violations = conformance::run_all(&fixture);
        assert!(
            violations.is_empty(),
            "reference substrate diverged ({loss_class:?}): {violations:?}"
        );
    }
}

/// A substrate that silently DROPS nacked messages (an at-least-once
/// violation) and lets topic specs mutate — the harness must catch both.
struct LossySubstrate {
    inner: InMemorySubstrate,
}

impl MessagingAdmin for LossySubstrate {
    fn ensure_topic(&self, topic: &TopicName, spec: &TopicSpec) -> Result<(), MessagingError> {
        // Broken: swallows spec conflicts by always ensuring with the
        // FIRST spec it saw (mutation-tolerant).
        match self.inner.ensure_topic(topic, spec) {
            Err(MessagingError::TopicSpecMismatch { .. }) => Ok(()),
            other => other,
        }
    }

    fn ensure_subscription(
        &self,
        topic: &TopicName,
        subscription: &SubscriptionName,
    ) -> Result<(), MessagingError> {
        self.inner.ensure_subscription(topic, subscription)
    }
}

impl MessageProducer for LossySubstrate {
    fn publish(
        &self,
        topic: &TopicName,
        envelope: MessageEnvelope,
    ) -> Result<MessageId, MessagingError> {
        self.inner.publish(topic, envelope)
    }
}

impl MessageConsumer for LossySubstrate {
    fn receive(
        &self,
        topic: &TopicName,
        subscription: &SubscriptionName,
        max: NonZeroU32,
    ) -> Result<Vec<Delivery>, MessagingError> {
        self.inner.receive(topic, subscription, max)
    }

    fn ack(&self, token: &AckToken) -> Result<(), MessagingError> {
        self.inner.ack(token)
    }

    fn nack(&self, token: &AckToken) -> Result<(), MessagingError> {
        // Broken: a nack acks — nacked messages are silently lost.
        self.inner.ack(token)
    }

    fn seek(
        &self,
        topic: &TopicName,
        subscription: &SubscriptionName,
        position: StreamPosition,
    ) -> Result<(), MessagingError> {
        self.inner.seek(topic, subscription, position)
    }
}

struct LossyFixture;

impl SubstrateFixture for LossyFixture {
    type Substrate = LossySubstrate;

    fn fresh_substrate(&self) -> Self::Substrate {
        LossySubstrate {
            inner: InMemorySubstrate::new(),
        }
    }

    fn topic_spec(&self) -> TopicSpec {
        TopicSpec {
            loss_class: LossClass::NeverLose,
            retention: None,
        }
    }
}

#[test]
fn harness_catches_message_loss_and_spec_mutation() {
    let violations = conformance::run_all(&LossyFixture);
    let failed: Vec<&str> = violations.iter().map(|v| v.check).collect();
    assert!(
        failed.contains(&"redelivery_after_nack"),
        "harness missed the nack-drops-message violation: {failed:?}"
    );
    assert!(
        failed.contains(&"topic_spec_immutability"),
        "harness missed the mutable-spec violation: {failed:?}"
    );
}
