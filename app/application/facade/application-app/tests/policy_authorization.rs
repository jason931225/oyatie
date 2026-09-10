#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AuthorizationRequest, AutonomyTier, Foundation, FoundationError, IdentityRegistration,
    PolicyEffect, PolicyRuleInput, PolicyScope, PolicyVersion, TenantRegistration,
};

#[test]
fn policy_publish_and_authorize_enforces_rbac_abac_by_tenant_scope() {
    let mut foundation = Foundation::default();
    let tenant = foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_policy".into(),
            legal_name: "Policy Tenant".into(),
            home_region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            regulatory_packs: vec!["pack-alpha".into()],
            autonomy_ceiling: AutonomyTier::T3ExecuteWithApproval,
        })
        .expect("tenant can be onboarded");
    let user = foundation
        .upsert_identity(IdentityRegistration {
            tenant_id: tenant.id.clone(),
            user_id: "usr_policy_admin".into(),
            primary_identifier: "admin@policy.example".into(),
            display_name: "Policy Admin".into(),
            roles: vec!["tenant-admin".into()],
        })
        .expect("identity can be upserted");

    let published = foundation
        .publish_policy(PolicyVersion {
            policy_id: "pol_tenant_admin".into(),
            version: "1.0.0".into(),
            scope: PolicyScope::Tenant(tenant.id.clone()),
            supersedes: None,
            rules: vec![PolicyRuleInput {
                effect: PolicyEffect::Allow,
                principal_role: "tenant-admin".into(),
                action: "tenant.settings.update".into(),
                resource_prefix: "tenant:".into(),
                required_attribute: Some(("region".into(), "region-home".into())),
                annotations: vec![],
            }],
        })
        .expect("policy version is valid");
    assert_eq!(published.version, "1.0.0");

    let allowed = foundation
        .authorize(AuthorizationRequest {
            tenant_id: tenant.id.clone(),
            user_id: user.id.value.as_str().to_string(),
            action: "tenant.settings.update".into(),
            resource: "tenant:ten_policy:settings".into(),
            attributes: vec![("region".into(), "region-home".into())],
        })
        .expect("authorization request is valid");
    assert!(allowed.allowed);

    let denied = foundation
        .authorize(AuthorizationRequest {
            tenant_id: tenant.id.clone(),
            user_id: user.id.value.as_str().to_string(),
            action: "tenant.settings.update".into(),
            resource: "tenant:ten_policy:settings".into(),
            attributes: vec![("region".into(), "region-recovery".into())],
        })
        .expect("authorization request is valid");
    assert!(!denied.allowed);
    assert_eq!(denied.reason, "no matching allow policy");

    let duplicate = foundation
        .publish_policy(PolicyVersion {
            policy_id: "pol_tenant_admin".into(),
            version: "1.0.0".into(),
            scope: PolicyScope::Tenant(tenant.id),
            supersedes: None,
            rules: vec![],
        })
        .expect_err("policy versions are immutable");
    assert_eq!(duplicate, FoundationError::PolicyVersionAlreadyExists);
    assert!(foundation.audit_chain().verify());
}
