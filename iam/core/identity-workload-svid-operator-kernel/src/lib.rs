//! Reconcile kernel for delivering the PDP's platform SVID as a Kubernetes
//! Secret. It decides only: minting the SVID and projecting the Secret belong to
//! the adapter, and driving the loop belongs to the facade app.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// An injected wall-clock source, so reconcile stays a deterministic transform
/// of its arguments.
pub trait Clock {
    /// The current time as unix epoch seconds.
    fn now_epoch_seconds(&self) -> u64;
}

/// The desired SVID-delivery state for the PDP.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DesiredState {
    /// The SPIFFE URI the issued leaf must carry as its *single* URI SAN.
    pub spiffe_id: String, // data_class: PUBLIC
    /// Requested certificate lifetime in seconds.
    pub ttl_secs: u64, // data_class: PUBLIC
    /// Remaining leaf lifetime at or below which a rotation is forced, so a
    /// caller never races the PDP's fail-closed boot against an expired SVID.
    pub rotation_window_secs: u64, // data_class: PUBLIC
    /// The Kubernetes Secret the `kubernetes.io/tls` material is written to;
    /// the PDP's mount contract fixes it to `cloud-iam-pdp-svid`.
    pub secret_name: String, // data_class: PUBLIC
    /// The namespace the Secret is projected into.
    pub secret_namespace: String, // data_class: PUBLIC
}

/// The observed state of the delivered Secret as read from the cluster.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedState {
    /// `None` is the cold-start case: nothing has ever been produced.
    pub secret: Option<ObservedSvidSecret>, // data_class: PUBLIC
}

impl ObservedState {
    /// An observation in which no Secret is present.
    #[must_use]
    pub fn absent() -> Self {
        Self { secret: None }
    }

    /// An observation carrying a present Secret.
    #[must_use]
    pub fn present(leaf_not_after_epoch_seconds: u64) -> Self {
        Self {
            secret: Some(ObservedSvidSecret {
                leaf_not_after_epoch_seconds,
            }),
        }
    }
}

/// The part of a delivered SVID Secret the decision consults; the cert material
/// itself is the adapter's concern.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedSvidSecret {
    /// The `notAfter` of the delivered leaf, as unix epoch seconds.
    pub leaf_not_after_epoch_seconds: u64, // data_class: PUBLIC
}

/// The single action the adapter must take to converge observed onto desired.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// Mint a fresh SVID and create the Secret.
    Issue {
        desired: DesiredState,           // data_class: PUBLIC
        requested_at_epoch_seconds: u64, // data_class: PUBLIC
    },
    /// Mint a fresh SVID and update the Secret in place.
    Rotate {
        desired: DesiredState,                      // data_class: PUBLIC
        observed_leaf_not_after_epoch_seconds: u64, // data_class: PUBLIC
        requested_at_epoch_seconds: u64,            // data_class: PUBLIC
    },
    /// The delivered leaf is still outside its rotation window.
    Noop, // data_class: PUBLIC
}

/// Decide the converging action for `observed` against `desired`, as of `clock`.
pub fn reconcile<C: Clock>(observed: &ObservedState, desired: &DesiredState, clock: &C) -> Action {
    let now = clock.now_epoch_seconds();
    match &observed.secret {
        None => Action::Issue {
            desired: desired.clone(),
            requested_at_epoch_seconds: now,
        },
        Some(secret) => {
            let remaining = secret.leaf_not_after_epoch_seconds.saturating_sub(now);
            if remaining <= desired.rotation_window_secs {
                Action::Rotate {
                    desired: desired.clone(),
                    observed_leaf_not_after_epoch_seconds: secret.leaf_not_after_epoch_seconds,
                    requested_at_epoch_seconds: now,
                }
            } else {
                Action::Noop
            }
        }
    }
}
