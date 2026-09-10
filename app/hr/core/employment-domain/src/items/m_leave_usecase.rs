const HR_LEAVE_PAYROLL_IMPACT_TOPIC: &str = "integration.hr.payroll.leave-impact";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrLeavePayrollImpactEnvelope {
    pub topic: data_boundary_kernel::Classified<String>,
    pub tenant_id: data_boundary_kernel::Classified<TenantId>,
    pub legal_entity_id: data_boundary_kernel::Classified<LegalEntityId>,
    pub employee_id: data_boundary_kernel::Classified<EmployeeId>,
    pub leave_request_id: data_boundary_kernel::Classified<LeaveRequestId>,
    pub approver_id: data_boundary_kernel::Classified<EmployeeId>,
    pub decision: data_boundary_kernel::Classified<LeaveDecision>,
    pub routing_mode: data_boundary_kernel::Classified<LeaveRoutingMode>,
    pub workflow_ref: data_boundary_kernel::Classified<WorkflowRef>,
    pub payroll_period: data_boundary_kernel::Classified<String>, // data_class: FINANCIAL
    pub payroll_impact_kind: data_boundary_kernel::Classified<PayrollImpactKind>, // data_class: FINANCIAL
    pub decision_evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>,
    pub routing_evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>,
    pub payroll_impact_evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>, // data_class: FINANCIAL
    pub idempotency_key: data_boundary_kernel::Classified<String>,
    pub payload_data_class: data_boundary_kernel::Classified<data_boundary_kernel::DataClass>,
    pub schema_version: data_boundary_kernel::Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeavePayrollImpactOutcome {
    pub plan: LeavePayrollImpactPlan, // data_class: FINANCIAL
    pub payroll_impact_envelope: HrLeavePayrollImpactEnvelope, // data_class: FINANCIAL
}

pub fn plan_leave_payroll_impact_envelope(
    input: LeavePayrollImpactInput,
) -> Result<LeavePayrollImpactOutcome, HrAppError> {
    let plan = plan_leave_payroll_impact(input)?;
    let payroll_impact_envelope = leave_payroll_impact_envelope(&plan);

    Ok(LeavePayrollImpactOutcome {
        plan,
        payroll_impact_envelope,
    })
}

fn leave_payroll_impact_envelope(plan: &LeavePayrollImpactPlan) -> HrLeavePayrollImpactEnvelope {
    HrLeavePayrollImpactEnvelope {
        topic: internal(HR_LEAVE_PAYROLL_IMPACT_TOPIC.to_owned()),
        tenant_id: internal(plan.tenant_id.value.clone()),
        legal_entity_id: internal(plan.legal_entity_id.value.clone()),
        employee_id: internal(plan.employee_id.value.clone()),
        leave_request_id: internal(plan.leave_request_id.value.clone()),
        approver_id: internal(plan.approver_id.value.clone()),
        decision: internal(plan.decision.value),
        routing_mode: internal(plan.routing_mode.value),
        workflow_ref: internal(plan.workflow_ref.value.clone()),
        payroll_period: data_boundary_kernel::Classified::new(
            plan.payroll_period.value.clone(),
            data_boundary_kernel::DataClass::Financial,
        ),
        payroll_impact_kind: data_boundary_kernel::Classified::new(
            plan.payroll_impact_kind.value,
            data_boundary_kernel::DataClass::Financial,
        ),
        decision_evidence_ref: internal(plan.decision_evidence_ref.value.clone()),
        routing_evidence_ref: internal(plan.routing_evidence_ref.value.clone()),
        payroll_impact_evidence_ref: data_boundary_kernel::Classified::new(
            plan.payroll_impact_evidence_ref.value.clone(),
            data_boundary_kernel::DataClass::Financial,
        ),
        idempotency_key: internal(plan.idempotency_key.value.clone()),
        payload_data_class: internal(data_boundary_kernel::DataClass::Financial),
        schema_version: public(1),
    }
}
