#[test]
fn test_happy_path_balance_projection() {
    // prior=5.0 + accrual=3.0 - deduction=2.0 = 6.0; cap=10.0 (not exceeded)
    let proj = evaluate_leave_balance_accrual(valid_input()).expect("happy-path projection");

    assert_eq!(proj.tenant_id.value.value, "ten_acme");
    assert_eq!(proj.legal_entity_id.value.value, "le_kr_001");
    assert_eq!(proj.employee_id.value.value, "emp_001");
    assert_eq!(proj.payroll_period.value, "2026-06");
    assert_eq!(proj.prior_accrued_units.value, 5.0);
    assert_eq!(proj.accrual_units.value, 3.0);
    assert_eq!(proj.deduction_units.value, 2.0);
    assert_eq!(proj.resulting_balance_units.value, 6.0);
    assert_eq!(proj.carried_over_units.value, 6.0);
    assert_eq!(proj.forfeited_units.value, 0.0);
    assert_eq!(proj.carry_over_cap_units.value, 10.0);
    assert_eq!(proj.rulepack_ref.value.value, "rulepack/kr-labor-2026");
    assert_eq!(proj.rulepack_effective_date.value.value, "2026-01-01");
    assert_eq!(
        proj.idempotency_key.value,
        "ten_acme:emp_001:2026-06:rulepack/kr-labor-2026"
    );
    assert_eq!(proj.schema_version.value, 1);

    // Financial DataClass assertions
    assert_eq!(
        proj.payroll_period.data_class.compatibility_data_class(),
        DataClass::Financial
    );
    assert_eq!(
        proj.resulting_balance_units
            .data_class
            .compatibility_data_class(),
        DataClass::Financial
    );
    assert_eq!(
        proj.carried_over_units
            .data_class
            .compatibility_data_class(),
        DataClass::Financial
    );
    assert_eq!(
        proj.forfeited_units.data_class.compatibility_data_class(),
        DataClass::Financial
    );
}

#[test]
fn test_exact_carry_over_cap_accepted() {
    // after_deduction == carry_over_cap_units → Ok
    let proj = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        prior_accrued_units: 5.0,
        accrual_units: 3.0,
        deduction_units: 2.0,
        carry_over_cap_units: 6.0, // exactly equals after_deduction
        ..valid_input()
    })
    .expect("exact cap boundary must be accepted");

    assert_eq!(proj.resulting_balance_units.value, 6.0);
    assert_eq!(proj.carried_over_units.value, 6.0);
    assert_eq!(proj.forfeited_units.value, 0.0);
}

fn valid_input() -> LeaveBalanceAccrualInput {
    LeaveBalanceAccrualInput {
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        employee_id: "emp_001".to_owned(),
        payroll_period: "2026-06".to_owned(),
        prior_accrued_units: 5.0,
        accrual_units: 3.0,
        deduction_units: 2.0,
        carry_over_cap_units: 10.0,
        rulepack_ref: "rulepack/kr-labor-2026".to_owned(),
        rulepack_effective_date: "2026-01-01".to_owned(),
        accrual_evidence_ref: "audit/hr/leave-balance/emp_001/accrual".to_owned(),
        deduction_evidence_ref: "audit/hr/leave-balance/emp_001/deduction".to_owned(),
        decided_at_epoch_seconds: 1_779_532_800,
    }
}

#[test]
fn test_all_financial_output_fields_carry_financial_data_class() {
    let proj = evaluate_leave_balance_accrual(valid_input()).expect("projection");

    // Every unit field must be classified FINANCIAL per the data-boundary contract
    for (name, dc) in [
        (
            "prior_accrued_units",
            proj.prior_accrued_units
                .data_class
                .compatibility_data_class(),
        ),
        (
            "accrual_units",
            proj.accrual_units.data_class.compatibility_data_class(),
        ),
        (
            "deduction_units",
            proj.deduction_units.data_class.compatibility_data_class(),
        ),
        (
            "resulting_balance_units",
            proj.resulting_balance_units
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
        (
            "carry_over_cap_units",
            proj.carry_over_cap_units
                .data_class
                .compatibility_data_class(),
        ),
    ] {
        assert_eq!(dc, DataClass::Financial, "field {name} must be FINANCIAL");
    }
}

#[test]
fn test_decided_at_and_rulepack_date_round_trip_in_projection() {
    let proj = evaluate_leave_balance_accrual(valid_input()).expect("projection");

    assert_eq!(proj.decided_at_epoch_seconds.value, 1_779_532_800);
    assert_eq!(proj.rulepack_effective_date.value.value, "2026-01-01");
    assert_eq!(
        proj.accrual_evidence_ref.value.value,
        "audit/hr/leave-balance/emp_001/accrual"
    );
    assert_eq!(
        proj.deduction_evidence_ref.value.value,
        "audit/hr/leave-balance/emp_001/deduction"
    );
}

#[test]
fn test_zero_balance_with_zero_accrual_and_deduction_is_valid() {
    // KR LSA: an employee with no prior balance and no accrual in the period
    // must still produce a valid projection (zero carry-over, zero forfeited)
    let proj = evaluate_leave_balance_accrual(LeaveBalanceAccrualInput {
        prior_accrued_units: 0.0,
        accrual_units: 0.0,
        deduction_units: 0.0,
        carry_over_cap_units: 0.0,
        ..valid_input()
    })
    .expect("zero-balance projection must succeed");

    assert_eq!(proj.resulting_balance_units.value, 0.0);
    assert_eq!(proj.carried_over_units.value, 0.0);
    assert_eq!(proj.forfeited_units.value, 0.0);
}
