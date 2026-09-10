#[derive(Clone, Debug, PartialEq)]
pub struct LeaveBalanceAccrualInput {
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub employee_id: String,
    pub payroll_period: String,    // data_class: FINANCIAL
    pub prior_accrued_units: f64,  // data_class: FINANCIAL
    pub accrual_units: f64,        // data_class: FINANCIAL
    pub deduction_units: f64,      // data_class: FINANCIAL
    pub carry_over_cap_units: f64, // data_class: FINANCIAL
    pub rulepack_ref: String,
    pub rulepack_effective_date: String,
    pub accrual_evidence_ref: String,
    pub deduction_evidence_ref: String,
    pub decided_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeaveBalanceLedgerProjection {
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub employee_id: Classified<EmployeeId>,
    pub payroll_period: Classified<String>, // data_class: FINANCIAL
    pub prior_accrued_units: Classified<f64>, // data_class: FINANCIAL
    pub accrual_units: Classified<f64>,     // data_class: FINANCIAL
    pub deduction_units: Classified<f64>,   // data_class: FINANCIAL
    pub resulting_balance_units: Classified<f64>, // data_class: FINANCIAL
    pub carried_over_units: Classified<f64>, // data_class: FINANCIAL
    pub forfeited_units: Classified<f64>,   // data_class: FINANCIAL
    pub carry_over_cap_units: Classified<f64>, // data_class: FINANCIAL
    pub rulepack_ref: Classified<RulepackRef>,
    pub rulepack_effective_date: Classified<RulepackEffectiveDate>,
    pub accrual_evidence_ref: Classified<AuditEvidenceRef>,
    pub deduction_evidence_ref: Classified<AuditEvidenceRef>,
    pub idempotency_key: Classified<String>,
    pub decided_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

pub fn evaluate_leave_balance_accrual(
    input: LeaveBalanceAccrualInput,
) -> Result<LeaveBalanceLedgerProjection, HrDomainError> {
    validate_identifier(
        &input.tenant_id,
        TENANT_ID_PREFIX,
        HrDomainError::InvalidTenantId,
    )?;
    validate_identifier(
        &input.legal_entity_id,
        LEGAL_ENTITY_ID_PREFIX,
        HrDomainError::InvalidLegalEntityId,
    )?;
    validate_identifier(
        &input.employee_id,
        EMPLOYEE_ID_PREFIX,
        HrDomainError::InvalidEmployeeId,
    )?;
    validate_payroll_period(&input.payroll_period)?;
    validate_ref(
        &input.rulepack_ref,
        RULEPACK_REF_PREFIX,
        HrDomainError::InvalidRulepackRef,
    )?;
    validate_iso_date(&input.rulepack_effective_date)?;
    validate_evidence_ref(&input.accrual_evidence_ref)?;
    validate_evidence_ref(&input.deduction_evidence_ref)?;
    if input.decided_at_epoch_seconds == 0 {
        return Err(HrDomainError::InvalidDecisionTimestamp);
    }
    for val in [
        input.prior_accrued_units,
        input.accrual_units,
        input.deduction_units,
        input.carry_over_cap_units,
    ] {
        if !val.is_finite() || val < 0.0 {
            return Err(HrDomainError::InvalidAccrualUnits);
        }
    }

    let gross = input.prior_accrued_units + input.accrual_units;
    let after_deduction = gross - input.deduction_units;
    if after_deduction < 0.0 {
        return Err(HrDomainError::NegativeLeaveBalance);
    }
    if after_deduction > input.carry_over_cap_units {
        return Err(HrDomainError::CarryOverCapExceeded);
    }

    let resulting_balance_units = after_deduction;
    let carried_over_units = resulting_balance_units;
    let forfeited_units = 0.0_f64;

    let idempotency_key = format!(
        "{}:{}:{}:{}",
        input.tenant_id, input.employee_id, input.payroll_period, input.rulepack_ref
    );

    Ok(LeaveBalanceLedgerProjection {
        tenant_id: internal(TenantId {
            value: input.tenant_id,
        }),
        legal_entity_id: internal(LegalEntityId {
            value: input.legal_entity_id,
        }),
        employee_id: internal(EmployeeId {
            value: input.employee_id,
        }),
        payroll_period: Classified::new(input.payroll_period, DataClass::Financial),
        prior_accrued_units: Classified::new(input.prior_accrued_units, DataClass::Financial),
        accrual_units: Classified::new(input.accrual_units, DataClass::Financial),
        deduction_units: Classified::new(input.deduction_units, DataClass::Financial),
        resulting_balance_units: Classified::new(resulting_balance_units, DataClass::Financial),
        carried_over_units: Classified::new(carried_over_units, DataClass::Financial),
        forfeited_units: Classified::new(forfeited_units, DataClass::Financial),
        carry_over_cap_units: Classified::new(input.carry_over_cap_units, DataClass::Financial),
        rulepack_ref: internal(RulepackRef {
            value: input.rulepack_ref,
        }),
        rulepack_effective_date: internal(RulepackEffectiveDate {
            value: input.rulepack_effective_date,
        }),
        accrual_evidence_ref: internal(AuditEvidenceRef {
            value: input.accrual_evidence_ref,
        }),
        deduction_evidence_ref: internal(AuditEvidenceRef {
            value: input.deduction_evidence_ref,
        }),
        idempotency_key: internal(idempotency_key),
        decided_at_epoch_seconds: internal(input.decided_at_epoch_seconds),
        schema_version: public(LEAVE_BALANCE_LEDGER_SCHEMA_VERSION),
    })
}
