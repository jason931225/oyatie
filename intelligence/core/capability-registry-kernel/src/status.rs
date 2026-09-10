//! Publication lifecycle discriminant for a registered capability.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::fmt;

/// The closed set of states a published capability endpoint can hold.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum CapabilityStatus {
    Active,
    Deprecated,
    Disabled,
}

impl CapabilityStatus {
    #[inline]
    pub fn is_discoverable(self) -> bool {
        matches!(self, Self::Active)
    }

    #[inline]
    pub fn is_invocable(self) -> bool {
        matches!(self, Self::Active | Self::Deprecated)
    }

    /// The stored representation; a rename is a schema migration.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Disabled => "disabled",
        }
    }

    /// A same-state transition is an error, not a no-op: the caller asked for
    /// a change that did not happen.
    pub fn try_transition_to(
        self,
        next: CapabilityStatus,
    ) -> Result<CapabilityStatus, CapabilityStatusTransitionError> {
        use CapabilityStatus::{Active, Deprecated, Disabled};
        match (self, next) {
            (Active, Deprecated)
            | (Active, Disabled)
            | (Deprecated, Active)
            | (Deprecated, Disabled)
            | (Disabled, Active) => Ok(next),
            _ => Err(CapabilityStatusTransitionError {
                from: self,
                to: next,
            }),
        }
    }
}

impl fmt::Display for CapabilityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned when a requested lifecycle transition is illegal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityStatusTransitionError {
    pub from: CapabilityStatus,
    pub to: CapabilityStatus,
}

impl fmt::Display for CapabilityStatusTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "illegal capability status transition: {} -> {}",
            self.from.as_str(),
            self.to.as_str(),
        )
    }
}

impl std::error::Error for CapabilityStatusTransitionError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityStatusParseError(pub String);

impl fmt::Display for CapabilityStatusParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown capability status: {}", self.0)
    }
}

impl std::error::Error for CapabilityStatusParseError {}

impl TryFrom<&str> for CapabilityStatus {
    type Error = CapabilityStatusParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "active" => Ok(Self::Active),
            "deprecated" => Ok(Self::Deprecated),
            "disabled" => Ok(Self::Disabled),
            other => Err(CapabilityStatusParseError(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_is_discoverable_and_invocable() {
        assert!(CapabilityStatus::Active.is_discoverable());
        assert!(CapabilityStatus::Active.is_invocable());
    }

    #[test]
    fn deprecated_not_discoverable_but_invocable() {
        assert!(!CapabilityStatus::Deprecated.is_discoverable());
        assert!(CapabilityStatus::Deprecated.is_invocable());
    }

    #[test]
    fn disabled_neither_discoverable_nor_invocable() {
        assert!(!CapabilityStatus::Disabled.is_discoverable());
        assert!(!CapabilityStatus::Disabled.is_invocable());
    }

    #[test]
    fn round_trip_parse() {
        for status in [
            CapabilityStatus::Active,
            CapabilityStatus::Deprecated,
            CapabilityStatus::Disabled,
        ] {
            let parsed = CapabilityStatus::try_from(status.as_str()).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn rejects_unknown_label() {
        assert!(CapabilityStatus::try_from("archived").is_err());
        assert!(CapabilityStatus::try_from("").is_err());
    }

    #[test]
    fn display_matches_as_str() {
        for status in [
            CapabilityStatus::Active,
            CapabilityStatus::Deprecated,
            CapabilityStatus::Disabled,
        ] {
            assert_eq!(format!("{status}"), status.as_str());
        }
    }

    #[test]
    fn ordering_active_lt_deprecated_lt_disabled() {
        assert!(CapabilityStatus::Active < CapabilityStatus::Deprecated);
        assert!(CapabilityStatus::Deprecated < CapabilityStatus::Disabled);
    }

    #[test]
    fn transition_active_to_deprecated() {
        assert_eq!(
            CapabilityStatus::Active.try_transition_to(CapabilityStatus::Deprecated),
            Ok(CapabilityStatus::Deprecated),
        );
    }

    #[test]
    fn transition_deprecated_to_active() {
        assert_eq!(
            CapabilityStatus::Deprecated.try_transition_to(CapabilityStatus::Active),
            Ok(CapabilityStatus::Active),
        );
    }

    #[test]
    fn transition_active_to_disabled() {
        assert_eq!(
            CapabilityStatus::Active.try_transition_to(CapabilityStatus::Disabled),
            Ok(CapabilityStatus::Disabled),
        );
    }

    #[test]
    fn transition_deprecated_to_disabled() {
        assert_eq!(
            CapabilityStatus::Deprecated.try_transition_to(CapabilityStatus::Disabled),
            Ok(CapabilityStatus::Disabled),
        );
    }

    #[test]
    fn transition_disabled_to_active() {
        assert_eq!(
            CapabilityStatus::Disabled.try_transition_to(CapabilityStatus::Active),
            Ok(CapabilityStatus::Active),
        );
    }

    #[test]
    fn transition_disabled_to_deprecated_is_illegal() {
        let err = CapabilityStatus::Disabled
            .try_transition_to(CapabilityStatus::Deprecated)
            .unwrap_err();
        assert_eq!(err.from, CapabilityStatus::Disabled);
        assert_eq!(err.to, CapabilityStatus::Deprecated);
    }

    #[test]
    fn transition_same_state_active_is_illegal() {
        assert!(
            CapabilityStatus::Active
                .try_transition_to(CapabilityStatus::Active)
                .is_err()
        );
    }

    #[test]
    fn transition_same_state_deprecated_is_illegal() {
        assert!(
            CapabilityStatus::Deprecated
                .try_transition_to(CapabilityStatus::Deprecated)
                .is_err()
        );
    }

    #[test]
    fn transition_same_state_disabled_is_illegal() {
        assert!(
            CapabilityStatus::Disabled
                .try_transition_to(CapabilityStatus::Disabled)
                .is_err()
        );
    }

    #[test]
    fn transition_error_display_is_human_readable() {
        let msg = format!(
            "{}",
            CapabilityStatus::Disabled
                .try_transition_to(CapabilityStatus::Deprecated)
                .unwrap_err()
        );
        assert!(msg.contains("disabled"), "message: {msg}");
        assert!(msg.contains("deprecated"), "message: {msg}");
    }
}
