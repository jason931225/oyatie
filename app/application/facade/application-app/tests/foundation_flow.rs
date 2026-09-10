#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AutonomyTier, CapabilityAction, CapabilityInvocationRequest, CapabilityRegistration, DataClass,
    Foundation, FoundationError, IdentityRegistration, Purpose, SubjectClass,
    TenantCapabilityGrant, TenantRegistration, TokenRequest,
};

#[test]
fn tenant_onboarding_invocation_and_audit_chain_obey_foundation_contracts() {
    let mut foundation = Foundation::default();

    let tenant = foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_acme".into(),
            legal_name: "Acme Manufacturing Korea".into(),
            home_region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            regulatory_packs: vec!["pack-alpha".into()],
            autonomy_ceiling: AutonomyTier::T2Advisory,
        })
        .expect("tenant can be onboarded");

    let cell = foundation
        .bind_cell(&tenant.id, "region-home-a", "cell-control-a")
        .expect("first cell binding succeeds");
    assert_eq!(cell.region, "region-home");

    let user = foundation
        .upsert_identity(IdentityRegistration {
            tenant_id: tenant.id.clone(),
            user_id: "usr_admin".into(),
            primary_identifier: "admin@acme.example".into(),
            display_name: "Acme Admin".into(),
            roles: vec!["tenant-admin".into()],
        })
        .expect("identity can be upserted");
    support::allow_capability_invocation(&mut foundation, &tenant.id, "tenant-admin");

    let token = foundation
        .issue_token(TokenRequest {
            tenant_id: tenant.id.clone(),
            user_id: user.id.value.as_str().to_string(),
            purpose: Purpose::CapabilityInvocation,
            ttl_seconds: 3_600,
            issued_at_epoch_seconds: 1_000,
        })
        .expect("one-hour purpose-bound token is valid");
    assert_eq!(token.expires_at_epoch_seconds, 4_600);

    support::seed_passing_eval(&mut foundation, "cap.workflow.approve-payroll");
    let capability = foundation
        .register_capability(CapabilityRegistration {
            capability_id: "cap.workflow.approve-payroll".into(),
            namespace: "workflow".into(),
            action: CapabilityAction::Other,
            required_tier: AutonomyTier::T3ExecuteWithApproval,
            touched_privacy_data_classes: application_app::privacy_data_classes_from(&[
                DataClass::PiiIdentifying,
                DataClass::FinancialRegulatedCredit,
            ])
            .unwrap(),
            evidence_topic: "oya.foundry.capability.invoked".into(),
        })
        .expect("capability registration is valid");
    foundation
        .grant_capability_to_tenant(TenantCapabilityGrant {
            tenant_id: tenant.id.clone(),
            capability_id: capability.id.clone(),
            mcp_visible: true,
        })
        .expect("tenant can be licensed for capability");

    let denied = foundation
        .invoke_capability_as_principal(
            support::principal(&tenant.id, user.id.value.as_str(), AutonomyTier::T2Advisory),
            CapabilityInvocationRequest {
                tenant_id: tenant.id.clone(),
                user_id: user.id.value.as_str().to_string(),
                capability_id: capability.id.clone(),
                purpose: Purpose::CapabilityInvocation,
                subject_class: SubjectClass::Adult,
                budget_window_id: "2026-05".into(),
                projected_cost_micros: 10,
                started_at_epoch_seconds: 1_000,
            },
        )
        .expect_err("T3 capability is blocked by tenant T2 ceiling");
    assert_eq!(denied, FoundationError::AutonomyCeilingExceeded);

    assert!(
        foundation.audit_chain().verify(),
        "audit hash chain verifies"
    );
    assert!(
        foundation
            .audit_chain()
            .events()
            .iter()
            .any(|event| event.surface == "foundry.capability.invoke" && event.decision == "DENY")
    );
}

#[test]
fn foundation_rejects_token_ttl_over_one_hour_and_cell_rebind() {
    let mut foundation = Foundation::default();
    let tenant = foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_beta".into(),
            legal_name: "Beta Logistics".into(),
            home_region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            regulatory_packs: vec!["pack-alpha".into()],
            autonomy_ceiling: AutonomyTier::T1ViewOnly,
        })
        .expect("tenant can be onboarded");

    foundation
        .bind_cell(&tenant.id, "region-home-a", "cell-control-a")
        .expect("first bind succeeds");
    let rebind = foundation.bind_cell(&tenant.id, "region-home-b", "cell-control-b");
    assert_eq!(rebind, Err(FoundationError::CellBindingImmutable));

    foundation
        .upsert_identity(IdentityRegistration {
            tenant_id: tenant.id.clone(),
            user_id: "usr_ops".into(),
            primary_identifier: "ops@beta.example".into(),
            display_name: "Beta Ops".into(),
            roles: vec!["ops".into()],
        })
        .expect("identity can be upserted");

    let token = foundation.issue_token(TokenRequest {
        tenant_id: tenant.id,
        user_id: "usr_ops".into(),
        purpose: Purpose::CoreService,
        ttl_seconds: 3_601,
        issued_at_epoch_seconds: 2_000,
    });
    assert_eq!(token, Err(FoundationError::TokenTtlTooLong));
}
