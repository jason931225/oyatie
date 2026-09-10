const HR_COMPLIANCE_WORKFLOW_TOPIC: &str = "workflow.hr.compliance.dispatch";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrWorkflowDispatchEnvelope {
    pub topic: data_boundary_kernel::Classified<String>,
    pub tenant_id: data_boundary_kernel::Classified<TenantId>,
    pub legal_entity_id: data_boundary_kernel::Classified<LegalEntityId>,
    pub workflow_ref: data_boundary_kernel::Classified<WorkflowRef>,
    pub obligation_kind: data_boundary_kernel::Classified<LaborComplianceObligationKind>,
    pub jurisdiction: data_boundary_kernel::Classified<Jurisdiction>,
    pub required_steps: data_boundary_kernel::Classified<Vec<LaborComplianceWorkflowStep>>,
    pub evidence_refs: data_boundary_kernel::Classified<Vec<AuditEvidenceRef>>,
    pub idempotency_key: data_boundary_kernel::Classified<String>,
    pub schema_version: data_boundary_kernel::Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaborWorkflowPlanOutcome {
    pub obligations: Vec<LaborComplianceObligation>,
    pub workflow_dispatches: Vec<HrWorkflowDispatchEnvelope>,
}

pub fn plan_labor_compliance_workflows(
    snapshot: LegalEntityWorkforceSnapshot,
) -> Result<LaborWorkflowPlanOutcome, HrAppError> {
    let obligations = evaluate_labor_compliance(snapshot)?;
    let workflow_dispatches = obligations.iter().map(workflow_dispatch).collect();

    Ok(LaborWorkflowPlanOutcome {
        obligations,
        workflow_dispatches,
    })
}

fn workflow_dispatch(obligation: &LaborComplianceObligation) -> HrWorkflowDispatchEnvelope {
    HrWorkflowDispatchEnvelope {
        topic: internal(HR_COMPLIANCE_WORKFLOW_TOPIC.to_owned()),
        tenant_id: internal(obligation.tenant_id.value.clone()),
        legal_entity_id: internal(obligation.legal_entity_id.value.clone()),
        workflow_ref: internal(obligation.workflow_ref.value.clone()),
        obligation_kind: internal(obligation.kind.value),
        jurisdiction: internal(obligation.jurisdiction.value),
        required_steps: internal(obligation.workflow_steps.value.clone()),
        evidence_refs: internal(obligation.evidence_paths.value.clone()),
        idempotency_key: internal(obligation.idempotency_key.value.clone()),
        schema_version: public(1),
    }
}
