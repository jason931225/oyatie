#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

mod common;

use common::*;
use policy_cedar_domain::authz_engine::{
    AuthzDecision, AuthzRequest, EvalLogFilter, PrincipalType,
};
use policy_cedar_domain::*;
use serde_json::json;

/// The `kind` wire values are lowercase.
#[test]
fn annotation_kinds_serde_roundtrip() {
    // AnnotationKind wire values must be lowercase.
    let obligation_json = serde_json::to_string(&AnnotationKind::Obligation)
        .expect("AnnotationKind::Obligation serializes");
    assert_eq!(obligation_json, "\"obligation\"");

    let advice_json =
        serde_json::to_string(&AnnotationKind::Advice).expect("AnnotationKind::Advice serializes");
    assert_eq!(advice_json, "\"advice\"");

    // Full PolicyAnnotation round-trip.
    let ann = PolicyAnnotation {
        kind: AnnotationKind::Obligation,
        key: "require_mfa".to_string(),
        value: "true".to_string(),
    };
    let json = serde_json::to_string(&ann).expect("PolicyAnnotation serializes");
    let roundtrip: PolicyAnnotation =
        serde_json::from_str(&json).expect("PolicyAnnotation deserializes");
    assert_eq!(ann, roundtrip);
}

#[test]
fn allow_decision_surfaces_rule_annotations() {
    let mut policies = PolicySet::default();
    policies
        .publish(PolicyVersion {
            policy_id: "pol_annotated_allow".to_string(),
            version: "1.0.0".to_string(),
            scope: PolicyScope::Tenant(TEST_TENANT_ID.to_string()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "tenant-admin".to_string(),
                action: "tenant.settings.update".to_string(),
                resource_prefix: "tenant:".to_string(),
                required_attribute: None,
                annotations: vec![
                    PolicyAnnotation {
                        kind: AnnotationKind::Obligation,
                        key: "require_mfa".to_string(),
                        value: "true".to_string(),
                    },
                    PolicyAnnotation {
                        kind: AnnotationKind::Advice,
                        key: "audit_event".to_string(),
                        value: "settings_change".to_string(),
                    },
                ],
            }],
        })
        .expect("annotated allow policy publishes");

    let decision = policies.authorize(&AuthorizationQuery {
        subject: AuthorizationSubject {
            tenant_id: TEST_TENANT_ID.to_string(),
            roles: vec!["tenant-admin".to_string()],
        },
        action: "tenant.settings.update".to_string(),
        resource: TEST_TENANT_RESOURCE.to_string(),
        attributes: BTreeMap::new(),
    });

    assert!(decision.allowed);
    assert_eq!(decision.annotations.len(), 2);
    assert_eq!(decision.annotations[0].kind, AnnotationKind::Obligation);
    assert_eq!(decision.annotations[0].key, "require_mfa");
    assert_eq!(decision.annotations[0].value, "true");
    assert_eq!(decision.annotations[1].kind, AnnotationKind::Advice);
    assert_eq!(decision.annotations[1].key, "audit_event");
}

#[test]
fn deny_wins_suppresses_annotations() {
    let mut policies = PolicySet::default();
    // First publish an annotated Allow.
    policies
        .publish(PolicyVersion {
            policy_id: "pol_annotated_allow_b".to_string(),
            version: "1.0.0".to_string(),
            scope: PolicyScope::Tenant(TEST_TENANT_ID.to_string()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "tenant-admin".to_string(),
                action: "tenant.settings.update".to_string(),
                resource_prefix: "tenant:".to_string(),
                required_attribute: None,
                annotations: vec![PolicyAnnotation {
                    kind: AnnotationKind::Obligation,
                    key: "require_mfa".to_string(),
                    value: "true".to_string(),
                }],
            }],
        })
        .expect("allow policy publishes");
    // Then publish an explicit Deny (policy_id sorts before allow so it fires first).
    policies
        .publish(PolicyVersion {
            policy_id: "pol_explicit_deny_b".to_string(),
            version: "1.0.0".to_string(),
            scope: PolicyScope::Tenant(TEST_TENANT_ID.to_string()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Deny,
                principal_role: "tenant-admin".to_string(),
                action: "tenant.settings.update".to_string(),
                resource_prefix: "tenant:".to_string(),
                required_attribute: None,
                annotations: Vec::new(),
            }],
        })
        .expect("deny policy publishes");

    let decision = policies.authorize(&AuthorizationQuery {
        subject: AuthorizationSubject {
            tenant_id: TEST_TENANT_ID.to_string(),
            roles: vec!["tenant-admin".to_string()],
        },
        action: "tenant.settings.update".to_string(),
        resource: TEST_TENANT_RESOURCE.to_string(),
        attributes: BTreeMap::new(),
    });

    assert!(!decision.allowed, "deny must win");
    assert!(
        decision.annotations.is_empty(),
        "deny must suppress all annotations; got: {:?}",
        decision.annotations
    );
}

#[test]
fn no_match_deny_has_empty_annotations() {
    let mut policies = PolicySet::default();
    policies
        .publish(policy_version(POLICY_ID, "1.0.0", tenant_scope(), None))
        .expect("policy publishes");

    let decision = policies.authorize(&AuthorizationQuery {
        subject: AuthorizationSubject {
            tenant_id: TEST_TENANT_ID.to_string(),
            roles: vec!["unknown-role".to_string()],
        },
        action: "tenant.settings.update".to_string(),
        resource: TEST_TENANT_RESOURCE.to_string(),
        attributes: BTreeMap::new(),
    });

    assert!(!decision.allowed);
    assert!(decision.annotations.is_empty());
    assert_eq!(decision.reason, "no matching allow policy");
}
