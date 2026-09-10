#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AutonomyTier, CapabilityAction, CapabilityInvocationRequest, CapabilityRegistration,
    CostBudgetRegistration, DataClass, Foundation, FoundationError, IdentityRegistration, Purpose,
    RunDisposition, RunState, SubjectClass, TenantCapabilityGrant, TenantRegistration,
};

#[test]
fn foundation_invocation_requires_tenant_capability_license() {
    let mut foundation = foundation_with_registered_capability();
    foundation
        .configure_tenant_cost_budget(CostBudgetRegistration {
            tenant_id: "ten_registry".into(),
            capability_id: None,
            window_id: "2026-05".into(),
            monthly_limit_micros: 1_000,
            per_invocation_limit_micros: 100,
            warning_threshold_percent: 80,
        })
        .unwrap();

    let denied = foundation
        .invoke_capability_as_principal(
            support::principal(
                "ten_registry",
                "usr_registry_admin",
                AutonomyTier::T2Advisory,
            ),
            CapabilityInvocationRequest {
                tenant_id: "ten_registry".into(),
                user_id: "usr_registry_admin".into(),
                capability_id: "cap.demo.registry".into(),
                purpose: Purpose::CapabilityInvocation,
                subject_class: SubjectClass::Adult,
                budget_window_id: "2026-05".into(),
                projected_cost_micros: 10,
                started_at_epoch_seconds: 1_000,
            },
        )
        .expect_err("unlicensed tenant must not invoke a registered capability");
    assert_eq!(denied, FoundationError::CapabilityNotLicensed);
    let denied_run = foundation
        .foundry_runs()
        .last()
        .expect("license denial records a rejected run");
    assert_eq!(denied_run.state.value, RunState::RejectedLicense);
    assert_eq!(
        denied_run.disposition.value,
        Some(RunDisposition::FailureLicense)
    );
    let denied_evidence = foundation
        .foundry_evidence_chain()
        .records()
        .last()
        .expect("license denial records evidence");
    assert_eq!(
        denied_evidence
            .fields
            .value
            .get("reason")
            .map(String::as_str),
        Some("license")
    );

    foundation
        .grant_capability_to_tenant(TenantCapabilityGrant {
            tenant_id: "ten_registry".into(),
            capability_id: "cap.demo.registry".into(),
            mcp_visible: true,
        })
        .unwrap();
    let receipt = foundation
        .invoke_capability_as_principal(
            support::principal(
                "ten_registry",
                "usr_registry_admin",
                AutonomyTier::T2Advisory,
            ),
            CapabilityInvocationRequest {
                tenant_id: "ten_registry".into(),
                user_id: "usr_registry_admin".into(),
                capability_id: "cap.demo.registry".into(),
                purpose: Purpose::CapabilityInvocation,
                subject_class: SubjectClass::Adult,
                budget_window_id: "2026-05".into(),
                projected_cost_micros: 10,
                started_at_epoch_seconds: 1_000,
            },
        )
        .expect("licensed tenant can invoke within policy and budget");
    assert_eq!(receipt.capability_id, "cap.demo.registry");
}

#[test]
fn foundation_discovery_returns_tenant_visible_capabilities_under_autonomy_ceiling() {
    let mut foundation = foundation_with_registered_capability();
    support::seed_passing_eval(&mut foundation, "cap.demo.high-risk");
    foundation
        .register_capability(CapabilityRegistration {
            capability_id: "cap.demo.high-risk".into(),
            namespace: "demo".into(),
            action: CapabilityAction::Other,
            required_tier: AutonomyTier::T4AutoExecute,
            touched_privacy_data_classes: application_app::privacy_data_classes_from(&[
                DataClass::InternalOnly,
            ])
            .unwrap(),
            evidence_topic: "oya.foundry.capability.invoked".into(),
        })
        .unwrap();
    foundation
        .grant_capability_to_tenant(TenantCapabilityGrant {
            tenant_id: "ten_registry".into(),
            capability_id: "cap.demo.registry".into(),
            mcp_visible: true,
        })
        .unwrap();
    foundation
        .grant_capability_to_tenant(TenantCapabilityGrant {
            tenant_id: "ten_registry".into(),
            capability_id: "cap.demo.high-risk".into(),
            mcp_visible: true,
        })
        .unwrap();

    let discovered = foundation
        .discover_tenant_capabilities("ten_registry")
        .expect("tenant discovery succeeds");
    assert_eq!(
        discovered
            .iter()
            .map(|capability| capability.id.as_str())
            .collect::<Vec<_>>(),
        vec!["cap.demo.registry"]
    );
}

fn foundation_with_registered_capability() -> Foundation {
    let mut foundation = Foundation::default();
    foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_registry".into(),
            legal_name: "Registry Tenant".into(),
            home_region: "failover-region".into(),
            residency_class: "global".into(),
            regulatory_packs: vec!["pack-gamma".into()],
            autonomy_ceiling: AutonomyTier::T2Advisory,
        })
        .unwrap();
    foundation
        .upsert_identity(IdentityRegistration {
            tenant_id: "ten_registry".into(),
            user_id: "usr_registry_admin".into(),
            primary_identifier: "registry@example.test".into(),
            display_name: "Registry Admin".into(),
            roles: vec!["tenant-admin".into()],
        })
        .unwrap();
    support::allow_capability_invocation(&mut foundation, "ten_registry", "tenant-admin");
    support::seed_passing_eval(&mut foundation, "cap.demo.registry");
    foundation
        .register_capability(CapabilityRegistration {
            capability_id: "cap.demo.registry".into(),
            namespace: "demo".into(),
            action: CapabilityAction::Other,
            required_tier: AutonomyTier::T2Advisory,
            touched_privacy_data_classes: application_app::privacy_data_classes_from(&[
                DataClass::InternalOnly,
            ])
            .unwrap(),
            evidence_topic: "oya.foundry.capability.invoked".into(),
        })
        .unwrap();
    foundation
}
