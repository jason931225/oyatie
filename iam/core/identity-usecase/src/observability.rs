//! Telemetry value objects for the `identity.token.issue` surface. A runtime
//! adapter at the binary boundary projects them into concrete exporters; this
//! module deliberately pulls in no telemetry crate of its own.

use data_boundary_kernel::{DataClassification, OperationalDataClass, parse_purpose_pascal_label};

use crate::{
    IdentityTokenIssueApiError, IdentityTokenIssueApiRequest, IdentityTokenRotationRequest,
};

/// Telemetry surface name, re-exported here so observability consumers need not
/// reach into the app-boundary API.
pub const SURFACE: &str = crate::IDENTITY_TOKEN_ISSUE_SURFACE;

/// Outcome label for `identity.token.issue` events.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeLabel {
    /// The issuance or rotation completed successfully.
    Success,
    /// The issuance or rotation was rejected.
    Failure,
}

impl OutcomeLabel {
    /// Stable string form for telemetry label values.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

/// One `identity.token.issue` call outcome. Every field is a bounded label so
/// the event can carry no unbounded user input into a metrics dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityTokenIssueEvent {
    pub surface: &'static str, // data_class: INTERNAL_ONLY
    pub outcome: OutcomeLabel, // data_class: INTERNAL_ONLY
    /// `None` on success.
    pub error_code: Option<&'static str>, // data_class: INTERNAL_ONLY
    /// `None` when the request failed validation before a purpose could be
    /// extracted.
    pub purpose: Option<&'static str>, // data_class: INTERNAL_ONLY
    /// Hashed, never the raw tenant identifier.
    pub tenant_id_hash: u64, // data_class: INTERNAL_ONLY
    pub data_class: &'static str, // data_class: AUDIT
}

pub fn identity_token_issue_event_for_success(
    request: &IdentityTokenIssueApiRequest,
) -> IdentityTokenIssueEvent {
    IdentityTokenIssueEvent {
        surface: SURFACE,
        outcome: OutcomeLabel::Success,
        error_code: None,
        purpose: purpose_label_from_body(&request.body.purpose),
        tenant_id_hash: fnv1a_hash(&request.boundary.tenant_id),
        data_class: AUDIT_LABEL,
    }
}

pub fn identity_token_issue_event_for_error(
    request: &IdentityTokenIssueApiRequest,
    error: &IdentityTokenIssueApiError,
) -> IdentityTokenIssueEvent {
    IdentityTokenIssueEvent {
        surface: SURFACE,
        outcome: OutcomeLabel::Failure,
        error_code: Some(error.code().as_str()),
        purpose: purpose_label_from_body(&request.body.purpose),
        tenant_id_hash: fnv1a_hash(&request.boundary.tenant_id),
        data_class: AUDIT_LABEL,
    }
}

pub fn identity_token_rotate_event_for_success(
    request: &IdentityTokenRotationRequest,
) -> IdentityTokenIssueEvent {
    identity_token_issue_event_for_success(&request.replacement)
}

pub fn identity_token_rotate_event_for_error(
    request: &IdentityTokenRotationRequest,
    error: &IdentityTokenIssueApiError,
) -> IdentityTokenIssueEvent {
    identity_token_issue_event_for_error(&request.replacement, error)
}

const AUDIT_LABEL: &str = DataClassification::Operational(OperationalDataClass::Audit).label();

/// `None` for an unrecognised label, which is what keeps `purpose` a validated
/// static string rather than arbitrary user input.
fn purpose_label_from_body(purpose: &str) -> Option<&'static str> {
    parse_purpose_pascal_label(purpose).map(|p| p.pascal_label())
}

fn fnv1a_hash(value: &str) -> u64 {
    let mut state: u64 = 0xcbf29ce484222325;
    for byte in value.bytes() {
        state ^= u64::from(byte);
        state = state.wrapping_mul(0x100000001b3);
    }
    state
}

#[cfg(test)]
mod tests {
    use super::{AUDIT_LABEL, OutcomeLabel, SURFACE, fnv1a_hash, purpose_label_from_body};

    #[test]
    fn surface_constant_value_is_stable() {
        assert_eq!(SURFACE, "identity.token.issue");
    }

    #[test]
    fn audit_label_constant_matches_operational_data_class() {
        assert_eq!(AUDIT_LABEL, "AUDIT");
    }

    #[test]
    fn outcome_label_strings_are_stable() {
        assert_eq!(OutcomeLabel::Success.as_str(), "success");
        assert_eq!(OutcomeLabel::Failure.as_str(), "failure");
    }

    #[test]
    fn fnv1a_hash_is_deterministic() {
        assert_eq!(fnv1a_hash("ten_alpha"), fnv1a_hash("ten_alpha"));
        assert_ne!(fnv1a_hash("ten_alpha"), fnv1a_hash("ten_beta"));
    }

    #[test]
    fn purpose_label_from_body_returns_none_for_unknown_labels() {
        assert_eq!(purpose_label_from_body("not-a-purpose"), None);
        assert_eq!(purpose_label_from_body(""), None);
        assert_eq!(purpose_label_from_body("Banana"), None);
    }

    #[test]
    fn purpose_label_from_body_returns_static_pascal_label_for_known_purposes() {
        assert_eq!(
            purpose_label_from_body("CapabilityInvocation"),
            Some("CapabilityInvocation")
        );
        assert_eq!(purpose_label_from_body("CoreService"), Some("CoreService"));
    }
}
