//! Payroll run in-memory storage adapter reference.
//!
//! SECURITY/OPERATIONS: NOT FOR PRODUCTION. This adapter is a volatile,
//! process-local reference implementation for payroll-run metadata storage
//! seams. It records app-layer trial-close audit, accounting dispatch, and HR
//! leave-impact intake envelopes so later durable Postgres/RLS and cloud
//! adapters have a tested contract. It does not persist to durable storage,
//! calculate payroll, submit statutory filings, disburse funds, call HR or
//! Accounting, execute Workflow, emit audit-chain events, or deploy cloud I/O.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use payroll_run_app::{
    PayrollAccountingDispatchEnvelope, PayrollAuditEnvelope, PayrollHrLeaveImpactEnvelope,
};

const IN_MEMORY_PAYROLL_STORAGE_LABEL: &str = "in-memory-payroll-reference";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PayrollStoredRecordKind {
    TrialCloseAudit,
    AccountingJournalDispatch,
    HrLeaveImpactIntake,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayrollStoredRecord {
    pub kind: PayrollStoredRecordKind,
    pub topic: String,
    pub tenant_id: String,
    pub legal_entity_id: String,
    pub run_id: String,
    pub primary_ref: String,
    pub idempotency_key: String,
    pub payload_data_class: String,
    pub evidence_ref_count: usize,
    pub storage_backend: String,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayrollStorageCapabilities {
    pub adapter: String,
    pub durable_backend_attached: bool,
    pub postgres_rls_attached: bool,
    pub payroll_calculation_attached: bool,
    pub statutory_filing_rails_attached: bool,
    pub disbursement_rails_attached: bool,
    pub workflow_dispatch_attached: bool,
    pub hr_network_call_attached: bool,
    pub accounting_network_call_attached: bool,
    pub audit_chain_emission_attached: bool,
    pub schema_version: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayrollStorageError {
    DuplicateIdempotencyKey(String),
    InvalidIdempotencyKey(String),
    MissingRecord(String),
}

pub trait PayrollRunStoragePort {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), PayrollStorageError>;
    fn put_record(&mut self, record: PayrollStoredRecord) -> Result<(), PayrollStorageError>;
    fn get_record(&self, idempotency_key: &str) -> Option<&PayrollStoredRecord>;
    fn require_record(
        &self,
        idempotency_key: &str,
    ) -> Result<&PayrollStoredRecord, PayrollStorageError>;
    fn list_records(&self) -> Vec<&PayrollStoredRecord>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryPayrollRunStore {
    records_by_idempotency_key: BTreeMap<String, PayrollStoredRecord>,
    reserved_idempotency_keys: BTreeSet<String>,
}

impl InMemoryPayrollRunStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn persist_trial_close_audit(
        &mut self,
        envelope: &PayrollAuditEnvelope,
    ) -> Result<PayrollStoredRecord, PayrollStorageError> {
        let record = PayrollStoredRecord {
            kind: PayrollStoredRecordKind::TrialCloseAudit,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            run_id: envelope.run_id.value.value.clone(),
            primary_ref: envelope.run_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 1,
            storage_backend: IN_MEMORY_PAYROLL_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_accounting_dispatch(
        &mut self,
        envelope: &PayrollAccountingDispatchEnvelope,
    ) -> Result<PayrollStoredRecord, PayrollStorageError> {
        let record = PayrollStoredRecord {
            kind: PayrollStoredRecordKind::AccountingJournalDispatch,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            run_id: envelope.run_id.value.value.clone(),
            primary_ref: envelope.journal_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 2,
            storage_backend: IN_MEMORY_PAYROLL_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_hr_leave_impact_intake(
        &mut self,
        envelope: &PayrollHrLeaveImpactEnvelope,
    ) -> Result<PayrollStoredRecord, PayrollStorageError> {
        let record = PayrollStoredRecord {
            kind: PayrollStoredRecordKind::HrLeaveImpactIntake,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            run_id: envelope.run_id.value.value.clone(),
            primary_ref: envelope.leave_request_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 2,
            storage_backend: IN_MEMORY_PAYROLL_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }
}

impl PayrollRunStoragePort for InMemoryPayrollRunStore {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), PayrollStorageError> {
        validate_idempotency_key(key)?;
        if self.records_by_idempotency_key.contains_key(key)
            || !self.reserved_idempotency_keys.insert(key.to_owned())
        {
            return Err(PayrollStorageError::DuplicateIdempotencyKey(key.to_owned()));
        }
        Ok(())
    }

    fn put_record(&mut self, record: PayrollStoredRecord) -> Result<(), PayrollStorageError> {
        validate_idempotency_key(&record.idempotency_key)?;
        if self
            .records_by_idempotency_key
            .contains_key(&record.idempotency_key)
        {
            return Err(PayrollStorageError::DuplicateIdempotencyKey(
                record.idempotency_key,
            ));
        }
        self.reserved_idempotency_keys
            .remove(&record.idempotency_key);
        self.records_by_idempotency_key
            .insert(record.idempotency_key.clone(), record);
        Ok(())
    }

    fn get_record(&self, idempotency_key: &str) -> Option<&PayrollStoredRecord> {
        self.records_by_idempotency_key.get(idempotency_key)
    }

    fn require_record(
        &self,
        idempotency_key: &str,
    ) -> Result<&PayrollStoredRecord, PayrollStorageError> {
        self.get_record(idempotency_key)
            .ok_or_else(|| PayrollStorageError::MissingRecord(idempotency_key.to_owned()))
    }

    fn list_records(&self) -> Vec<&PayrollStoredRecord> {
        self.records_by_idempotency_key.values().collect()
    }

    fn len(&self) -> usize {
        self.records_by_idempotency_key.len()
    }

    fn is_empty(&self) -> bool {
        self.records_by_idempotency_key.is_empty()
    }
}

pub fn payroll_storage_capabilities() -> PayrollStorageCapabilities {
    PayrollStorageCapabilities {
        adapter: IN_MEMORY_PAYROLL_STORAGE_LABEL.to_owned(),
        durable_backend_attached: false,
        postgres_rls_attached: false,
        payroll_calculation_attached: false,
        statutory_filing_rails_attached: false,
        disbursement_rails_attached: false,
        workflow_dispatch_attached: false,
        hr_network_call_attached: false,
        accounting_network_call_attached: false,
        audit_chain_emission_attached: false,
        schema_version: 1,
    }
}

fn validate_idempotency_key(key: &str) -> Result<(), PayrollStorageError> {
    if key.trim().is_empty()
        || key.trim() != key
        || key.contains("..")
        || key.chars().any(char::is_whitespace)
    {
        return Err(PayrollStorageError::InvalidIdempotencyKey(key.to_owned()));
    }
    Ok(())
}
