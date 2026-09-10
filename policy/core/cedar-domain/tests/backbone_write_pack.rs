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
fn backbone_write_policy_pack_allows_every_implemented_write_action() {
    let mut evaluator =
        CedarRuntimeEvaluator::with_backbone_write_policies(TEST_TENANT_ID).unwrap();

    for operation in BackboneWriteOperation::all() {
        let evaluation = evaluator
            .evaluate(backbone_request(operation, TEST_TENANT_ID), "audit-cedar")
            .unwrap();

        assert!(evaluation.decision.is_allowed());
        assert!(
            evaluation
                .decision_ref
                .starts_with(&format!("cedar:allow:{}", operation.policy_id()))
        );
        assert_eq!(
            evaluation.decision.determining_policies,
            vec![operation.policy_id().to_string()]
        );
        assert_eq!(evaluation.log_entry.audit_correlation_id, "audit-cedar");
    }

    assert_eq!(evaluator.log_len(), BackboneWriteOperation::all().len());
}

#[test]
fn backbone_write_policy_pack_denies_wrong_tenant_by_default() {
    let mut evaluator =
        CedarRuntimeEvaluator::with_backbone_write_policies(TEST_TENANT_ID).unwrap();
    let evaluation = evaluator
        .evaluate(
            backbone_request(BackboneWriteOperation::MessengerPostMessage, "ten_other"),
            "audit-cedar",
        )
        .unwrap();

    assert!(!evaluation.decision.is_allowed());
    assert_eq!(evaluation.decision_ref, "cedar:deny:default:1");
    assert!(evaluation.decision.determining_policies.is_empty());
    assert_eq!(evaluation.log_entry.reason, "no matching allow policy");
}

#[test]
fn explicit_deny_policy_wins_over_backbone_allow() {
    let operation = BackboneWriteOperation::MailSubmitMessage;
    let mut versions = backbone_write_policy_versions(TEST_TENANT_ID);
    versions.push(PolicyVersion {
        policy_id: "pol_backbone_mail_submit_freeze".to_string(),
        version: "1.0.0".to_string(),
        scope: tenant_scope(),
        supersedes: None,
        rules: vec![PolicyRuleInput {
            effect: PolicyEffect::Deny,
            principal_role: operation.principal_role().to_string(),
            action: operation.action().to_string(),
            resource_prefix: operation.resource_prefix().to_string(),
            required_attribute: Some(("data_plane".to_string(), "backbone".to_string())),
            annotations: Vec::new(),
        }],
    });
    let mut evaluator = CedarRuntimeEvaluator::from_policy_versions(versions).unwrap();

    let evaluation = evaluator
        .evaluate(backbone_request(operation, TEST_TENANT_ID), "audit-cedar")
        .unwrap();

    assert!(!evaluation.decision.is_allowed());
    assert_eq!(
        evaluation.decision.determining_policies,
        vec!["pol_backbone_mail_submit_freeze".to_string()]
    );
    assert_eq!(evaluation.log_entry.reason, "explicit deny policy");
}

#[test]
fn eval_log_filter_selects_effect_principal_and_resource_type() {
    let mut evaluator =
        CedarRuntimeEvaluator::with_backbone_write_policies(TEST_TENANT_ID).unwrap();
    evaluator
        .evaluate(
            backbone_request(BackboneWriteOperation::CommunityCreatePost, TEST_TENANT_ID),
            "audit-1",
        )
        .unwrap();
    evaluator
        .evaluate(
            backbone_request(BackboneWriteOperation::CommunityCastVote, "ten_other"),
            "audit-2",
        )
        .unwrap();

    let allow_filter = EvalLogFilter {
        principal_id: Some("user:u".to_string()),
        effect: Some(PolicyEffect::Allow),
        resource_type: Some("community:space".to_string()),
        limit: 10,
    };
    let allow_entries = evaluator.eval_log(&allow_filter);
    assert_eq!(allow_entries.len(), 1);
    assert_eq!(
        allow_entries[0].determining_policies,
        vec!["pol_backbone_community_post_create".to_string()]
    );

    let deny_filter = EvalLogFilter {
        principal_id: None,
        effect: Some(PolicyEffect::Deny),
        resource_type: None,
        limit: 1,
    };
    let deny_entries = evaluator.eval_log(&deny_filter);
    assert_eq!(deny_entries.len(), 1);
    assert_eq!(deny_entries[0].decision_ref, "cedar:deny:default:2");
}

#[test]
fn runtime_evaluator_rejects_missing_audit_and_invalid_role_context() {
    let mut evaluator =
        CedarRuntimeEvaluator::with_backbone_write_policies(TEST_TENANT_ID).unwrap();
    assert_eq!(
        evaluator.evaluate(
            backbone_request(BackboneWriteOperation::MessengerPostMessage, TEST_TENANT_ID),
            "",
        ),
        Err(CedarRuntimeError::MissingAuditCorrelation)
    );

    let mut request =
        backbone_request(BackboneWriteOperation::MessengerPostMessage, TEST_TENANT_ID);
    request
        .context
        .insert("roles".to_string(), json!({"not": "a-role"}));
    assert_eq!(
        evaluator.evaluate(request, "audit"),
        Err(CedarRuntimeError::InvalidRoleContext)
    );
}
