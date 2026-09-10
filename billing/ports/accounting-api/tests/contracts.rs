#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use billing_accounting_api::{
    ApiErrorEnvelope, JournalLineRequest, JournalPostRequest, JurisdictionDto,
    PayrollPostingRequest, PeriodStateDto, VatDeadlineRequest,
};
use billing_accounting_app::{plan_vat_workflow, post_journal_with_audit, record_payroll_posting};
use serde_json::json;

#[test]
fn journal_post_request_uses_camel_case_and_stable_enums() {
    let request = journal_request();
    let body = serde_json::to_value(&request).expect("serialize request");

    assert_eq!(body["journalId"], "jrn_2026_01");
    assert_eq!(body["periodState"], "OPEN");
    assert_eq!(body["lines"][0]["debitMinor"], 1_000_000);

    let outcome = post_journal_with_audit(request.into_domain()).expect("journal outcome");
    assert_eq!(outcome.audit_envelope.journal_id.value.value, "jrn_2026_01");
}

#[test]
fn payroll_posting_request_converts_to_app_input() {
    let outcome = record_payroll_posting(payroll_request().into_domain()).expect("posting");

    assert_eq!(
        outcome.audit_envelope.idempotency_key.value,
        "idem-v2:ten_acme:payroll-posted:jrn_payroll_2026_01"
    );
    assert!(
        !outcome.audit_envelope.body_fingerprint.value.is_empty(),
        "body fingerprint must be carried as a separate field, not in the key"
    );
}

#[test]
fn vat_deadline_request_converts_to_workflow_input() {
    let outcome = plan_vat_workflow(vat_request().into_domain()).expect("VAT outcome");

    assert!(outcome.dispatch_envelope.is_some());
}

#[test]
fn error_envelope_has_consistent_shape() {
    let envelope =
        ApiErrorEnvelope::validation("Invalid accounting request", Some("journalId".to_owned()));

    assert_eq!(
        serde_json::to_value(envelope).expect("serialize error"),
        json!({
            "error": {
                "code": "VALIDATION_ERROR",
                "message": "Invalid accounting request",
                "details": "journalId"
            }
        })
    );
}

fn digest() -> String {
    format!("sha256:{}", "b".repeat(64))
}

fn lines() -> Vec<JournalLineRequest> {
    vec![
        JournalLineRequest {
            account_code: "EXP-WAGES".to_owned(),
            debit_minor: 1_000_000,
            credit_minor: 0,
        },
        JournalLineRequest {
            account_code: "LIAB-NETPAY".to_owned(),
            debit_minor: 0,
            credit_minor: 1_000_000,
        },
    ]
}

fn journal_request() -> JournalPostRequest {
    JournalPostRequest {
        journal_id: "jrn_2026_01".to_owned(),
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        period: "2026-01".to_owned(),
        period_state: PeriodStateDto::Open,
        source_documents: vec!["src/payroll/run/prun_kr_2026_01".to_owned()],
        approval_evidence_ref: "audit/accounting/journal/approval".to_owned(),
        lines: lines(),
    }
}

fn payroll_request() -> PayrollPostingRequest {
    PayrollPostingRequest {
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

fn vat_request() -> VatDeadlineRequest {
    VatDeadlineRequest {
        return_id: "vat_2026_q1".to_owned(),
        tenant_id: "ten_acme".to_owned(),
        legal_entity_id: "le_kr_001".to_owned(),
        jurisdiction: JurisdictionDto::Korea,
        period: "2026-01".to_owned(),
        deadline_epoch_seconds: 1_779_519_600,
        now_epoch_seconds: 1_779_519_601,
        workflow_ref: "workflow/accounting/vat/kr".to_owned(),
        hometax_export_hash: digest(),
        evidence_ref: "audit/accounting/vat/evidence".to_owned(),
    }
}
