//! Shared fixtures for the cedar-domain integration suites.

use std::collections::BTreeMap;

use policy_cedar_domain::authz_engine::{AuthzRequest, PrincipalType};
use policy_cedar_domain::*;
use serde_json::json;

pub const POLICY_ID: &str = "pol_tenant_admin";
pub const TEST_TENANT_ID: &str = "ten_alpha";
pub const TEST_TENANT_RESOURCE: &str = "tenant:ten_alpha:settings";

// ── helpers ───────────────────────────────────────────────────────────────

pub fn tenant_scope() -> PolicyScope {
    PolicyScope::Tenant(TEST_TENANT_ID.to_string())
}

pub fn policy_version(
    policy_id: &str,
    version: &str,
    scope: PolicyScope,
    supersedes: Option<&str>,
) -> PolicyVersion {
    policy_version_with_effect(policy_id, version, scope, supersedes, PolicyEffect::Allow)
}

pub fn policy_version_with_effect(
    policy_id: &str,
    version: &str,
    scope: PolicyScope,
    supersedes: Option<&str>,
    effect: PolicyEffect,
) -> PolicyVersion {
    PolicyVersion {
        policy_id: policy_id.to_string(),
        version: version.to_string(),
        scope,
        supersedes: supersedes.map(str::to_string),
        rules: vec![PolicyRuleInput {
            effect,
            principal_role: "tenant-admin".to_string(),
            action: "tenant.settings.update".to_string(),
            resource_prefix: "tenant:".to_string(),
            required_attribute: None,
            annotations: Vec::new(),
        }],
    }
}

pub fn backbone_request(operation: BackboneWriteOperation, tenant_id: &str) -> AuthzRequest {
    let mut context = BTreeMap::new();
    context.insert("roles".to_string(), json!([operation.principal_role()]));
    context.insert("data_plane".to_string(), json!("backbone"));
    AuthzRequest {
        tenant_id: tenant_id.to_string(),
        principal_type: PrincipalType::User,
        principal_id: Some("user:u".to_string()),
        action: operation.action().to_string(),
        resource_type: operation.resource_type().to_string(),
        resource_id: Some(sample_resource_id(operation).to_string()),
        context,
    }
}

pub fn sample_resource_id(operation: BackboneWriteOperation) -> &'static str {
    match operation {
        BackboneWriteOperation::MessengerPostMessage => "channel:c",
        BackboneWriteOperation::MailSubmitMessage => "mailbox:b",
        BackboneWriteOperation::SocialPublishPost => "profile:p",
        BackboneWriteOperation::CommunityCreatePost => "space:s",
        BackboneWriteOperation::CommunityCastVote
        | BackboneWriteOperation::CommunityApplyModerationAction => "post:p",
    }
}

// ── policy-diff fixtures ─────────────────────────────────────────────────────

pub fn allow_rule(role: &str, action: &str, prefix: &str) -> PolicyRuleInput {
    PolicyRuleInput {
        effect: PolicyEffect::Allow,
        principal_role: role.to_string(),
        action: action.to_string(),
        resource_prefix: prefix.to_string(),
        required_attribute: None,
        annotations: Vec::new(),
    }
}

pub fn deny_rule(role: &str, action: &str, prefix: &str) -> PolicyRuleInput {
    PolicyRuleInput {
        effect: PolicyEffect::Deny,
        principal_role: role.to_string(),
        action: action.to_string(),
        resource_prefix: prefix.to_string(),
        required_attribute: None,
        annotations: Vec::new(),
    }
}

pub fn allow_rule_attr(
    role: &str,
    action: &str,
    prefix: &str,
    attr: Option<(&str, &str)>,
) -> PolicyRuleInput {
    PolicyRuleInput {
        effect: PolicyEffect::Allow,
        principal_role: role.to_string(),
        action: action.to_string(),
        resource_prefix: prefix.to_string(),
        required_attribute: attr.map(|(k, v)| (k.to_string(), v.to_string())),
        annotations: Vec::new(),
    }
}

pub fn pv(version: &str, rules: Vec<PolicyRuleInput>) -> PolicyVersion {
    PolicyVersion {
        policy_id: "pol_test".to_string(),
        version: version.to_string(),
        scope: PolicyScope::Global,
        supersedes: None,
        rules,
    }
}
