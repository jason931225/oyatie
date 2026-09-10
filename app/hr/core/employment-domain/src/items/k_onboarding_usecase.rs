const HR_LIFECYCLE_TOPIC: &str = "audit.hr.employment.lifecycle";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrAuditEnvelope {
    pub topic: data_boundary_kernel::Classified<String>,
    pub tenant_id: data_boundary_kernel::Classified<TenantId>,
    pub legal_entity_id: data_boundary_kernel::Classified<LegalEntityId>,
    pub aggregate_ref: data_boundary_kernel::Classified<String>,
    pub evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>,
    pub payload_kind: data_boundary_kernel::Classified<String>,
    pub idempotency_key: data_boundary_kernel::Classified<String>,
    pub payload_data_class: data_boundary_kernel::Classified<data_boundary_kernel::DataClass>,
    pub schema_version: data_boundary_kernel::Classified<u32>,
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

fn lifecycle_audit_envelope(event: &EmployeeLifecycleEvent) -> HrAuditEnvelope {
    HrAuditEnvelope {
        topic: internal(HR_LIFECYCLE_TOPIC.to_owned()),
        tenant_id: internal(event.tenant_id.value.clone()),
        legal_entity_id: internal(event.legal_entity_id.value.clone()),
        aggregate_ref: internal(format!("hr/employee/{}", event.employee_id.value.value)),
        evidence_ref: internal(event.audit_evidence_ref.value.clone()),
        payload_kind: internal(format!("{:?}", event.lifecycle_kind.value)),
        idempotency_key: internal(event.idempotency_key.value.clone()),
        payload_data_class: internal(data_boundary_kernel::DataClass::PiiIdentifying),
        schema_version: public(1),
    }
}
