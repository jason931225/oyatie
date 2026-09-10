#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use application_app::{
    AutonomyBreakGlassInput, AutonomyTier, CapabilityAction, CapabilityInvocationRequest,
    CapabilityRegistration, CostBudgetRegistration, DataClass, EvidenceKind, Foundation,
    FoundationError, IdentityRegistration, Purpose, RunState, SubjectClass, TenantCapabilityGrant,
    TenantRegistration,
};

const DAY: u64 = 86_400;

#[test]
fn active_break_glass_allows_autonomy_denied_invocation_with_non_masking_evidence() {
    let mut foundation =
        foundation_with_t3_capability("ten_break_glass", "cap.break.glass.runtime");
    foundation
        .register_autonomy_break_glass(AutonomyBreakGlassInput {
            id: "abg_runtime_allow".into(),
            tenant_id: "ten_break_glass".into(),
            capability_id: "cap.break.glass.runtime".into(),
            requested_tier: AutonomyTier::T3ExecuteWithApproval,
            permitted_tier: AutonomyTier::T3ExecuteWithApproval,
            requesting_actor: "usr_operator".into(),
            approving_actors: vec!["usr_security".into(), "usr_privacy".into()],
            approval_quorum: "two-of-three".into(),
            rationale: "restore tenant workflow during Sev 1 incident with bounded approval".into(),
            created_at_epoch_days: 10,
            expires_at_epoch_days: 12,
            revoked_at_epoch_days: None,
        })
        .expect("valid break-glass record enters runtime ledger");

    let receipt = foundation
        .invoke_capability_as_principal(
            support::principal("ten_break_glass", "usr_operator", AutonomyTier::T2Advisory),
            invocation("ten_break_glass", "cap.break.glass.runtime", 11 * DAY),
        )
        .expect("active break-glass raises T2 ceiling for the matching T3 capability");

    assert!(receipt.run_id.is_some());
    let run = foundation
        .foundry_runs()
        .last()
        .expect("break-glass invocation records a run");
    assert_eq!(run.state.value, RunState::Succeeded);
    assert_eq!(
        run.autonomy_tier_used.value,
        AutonomyTier::T3ExecuteWithApproval
    );
    assert!(foundation.audit_chain().events().iter().any(|event| {
        event.surface == "foundry.autonomy.break_glass.approve" && event.decision == "ALLOW"
    }));
    assert!(foundation.audit_chain().events().iter().any(|event| {
        event.surface == "foundry.autonomy.break_glass.invoke" && event.decision == "ALLOW"
    }));

    let autonomy_evidence = latest_autonomy_evidence(&foundation);
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("decision")
            .map(String::as_str),
        Some("ALLOW")
    );
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("break_glass_id")
            .map(String::as_str),
        Some("abg_runtime_allow")
    );
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("pre_break_glass_decision")
            .map(String::as_str),
        Some("DENY")
    );
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("pre_break_glass_effective_ceiling")
            .map(String::as_str),
        Some("T2Advisory")
    );
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("pre_break_glass_blocking_cap_source")
            .map(String::as_str),
        Some("tenant_configured")
    );
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("break_glass_permitted_tier")
            .map(String::as_str),
        Some("T3ExecuteWithApproval")
    );
}

