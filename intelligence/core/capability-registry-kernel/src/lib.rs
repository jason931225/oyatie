//! Value types describing a published capability and its autonomy
//! classification.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod registry_view;
pub mod status;

pub use registry_view::{RegistryViews, partition_views};
pub use status::{CapabilityStatus, CapabilityStatusParseError, CapabilityStatusTransitionError};

use std::fmt;

/// Stable id for a capability (e.g. `foundry.account.list`).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CapabilityId(pub String);

impl CapabilityId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The id only; the payload stays in the evidence store.
// data_class: INTERNAL_ONLY
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct EvidenceRef {
    pub evidence_id: String,         // data_class: INTERNAL_ONLY
    pub capability_id: CapabilityId, // data_class: INTERNAL_ONLY
    pub emitted_at_epoch_secs: u64,  // data_class: INTERNAL_ONLY
}

impl EvidenceRef {
    pub fn new(
        evidence_id: impl Into<String>,
        capability_id: CapabilityId,
        emitted_at_epoch_secs: u64,
    ) -> Self {
        Self {
            evidence_id: evidence_id.into(),
            capability_id,
            emitted_at_epoch_secs,
        }
    }
}

/// Autonomy classification for a capability (ADR-0003). Nothing in this
/// kernel enforces the tier; it records what a capability is allowed to be.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum AutonomyTier {
    /// Never mutates state.
    T1Read = 1,
    /// Proposes, never executes.
    T2Suggest = 2,
    /// Acts within a bounded scope, after an explicit grant.
    T3PropAct = 3,
    /// Direct actuation, off unless policy turns it on.
    T4Actuate = 4,
}

impl AutonomyTier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::T1Read => "T1Read",
            Self::T2Suggest => "T2Suggest",
            Self::T3PropAct => "T3PropAct",
            Self::T4Actuate => "T4Actuate",
        }
    }

    pub fn rank(&self) -> u8 {
        *self as u8
    }
}

impl fmt::Display for AutonomyTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutonomyTierError(pub String);

impl fmt::Display for AutonomyTierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown autonomy tier: {}", self.0)
    }
}

impl TryFrom<&str> for AutonomyTier {
    type Error = AutonomyTierError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "T1Read" | "T1" => Ok(Self::T1Read),
            "T2Suggest" | "T2" => Ok(Self::T2Suggest),
            "T3PropAct" | "T3" => Ok(Self::T3PropAct),
            "T4Actuate" | "T4" => Ok(Self::T4Actuate),
            other => Err(AutonomyTierError(other.to_owned())),
        }
    }
}

/// `owner_capability_id` is what composes capabilities: a parent owns a child.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    pub id: CapabilityId,                          // data_class: INTERNAL_ONLY
    pub name: String,                              // data_class: INTERNAL_ONLY
    pub autonomy_tier: AutonomyTier,               // data_class: INTERNAL_ONLY
    pub evidence_emit_required: bool,              // data_class: INTERNAL_ONLY
    pub owner_capability_id: Option<CapabilityId>, // data_class: INTERNAL_ONLY
    pub status: CapabilityStatus,                  // data_class: INTERNAL_ONLY
}

impl Capability {
    pub fn new(
        id: CapabilityId,
        name: impl Into<String>,
        autonomy_tier: AutonomyTier,
        evidence_emit_required: bool,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            autonomy_tier,
            evidence_emit_required,
            owner_capability_id: None,
            status: CapabilityStatus::Active,
        }
    }

    pub fn owned_by(mut self, owner: CapabilityId) -> Self {
        self.owner_capability_id = Some(owner);
        self
    }

    /// A rejected transition leaves `self` untouched.
    pub fn transition_status(
        &mut self,
        next: CapabilityStatus,
    ) -> Result<(), CapabilityStatusTransitionError> {
        self.status.try_transition_to(next)?;
        self.status = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_id_display() {
        let id = CapabilityId::new("foundry.account.list");
        assert_eq!(format!("{id}"), "foundry.account.list");
    }

    #[test]
    fn autonomy_tier_ordering() {
        assert!(AutonomyTier::T1Read < AutonomyTier::T2Suggest);
        assert!(AutonomyTier::T2Suggest < AutonomyTier::T3PropAct);
        assert!(AutonomyTier::T3PropAct < AutonomyTier::T4Actuate);
    }

    #[test]
    fn autonomy_tier_label_round_trip() {
        for t in [
            AutonomyTier::T1Read,
            AutonomyTier::T2Suggest,
            AutonomyTier::T3PropAct,
            AutonomyTier::T4Actuate,
        ] {
            let parsed = AutonomyTier::try_from(t.label()).unwrap();
            assert_eq!(parsed, t);
        }
    }

    #[test]
    fn autonomy_tier_short_alias() {
        assert_eq!(AutonomyTier::try_from("T1").unwrap(), AutonomyTier::T1Read);
        assert_eq!(
            AutonomyTier::try_from("T4").unwrap(),
            AutonomyTier::T4Actuate
        );
    }

    #[test]
    fn autonomy_tier_rejects_unknown() {
        assert!(AutonomyTier::try_from("T5Doom").is_err());
    }

    #[test]
    fn capability_construction() {
        let c = Capability::new(
            CapabilityId::new("foundry.audit.tail"),
            "Tail audit chain",
            AutonomyTier::T1Read,
            true,
        );
        assert_eq!(c.autonomy_tier, AutonomyTier::T1Read);
        assert!(c.evidence_emit_required);
        assert!(c.owner_capability_id.is_none());
    }

    #[test]
    fn capability_with_owner() {
        let owner = CapabilityId::new("foundry.account.list");
        let c = Capability::new(
            CapabilityId::new("foundry.account.health"),
            "Account health",
            AutonomyTier::T1Read,
            true,
        )
        .owned_by(owner.clone());
        assert_eq!(c.owner_capability_id, Some(owner));
    }

    #[test]
    fn evidence_ref_construction() {
        let er = EvidenceRef::new(
            "ev-1",
            CapabilityId::new("foundry.route.explain"),
            1_700_000_000,
        );
        assert_eq!(er.evidence_id, "ev-1");
        assert_eq!(er.emitted_at_epoch_secs, 1_700_000_000);
    }
}
