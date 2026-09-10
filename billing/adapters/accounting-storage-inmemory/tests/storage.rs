#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use billing_accounting_app::{plan_vat_workflow, post_journal_with_audit, record_payroll_posting};
use billing_accounting_journal::{
    AccountingJournalStoragePort, AccountingStorageError, AccountingStoredRecordKind,
    JournalLineInput, JournalPostInput, Jurisdiction, PayrollPostingInput, PeriodState,
    VatDeadlineInput,
};
use billing_accounting_storage_inmemory_adapter::{
    InMemoryAccountingJournalStore, accounting_storage_capabilities,
};

#[test]
fn accounting_storage_records_metadata_without_durable_backend_claim() {
    let mut store = InMemoryAccountingJournalStore::new();

    let journal = post_journal_with_audit(journal_input()).expect("journal outcome");
    let journal_record = store
        .persist_journal_post_audit(&journal.audit_envelope)
        .expect("persist journal audit");
    assert_eq!(
        journal_record.kind,
        AccountingStoredRecordKind::JournalPostAudit
    );
    assert_eq!(journal_record.topic, "audit.accounting.journal.posted");
    assert_eq!(journal_record.primary_ref, "jrn_2026_01");
    assert_eq!(journal_record.evidence_ref_count, 1);
    assert_eq!(
        journal_record.storage_backend,
        "in-memory-accounting-reference"
    );

    let payroll = record_payroll_posting(payroll_input()).expect("payroll posting outcome");
    let payroll_record = store
        .persist_payroll_posting_audit(&payroll.audit_envelope)
        .expect("persist payroll posting audit");
    assert_eq!(
        payroll_record.kind,
        AccountingStoredRecordKind::PayrollPostingAudit
    );
    assert_eq!(payroll_record.topic, "audit.accounting.payroll.posted");
    assert_eq!(payroll_record.primary_ref, "jrn_payroll_2026_01");
    assert_eq!(payroll_record.evidence_ref_count, 3);

    let vat = plan_vat_workflow(vat_input()).expect("VAT workflow outcome");
    let dispatch = vat.dispatch_envelope.expect("dispatch envelope");
    let vat_record = store
        .persist_vat_workflow_dispatch(&dispatch)
        .expect("persist VAT workflow dispatch");
    assert_eq!(
        vat_record.kind,
        AccountingStoredRecordKind::VatWorkflowDispatch
    );
    assert_eq!(vat_record.topic, "workflow.accounting.vat.dispatch");
    assert_eq!(vat_record.primary_ref, "vat_2026_q1");
    assert_eq!(vat_record.payload_data_class, "Financial");

    assert_eq!(store.len(), 3);
    assert!(
        store
            .require_record(&journal_record.idempotency_key)
            .is_ok()
    );

    let capabilities = accounting_storage_capabilities();
    assert_eq!(capabilities.adapter, "in-memory-accounting-reference");
    assert!(!capabilities.durable_ledger_backend_attached);
    assert!(!capabilities.postgres_rls_attached);
    assert!(!capabilities.workflow_execution_attached);
    assert!(!capabilities.statutory_filing_rails_attached);
    assert!(!capabilities.payment_execution_attached);
    assert!(!capabilities.payroll_network_call_attached);
    assert!(!capabilities.audit_chain_emission_attached);
}

#[test]
fn accounting_storage_refuses_duplicate_idempotency_keys() {
    let mut store = InMemoryAccountingJournalStore::new();
    let journal = post_journal_with_audit(journal_input()).expect("journal outcome");
    store
        .persist_journal_post_audit(&journal.audit_envelope)
        .expect("first persist");

    let key = journal.audit_envelope.idempotency_key.value.clone();
    let error = store
        .persist_journal_post_audit(&journal.audit_envelope)
        .expect_err("duplicate idempotency key must be refused");
    assert_eq!(
        error,
        AccountingStorageError::DuplicateIdempotencyKey(key.clone())
    );
    assert!(
        key.contains("ten_acme"),
        "journal-post idempotency key must be tenant-scoped, got: {key}"
    );
}

#[test]
fn accounting_storage_same_journal_id_across_tenants_does_not_collide() {
    let mut store = InMemoryAccountingJournalStore::new();

    let mut tenant_a = journal_input();
    tenant_a.tenant_id = "ten_alpha".to_owned();
    tenant_a.journal_id = "jrn_shared_id".to_owned();
    let outcome_a = post_journal_with_audit(tenant_a).expect("tenant A journal");

    let mut tenant_b = journal_input();
    tenant_b.tenant_id = "ten_beta".to_owned();
    tenant_b.journal_id = "jrn_shared_id".to_owned();
    let outcome_b = post_journal_with_audit(tenant_b).expect("tenant B journal");

    assert_ne!(
        outcome_a.audit_envelope.idempotency_key.value,
        outcome_b.audit_envelope.idempotency_key.value,
        "cross-tenant idempotency keys must not collide on a shared journal_id"
    );

    store
        .persist_journal_post_audit(&outcome_a.audit_envelope)
        .expect("tenant A persists");
    store
        .persist_journal_post_audit(&outcome_b.audit_envelope)
        .expect("tenant B must persist independently, not collide with tenant A");
    assert_eq!(store.len(), 2, "both tenants' records must coexist");
}

