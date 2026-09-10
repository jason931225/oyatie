#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use check_cost_budget::{
    BudgetCeiling, BudgetError, BudgetLedger, BudgetScope, BudgetWarning, ReservationStatus,
};

#[test]
fn reservation_enforces_per_invocation_tenant_and_capability_ceilings() {
    let summarize = BudgetScope::new(
        "ten_alpha".into(),
        "cap.demo.summarize".into(),
        "2026-05".into(),
    )
    .expect("scope is valid");
    let classify = BudgetScope::new(
        "ten_alpha".into(),
        "cap.demo.classify".into(),
        "2026-05".into(),
    )
    .expect("scope is valid");

    let mut ledger = BudgetLedger::default();
    ledger
        .configure_tenant_ceiling(
            "ten_alpha".into(),
            "2026-05".into(),
            BudgetCeiling::new(1_000, 500, 80).unwrap(),
        )
        .unwrap();

    let first = ledger.reserve(&summarize, 400).unwrap();
    ledger.commit(&first.reservation_id.value).unwrap();

    assert_eq!(
        ledger.reserve(&summarize, 600),
        Err(BudgetError::PerInvocationLimitExceeded)
    );

    ledger
        .configure_capability_ceiling(summarize.clone(), BudgetCeiling::new(450, 450, 80).unwrap())
        .unwrap();
    assert_eq!(
        ledger.reserve(&summarize, 60),
        Err(BudgetError::CapabilityMonthlyLimitExceeded)
    );

    let second = ledger.reserve(&classify, 500).unwrap();
    ledger.commit(&second.reservation_id.value).unwrap();
    assert_eq!(
        ledger.reserve(&classify, 101),
        Err(BudgetError::TenantMonthlyLimitExceeded)
    );
}

#[test]
fn preflight_reports_soft_warning_at_configured_threshold() {
    let scope = BudgetScope::new(
        "ten_alpha".into(),
        "cap.demo.summarize".into(),
        "2026-05".into(),
    )
    .expect("scope is valid");
    let mut ledger = BudgetLedger::default();
    ledger
        .configure_tenant_ceiling(
            "ten_alpha".into(),
            "2026-05".into(),
            BudgetCeiling::new(1_000, 1_000, 80).unwrap(),
        )
        .unwrap();

    let below_threshold = ledger.evaluate(&scope, 799).unwrap();
    assert!(below_threshold.allowed.value);
    assert_eq!(below_threshold.warning.value, None);

    let at_threshold = ledger.evaluate(&scope, 800).unwrap();
    assert!(at_threshold.allowed.value);
    assert_eq!(
        at_threshold.warning.value,
        Some(BudgetWarning::RunningSpendThresholdReached)
    );
}

#[test]
fn reservation_lifecycle_separates_pending_committed_and_released_spend() {
    let scope = BudgetScope::new(
        "ten_alpha".into(),
        "cap.demo.summarize".into(),
        "2026-05".into(),
    )
    .expect("scope is valid");
    let mut ledger = BudgetLedger::default();
    ledger
        .configure_tenant_ceiling(
            "ten_alpha".into(),
            "2026-05".into(),
            BudgetCeiling::new(1_000, 1_000, 80).unwrap(),
        )
        .unwrap();

    let pending = ledger.reserve(&scope, 400).unwrap();
    assert_eq!(pending.status.value, ReservationStatus::Pending);
    let snapshot = ledger.snapshot(&scope).unwrap();
    assert_eq!(snapshot.running_spend_micros.value, 400);
    assert_eq!(snapshot.committed_scope_spend_micros.value, 0);
    assert_eq!(snapshot.pending_scope_spend_micros.value, 400);

    let released = ledger.release(&pending.reservation_id.value).unwrap();
    assert_eq!(released.status.value, ReservationStatus::Released);
    let snapshot = ledger.snapshot(&scope).unwrap();
    assert_eq!(snapshot.running_spend_micros.value, 0);

    let pending = ledger.reserve(&scope, 300).unwrap();
    let committed = ledger.commit(&pending.reservation_id.value).unwrap();
    assert_eq!(committed.status.value, ReservationStatus::Committed);
    let snapshot = ledger.snapshot(&scope).unwrap();
    assert_eq!(snapshot.committed_scope_spend_micros.value, 300);
    assert_eq!(snapshot.pending_scope_spend_micros.value, 0);
    assert_eq!(snapshot.running_spend_micros.value, 300);
    assert_eq!(
        ledger.release(&pending.reservation_id.value),
        Err(BudgetError::ReservationNotPending)
    );
}

#[test]
fn budget_scope_and_ceiling_validate_shape() {
    assert_eq!(
        BudgetScope::new(
            "tenant-alpha".into(),
            "cap.demo.summarize".into(),
            "2026-05".into(),
        ),
        Err(BudgetError::InvalidTenantId)
    );
    assert_eq!(
        BudgetScope::new("ten_alpha".into(), "summarize".into(), "2026-05".into()),
        Err(BudgetError::InvalidCapabilityId)
    );
    assert_eq!(
        BudgetScope::new("ten_alpha".into(), "cap.demo.summarize".into(), "".into()),
        Err(BudgetError::InvalidWindowId)
    );
    assert_eq!(
        BudgetCeiling::new(0, 100, 80),
        Err(BudgetError::InvalidBudgetCeiling)
    );
    assert_eq!(
        BudgetCeiling::new(1_000, 0, 80),
        Err(BudgetError::InvalidBudgetCeiling)
    );
    assert_eq!(
        BudgetCeiling::new(1_000, 100, 0),
        Err(BudgetError::InvalidBudgetCeiling)
    );
}
