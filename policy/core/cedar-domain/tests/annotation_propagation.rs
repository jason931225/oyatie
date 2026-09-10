#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;

mod common;

use common::*;
use policy_cedar_domain::authz_engine::{
    AuthzDecision, AuthzRequest, EvalLogFilter, PrincipalType,
};
use policy_cedar_domain::*;
use serde_json::json;

#[test]
fn multiple_annotation_kinds_on_one_rule_all_surface() {
    let annotations = vec![
        PolicyAnnotation {
            kind: AnnotationKind::Obligation,
            key: "step_up".to_string(),
            value: "mfa".to_string(),
        },
        PolicyAnnotation {
            kind: AnnotationKind::Obligation,
            key: "redact_fields".to_string(),
            value: "pii".to_string(),
        },
        PolicyAnnotation {
            kind: AnnotationKind::Advice,
            key: "audit_event".to_string(),
            value: "pii_access".to_string(),
        },
    ];
    let mut policies = PolicySet::default();
    policies
        .publish(PolicyVersion {
            policy_id: "pol_multi_annotation".to_string(),
            version: "1.0.0".to_string(),
            scope: PolicyScope::Tenant(TEST_TENANT_ID.to_string()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "tenant-admin".to_string(),
                action: "tenant.settings.update".to_string(),
                resource_prefix: "tenant:".to_string(),
                required_attribute: None,
                annotations: annotations.clone(),
            }],
        })
        .expect("multi-annotation policy publishes");

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
    assert_eq!(decision.annotations, annotations);
}

#[test]
fn authorization_decision_serde_with_annotations() {
    // `AuthorizationDecision` derives no serde impl, so the name overstates
    // this: what round-trips is its `PolicyAnnotation` payload.
    let annotations = vec![
        PolicyAnnotation {
            kind: AnnotationKind::Obligation,
            key: "require_mfa".to_string(),
            value: "true".to_string(),
        },
        PolicyAnnotation {
            kind: AnnotationKind::Advice,
            key: "label".to_string(),
            value: "sensitive".to_string(),
        },
    ];
    let json = serde_json::to_string(&annotations).expect("Vec<PolicyAnnotation> serializes");
    let roundtrip: Vec<PolicyAnnotation> =
        serde_json::from_str(&json).expect("Vec<PolicyAnnotation> deserializes");
    assert_eq!(annotations, roundtrip);
}

#[test]
fn policy_rule_input_annotations_propagate_through_try_from() {
    let annotations = vec![PolicyAnnotation {
        kind: AnnotationKind::Obligation,
        key: "step_up".to_string(),
        value: "mfa".to_string(),
    }];
    let input = PolicyRuleInput {
        effect: PolicyEffect::Allow,
        principal_role: "admin".to_string(),
        action: "resource.read".to_string(),
        resource_prefix: "res:".to_string(),
        required_attribute: None,
        annotations: annotations.clone(),
    };
    let rule = PolicyRule::try_from(input).expect("valid rule converts");
    assert_eq!(rule.annotations, annotations);
}
