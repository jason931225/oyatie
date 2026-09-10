#[test]
fn test_carry_over_cap_exceeded_returns_error() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        prior_accrued_units: 5.0,
        accrual_units: 3.0,
        deduction_units: 2.0,
        carry_over_cap_units: 5.9, // after_deduction=6.0 > cap=5.9
        ..valid_input()
    })
    .expect_err("over-cap must return CarryOverCapExceeded");

    assert_eq!(err, HrDomainError::CarryOverCapExceeded);
}

#[test]
fn test_negative_balance_returns_error() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        prior_accrued_units: 1.0,
        accrual_units: 0.0,
        deduction_units: 5.0, // deduction > gross → negative
        ..valid_input()
    })
    .expect_err("negative result must return NegativeLeaveBalance");

    assert_eq!(err, HrDomainError::NegativeLeaveBalance);
}

#[test]
fn test_invalid_accrual_units_negative() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        accrual_units: -1.0,
        ..valid_input()
    })
    .expect_err("negative accrual_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn test_invalid_prior_accrued_units_nan() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        prior_accrued_units: f64::NAN,
        ..valid_input()
    })
    .expect_err("NaN prior_accrued_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn test_invalid_evidence_ref_rejected() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        accrual_evidence_ref: "audit/".to_owned(), // empty suffix
        ..valid_input()
    })
    .expect_err("empty evidence ref suffix must be rejected");

    assert_eq!(err, HrDomainError::InvalidAuditEvidenceRef);
}

#[test]
fn test_invalid_rulepack_ref_rejected() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        rulepack_ref: "policy/kr-labor".to_owned(), // wrong prefix
        ..valid_input()
    })
    .expect_err("wrong rulepack prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidRulepackRef);
}

#[test]
fn test_invalid_tenant_id_rejected() {
    // missing "ten_" prefix
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        tenant_id: "acme".to_owned(),
        ..valid_input()
    })
    .expect_err("tenant_id without ten_ prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidTenantId);
}

#[test]
fn test_invalid_legal_entity_id_rejected() {
    // missing "le_" prefix
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        legal_entity_id: "kr_001".to_owned(),
        ..valid_input()
    })
    .expect_err("legal_entity_id without le_ prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidLegalEntityId);
}

#[test]
fn test_invalid_employee_id_rejected() {
    // missing "emp_" prefix
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        employee_id: "001".to_owned(),
        ..valid_input()
    })
    .expect_err("employee_id without emp_ prefix must be rejected");

    assert_eq!(err, HrDomainError::InvalidEmployeeId);
}

#[test]
fn test_zero_decided_at_returns_invalid_decision_timestamp() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        decided_at_epoch_seconds: 0,
        ..valid_input()
    })
    .expect_err("zero decided_at_epoch_seconds must return InvalidDecisionTimestamp");

    assert_eq!(err, HrDomainError::InvalidDecisionTimestamp);
}

#[test]
fn test_invalid_payroll_period_returns_error() {
    // month 99 is not a valid calendar month
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        payroll_period: "2026-99".to_owned(),
        ..valid_input()
    })
    .expect_err("payroll_period with invalid month must return InvalidPayrollPeriod");

    assert_eq!(err, HrDomainError::InvalidPayrollPeriod);
}

#[test]
fn test_invalid_rulepack_effective_date_returns_error() {
    // not ISO-8601 YYYY-MM-DD format
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        rulepack_effective_date: "01-01-2026".to_owned(),
        ..valid_input()
    })
    .expect_err("rulepack_effective_date not in YYYY-MM-DD format must be rejected");

    assert_eq!(err, HrDomainError::InvalidRulepackEffectiveDate);
}

#[test]
fn test_deduction_evidence_ref_empty_suffix_rejected() {
    // deduction_evidence_ref with only the prefix and no suffix
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        deduction_evidence_ref: "audit/".to_owned(),
        ..valid_input()
    })
    .expect_err("deduction_evidence_ref with empty suffix must be rejected");

    assert_eq!(err, HrDomainError::InvalidAuditEvidenceRef);
}

#[test]
fn test_deduction_evidence_ref_credential_like_rejected() {
    // evidence refs that look like credentials must be rejected
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        deduction_evidence_ref: "audit/hr/leave-balance/bearer-token".to_owned(),
        ..valid_input()
    })
    .expect_err("deduction_evidence_ref containing 'bearer' must be rejected");

    assert_eq!(err, HrDomainError::InvalidAuditEvidenceRef);
}

#[test]
fn test_infinite_carry_over_cap_returns_invalid_accrual_units() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        carry_over_cap_units: f64::INFINITY,
        ..valid_input()
    })
    .expect_err("infinite carry_over_cap_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn test_negative_carry_over_cap_returns_invalid_accrual_units() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        carry_over_cap_units: -0.1,
        ..valid_input()
    })
    .expect_err("negative carry_over_cap_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn test_infinite_accrual_units_returns_invalid_accrual_units() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        accrual_units: f64::INFINITY,
        ..valid_input()
    })
    .expect_err("infinite accrual_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}

#[test]
fn test_negative_deduction_units_returns_invalid_accrual_units() {
    let err = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        deduction_units: -1.0,
        ..valid_input()
    })
    .expect_err("negative deduction_units must return InvalidAccrualUnits");

    assert_eq!(err, HrDomainError::InvalidAccrualUnits);
}
