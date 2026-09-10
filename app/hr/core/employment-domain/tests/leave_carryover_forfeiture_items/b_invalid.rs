#[test]
fn cap_below_floor_returns_error() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        statutory_min_floor_units: 5.0,
        carry_over_cap_units: 3.0, // cap < floor
        ..valid_input()
    })
    .expect_err("cap < floor must return CarryOverCapBelowFloor");

    assert_eq!(err, HrDomainError::CarryOverCapBelowFloor);
}

#[test]
fn negative_closing_balance_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: -1.0,
        ..valid_input()
    })
    .expect_err("negative closing_balance_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn nan_closing_balance_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        closing_balance_units: f64::NAN,
        ..valid_input()
    })
    .expect_err("NaN closing_balance_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn negative_floor_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        statutory_min_floor_units: -0.5,
        ..valid_input()
    })
    .expect_err("negative statutory_min_floor_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn nan_floor_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        statutory_min_floor_units: f64::NAN,
        ..valid_input()
    })
    .expect_err("NaN statutory_min_floor_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn negative_cap_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        carry_over_cap_units: -1.0,
        ..valid_input()
    })
    .expect_err("negative carry_over_cap_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn infinite_cap_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        carry_over_cap_units: f64::INFINITY,
        ..valid_input()
    })
    .expect_err("infinite carry_over_cap_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn invalid_tenant_id_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        tenant_id: "acme".to_owned(),
        ..valid_input()
    })
    .expect_err("tenant_id without ten_ prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidTenantId);
}

#[test]
fn invalid_legal_entity_id_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        legal_entity_id: "kr_001".to_owned(),
        ..valid_input()
    })
    .expect_err("legal_entity_id without le_ prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidLegalEntityId);
}

#[test]
fn invalid_employee_id_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        employee_id: "001".to_owned(),
        ..valid_input()
    })
    .expect_err("employee_id without emp_ prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidEmployeeId);
}

#[test]
fn invalid_rulepack_ref_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        rulepack_ref: "policy/kr-labor-2026".to_owned(),
        ..valid_input()
    })
    .expect_err("rulepack_ref with wrong prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidRulepackRef);
}

#[test]
fn invalid_period_boundary_date_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        period_boundary_date: "31-12-2026".to_owned(),
        ..valid_input()
    })
    .expect_err("period_boundary_date not YYYY-MM-DD must be rejected");

    assert_eq!(err, HrDomainError::InvalidRulepackEffectiveDate);
}

#[test]
fn zero_evaluated_at_rejected() {
    let err = evaluate_leave_carryover_forfeiture(LeaveCarryoverForfeitureInput {
        evaluated_at_epoch_seconds: 0,
        ..valid_input()
    })
    .expect_err("zero evaluated_at_epoch_seconds must be rejected");

    assert_eq!(err, HrDomainError::InvalidEvaluatedAt);
}
