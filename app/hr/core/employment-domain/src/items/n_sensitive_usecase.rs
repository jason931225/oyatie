const HR_SENSITIVE_READ_TOPIC: &str = "audit.hr.sensitive-read.policy";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrSensitiveReadEnvelope {
    pub topic: data_boundary_kernel::Classified<String>,
    pub tenant_id: data_boundary_kernel::Classified<TenantId>,
    pub legal_entity_id: data_boundary_kernel::Classified<LegalEntityId>,
    pub actor_employee_id: data_boundary_kernel::Classified<EmployeeId>,
    pub subject_employee_id: data_boundary_kernel::Classified<EmployeeId>,
    pub data_kind: data_boundary_kernel::Classified<SensitiveHrDataKind>, // data_class: SENSITIVE_PIPA_ART23
    pub purpose: data_boundary_kernel::Classified<SensitiveReadPurpose>,
    pub legal_basis: data_boundary_kernel::Classified<SensitiveReadLegalBasis>,
    pub policy_ref: data_boundary_kernel::Classified<PolicyRef>,
    pub basis_evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>,
    pub consent_evidence_ref: data_boundary_kernel::Classified<Option<AuditEvidenceRef>>,
    pub request_evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>,
    pub read_log_evidence_ref: data_boundary_kernel::Classified<AuditEvidenceRef>,
    pub decision_status: data_boundary_kernel::Classified<SensitiveReadDecisionStatus>,
    pub idempotency_key: data_boundary_kernel::Classified<String>,
    pub payload_data_class: data_boundary_kernel::Classified<data_boundary_kernel::DataClass>,
    pub schema_version: data_boundary_kernel::Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SensitiveHrReadOutcome {
    pub decision: SensitiveHrReadDecision, // data_class: SENSITIVE_PIPA_ART23
    pub audit_envelope: HrSensitiveReadEnvelope, // data_class: SENSITIVE_PIPA_ART23
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

fn sensitive_read_envelope(decision: &SensitiveHrReadDecision) -> HrSensitiveReadEnvelope {
    HrSensitiveReadEnvelope {
        topic: internal(HR_SENSITIVE_READ_TOPIC.to_owned()),
        tenant_id: internal(decision.tenant_id.value.clone()),
        legal_entity_id: internal(decision.legal_entity_id.value.clone()),
        actor_employee_id: internal(decision.actor_employee_id.value.clone()),
        subject_employee_id: internal(decision.subject_employee_id.value.clone()),
        data_kind: data_boundary_kernel::Classified::new(
            decision.data_kind.value,
            data_boundary_kernel::DataClass::SensitivePipaArticle23,
        ),
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

fn sensitive_read_payload_data_class(
    data_kind: SensitiveHrDataKind,
) -> data_boundary_kernel::DataClass {
    match data_kind {
        SensitiveHrDataKind::Medical | SensitiveHrDataKind::DisabilityAccommodation => {
            data_boundary_kernel::DataClass::Phi
        }
        SensitiveHrDataKind::Compensation => data_boundary_kernel::DataClass::Financial,
        SensitiveHrDataKind::GovernmentIdentifier => {
            data_boundary_kernel::DataClass::PiiIdentifying
        }
        SensitiveHrDataKind::Disciplinary => {
            data_boundary_kernel::DataClass::SensitivePipaArticle23
        }
    }
}
