use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::Mutex;

use crate::{
    AckToken, Delivery, MessageConsumer, MessageEnvelope, MessageId, MessageProducer,
    MessagingAdmin, MessagingError, StreamPosition, SubscriptionName, TopicName, TopicSpec,
};

#[derive(Clone, Debug)]
struct StoredMessage {
    id: MessageId,
    position: StreamPosition,
    envelope: MessageEnvelope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettleState {
    Available,
    InFlight,
    Acked,
}

#[derive(Clone, Debug)]
struct SubscriptionState {
    settle: BTreeMap<u64, SettleState>,
    delivery_count: BTreeMap<u64, u32>,
    floor: u64,
}

#[derive(Clone, Debug)]
struct TopicState {
    spec: TopicSpec,
    log: Vec<StoredMessage>,
    subscriptions: BTreeMap<SubscriptionName, SubscriptionState>,
}

#[derive(Clone, Debug)]
struct TokenState {
    topic: TopicName,
    subscription: SubscriptionName,
    position: u64,
    live: bool,
}

#[derive(Debug, Default)]
struct Inner {
    topics: BTreeMap<TopicName, TopicState>,
    tokens: BTreeMap<String, TokenState>,
    token_seq: u64,
}

/// In-memory reference implementation of the ONE messaging substrate.
#[derive(Debug, Default)]
pub struct InMemorySubstrate {
    inner: Mutex<Inner>,
}

impl InMemorySubstrate {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn locked(&self) -> Result<std::sync::MutexGuard<'_, Inner>, MessagingError> {
        self.inner
            .lock()
            .map_err(|_| MessagingError::BackendUnavailable {
                detail: "reference substrate lock poisoned".to_owned(),
            })
    }
}

impl MessagingAdmin for InMemorySubstrate {
    fn ensure_topic(&self, topic: &TopicName, spec: &TopicSpec) -> Result<(), MessagingError> {
        let mut inner = self.locked()?;
        match inner.topics.get(topic) {
            None => {
                inner.topics.insert(
                    topic.clone(),
                    TopicState {
                        spec: spec.clone(),
                        log: Vec::new(),
                        subscriptions: BTreeMap::new(),
                    },
                );
                Ok(())
            }
            Some(existing) if existing.spec == *spec => Ok(()),
            Some(_) => Err(MessagingError::TopicSpecMismatch {
                topic: topic.as_str().to_owned(),
            }),
        }
    }

    fn ensure_subscription(
        &self,
        topic: &TopicName,
        subscription: &SubscriptionName,
    ) -> Result<(), MessagingError> {
        let mut inner = self.locked()?;
        let state = inner
            .topics
            .get_mut(topic)
            .ok_or_else(|| MessagingError::TopicNotFound {
                topic: topic.as_str().to_owned(),
            })?;
        let head = state.log.len() as u64;
        state
            .subscriptions
            .entry(subscription.clone())
            .or_insert_with(|| SubscriptionState {
                settle: BTreeMap::new(),
                delivery_count: BTreeMap::new(),
                floor: head,
            });
        Ok(())
    }
}

impl MessageProducer for InMemorySubstrate {
    fn publish(
        &self,
        topic: &TopicName,
        envelope: MessageEnvelope,
    ) -> Result<MessageId, MessagingError> {
        envelope.validate()?;
        let mut inner = self.locked()?;
        let state = inner
            .topics
            .get_mut(topic)
            .ok_or_else(|| MessagingError::TopicNotFound {
                topic: topic.as_str().to_owned(),
            })?;
        let position = state.log.len() as u64;
        let id = MessageId::new(format!("{}/{position}", topic.as_str()));
        state.log.push(StoredMessage {
            id: id.clone(),
            position: StreamPosition(position),
            envelope,
        });
        for sub in state.subscriptions.values_mut() {
            sub.settle.insert(position, SettleState::Available);
        }
        Ok(id)
    }
}

