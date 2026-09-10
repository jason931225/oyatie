//! HR employment application layer.
//!
//! This crate orchestrates the pure HR employment domain into metadata-only
//! audit and Workflow dispatch envelopes for later cloud/runtime adapters. It
//! does not persist data, call Workflow, emit audit-chain records, or perform
//! network I/O.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use hr_employment_domain::{
    AuditEvidenceRef, Employee, EmployeeCreate, EmployeeId, EmployeeLifecycleEvent, HrDomainError,
    HrLifecycleKind, Jurisdiction, LaborComplianceObligation, LaborComplianceObligationKind,
    LaborComplianceWorkflowStep, LeaveDecision, LeavePayrollImpactInput, LeavePayrollImpactPlan,
    LeaveRequestId, LeaveRoutingMode, LegalEntityId, LegalEntityWorkforceSnapshot,
    PayrollImpactKind, PolicyRef, SensitiveHrDataKind, SensitiveHrReadDecision,
    SensitiveHrReadInput, SensitiveReadDecisionStatus, SensitiveReadLegalBasis,
    SensitiveReadPurpose, TenantId, WorkflowRef, evaluate_labor_compliance,
    evaluate_sensitive_hr_read, plan_leave_payroll_impact,
};

const HR_LIFECYCLE_TOPIC: &str = "audit.hr.employment.lifecycle";
const HR_COMPLIANCE_WORKFLOW_TOPIC: &str = "workflow.hr.compliance.dispatch";
const HR_LEAVE_PAYROLL_IMPACT_TOPIC: &str = "integration.hr.payroll.leave-impact";
const HR_SENSITIVE_READ_TOPIC: &str = "audit.hr.sensitive-read.policy";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrAuditEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub aggregate_ref: Classified<String>,
    pub evidence_ref: Classified<AuditEvidenceRef>,
    pub payload_kind: Classified<String>,
    pub idempotency_key: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrWorkflowDispatchEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub workflow_ref: Classified<WorkflowRef>,
    pub obligation_kind: Classified<LaborComplianceObligationKind>,
    pub jurisdiction: Classified<Jurisdiction>,
    pub required_steps: Classified<Vec<LaborComplianceWorkflowStep>>,
    pub evidence_refs: Classified<Vec<AuditEvidenceRef>>,
    pub idempotency_key: Classified<String>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrLeavePayrollImpactEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub employee_id: Classified<EmployeeId>,
    pub leave_request_id: Classified<LeaveRequestId>,
    pub approver_id: Classified<EmployeeId>,
    pub decision: Classified<LeaveDecision>,
    pub routing_mode: Classified<LeaveRoutingMode>,
    pub workflow_ref: Classified<WorkflowRef>,
    pub payroll_period: Classified<String>, // data_class: FINANCIAL
    pub payroll_impact_kind: Classified<PayrollImpactKind>, // data_class: FINANCIAL
    pub decision_evidence_ref: Classified<AuditEvidenceRef>,
    pub routing_evidence_ref: Classified<AuditEvidenceRef>,
    pub payroll_impact_evidence_ref: Classified<AuditEvidenceRef>, // data_class: FINANCIAL
    pub idempotency_key: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrSensitiveReadEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub actor_employee_id: Classified<EmployeeId>,
    pub subject_employee_id: Classified<EmployeeId>,
    pub data_kind: Classified<SensitiveHrDataKind>, // data_class: SENSITIVE_PIPA_ART23
    pub purpose: Classified<SensitiveReadPurpose>,
    pub legal_basis: Classified<SensitiveReadLegalBasis>,
    pub policy_ref: Classified<PolicyRef>,
    pub basis_evidence_ref: Classified<AuditEvidenceRef>,
    pub consent_evidence_ref: Classified<Option<AuditEvidenceRef>>,
    pub request_evidence_ref: Classified<AuditEvidenceRef>,
    pub read_log_evidence_ref: Classified<AuditEvidenceRef>,
    pub decision_status: Classified<SensitiveReadDecisionStatus>,
    pub idempotency_key: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardEmployeeCommand {
    pub employee: EmployeeCreate, // data_class: PII_IDENTIFYING
    pub event_id: String,
    pub lifecycle_kind: HrLifecycleKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OnboardEmployeeOutcome {
    pub employee: Employee, // data_class: PII_IDENTIFYING
    pub lifecycle_event: EmployeeLifecycleEvent,
    pub audit_envelope: HrAuditEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaborWorkflowPlanOutcome {
    pub obligations: Vec<LaborComplianceObligation>,
    pub workflow_dispatches: Vec<HrWorkflowDispatchEnvelope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeavePayrollImpactOutcome {
    pub plan: LeavePayrollImpactPlan, // data_class: FINANCIAL
    pub payroll_impact_envelope: HrLeavePayrollImpactEnvelope, // data_class: FINANCIAL
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SensitiveHrReadOutcome {
    pub decision: SensitiveHrReadDecision, // data_class: SENSITIVE_PIPA_ART23
    pub audit_envelope: HrSensitiveReadEnvelope, // data_class: SENSITIVE_PIPA_ART23
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HrAppError {
    Domain(HrDomainError),
}

impl From<HrDomainError> for HrAppError {
    fn from(error: HrDomainError) -> Self {
        Self::Domain(error)
    }
}

pub fn onboard_employee(
    command: OnboardEmployeeCommand,
) -> Result<OnboardEmployeeOutcome, HrAppError> {
    let employee = Employee::new(command.employee)?;
    let lifecycle_event = employee.lifecycle_event(&command.event_id, command.lifecycle_kind)?;
    let audit_envelope = lifecycle_audit_envelope(&lifecycle_event);
    Ok(OnboardEmployeeOutcome {
        employee,
        lifecycle_event,
        audit_envelope,
    })
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

pub fn prepare_sensitive_hr_read_envelope(
    input: SensitiveHrReadInput,
) -> Result<SensitiveHrReadOutcome, HrAppError> {
    let decision = evaluate_sensitive_hr_read(input)?;
    let audit_envelope = sensitive_read_envelope(&decision);
    Ok(SensitiveHrReadOutcome {
        decision,
        audit_envelope,
    })
}

fn lifecycle_audit_envelope(event: &EmployeeLifecycleEvent) -> HrAuditEnvelope {
    HrAuditEnvelope {
        topic: internal(HR_LIFECYCLE_TOPIC.to_owned()),
        tenant_id: internal(event.tenant_id.value.clone()),
        legal_entity_id: internal(event.legal_entity_id.value.clone()),
        aggregate_ref: internal(format!("hr/employee/{}", event.employee_id.value.value)),
        evidence_ref: internal(event.audit_evidence_ref.value.clone()),
        payload_kind: internal(format!("{:?}", event.lifecycle_kind.value)),
        idempotency_key: internal(event.idempotency_key.value.clone()),
        payload_data_class: internal(DataClass::PiiIdentifying),
        schema_version: public(1),
    }
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
        payroll_period: Classified::new(plan.payroll_period.value.clone(), DataClass::Financial),
        payroll_impact_kind: Classified::new(plan.payroll_impact_kind.value, DataClass::Financial),
        decision_evidence_ref: internal(plan.decision_evidence_ref.value.clone()),
        routing_evidence_ref: internal(plan.routing_evidence_ref.value.clone()),
        payroll_impact_evidence_ref: Classified::new(
            plan.payroll_impact_evidence_ref.value.clone(),
            DataClass::Financial,
        ),
        idempotency_key: internal(plan.idempotency_key.value.clone()),
        payload_data_class: internal(DataClass::Financial),
        schema_version: public(1),
    }
}

fn sensitive_read_envelope(decision: &SensitiveHrReadDecision) -> HrSensitiveReadEnvelope {
    HrSensitiveReadEnvelope {
        topic: internal(HR_SENSITIVE_READ_TOPIC.to_owned()),
        tenant_id: internal(decision.tenant_id.value.clone()),
        legal_entity_id: internal(decision.legal_entity_id.value.clone()),
        actor_employee_id: internal(decision.actor_employee_id.value.clone()),
        subject_employee_id: internal(decision.subject_employee_id.value.clone()),
        data_kind: Classified::new(decision.data_kind.value, DataClass::SensitivePipaArticle23),
        purpose: internal(decision.purpose.value),
        legal_basis: internal(decision.legal_basis.value),
        policy_ref: internal(decision.policy_ref.value.clone()),
        basis_evidence_ref: internal(decision.basis_evidence_ref.value.clone()),
        consent_evidence_ref: internal(decision.consent_evidence_ref.value.clone()),
        request_evidence_ref: internal(decision.request_evidence_ref.value.clone()),
        read_log_evidence_ref: internal(decision.read_log_evidence_ref.value.clone()),
        decision_status: internal(decision.decision_status.value),
        idempotency_key: internal(decision.idempotency_key.value.clone()),
        payload_data_class: internal(sensitive_read_payload_data_class(decision.data_kind.value)),
        schema_version: public(1),
    }
}

fn sensitive_read_payload_data_class(data_kind: SensitiveHrDataKind) -> DataClass {
    match data_kind {
        SensitiveHrDataKind::Medical | SensitiveHrDataKind::DisabilityAccommodation => {
            DataClass::Phi
        }
        SensitiveHrDataKind::Compensation => DataClass::Financial,
        SensitiveHrDataKind::GovernmentIdentifier => DataClass::PiiIdentifying,
        SensitiveHrDataKind::Disciplinary => DataClass::SensitivePipaArticle23,
    }
}

fn internal<T>(value: T) -> Classified<T> {
    Classified::new(value, PrivacyDataClass::internal_only())
}

fn public<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::Public)
}
