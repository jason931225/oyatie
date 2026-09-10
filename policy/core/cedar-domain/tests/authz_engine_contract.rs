#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

mod common;

use common::*;
use policy_cedar_domain::authz_engine::{
    AuthzDecision, AuthzRequest, EvalLogFilter, PrincipalType,
};
use policy_cedar_domain::*;
use serde_json::json;

/// `PolicyEffect`, `AuthzDecision`, `AuthzRequest`, and `EvalLogFilter` must
/// round-trip through JSON without compile or runtime errors.
#[test]
fn authz_engine_types_serialize_and_deserialize_via_serde_json() {
    let request = AuthzRequest {
        tenant_id: TEST_TENANT_ID.to_string(),
        principal_type: PrincipalType::User,
        principal_id: Some("usr_001".to_string()),
        action: "Read".to_string(),
        resource_type: "Object".to_string(),
        resource_id: Some("obj_abc".to_string()),
        context: BTreeMap::new(),
    };
    let json = serde_json::to_string(&request).expect("AuthzRequest serializes");
    let roundtrip: AuthzRequest = serde_json::from_str(&json).expect("AuthzRequest deserializes");
    assert_eq!(request, roundtrip);

    let decision = AuthzDecision::allow(vec!["pol_allow_read".to_string()]);
    let json = serde_json::to_string(&decision).expect("AuthzDecision serializes");
    let roundtrip: AuthzDecision = serde_json::from_str(&json).expect("AuthzDecision deserializes");
    assert_eq!(decision, roundtrip);
    assert!(roundtrip.is_allowed());

    let filter = EvalLogFilter::default();
    let json = serde_json::to_string(&filter).expect("EvalLogFilter serializes");
    let roundtrip: EvalLogFilter = serde_json::from_str(&json).expect("EvalLogFilter deserializes");
    assert_eq!(filter, roundtrip);
    assert_eq!(roundtrip.limit, 100);
}

/// `PrincipalType` wire values must match Cedar PascalCase entity names, not
/// snake_case. A payload carrying `"User"` must deserialize correctly, and
/// the serialized form must equal `"User"` (not `"user"`).
#[test]
fn principal_type_serde_uses_cedar_pascalcase_wire_names() {
    // Serialize and check wire value is PascalCase.
    let serialized =
        serde_json::to_string(&PrincipalType::Employee).expect("PrincipalType serializes");
    assert_eq!(
        serialized, "\"Employee\"",
        "wire value must be Cedar PascalCase, got {serialized}"
    );

    // Deserialize Cedar-style value (as a downstream client would send it).
    let from_cedar: PrincipalType =
        serde_json::from_str("\"Workflow\"").expect("Cedar wire value deserializes");
    assert_eq!(from_cedar, PrincipalType::Workflow);

    // snake_case must NOT deserialize (would indicate wire-format mismatch).
    let snake_result = serde_json::from_str::<PrincipalType>("\"workflow\"");
    assert!(
        snake_result.is_err(),
        "snake_case wire value must be rejected to prevent silent Cedar mismatch"
    );
}

/// `PolicyEffect` must serialize to UPPERCASE values so it is unambiguous on
/// the wire and does not collide with Cedar reserved lowercase tokens.
#[test]
fn policy_effect_serde_uses_uppercase_wire_values() {
    let allow_json =
        serde_json::to_string(&PolicyEffect::Allow).expect("PolicyEffect::Allow serializes");
    assert_eq!(allow_json, "\"ALLOW\"");

    let deny_json =
        serde_json::to_string(&PolicyEffect::Deny).expect("PolicyEffect::Deny serializes");
    assert_eq!(deny_json, "\"DENY\"");

    let roundtrip: PolicyEffect = serde_json::from_str("\"ALLOW\"").expect("ALLOW deserializes");
    assert_eq!(roundtrip, PolicyEffect::Allow);
}

/// Synthetic violation: serializing a decision must not mutate state that
/// could corrupt an append-only ledger entry if accidentally re-serialized.
#[test]
fn authz_decision_default_deny_is_immutable_across_serialization_roundtrip() {
    let d1 = AuthzDecision::default_deny();
    let json = serde_json::to_string(&d1).expect("default_deny serializes");
    let d2: AuthzDecision = serde_json::from_str(&json).expect("default_deny deserializes");
    assert_eq!(d1, d2);
    assert!(!d2.is_allowed());
    assert!(d2.determining_policies.is_empty());
}

/// `EvalLogFilter::default()` must yield `limit = 100`, not `0`.
#[test]
fn eval_log_filter_default_limit_is_100() {
    assert_eq!(EvalLogFilter::default().limit, 100);

    // Deserializing a payload that omits `limit` must also default to 100.
    let from_partial: EvalLogFilter =
        serde_json::from_str(r#"{"principal_id":null,"effect":null,"resource_type":null}"#)
            .expect("partial EvalLogFilter deserializes");
    assert_eq!(from_partial.limit, 100);
}
