fn valid_input() -> LeaveCarryoverForfeitureInput {
    LeaveCarryoverForfeitureInput {
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        employee_id: "emp_001".to_owned(),
        period_boundary_date: "2026-12-31".to_owned(),
        closing_balance_units: 6.0,
        statutory_min_floor_units: 5.0,
        carry_over_cap_units: 10.0,
        rulepack_ref: "rulepack/kr-labor-2026".to_owned(),
        rulepack_effective_date: "2026-01-01".to_owned(),
        evidence_ref: "audit/hr/leave-carryover/emp_001/period-close".to_owned(),
        evaluated_at_epoch_seconds: 1_779_532_800,
    }
}

#[test]
fn balance_at_or_below_cap_zero_forfeiture() {
    let proj = evaluate_leave_carryover_forfeiture(valid_input())
        .expect("balance=6, cap=10 → should succeed");

    assert_eq!(proj.closing_balance_units.value, 6.0);
    assert_eq!(proj.carry_over_cap_units.value, 10.0);
    assert_eq!(
        proj.carried_over_units.value, 6.0,
        "carried_over must equal balance when <= cap"
    );
    assert_eq!(
        proj.forfeited_units.value, 0.0,
        "forfeited must be zero when balance <= cap"
    );
}

#[test]
fn balance_exactly_at_cap_zero_forfeiture() {
    let proj = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: 10.0,
        carry_over_cap_units: 10.0,
        ..valid_input()
    })
    .expect("balance==cap → should succeed");

    assert_eq!(proj.carried_over_units.value, 10.0);
    assert_eq!(proj.forfeited_units.value, 0.0);
}

#[test]
fn balance_above_cap_splits_correctly() {
    let proj = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: 12.0,
        statutory_min_floor_units: 5.0,
        carry_over_cap_units: 10.0,
        ..valid_input()
    })
    .expect("balance=12, cap=10 → should succeed");

    assert_eq!(
        proj.carried_over_units.value, 10.0,
        "carried_over must be capped at cap"
    );
    assert_eq!(proj.forfeited_units.value, 2.0, "forfeited = balance - cap");
}

#[test]
fn balance_far_above_cap_correct_split() {
    let proj = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: 25.0,
        statutory_min_floor_units: 3.0,
        carry_over_cap_units: 15.0,
        ..valid_input()
    })
    .expect("balance=25, cap=15 → should succeed");

    assert_eq!(proj.carried_over_units.value, 15.0);
    assert_eq!(proj.forfeited_units.value, 10.0);
}

#[test]
fn balance_below_floor_floor_granted() {
    // balance=2, floor=5, cap=10 → carried=5 (statutory minimum), forfeited=0
    let proj = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: 2.0,
        statutory_min_floor_units: 5.0,
        carry_over_cap_units: 10.0,
        ..valid_input()
    })
    .expect("balance < floor → should succeed with floor grant");

    assert_eq!(
        proj.carried_over_units.value, 5.0,
        "statutory minimum floor must be granted even when balance < floor"
    );
    assert_eq!(
        proj.forfeited_units.value, 0.0,
        "no forfeiture when below floor"
    );
}

#[test]
fn balance_zero_floor_zero_cap_zero() {
    // All three zeros: carried=0, forfeited=0
    let proj = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: 0.0,
        statutory_min_floor_units: 0.0,
        carry_over_cap_units: 0.0,
        ..valid_input()
    })
    .expect("all zeros → should succeed");

    assert_eq!(proj.carried_over_units.value, 0.0);
    assert_eq!(proj.forfeited_units.value, 0.0);
}

#[test]
fn balance_equals_floor_equals_cap_zero_forfeiture() {
    // balance=5, floor=5, cap=5 → carried=5, forfeited=0
    let proj = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: 5.0,
        statutory_min_floor_units: 5.0,
        carry_over_cap_units: 5.0,
        ..valid_input()
    })
    .expect("balance==floor==cap → should succeed");

    assert_eq!(proj.carried_over_units.value, 5.0);
    assert_eq!(proj.forfeited_units.value, 0.0);
}

#[test]
fn financial_class_on_all_unit_fields() {
    let proj = evaluate_leave_carryover_forfeiture(valid_input()).expect("projection");

    for (name, dc) in [
        (
            "closing_balance_units",
            proj.closing_balance_units
                .data_class
                .compatibility_data_class(),
        ),
        (
            "statutory_min_floor_units",
            proj.statutory_min_floor_units
                .data_class
                .compatibility_data_class(),
        ),
        (
            "carry_over_cap_units",
            proj.carry_over_cap_units
                .data_class
                .compatibility_data_class(),
        ),
        (
            "carried_over_units",
            proj.carried_over_units
                .data_class
                .compatibility_data_class(),
        ),
        (
            "forfeited_units",
            proj.forfeited_units.data_class.compatibility_data_class(),
        ),
    ] {
        assert_eq!(
            dc,
            DataClass::Financial,
            "field {name} must carry FINANCIAL data class"
        );
    }
}

#[test]
fn idempotency_key_format() {
    let proj = evaluate_leave_carryover_forfeiture(valid_input()).expect("projection");

    assert_eq!(
        proj.idempotency_key.value, "ten_acme:emp_001:2026-12-31:rulepack/kr-labor-2026",
        "idempotency_key must be tenant:emp:date:rulepack"
    );
}

#[test]
fn schema_version_is_1_and_public() {
    use data_boundary_kernel::DataClass;

    let proj = evaluate_leave_carryover_forfeiture(valid_input()).expect("projection");

    assert_eq!(proj.schema_version.value, 1);
    assert_eq!(
        proj.schema_version.data_class.compatibility_data_class(),
        DataClass::Public
    );
}
