#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EmploymentStatus {
    Draft,
    Active,
    Suspended,
    Terminated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TenantTierSnapshot {
    SmbSelfServe,
    EnterpriseSingleEntity,
    EnterpriseGroup,
    RegulatedEnterprise,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum HrLifecycleKind {
    Created,
    Updated,
    Suspended,
    Terminated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmployeeCreate {
    pub employee_id: String,
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub person_ref: String, // data_class: PII_IDENTIFYING
    pub manager_id: Option<String>,
    pub employment_status: EmploymentStatus,
    pub tenant_tier_snapshot: TenantTierSnapshot,
    pub audit_evidence_ref: String,
    pub data_class: Option<PrivacyDataClass>,
    pub version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Employee {
    pub employee_id: Classified<EmployeeId>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub person_ref: Classified<PersonRef>, // data_class: PII_IDENTIFYING
    pub manager_id: Classified<Option<EmployeeId>>,
    pub employment_status: Classified<EmploymentStatus>,
    pub tenant_tier_snapshot: Classified<TenantTierSnapshot>,
    pub audit_evidence_ref: Classified<AuditEvidenceRef>,
    pub data_class: Classified<PrivacyDataClass>,
    pub version: Classified<u32>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmployeeLifecycleEvent {
    pub event_id: Classified<HrEventId>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub employee_id: Classified<EmployeeId>,
    pub lifecycle_kind: Classified<HrLifecycleKind>,
    pub audit_evidence_ref: Classified<AuditEvidenceRef>,
    pub idempotency_key: Classified<String>,
    pub schema_version: Classified<u32>,
}

impl Employee {
    pub fn new(input: EmployeeCreate) -> Result<Self, HrDomainError> {
        validate_identifier(
            &input.employee_id,
            EMPLOYEE_ID_PREFIX,
            HrDomainError::InvalidEmployeeId,
        )?;
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
        validate_ref(
            &input.person_ref,
            PERSON_REF_PREFIX,
            HrDomainError::InvalidPersonRef,
        )?;
        validate_evidence_ref(&input.audit_evidence_ref)?;
        if input.version == 0 {
            return Err(HrDomainError::InvalidVersion);
        }
        let manager_id = input
            .manager_id
            .map(|manager_id| employee_id(&manager_id))
            .transpose()?;
        let data_class = input
            .data_class
            .unwrap_or(PrivacyDataClass::pii_identifying());
        if data_class.data_class() != DataClass::PiiIdentifying {
            return Err(HrDomainError::InvalidDataClass);
        }
        Ok(Self {
            employee_id: internal(EmployeeId {
                value: input.employee_id,
            }),
            tenant_id: internal(TenantId {
                value: input.tenant_id,
            }),
            legal_entity_id: internal(LegalEntityId {
                value: input.legal_entity_id,
            }),
            person_ref: Classified::new(
                PersonRef {
                    value: input.person_ref,
                },
                PrivacyDataClass::pii_identifying(),
            ),
            manager_id: internal(manager_id),
            employment_status: internal(input.employment_status),
            tenant_tier_snapshot: internal(input.tenant_tier_snapshot),
            audit_evidence_ref: internal(AuditEvidenceRef {
                value: input.audit_evidence_ref,
            }),
            data_class: internal(data_class),
            version: internal(input.version),
            schema_version: public(EMPLOYEE_SCHEMA_VERSION),
        })
    }

    pub fn lifecycle_event(
        &self,
        event_id: &str,
        lifecycle_kind: HrLifecycleKind,
    ) -> Result<EmployeeLifecycleEvent, HrDomainError> {
        validate_identifier(
            event_id,
            HR_EVENT_ID_PREFIX,
            HrDomainError::InvalidHrEventId,
        )?;
        let idempotency_key = format!("{}:{}", self.employee_id.value.value, self.version.value);
        Ok(EmployeeLifecycleEvent {
            event_id: internal(HrEventId {
                value: event_id.to_owned(),
            }),
            tenant_id: internal(self.tenant_id.value.clone()),
            legal_entity_id: internal(self.legal_entity_id.value.clone()),
            employee_id: internal(self.employee_id.value.clone()),
            lifecycle_kind: internal(lifecycle_kind),
            audit_evidence_ref: internal(self.audit_evidence_ref.value.clone()),
            idempotency_key: internal(idempotency_key),
            schema_version: public(HR_EVENT_SCHEMA_VERSION),
        })
    }
}