#[test]
fn accounting_storage_same_logical_key_changed_body_is_rejected() {
    let mut store = InMemoryAccountingJournalStore::new();

    let first_outcome = post_journal_with_audit(journal_input()).expect("first journal outcome");
    let first = store
        .persist_journal_post_audit(&first_outcome.audit_envelope)
        .expect("first persist");

    let mut changed = journal_input();
    changed.lines = vec![
        JournalLineInput {
            account_code: "EXP-WAGES".to_owned(),
            debit_minor: 2_000_000,
            credit_minor: 0,
        },
        JournalLineInput {
            account_code: "LIAB-NETPAY".to_owned(),
            debit_minor: 0,
            credit_minor: 2_000_000,
        },
    ];
    let changed_outcome = post_journal_with_audit(changed).expect("changed journal outcome");

    assert_eq!(
        first_outcome.audit_envelope.idempotency_key.value,
        changed_outcome.audit_envelope.idempotency_key.value,
        "same tenant + journal_id must produce the same logical idempotency key"
    );
    assert_ne!(
        first_outcome.audit_envelope.body_fingerprint.value,
        changed_outcome.audit_envelope.body_fingerprint.value,
        "a changed line amount must change the body fingerprint"
    );

    let error = store
        .persist_journal_post_audit(&changed_outcome.audit_envelope)
        .expect_err("changed body under a reused logical key must be rejected");
    assert_eq!(
        error,
        AccountingStorageError::IdempotencyKeyBodyMismatch {
            key: first.idempotency_key.clone(),
            stored: first.body_fingerprint.clone(),
            candidate: changed_outcome
                .audit_envelope
                .body_fingerprint
                .value
                .clone(),
        }
    );
    assert_eq!(
        store.len(),
        1,
        "the changed-body record must NOT be inserted; store must not proliferate records"
    );
}

#[test]
fn accounting_storage_same_logical_key_identical_body_is_idempotent() {
    let mut store = InMemoryAccountingJournalStore::new();

    let first_outcome = post_journal_with_audit(journal_input()).expect("first journal outcome");
    store
        .persist_journal_post_audit(&first_outcome.audit_envelope)
        .expect("first persist");

    let replay_outcome = post_journal_with_audit(journal_input()).expect("replay journal outcome");
    assert_eq!(
        first_outcome.audit_envelope.idempotency_key.value,
        replay_outcome.audit_envelope.idempotency_key.value
    );
    assert_eq!(
        first_outcome.audit_envelope.body_fingerprint.value,
        replay_outcome.audit_envelope.body_fingerprint.value
    );

    let key = replay_outcome.audit_envelope.idempotency_key.value.clone();
    let error = store
        .persist_journal_post_audit(&replay_outcome.audit_envelope)
        .expect_err("identical replay is refused as a duplicate");
    assert_eq!(error, AccountingStorageError::DuplicateIdempotencyKey(key));
    assert_eq!(store.len(), 1, "idempotent replay must not add a record");
}

#[test]
fn accounting_storage_reservation_validates_key_shape_and_allows_commit() {
    let mut store = InMemoryAccountingJournalStore::new();
    assert_eq!(
        store.reserve_idempotency_key("bad key"),
        Err(AccountingStorageError::InvalidIdempotencyKey(
            "bad key".to_owned()
        ))
    );

    let vat = plan_vat_workflow(vat_input()).expect("VAT workflow outcome");
    let dispatch = vat.dispatch_envelope.expect("dispatch envelope");
    let key = dispatch.idempotency_key.value.clone();
    store.reserve_idempotency_key(&key).expect("reserve key");
    store
        .persist_vat_workflow_dispatch(&dispatch)
        .expect("reserved key can be committed once");
    assert_eq!(store.len(), 1);
}

fn digest() -> String {
    format!("sha256:{}", "b".repeat(64))
}

fn lines() -> Vec<JournalLineInput> {
    vec![
        JournalLineInput {
            account_code: "EXP-WAGES".to_owned(),
            debit_minor: 1_000_000,
            credit_minor: 0,
        },
        JournalLineInput {
            account_code: "LIAB-NETPAY".to_owned(),
            debit_minor: 0,
            credit_minor: 1_000_000,
        },
    ]
}

fn journal_input() -> JournalPostInput {
    JournalPostInput {
        journal_id: "jrn_2026_01".to_owned(),
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        period: "2026-01".to_owned(),
        period_state: PeriodState::Open,
        source_documents: vec!["src/payroll/run/prun_kr_2026_01".to_owned()],
        approval_evidence_ref: "audit/accounting/journal/approval".to_owned(),
        lines: lines(),
    }
}

fn vat_input() -> VatDeadlineInput {
    VatDeadlineInput {
        return_id: "vat_2026_q1".to_owned(),
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        jurisdiction: Jurisdiction::Korea,
        period: "2026-01".to_owned(),
        deadline_epoch_seconds: 1_779_519_600,
        now_epoch_seconds: 1_779_519_601,
        workflow_ref: "workflow/accounting/vat/kr".to_owned(),
        hometax_export_hash: digest(),
        evidence_ref: "audit/accounting/vat/evidence".to_owned(),
    }
}

fn payroll_input() -> PayrollPostingInput {
    PayrollPostingInput {
        journal_id: "jrn_payroll_2026_01".to_owned(),
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        period: "2026-01".to_owned(),
        source_payroll_digest: digest(),
        wage_ledger_refs: vec!["audit/payroll/wage-ledger/001".to_owned()],
        approval_evidence_ref: "audit/accounting/payroll/approval".to_owned(),
        reversal_path_ref: "audit/accounting/payroll/reversal".to_owned(),
        lines: lines(),
    }
}