impl MessageConsumer for InMemorySubstrate {
    fn receive(
        &self,
        topic: &TopicName,
        subscription: &SubscriptionName,
        max: NonZeroU32,
    ) -> Result<Vec<Delivery>, MessagingError> {
        let mut inner = self.locked()?;
        let Inner {
            topics,
            tokens,
            token_seq,
        } = &mut *inner;
        let state = topics
            .get_mut(topic)
            .ok_or_else(|| MessagingError::TopicNotFound {
                topic: topic.as_str().to_owned(),
            })?;
        let log = &state.log;
        let sub = state.subscriptions.get_mut(subscription).ok_or_else(|| {
            MessagingError::SubscriptionNotFound {
                topic: topic.as_str().to_owned(),
                subscription: subscription.as_str().to_owned(),
            }
        })?;

        let mut blocked_keys: Vec<&crate::MessageKey> = Vec::new();
        for (position, settle) in &sub.settle {
            if *settle == SettleState::InFlight
                && let Some(stored) = log.get(usize::try_from(*position).unwrap_or(usize::MAX))
                && let Some(key) = &stored.envelope.key
            {
                blocked_keys.push(key);
            }
        }

        let mut deliveries = Vec::new();
        let candidates: Vec<u64> = sub
            .settle
            .iter()
            .filter(|(_, settle)| **settle == SettleState::Available)
            .map(|(position, _)| *position)
            .collect();
        for position in candidates {
            if deliveries.len() as u32 >= max.get() {
                break;
            }
            let Some(stored) = log.get(usize::try_from(position).unwrap_or(usize::MAX)) else {
                continue;
            };
            if let Some(key) = &stored.envelope.key {
                if blocked_keys.contains(&key) {
                    continue;
                }
                blocked_keys.push(key);
            }
            let count = sub
                .delivery_count
                .get(&position)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            sub.delivery_count.insert(position, count);
            sub.settle.insert(position, SettleState::InFlight);
            *token_seq = token_seq.saturating_add(1);
            let token = AckToken::new(format!("tok-{token_seq}"));
            tokens.insert(
                token.as_str().to_owned(),
                TokenState {
                    topic: topic.clone(),
                    subscription: subscription.clone(),
                    position,
                    live: true,
                },
            );
            deliveries.push(Delivery {
                message_id: stored.id.clone(),
                position: stored.position,
                envelope: stored.envelope.clone(),
                delivery_count: count,
                ack_token: token,
            });
        }
        Ok(deliveries)
    }

    fn ack(&self, token: &AckToken) -> Result<(), MessagingError> {
        self.settle(token, SettleState::Acked)
    }

    fn nack(&self, token: &AckToken) -> Result<(), MessagingError> {
        self.settle(token, SettleState::Available)
    }

    fn seek(
        &self,
        topic: &TopicName,
        subscription: &SubscriptionName,
        position: StreamPosition,
    ) -> Result<(), MessagingError> {
        let mut inner = self.locked()?;
        let Inner { topics, tokens, .. } = &mut *inner;
        let state = topics
            .get_mut(topic)
            .ok_or_else(|| MessagingError::TopicNotFound {
                topic: topic.as_str().to_owned(),
            })?;
        let head = state.log.len() as u64;
        if position.0 > head {
            return Err(MessagingError::InvalidSeekPosition {
                position: position.0,
            });
        }
        let sub = state.subscriptions.get_mut(subscription).ok_or_else(|| {
            MessagingError::SubscriptionNotFound {
                topic: topic.as_str().to_owned(),
                subscription: subscription.as_str().to_owned(),
            }
        })?;
        sub.floor = sub.floor.min(position.0);
        for stored_position in position.0..head {
            sub.settle.insert(stored_position, SettleState::Available);
        }
        for token in tokens.values_mut() {
            if token.topic == *topic && token.subscription == *subscription {
                token.live = false;
            }
        }
        Ok(())
    }
}

impl InMemorySubstrate {
    fn settle(&self, token: &AckToken, outcome: SettleState) -> Result<(), MessagingError> {
        let mut inner = self.locked()?;
        let Inner { topics, tokens, .. } = &mut *inner;
        let state = tokens
            .remove(token.as_str())
            .ok_or(MessagingError::UnknownAckToken)?;
        if !state.live {
            return Err(MessagingError::UnknownAckToken);
        }
        let sub = topics
            .get_mut(&state.topic)
            .and_then(|topic_state| topic_state.subscriptions.get_mut(&state.subscription))
            .ok_or(MessagingError::UnknownAckToken)?;
        sub.settle.insert(state.position, outcome);
        Ok(())
    }
}