#[test]
fn expired_or_under_tier_break_glass_records_do_not_override_autonomy_denial() {
    let mut foundation =
        foundation_with_t3_capability("ten_break_glass_fail", "cap.break.glass.fail");
    foundation
        .register_autonomy_break_glass(AutonomyBreakGlassInput {
            id: "abg_runtime_expired".into(),
            tenant_id: "ten_break_glass_fail".into(),
            capability_id: "cap.break.glass.fail".into(),
            requested_tier: AutonomyTier::T3ExecuteWithApproval,
            permitted_tier: AutonomyTier::T3ExecuteWithApproval,
            requesting_actor: "usr_operator".into(),
            approving_actors: vec!["usr_security".into(), "usr_privacy".into()],
            approval_quorum: "two-of-three".into(),
            rationale: "expired emergency window".into(),
            created_at_epoch_days: 1,
            expires_at_epoch_days: 2,
            revoked_at_epoch_days: None,
        })
        .expect("expired-by-invocation records can exist but must not apply");
    foundation
        .register_autonomy_break_glass(AutonomyBreakGlassInput {
            id: "abg_runtime_under_tier".into(),
            tenant_id: "ten_break_glass_fail".into(),
            capability_id: "cap.break.glass.fail".into(),
            requested_tier: AutonomyTier::T3ExecuteWithApproval,
            permitted_tier: AutonomyTier::T2Advisory,
            requesting_actor: "usr_operator".into(),
            approving_actors: vec!["usr_security".into(), "usr_privacy".into()],
            approval_quorum: "two-of-three".into(),
            rationale: "insufficient permitted tier".into(),
            created_at_epoch_days: 1,
            expires_at_epoch_days: 5,
            revoked_at_epoch_days: None,
        })
        .expect("under-tier break-glass records remain explicit but non-authorizing");

    let denied = foundation
        .invoke_capability_as_principal(
            support::principal(
                "ten_break_glass_fail",
                "usr_operator",
                AutonomyTier::T2Advisory,
            ),
            invocation("ten_break_glass_fail", "cap.break.glass.fail", 3 * DAY),
        )
        .expect_err("no active record permits the T3 capability");

    assert_eq!(denied, FoundationError::AutonomyCeilingExceeded);
    assert!(!foundation.audit_chain().events().iter().any(|event| {
        event.surface == "foundry.autonomy.break_glass.invoke" && event.decision == "ALLOW"
    }));
    let autonomy_evidence = latest_autonomy_evidence(&foundation);
    assert_eq!(
        autonomy_evidence
            .fields
            .value
            .get("decision")
            .map(String::as_str),
        Some("DENY")
    );
    assert_eq!(
        autonomy_evidence.fields.value.get("break_glass_id"),
        None,
        "non-authorizing break-glass records must not be represented as applied"
    );
}

fn foundation_with_t3_capability(tenant_id: &str, capability_id: &str) -> Foundation {
    let mut foundation = Foundation::default();
    foundation
        .onboard_tenant(TenantRegistration {
            tenant_id: tenant_id.into(),
            legal_name: format!("{tenant_id} Legal"),
            home_region: "failover-region".into(),
            residency_class: "global".into(),
            regulatory_packs: vec!["pack-gamma".into()],
            autonomy_ceiling: AutonomyTier::T2Advisory,
        })
        .unwrap();
    foundation
        .upsert_identity(IdentityRegistration {
            tenant_id: tenant_id.into(),
            user_id: "usr_operator".into(),
            primary_identifier: format!("operator@{tenant_id}.example.test"),
            display_name: "Operator".into(),
            roles: vec!["tenant-admin".into()],
        })
        .unwrap();
    support::allow_capability_invocation(&mut foundation, tenant_id, "tenant-admin");
    foundation
        .grant_data_use(
            tenant_id,
            Purpose::CapabilityInvocation,
            support::privacy_data_class(DataClass::InternalOnly),
        )
        .unwrap();
    foundation
        .configure_tenant_cost_budget(CostBudgetRegistration {
            tenant_id: tenant_id.into(),
            capability_id: None,
            window_id: "2026-05".into(),
            monthly_limit_micros: 10_000,
            per_invocation_limit_micros: 1_000,
            warning_threshold_percent: 80,
        })
        .unwrap();
    support::seed_passing_eval(&mut foundation, capability_id);
    foundation
        .register_capability(CapabilityRegistration {
            capability_id: capability_id.into(),
            namespace: "break_glass".into(),
            action: CapabilityAction::Other,
            required_tier: AutonomyTier::T3ExecuteWithApproval,
            touched_privacy_data_classes: application_app::privacy_data_classes_from(&[
                DataClass::InternalOnly,
            ])
            .unwrap(),
            evidence_topic: "oya.foundry.capability.invoked".into(),
        })
        .unwrap();
    foundation
        .grant_capability_to_tenant(TenantCapabilityGrant {
            tenant_id: tenant_id.into(),
            capability_id: capability_id.into(),
            mcp_visible: true,
        })
        .unwrap();
    foundation
}

fn invocation(
    tenant_id: &str,
    capability_id: &str,
    started_at_epoch_seconds: u64,
) -> CapabilityInvocationRequest {
    CapabilityInvocationRequest {
        tenant_id: tenant_id.into(),
        user_id: "usr_operator".into(),
        capability_id: capability_id.into(),
        purpose: Purpose::CapabilityInvocation,
        subject_class: SubjectClass::Adult,
        budget_window_id: "2026-05".into(),
        projected_cost_micros: 10,
        started_at_epoch_seconds,
    }
}

fn latest_autonomy_evidence(foundation: &Foundation) -> &application_app::EvidenceRecord {
    foundation
        .foundry_evidence_chain()
        .records()
        .iter()
        .rev()
        .find(|evidence| evidence.kind.value == EvidenceKind::AutonomyDecision)
        .expect("autonomy decision evidence exists")
}
