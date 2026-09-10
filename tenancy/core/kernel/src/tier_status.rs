//! Tenant tier and lifecycle-status value objects. Pure kernel primitives: no I/O.

use std::fmt;
use std::str::FromStr;

/// Commercial tier a tenant is enrolled in; determines feature access and limits.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TenantTier {
    Starter,
    Pro,
    Enterprise,
}

impl TenantTier {
    /// Canonical wire label (lowercase, stable across renames).
    pub const fn label(self) -> &'static str {
        match self {
            Self::Starter => "starter",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    /// Parse from a canonical wire label. Returns `None` for unknown labels.
    pub fn parse_label(value: &str) -> Option<Self> {
        match value {
            "starter" => Some(Self::Starter),
            "pro" => Some(Self::Pro),
            "enterprise" => Some(Self::Enterprise),
            _ => None,
        }
    }
}

impl fmt::Display for TenantTier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

impl FromStr for TenantTier {
    type Err = TenantTierParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_label(value).ok_or(TenantTierParseError(value.to_string()))
    }
}

/// Returned when `TenantTier::from_str` receives an unrecognised label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantTierParseError(pub String); // data_class: INTERNAL_ONLY

impl fmt::Display for TenantTierParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown tenant tier label: {:?}", self.0)
    }
}

impl std::error::Error for TenantTierParseError {}

/// Lifecycle FSM state of a tenant record.
///
/// `Terminated` is terminal: re-activation is never permitted.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TenantStatus {
    Active,
    Suspended,
    Terminated,
}

impl TenantStatus {
    /// Canonical wire label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Terminated => "terminated",
        }
    }

    /// Parse from a canonical wire label. Returns `None` for unknown labels.
    pub fn parse_label(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            "terminated" => Some(Self::Terminated),
            _ => None,
        }
    }

    /// Returns `true` only when the tenant is `Active` and may accept new requests.
    pub fn is_operable(self) -> bool {
        matches!(self, Self::Active)
    }
}

impl fmt::Display for TenantStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

impl FromStr for TenantStatus {
    type Err = TenantStatusParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse_label(value).ok_or(TenantStatusParseError(value.to_string()))
    }
}

/// Returned when `TenantStatus::from_str` receives an unrecognised label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantStatusParseError(pub String); // data_class: INTERNAL_ONLY

impl fmt::Display for TenantStatusParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown tenant status label: {:?}", self.0)
    }
}

impl std::error::Error for TenantStatusParseError {}

/// Typed reason attached to a `TenantStatus::Suspended` transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuspensionReason {
    /// Subscription payment is overdue beyond the grace period.
    PaymentOverdue,
    /// Tenant has violated the Terms of Service.
    TosViolation,
    /// An operator administrator manually requested suspension.
    AdminRequest,
    /// Any other reason; carries a free-form audit message.
    Other(String), // data_class: INTERNAL_ONLY
}

impl SuspensionReason {
    /// Returns the stable discriminant label used in audit events and DB rows.
    pub fn label(&self) -> &str {
        match self {
            Self::PaymentOverdue => "payment_overdue",
            Self::TosViolation => "tos_violation",
            Self::AdminRequest => "admin_request",
            Self::Other(_) => "other",
        }
    }

    /// Returns the free-form detail string for `Other`, if any.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Other(detail) => Some(detail.as_str()),
            _ => None,
        }
    }
}

impl fmt::Display for SuspensionReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Other(detail) => write!(formatter, "other({})", detail),
            _ => formatter.write_str(self.label()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_tier_labels_are_stable_and_round_trip() {
        for (tier, expected) in [
            (TenantTier::Starter, "starter"),
            (TenantTier::Pro, "pro"),
            (TenantTier::Enterprise, "enterprise"),
        ] {
            assert_eq!(tier.label(), expected);
            assert_eq!(TenantTier::parse_label(expected), Some(tier));
            assert_eq!(tier.to_string(), expected);
            let parsed: TenantTier = expected.parse().expect("valid label must parse");
            assert_eq!(parsed, tier);
        }
    }

    #[test]
    fn tenant_tier_parse_label_returns_none_for_unknown() {
        assert_eq!(TenantTier::parse_label("free"), None);
        assert_eq!(TenantTier::parse_label(""), None);
        assert_eq!(TenantTier::parse_label("STARTER"), None); // case-sensitive
    }

    #[test]
    fn tenant_tier_from_str_returns_parse_error_for_unknown() {
        let err = "unknown_tier".parse::<TenantTier>().unwrap_err();
        assert_eq!(err.0, "unknown_tier");
        assert!(err.to_string().contains("unknown_tier"));
    }

    #[test]
    fn tenant_status_labels_are_stable_and_round_trip() {
        for (status, expected) in [
            (TenantStatus::Active, "active"),
            (TenantStatus::Suspended, "suspended"),
            (TenantStatus::Terminated, "terminated"),
        ] {
            assert_eq!(status.label(), expected);
            assert_eq!(TenantStatus::parse_label(expected), Some(status));
            assert_eq!(status.to_string(), expected);
            let parsed: TenantStatus = expected.parse().expect("valid label must parse");
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn tenant_status_is_operable_only_for_active() {
        assert!(TenantStatus::Active.is_operable());
        assert!(!TenantStatus::Suspended.is_operable());
        assert!(!TenantStatus::Terminated.is_operable());
    }

    #[test]
    fn suspended_tenant_must_not_pass_operability_guard_fr04() {
        assert!(
            !TenantStatus::Suspended.is_operable(),
            "FR-04 violation: Suspended must block new requests; is_operable() must return false"
        );
    }

    #[test]
    fn tenant_status_parse_label_returns_none_for_unknown() {
        assert_eq!(TenantStatus::parse_label("deleted"), None);
        assert_eq!(TenantStatus::parse_label(""), None);
        assert_eq!(TenantStatus::parse_label("ACTIVE"), None);
    }

    #[test]
    fn tenant_status_from_str_returns_parse_error_for_unknown() {
        let err = "unknown_status".parse::<TenantStatus>().unwrap_err();
        assert_eq!(err.0, "unknown_status");
        assert!(err.to_string().contains("unknown_status"));
    }

    #[test]
    fn suspension_reason_labels_are_stable() {
        assert_eq!(SuspensionReason::PaymentOverdue.label(), "payment_overdue");
        assert_eq!(SuspensionReason::TosViolation.label(), "tos_violation");
        assert_eq!(SuspensionReason::AdminRequest.label(), "admin_request");
        assert_eq!(
            SuspensionReason::Other("fraud".to_string()).label(),
            "other"
        );
    }

    #[test]
    fn suspension_reason_other_carries_free_form_detail() {
        let reason = SuspensionReason::Other("suspected fraud".to_string());
        assert_eq!(reason.detail(), Some("suspected fraud"));
        assert_eq!(SuspensionReason::PaymentOverdue.detail(), None);
    }

    #[test]
    fn suspension_reason_display_includes_detail_for_other() {
        let reason = SuspensionReason::Other("abuse".to_string());
        assert_eq!(reason.to_string(), "other(abuse)");
        assert_eq!(SuspensionReason::AdminRequest.to_string(), "admin_request");
    }

    #[test]
    fn suspension_reason_equality_includes_other_detail() {
        assert_eq!(
            SuspensionReason::Other("a".to_string()),
            SuspensionReason::Other("a".to_string())
        );
        assert_ne!(
            SuspensionReason::Other("a".to_string()),
            SuspensionReason::Other("b".to_string())
        );
        assert_ne!(
            SuspensionReason::Other("a".to_string()),
            SuspensionReason::AdminRequest
        );
    }
}
