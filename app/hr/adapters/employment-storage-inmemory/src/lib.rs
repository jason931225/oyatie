//! HR employment in-memory storage adapter reference.
//!
//! SECURITY/OPERATIONS: NOT FOR PRODUCTION. This adapter is a volatile,
//! process-local reference implementation for HR employment metadata storage
//! seams. It records app-layer audit, Workflow, payroll-impact, and sensitive
//! read envelopes so later durable Postgres/RLS and cloud adapters have a tested
//! contract. It does not persist to durable storage, retrieve sensitive data,
//! execute Workflow, call Payroll, emit audit-chain events, or deploy cloud I/O.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use hr_employment_app::{
    HrAuditEnvelope, HrLeavePayrollImpactEnvelope, HrSensitiveReadEnvelope,
    HrWorkflowDispatchEnvelope,
};

const IN_MEMORY_HR_STORAGE_LABEL: &str = "in-memory-hr-reference";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HrStoredRecordKind {
    LifecycleAudit,
    LaborWorkflowDispatch,
    LeavePayrollImpact,
    SensitiveReadPolicy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrStoredRecord {
    pub kind: HrStoredRecordKind,
    pub topic: String,
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub primary_ref: String,
    pub idempotency_key: String,
    pub payload_data_class: String,
    pub evidence_ref_count: usize,
    pub storage_backend: String,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrStorageCapabilities {
    pub adapter: String,
    pub durable_backend_attached: bool,
    pub postgres_rls_attached: bool,
    pub sensitive_data_retrieval_attached: bool,
    pub workflow_execution_attached: bool,
    pub payroll_network_call_attached: bool,
    pub audit_chain_emission_attached: bool,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HrStorageError {
    DuplicateIdempotencyKey(String),
    InvalidIdempotencyKey(String),
    MissingRecord(String),
}

pub trait HrEmploymentStoragePort {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), HrStorageError>;
    fn put_record(&mut self, record: HrStoredRecord) -> Result<(), HrStorageError>;
    fn get_record(&self, idempotency_key: &str) -> Option<&HrStoredRecord>;
    fn require_record(&self, idempotency_key: &str) -> Result<&HrStoredRecord, HrStorageError>;
    fn list_records(&self) -> Vec<&HrStoredRecord>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryHrEmploymentStore {
    records_by_idempotency_key: BTreeMap<String, HrStoredRecord>,
    reserved_idempotency_keys: BTreeSet<String>,
}

impl InMemoryHrEmploymentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn persist_lifecycle_audit(
        &mut self,
        envelope: &HrAuditEnvelope,
    ) -> Result<HrStoredRecord, HrStorageError> {
        let record = HrStoredRecord {
            kind: HrStoredRecordKind::LifecycleAudit,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.aggregate_ref.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 1,
            storage_backend: IN_MEMORY_HR_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_labor_workflow_dispatch(
        &mut self,
        envelope: &HrWorkflowDispatchEnvelope,
    ) -> Result<HrStoredRecord, HrStorageError> {
        let record = HrStoredRecord {
            kind: HrStoredRecordKind::LaborWorkflowDispatch,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.workflow_ref.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: "InternalOnly".to_owned(),
            evidence_ref_count: envelope.evidence_refs.value.len(),
            storage_backend: IN_MEMORY_HR_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_leave_payroll_impact(
        &mut self,
        envelope: &HrLeavePayrollImpactEnvelope,
    ) -> Result<HrStoredRecord, HrStorageError> {
        let record = HrStoredRecord {
            kind: HrStoredRecordKind::LeavePayrollImpact,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.leave_request_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 3,
            storage_backend: IN_MEMORY_HR_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_sensitive_read_policy(
        &mut self,
        envelope: &HrSensitiveReadEnvelope,
    ) -> Result<HrStoredRecord, HrStorageError> {
        let consent_count = usize::from(envelope.consent_evidence_ref.value.is_some());
        let record = HrStoredRecord {
            kind: HrStoredRecordKind::SensitiveReadPolicy,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.subject_employee_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 3 + consent_count,
            storage_backend: IN_MEMORY_HR_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }
}

impl HrEmploymentStoragePort for InMemoryHrEmploymentStore {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), HrStorageError> {
        validate_idempotency_key(key)?;
        if self.records_by_idempotency_key.contains_key(key)
            || !self.reserved_idempotency_keys.insert(key.to_owned())
        {
            return Err(HrStorageError::DuplicateIdempotencyKey(key.to_owned()));
        }
        Ok(())
    }

    fn put_record(&mut self, record: HrStoredRecord) -> Result<(), HrStorageError> {
        validate_idempotency_key(&record.idempotency_key)?;
        if self
            .records_by_idempotency_key
            .contains_key(&record.idempotency_key)
        {
            return Err(HrStorageError::DuplicateIdempotencyKey(
                record.idempotency_key,
            ));
        }
        self.reserved_idempotency_keys
            .remove(&record.idempotency_key);
        self.records_by_idempotency_key
            .insert(record.idempotency_key.clone(), record);
        Ok(())
    }

    fn get_record(&self, idempotency_key: &str) -> Option<&HrStoredRecord> {
        self.records_by_idempotency_key.get(idempotency_key)
    }

    fn require_record(&self, idempotency_key: &str) -> Result<&HrStoredRecord, HrStorageError> {
        self.get_record(idempotency_key)
            .ok_or_else(|| HrStorageError::MissingRecord(idempotency_key.to_owned()))
    }

    fn list_records(&self) -> Vec<&HrStoredRecord> {
        self.records_by_idempotency_key.values().collect()
    }

    fn len(&self) -> usize {
        self.records_by_idempotency_key.len()
    }

    fn is_empty(&self) -> bool {
        self.records_by_idempotency_key.is_empty()
    }
}

pub fn hr_storage_capabilities() -> HrStorageCapabilities {
    HrStorageCapabilities {
        adapter: IN_MEMORY_HR_STORAGE_LABEL.to_owned(),
        durable_backend_attached: false,
        postgres_rls_attached: false,
        sensitive_data_retrieval_attached: false,
        workflow_execution_attached: false,
        payroll_network_call_attached: false,
        audit_chain_emission_attached: false,
        schema_version: 1,
    }
}

fn validate_idempotency_key(key: &str) -> Result<(), HrStorageError> {
    if key.trim().is_empty()
        || key.trim() != key
        || key.contains("..")
        || key.chars().any(char::is_whitespace)
    {
        return Err(HrStorageError::InvalidIdempotencyKey(key.to_owned()));
    }
    Ok(())
}
