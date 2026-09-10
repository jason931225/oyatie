#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Jurisdiction {
    Korea,
    UnitedStates,
    EuropeanUnion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LaborComplianceObligationKind {
    KoreaRulesOfEmployment,
    KoreaLaborManagementCouncil,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LaborComplianceWorkflowStep {
    Drafted,
    EmployeeReviewSent,
    MajorityConsentObtained,
    MoelFiled,
    CouncilRosterRequired,
    MeetingCadenceRequired,
    MinutesEvidenceRequired,
    Active,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LaborComplianceObligationState {
    Open,
    Active,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegalEntityWorkforceSnapshot {
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub jurisdiction: Jurisdiction,
    pub active_employee_count: u32,
    pub rulepack_ref: String,
    pub rulepack_effective_date: String,
    pub workflow_ref: String,
    pub evidence_ref: String,
    pub evaluated_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaborComplianceObligation {
    pub obligation_id: Classified<LaborComplianceObligationId>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub jurisdiction: Classified<Jurisdiction>,
    pub kind: Classified<LaborComplianceObligationKind>,
    pub state: Classified<LaborComplianceObligationState>,
    pub threshold_employee_count: Classified<u32>,
    pub active_employee_count: Classified<u32>,
    pub rulepack_ref: Classified<RulepackRef>,
    pub rulepack_effective_date: Classified<RulepackEffectiveDate>,
    pub workflow_ref: Classified<WorkflowRef>,
    pub workflow_steps: Classified<Vec<LaborComplianceWorkflowStep>>,
    pub evidence_paths: Classified<Vec<AuditEvidenceRef>>,
    pub idempotency_key: Classified<String>,
    pub evaluated_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

pub fn evaluate_labor_compliance(
    snapshot: LegalEntityWorkforceSnapshot,
) -> Result<Vec<LaborComplianceObligation>, HrDomainError> {
    validate_snapshot(&snapshot)?;
    if snapshot.jurisdiction != Jurisdiction::Korea {
        return Ok(Vec::new());
    }

    let mut obligations = Vec::new();
    if snapshot.active_employee_count >= 10 {
        obligations.push(build_obligation(
            &snapshot,
            LaborComplianceObligationKind::KoreaRulesOfEmployment,
            10,
            vec![
                LaborComplianceWorkflowStep::Drafted,
                LaborComplianceWorkflowStep::EmployeeReviewSent,
                LaborComplianceWorkflowStep::MajorityConsentObtained,
                LaborComplianceWorkflowStep::MoelFiled,
                LaborComplianceWorkflowStep::Active,
            ],
            "moel/rules-of-employment/report",
        ));
    }
    if snapshot.active_employee_count >= 30 {
        obligations.push(build_obligation(
            &snapshot,
            LaborComplianceObligationKind::KoreaLaborManagementCouncil,
            30,
            vec![
                LaborComplianceWorkflowStep::CouncilRosterRequired,
                LaborComplianceWorkflowStep::MeetingCadenceRequired,
                LaborComplianceWorkflowStep::MinutesEvidenceRequired,
                LaborComplianceWorkflowStep::Active,
            ],
            "moel/labor-management-council/minutes",
        ));
    }
    Ok(obligations)
}

fn build_obligation(
    snapshot: &LegalEntityWorkforceSnapshot,
    kind: LaborComplianceObligationKind,
    threshold_employee_count: u32,
    workflow_steps: Vec<LaborComplianceWorkflowStep>,
    evidence_suffix: &str,
) -> LaborComplianceObligation {
    let obligation_kind_key = obligation_kind_key(kind);
    let obligation_id = format!(
        "{LABOR_OBLIGATION_ID_PREFIX}{}_{}_{}",
        snapshot.legal_entity_id, obligation_kind_key, snapshot.rulepack_effective_date
    );
    let idempotency_key = format!(
        "{}:{}:{}:{}",
        snapshot.tenant_id,
        snapshot.legal_entity_id,
        obligation_kind_key,
        snapshot.rulepack_effective_date
    );
    let evidence_paths = vec![
        AuditEvidenceRef {
            value: snapshot.evidence_ref.clone(),
        },
        AuditEvidenceRef {
            value: format!("audit/{}/{evidence_suffix}", snapshot.legal_entity_id),
        },
    ];
    LaborComplianceObligation {
        obligation_id: internal(LaborComplianceObligationId {
            value: obligation_id,
        }),
        tenant_id: internal(TenantId {
            value: snapshot.tenant_id.clone(),
        }),
        legal_entity_id: internal(LegalEntityId {
            value: snapshot.legal_entity_id.clone(),
        }),
        jurisdiction: internal(snapshot.jurisdiction),
        kind: internal(kind),
        state: internal(LaborComplianceObligationState::Open),
        threshold_employee_count: internal(threshold_employee_count),
        active_employee_count: internal(snapshot.active_employee_count),
        rulepack_ref: internal(RulepackRef {
            value: snapshot.rulepack_ref.clone(),
        }),
        rulepack_effective_date: internal(RulepackEffectiveDate {
            value: snapshot.rulepack_effective_date.clone(),
        }),
        workflow_ref: internal(WorkflowRef {
            value: snapshot.workflow_ref.clone(),
        }),
        workflow_steps: internal(workflow_steps),
        evidence_paths: internal(evidence_paths),
        idempotency_key: internal(idempotency_key),
        evaluated_at_epoch_seconds: internal(snapshot.evaluated_at_epoch_seconds),
        schema_version: public(LABOR_OBLIGATION_SCHEMA_VERSION),
    }
}

fn validate_snapshot(snapshot: &LegalEntityWorkforceSnapshot) -> Result<(), HrDomainError> {
    validate_identifier(
        &snapshot.tenant_id,
        TENANT_ID_PREFIX,
        HrDomainError::InvalidTenantId,
    )?;
    validate_identifier(
        &snapshot.legal_entity_id,
        LEGAL_ENTITY_ID_PREFIX,
        HrDomainError::InvalidLegalEntityId,
    )?;
    validate_ref(
        &snapshot.rulepack_ref,
        RULEPACK_REF_PREFIX,
        HrDomainError::InvalidRulepackRef,
    )?;
    validate_iso_date(&snapshot.rulepack_effective_date)?;
    validate_ref(
        &snapshot.workflow_ref,
        WORKFLOW_REF_PREFIX,
        HrDomainError::InvalidWorkflowRef,
    )?;
    validate_evidence_ref(&snapshot.evidence_ref)?;
    if snapshot.evaluated_at_epoch_seconds == 0 {
        return Err(HrDomainError::InvalidEvaluatedAt);
    }
    Ok(())
}

fn obligation_kind_key(kind: LaborComplianceObligationKind) -> &'static str {
    match kind {
        LaborComplianceObligationKind::KoreaRulesOfEmployment => "korea_rules_of_employment",
        LaborComplianceObligationKind::KoreaLaborManagementCouncil => {
            "korea_labor_management_council"
        }
    }
}
