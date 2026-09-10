#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use billing_accounting_app::{plan_vat_workflow, post_journal_with_audit, record_payroll_posting};
use billing_accounting_journal::{
    JournalLineInput, JournalPostInput, Jurisdiction, PayrollPostingInput, PeriodState,
    VatDeadlineInput, VatWorkflowStep,
};
use data_boundary_kernel::DataClass;

#[test]
fn post_journal_emits_audit_event() {
    let outcome = post_journal_with_audit(journal_input()).expect("journal outcome");

    assert_eq!(
        outcome.journal.total_debit_minor.value,
        outcome.journal.total_credit_minor.value
    );
    assert_eq!(
        outcome.audit_envelope.topic.value,
        "audit.accounting.journal.posted"
    );
    assert_eq!(outcome.audit_envelope.journal_id.value.value, "jrn_2026_01");
    assert_eq!(
        outcome.audit_envelope.payload_data_class.value,
        DataClass::Financial
    );
    assert_eq!(
        outcome
            .audit_envelope
            .schema_version
            .data_class
            .compatibility_data_class(),
        DataClass::Public
    );
}

#[test]
fn vat_deadline_emits_workflow_dispatch() {
    let outcome = plan_vat_workflow(vat_input()).expect("VAT workflow outcome");
    let workflow = outcome.workflow.expect("workflow opened");
    let dispatch = outcome.dispatch_envelope.expect("dispatch envelope");

    assert_eq!(workflow.return_id.value.value, "vat_2026_q1");
    assert_eq!(dispatch.topic.value, "workflow.accounting.vat.dispatch");
    assert_eq!(
        dispatch.workflow_ref.value.value,
        "workflow/accounting/vat/kr"
    );
    assert_eq!(dispatch.hometax_export_hash.value.value, digest());
    assert_eq!(
        dispatch
            .hometax_export_hash
            .data_class
            .compatibility_data_class(),
        DataClass::Financial
    );
    assert!(
        dispatch
            .required_steps
            .value
            .contains(&VatWorkflowStep::EvidencePackAttached)
    );
    assert_eq!(
        dispatch.evidence_refs.value[0].value,
        "audit/accounting/vat/evidence"
    );
}

#[test]
fn payroll_posting_emits_accounting_audit_event() {
    let outcome = record_payroll_posting(payroll_input()).expect("payroll posting outcome");

    assert_eq!(outcome.evidence.source_payroll_digest.value.value, digest());
    assert_eq!(
        outcome.audit_envelope.topic.value,
        "audit.accounting.payroll.posted"
    );
    assert_eq!(
        outcome.audit_envelope.source_payroll_digest.value.value,
        digest()
    );
    assert_eq!(
        outcome
            .audit_envelope
            .source_payroll_digest
            .data_class
            .compatibility_data_class(),
        DataClass::Financial
    );
    assert_eq!(
        outcome.audit_envelope.reversal_path_ref.value.value,
        "audit/accounting/payroll/reversal"
    );
    assert_eq!(outcome.audit_envelope.wage_ledger_refs.value.len(), 1);
    let key = &outcome.audit_envelope.idempotency_key.value;
    assert_eq!(
        key, "idem-v2:ten_acme:payroll-posted:jrn_payroll_2026_01",
        "payroll-posting key must be the tenant-scoped logical key, got: {key}"
    );
    assert!(
        !outcome.audit_envelope.body_fingerprint.value.is_empty(),
        "payroll-posting envelope must carry a body fingerprint as a separate field"
    );
    assert!(
        !key.contains('#'),
        "logical key must not embed the body fingerprint, got: {key}"
    );
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
