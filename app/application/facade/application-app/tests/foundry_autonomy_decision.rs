#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AutonomyTier, CapabilityAction, CapabilityInvocationRequest, CapabilityRegistration,
    CostBudgetRegistration, DataClass, EvidenceKind, Foundation, FoundationError,
    IdentityRegistration, Purpose, RunDisposition, RunState, SubjectClass, TenantCapabilityGrant,
    TenantRegistration,
};

#[test]
fn capability_invocation_emits_autonomy_decision_for_allow_and_deny() {
    let mut foundation = Foundation::default();
    onboard_tenant(&mut foundation);
    foundation
        .upsert_identity(IdentityRegistration {
            tenant_id: "ten_autonomy".into(),
            user_id: "usr_operator".into(),
            primary_identifier: "operator@autonomy.oyatie.test".into(),
            display_name: "Autonomy Operator".into(),
            roles: vec!["tenant-admin".into()],
        })
        .unwrap();
    support::allow_capability_invocation(&mut foundation, "ten_autonomy", "tenant-admin");
    foundation
        .grant_data_use(
            "ten_autonomy",
            Purpose::CapabilityInvocation,
            support::privacy_data_class(DataClass::InternalOnly),
        )
        .unwrap();
    register_capability(
        &mut foundation,
        "cap.demo.allowed",
        AutonomyTier::T1ViewOnly,
    );
    register_capability(
        &mut foundation,
        "cap.demo.denied",
        AutonomyTier::T3ExecuteWithApproval,
    );
    foundation
        .configure_tenant_cost_budget(CostBudgetRegistration {
            tenant_id: "ten_autonomy".into(),
            capability_id: None,
            window_id: "autonomy-window".into(),
            monthly_limit_micros: 1_000_000,
            per_invocation_limit_micros: 1_000,
            warning_threshold_percent: 80,
        })
        .unwrap();

    foundation
        .invoke_capability_as_principal(
            support::principal("ten_autonomy", "usr_operator", AutonomyTier::T2Advisory),
            invocation("cap.demo.allowed", 3_000),
        )
        .expect("T1 capability is within T2 ceiling");
    assert!(foundation.audit_chain().events().iter().any(|event| {
        event.surface == "foundry.autonomy.decision" && event.decision == "ALLOW"
    }));
    let runs_after_allow = foundation.foundry_runs().len();
    let steps_after_allow = foundation.foundry_steps().len();
    let evidence_after_allow = foundation.foundry_evidence_chain().records().len();
    let run_complete_allow_after_allow = allow_surface_count(&foundation, "foundry.run.complete");
    let evidence_emit_allow_after_allow = allow_surface_count(&foundation, "foundry.evidence.emit");
    let capability_invoke_allow_after_allow =
        allow_surface_count(&foundation, "foundry.capability.invoke");

    assert_eq!(
        foundation.invoke_capability_as_principal(
            support::principal("ten_autonomy", "usr_operator", AutonomyTier::T2Advisory),
            invocation("cap.demo.denied", 4_000)
        ),
        Err(FoundationError::AutonomyCeilingExceeded)
    );
    assert_eq!(foundation.foundry_runs().len(), runs_after_allow + 1);
    assert_eq!(foundation.foundry_steps().len(), steps_after_allow);
    assert_eq!(
        foundation.foundry_evidence_chain().records().len(),
        evidence_after_allow + 1
    );
    let rejected_run = foundation
        .foundry_runs()
        .last()
        .expect("denial records a run");
    assert_eq!(rejected_run.state.value, RunState::RejectedAutonomy);
    assert_eq!(
        rejected_run.disposition.value,
        Some(RunDisposition::FailureAutonomy)
    );
    let denial_evidence = foundation
        .foundry_evidence_chain()
        .records()
        .last()
        .expect("denial records evidence");
    assert_eq!(denial_evidence.kind.value, EvidenceKind::AutonomyDecision);
    assert_eq!(denial_evidence.step_id.value, None);
    assert_eq!(
        denial_evidence
            .fields
            .value
            .get("reason")
            .map(String::as_str),
        Some("autonomy")
    );
    assert_eq!(
        denial_evidence
            .fields
            .value
            .get("decision")
            .map(String::as_str),
        Some("DENY")
    );
    assert!(
        foundation.audit_chain().events().iter().any(|event| {
            event.surface == "foundry.autonomy.decision" && event.decision == "DENY"
        })
    );
    assert_eq!(
        allow_surface_count(&foundation, "foundry.run.complete"),
        run_complete_allow_after_allow
    );
    assert_eq!(
        allow_surface_count(&foundation, "foundry.evidence.emit"),
        evidence_emit_allow_after_allow + 1
    );
    assert_eq!(
        allow_surface_count(&foundation, "foundry.capability.invoke"),
        capability_invoke_allow_after_allow
    );
}

fn onboard_tenant(foundation: &mut Foundation) {
    foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: "ten_autonomy".into(),
            legal_name: "Autonomy Tenant".into(),
            home_region: "region-home".into(),
            residency_class: "strict_home_region".into(),
            regulatory_packs: vec!["pack-alpha".into()],
            autonomy_ceiling: AutonomyTier::T2Advisory,
        })
        .unwrap();
}

fn register_capability(foundation: &mut Foundation, capability_id: &str, tier: AutonomyTier) {
    support::seed_passing_eval(foundation, capability_id);
    foundation
        .register_capability(CapabilityRegistration {
            capability_id: capability_id.into(),
            namespace: "demo".into(),
            action: CapabilityAction::Other,
            required_tier: tier,
            touched_privacy_data_classes: application_app::privacy_data_classes_from(&[
                DataClass::InternalOnly,
            ])
            .unwrap(),
            evidence_topic: "oya.foundry.capability.invoked".into(),
        })
        .unwrap();
    foundation
        .grant_capability_to_tenant(TenantCapabilityGrant {
            tenant_id: "ten_autonomy".into(),
            capability_id: capability_id.into(),
            mcp_visible: true,
        })
        .unwrap();
}

fn invocation(capability_id: &str, started_at_epoch_seconds: u64) -> CapabilityInvocationRequest {
    CapabilityInvocationRequest {
        tenant_id: "ten_autonomy".into(),
        user_id: "usr_operator".into(),
        capability_id: capability_id.into(),
        purpose: Purpose::CapabilityInvocation,
        subject_class: SubjectClass::Adult,
        budget_window_id: "autonomy-window".into(),
        projected_cost_micros: 125,
        started_at_epoch_seconds,
    }
}

fn allow_surface_count(foundation: &Foundation, surface: &str) -> usize {
    foundation
        .audit_chain()
        .events()
        .iter()
        .filter(|event| event.surface == surface && event.decision == "ALLOW")
        .count()
}
