/// Input to the leave carry-over / forfeiture period-boundary evaluator.
///
/// Distinct from `LeaveBalanceAccrualInput`: this function splits the closing
/// balance instead of hard-erroring when the cap is exceeded.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaveCarryoverForfeitureInput {
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub employee_id: String,
    pub period_boundary_date: String,
    pub closing_balance_units: f64,     // data_class: FINANCIAL
    pub statutory_min_floor_units: f64, // data_class: FINANCIAL
    pub carry_over_cap_units: f64,      // data_class: FINANCIAL
    pub rulepack_ref: String,
    pub rulepack_effective_date: String,
    pub evidence_ref: String,
    pub evaluated_at_epoch_seconds: u64,
}

/// Projection produced by `evaluate_leave_carryover_forfeiture`.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaveCarryoverForfeitureProjection {
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub employee_id: Classified<EmployeeId>,
    pub period_boundary_date: Classified<String>,
    pub closing_balance_units: Classified<f64>, // data_class: FINANCIAL
    pub statutory_min_floor_units: Classified<f64>, // data_class: FINANCIAL
    pub carry_over_cap_units: Classified<f64>,  // data_class: FINANCIAL
    pub carried_over_units: Classified<f64>,    // data_class: FINANCIAL
    pub forfeited_units: Classified<f64>,       // data_class: FINANCIAL
    pub rulepack_ref: Classified<RulepackRef>,
    pub rulepack_effective_date: Classified<RulepackEffectiveDate>,
    pub evidence_ref: Classified<AuditEvidenceRef>,
    pub idempotency_key: Classified<String>,
    pub evaluated_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

/// Pure period-boundary evaluator that splits a closing leave balance into
/// `carried_over_units` (clamped to `[statutory_min_floor, cap]`) and
/// `forfeited_units` (excess above cap).
///
/// Unlike `evaluate_leave_balance_accrual`, this function does **not** error
/// when the closing balance exceeds the cap; it forfeits the excess instead.
///
/// # Errors
///
/// - `InvalidTenantId` / `InvalidLegalEntityId` / `InvalidEmployeeId` — bad ID prefix or format.
/// - `InvalidRulepackEffectiveDate` — `period_boundary_date` or `rulepack_effective_date` not ISO-8601.
/// - `InvalidRulepackRef` — wrong prefix.
/// - `InvalidAuditEvidenceRef` — bad evidence-ref.
/// - `InvalidEvaluatedAt` — `evaluated_at_epoch_seconds` is zero.
/// - `InvalidAccrualUnits` — any f64 input is negative, NaN, or infinite.
/// - `CarryOverCapBelowFloor` — `carry_over_cap_units < statutory_min_floor_units`.
pub fn evaluate_leave_carryover_forfeiture(
    input: LeaveCarryoverForfeitureInput,
) -> Result<LeaveCarryoverForfeitureProjection, HrDomainError> {
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
    validate_iso_date(&input.period_boundary_date)?;
    validate_ref(
        &input.rulepack_ref,
        RULEPACK_REF_PREFIX,
        HrDomainError::InvalidRulepackRef,
    )?;
    validate_iso_date(&input.rulepack_effective_date)?;
    validate_evidence_ref(&input.evidence_ref)?;
    if input.evaluated_at_epoch_seconds == 0 {
        return Err(HrDomainError::InvalidEvaluatedAt);
    }

    for val in [
        input.closing_balance_units,
        input.statutory_min_floor_units,
        input.carry_over_cap_units,
    ] {
        if !val.is_finite() || val < 0.0 {
            return Err(HrDomainError::InvalidAccrualUnits);
        }
    }

    if input.carry_over_cap_units < input.statutory_min_floor_units {
        return Err(HrDomainError::CarryOverCapBelowFloor);
    }

    let carried_over_units = input
        .closing_balance_units
        .clamp(input.statutory_min_floor_units, input.carry_over_cap_units);
    let forfeited_units = (input.closing_balance_units - input.carry_over_cap_units).max(0.0);

    let idempotency_key = format!(
        "{}:{}:{}:{}",
        input.tenant_id, input.employee_id, input.period_boundary_date, input.rulepack_ref
    );

    Ok(LeaveCarryoverForfeitureProjection {
        tenant_id: internal(TenantId {
            value: input.tenant_id,
        }),
        legal_entity_id: internal(LegalEntityId {
            value: input.legal_entity_id,
        }),
        employee_id: internal(EmployeeId {
            value: input.employee_id,
        }),
        period_boundary_date: internal(input.period_boundary_date),
        closing_balance_units: Classified::new(input.closing_balance_units, DataClass::Financial),
        statutory_min_floor_units: Classified::new(
            input.statutory_min_floor_units,
            DataClass::Financial,
        ),
        carry_over_cap_units: Classified::new(input.carry_over_cap_units, DataClass::Financial),
        carried_over_units: Classified::new(carried_over_units, DataClass::Financial),
        forfeited_units: Classified::new(forfeited_units, DataClass::Financial),
        rulepack_ref: internal(RulepackRef {
            value: input.rulepack_ref,
        }),
        rulepack_effective_date: internal(RulepackEffectiveDate {
            value: input.rulepack_effective_date,
        }),
        evidence_ref: internal(AuditEvidenceRef {
            value: input.evidence_ref,
        }),
        idempotency_key: internal(idempotency_key),
        evaluated_at_epoch_seconds: internal(input.evaluated_at_epoch_seconds),
        schema_version: public(LEAVE_CARRYOVER_FORFEITURE_SCHEMA_VERSION),
    })
}
