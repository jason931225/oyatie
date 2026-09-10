//! Accounting journal in-memory storage adapter reference.
//!
//! SECURITY/OPERATIONS: NOT FOR PRODUCTION. This adapter is a volatile,
//! process-local reference implementation of the accounting journal storage
//! port; `AccountingStorageCapabilities` below records what it does and does
//! not attach.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use billing_accounting_app::{
    AccountingAuditEnvelope, AccountingPayrollPostingAuditEnvelope,
    AccountingWorkflowDispatchEnvelope,
};
use billing_accounting_journal::{
    AccountingJournalStoragePort, AccountingStorageError, AccountingStoredRecord,
    AccountingStoredRecordKind,
};

const IN_MEMORY_ACCOUNTING_STORAGE_LABEL: &str = "in-memory-accounting-reference";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingStorageCapabilities {
    pub adapter: String,                       // data_class: PUBLIC
    pub durable_ledger_backend_attached: bool, // data_class: PUBLIC
    pub postgres_rls_attached: bool,           // data_class: PUBLIC
    pub workflow_execution_attached: bool,     // data_class: PUBLIC
    pub statutory_filing_rails_attached: bool, // data_class: PUBLIC
    pub payment_execution_attached: bool,      // data_class: PUBLIC
    pub payroll_network_call_attached: bool,   // data_class: PUBLIC
    pub audit_chain_emission_attached: bool,   // data_class: PUBLIC
    pub schema_version: u32,                   // data_class: PUBLIC
}

/// In-memory reference store for accounting metadata records.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryAccountingJournalStore {
    records_by_idempotency_key: BTreeMap<String, AccountingStoredRecord>,
    reserved_idempotency_keys: BTreeSet<String>,
}

impl InMemoryAccountingJournalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn persist_journal_post_audit(
        &mut self,
        envelope: &AccountingAuditEnvelope,
    ) -> Result<AccountingStoredRecord, AccountingStorageError> {
        let record = AccountingStoredRecord {
            kind: AccountingStoredRecordKind::JournalPostAudit,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.journal_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            body_fingerprint: envelope.body_fingerprint.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 1,
            storage_backend: IN_MEMORY_ACCOUNTING_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_payroll_posting_audit(
        &mut self,
        envelope: &AccountingPayrollPostingAuditEnvelope,
    ) -> Result<AccountingStoredRecord, AccountingStorageError> {
        let record = AccountingStoredRecord {
            kind: AccountingStoredRecordKind::PayrollPostingAudit,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.journal_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            body_fingerprint: envelope.body_fingerprint.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: 2 + envelope.wage_ledger_refs.value.len(),
            storage_backend: IN_MEMORY_ACCOUNTING_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }

    pub fn persist_vat_workflow_dispatch(
        &mut self,
        envelope: &AccountingWorkflowDispatchEnvelope,
    ) -> Result<AccountingStoredRecord, AccountingStorageError> {
        let record = AccountingStoredRecord {
            kind: AccountingStoredRecordKind::VatWorkflowDispatch,
            topic: envelope.topic.value.clone(),
            tenant_id: envelope.tenant_id.value.value.clone(),
            legal_entity_id: envelope.legal_entity_id.value.value.clone(),
            primary_ref: envelope.return_id.value.value.clone(),
            idempotency_key: envelope.idempotency_key.value.clone(),
            body_fingerprint: envelope.body_fingerprint.value.clone(),
            payload_data_class: format!("{:?}", envelope.payload_data_class.value),
            evidence_ref_count: envelope.evidence_refs.value.len(),
            storage_backend: IN_MEMORY_ACCOUNTING_STORAGE_LABEL.to_owned(),
            schema_version: envelope.schema_version.value,
        };
        self.put_record(record.clone())?;
        Ok(record)
    }
}

impl AccountingJournalStoragePort for InMemoryAccountingJournalStore {
    fn reserve_idempotency_key(&mut self, key: &str) -> Result<(), AccountingStorageError> {
        validate_idempotency_key(key)?;
        if self.records_by_idempotency_key.contains_key(key)
            || !self.reserved_idempotency_keys.insert(key.to_owned())
        {
            return Err(AccountingStorageError::DuplicateIdempotencyKey(
                key.to_owned(),
            ));
        }
        Ok(())
    }

    fn put_record(&mut self, record: AccountingStoredRecord) -> Result<(), AccountingStorageError> {
        validate_idempotency_key(&record.idempotency_key)?;
        if let Some(existing) = self.records_by_idempotency_key.get(&record.idempotency_key) {
            // A reused key carrying a DIFFERENT fingerprint is a changed
            // command, not a replay: returning the stored outcome would
            // silently substitute it, so refuse distinctly instead.
            if existing.body_fingerprint != record.body_fingerprint {
                return Err(AccountingStorageError::IdempotencyKeyBodyMismatch {
                    key: record.idempotency_key,
                    stored: existing.body_fingerprint.clone(),
                    candidate: record.body_fingerprint,
                });
            }
            return Err(AccountingStorageError::DuplicateIdempotencyKey(
                record.idempotency_key,
            ));
        }
        self.reserved_idempotency_keys
            .remove(&record.idempotency_key);
        self.records_by_idempotency_key
            .insert(record.idempotency_key.clone(), record);
        Ok(())
    }

    fn get_record(&self, idempotency_key: &str) -> Option<&AccountingStoredRecord> {
        self.records_by_idempotency_key.get(idempotency_key)
    }

    fn require_record(
        &self,
        idempotency_key: &str,
    ) -> Result<&AccountingStoredRecord, AccountingStorageError> {
        self.get_record(idempotency_key)
            .ok_or_else(|| AccountingStorageError::MissingRecord(idempotency_key.to_owned()))
    }

    fn list_records(&self) -> Vec<&AccountingStoredRecord> {
        self.records_by_idempotency_key.values().collect()
    }

    fn len(&self) -> usize {
        self.records_by_idempotency_key.len()
    }

    fn is_empty(&self) -> bool {
        self.records_by_idempotency_key.is_empty()
    }
}

pub fn accounting_storage_capabilities() -> AccountingStorageCapabilities {
    AccountingStorageCapabilities {
        adapter: IN_MEMORY_ACCOUNTING_STORAGE_LABEL.to_owned(),
        durable_ledger_backend_attached: false,
        postgres_rls_attached: false,
        workflow_execution_attached: false,
        statutory_filing_rails_attached: false,
        payment_execution_attached: false,
        payroll_network_call_attached: false,
        audit_chain_emission_attached: false,
        schema_version: 1,
    }
}

fn validate_idempotency_key(key: &str) -> Result<(), AccountingStorageError> {
    if key.trim().is_empty()
        || key.trim() != key
        || key.contains("..")
        || key.chars().any(char::is_whitespace)
    {
        return Err(AccountingStorageError::InvalidIdempotencyKey(
            key.to_owned(),
        ));
    }
    Ok(())
}
