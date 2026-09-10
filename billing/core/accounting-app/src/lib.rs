//! Accounting journal application layer.
//!
//! This crate turns pure accounting-domain outcomes into metadata-only audit
//! and Workflow dispatch envelopes for later cloud/runtime adapters.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use billing_accounting_journal::{
    AccountingDomainError, EvidenceDigest, EvidenceRef, JournalId, JournalPostInput,
    JournalVoucher, LegalEntityId, PayrollPostingEvidence, PayrollPostingInput, TenantId,
    VatDeadlineInput, VatReturnId, VatReturnWorkflow, VatWorkflowStep, WorkflowRef,
    evaluate_vat_deadline, idempotency_body_fingerprint, payroll_posting, post_journal,
    scoped_idempotency_key,
};
use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};

const ACCOUNTING_JOURNAL_TOPIC: &str = "audit.accounting.journal.posted";
const ACCOUNTING_PAYROLL_POSTING_TOPIC: &str = "audit.accounting.payroll.posted";
const ACCOUNTING_VAT_WORKFLOW_TOPIC: &str = "workflow.accounting.vat.dispatch";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingAuditEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub journal_id: Classified<JournalId>,
    pub approval_evidence_ref: Classified<EvidenceRef>,
    pub idempotency_key: Classified<String>,
    pub body_fingerprint: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingWorkflowDispatchEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub workflow_ref: Classified<WorkflowRef>,
    pub return_id: Classified<VatReturnId>,
    pub hometax_export_hash: Classified<EvidenceDigest>, // data_class: FINANCIAL
    pub required_steps: Classified<Vec<VatWorkflowStep>>,
    pub evidence_refs: Classified<Vec<EvidenceRef>>,
    pub idempotency_key: Classified<String>,
    pub body_fingerprint: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingPayrollPostingAuditEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub journal_id: Classified<JournalId>,
    pub source_payroll_digest: Classified<EvidenceDigest>, // data_class: FINANCIAL
    pub approval_evidence_ref: Classified<EvidenceRef>,
    pub reversal_path_ref: Classified<EvidenceRef>,
    pub wage_ledger_refs: Classified<Vec<EvidenceRef>>,
    pub idempotency_key: Classified<String>,
    pub body_fingerprint: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalPostOutcome {
    pub journal: JournalVoucher, // data_class: FINANCIAL
    pub audit_envelope: AccountingAuditEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VatWorkflowOutcome {
    pub workflow: Option<VatReturnWorkflow>, // data_class: FINANCIAL
    pub dispatch_envelope: Option<AccountingWorkflowDispatchEnvelope>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingPayrollPostingOutcome {
    pub evidence: PayrollPostingEvidence, // data_class: FINANCIAL
    pub audit_envelope: AccountingPayrollPostingAuditEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccountingAppError {
    Domain(AccountingDomainError),
}

impl From<AccountingDomainError> for AccountingAppError {
    fn from(error: AccountingDomainError) -> Self {
        Self::Domain(error)
    }
}

pub fn post_journal_with_audit(
    input: JournalPostInput,
) -> Result<JournalPostOutcome, AccountingAppError> {
    let journal = post_journal(input)?;
    let audit_envelope = journal_audit_envelope(&journal);
    Ok(JournalPostOutcome {
        journal,
        audit_envelope,
    })
}

pub fn plan_vat_workflow(
    input: VatDeadlineInput,
) -> Result<VatWorkflowOutcome, AccountingAppError> {
    let workflow = evaluate_vat_deadline(input)?;
    let dispatch_envelope = workflow.as_ref().map(vat_dispatch_envelope);
    Ok(VatWorkflowOutcome {
        workflow,
        dispatch_envelope,
    })
}

pub fn record_payroll_posting(
    input: PayrollPostingInput,
) -> Result<AccountingPayrollPostingOutcome, AccountingAppError> {
    let evidence = payroll_posting(input)?;
    let audit_envelope = payroll_posting_audit_envelope(&evidence);
    Ok(AccountingPayrollPostingOutcome {
        evidence,
        audit_envelope,
    })
}

fn journal_audit_envelope(journal: &JournalVoucher) -> AccountingAuditEnvelope {
    let tenant = journal.tenant_id.value.value.as_str();
    // Selection rule: every caller-mutable, money-material field, including the
    // caller-supplied source_documents.
    let mut fp_fields: Vec<String> = vec![
        tenant.to_owned(),
        journal.legal_entity_id.value.value.clone(),
        journal.journal_id.value.value.clone(),
        journal.period.value.clone(),
        journal.approval_evidence_ref.value.value.clone(),
        journal.total_debit_minor.value.to_string(),
        journal.total_credit_minor.value.to_string(),
    ];
    for source in &journal.source_documents.value {
        fp_fields.push(source.value.clone());
    }
    for line in &journal.lines.value {
        fp_fields.push(line.account_code.value.clone());
        fp_fields.push(line.debit_minor.value.to_string());
        fp_fields.push(line.credit_minor.value.to_string());
    }
    let fingerprint =
        idempotency_body_fingerprint(&fp_fields.iter().map(String::as_str).collect::<Vec<_>>());
    AccountingAuditEnvelope {
        topic: internal(ACCOUNTING_JOURNAL_TOPIC.to_owned()),
        tenant_id: internal(journal.tenant_id.value.clone()),
        legal_entity_id: internal(journal.legal_entity_id.value.clone()),
        journal_id: internal(journal.journal_id.value.clone()),
        approval_evidence_ref: internal(journal.approval_evidence_ref.value.clone()),
        idempotency_key: internal(scoped_idempotency_key(
            tenant,
            "journal-posted",
            journal.journal_id.value.value.as_str(),
        )),
        body_fingerprint: internal(fingerprint),
        payload_data_class: internal(DataClass::Financial),
        schema_version: public(1),
    }
}

fn vat_dispatch_envelope(workflow: &VatReturnWorkflow) -> AccountingWorkflowDispatchEnvelope {
    let tenant = workflow.tenant_id.value.value.as_str();
    // Selection rule: every caller-mutable field, including evidence_paths.
    let mut fp_fields: Vec<String> = vec![
        tenant.to_owned(),
        workflow.legal_entity_id.value.value.clone(),
        workflow.return_id.value.value.clone(),
        workflow.period.value.clone(),
        workflow.workflow_ref.value.value.clone(),
        workflow.hometax_export_hash.value.value.clone(),
    ];
    for evidence in &workflow.evidence_paths.value {
        fp_fields.push(evidence.value.clone());
    }
    let fingerprint =
        idempotency_body_fingerprint(&fp_fields.iter().map(String::as_str).collect::<Vec<_>>());
    AccountingWorkflowDispatchEnvelope {
        topic: internal(ACCOUNTING_VAT_WORKFLOW_TOPIC.to_owned()),
        tenant_id: internal(workflow.tenant_id.value.clone()),
        legal_entity_id: internal(workflow.legal_entity_id.value.clone()),
        workflow_ref: internal(workflow.workflow_ref.value.clone()),
        return_id: internal(workflow.return_id.value.clone()),
        hometax_export_hash: financial(workflow.hometax_export_hash.value.clone()),
        required_steps: internal(workflow.required_steps.value.clone()),
        evidence_refs: internal(workflow.evidence_paths.value.clone()),
        idempotency_key: internal(scoped_idempotency_key(
            tenant,
            "vat-workflow",
            workflow.return_id.value.value.as_str(),
        )),
        body_fingerprint: internal(fingerprint),
        payload_data_class: internal(DataClass::Financial),
        schema_version: public(1),
    }
}

fn payroll_posting_audit_envelope(
    evidence: &PayrollPostingEvidence,
) -> AccountingPayrollPostingAuditEnvelope {
    let tenant = evidence.journal.tenant_id.value.value.as_str();
    // Selection rule: every caller-mutable, money-material field, including the
    // approval evidence ref and per-line detail — not just the totals.
    let mut fp_fields: Vec<String> = vec![
        tenant.to_owned(),
        evidence.journal.legal_entity_id.value.value.clone(),
        evidence.journal.journal_id.value.value.clone(),
        evidence.journal.period.value.clone(),
        evidence.journal.approval_evidence_ref.value.value.clone(),
        evidence.source_payroll_digest.value.value.clone(),
        evidence.reversal_path_ref.value.value.clone(),
        evidence.journal.total_debit_minor.value.to_string(),
        evidence.journal.total_credit_minor.value.to_string(),
    ];
    for wage_ref in &evidence.wage_ledger_refs.value {
        fp_fields.push(wage_ref.value.clone());
    }
    for line in &evidence.journal.lines.value {
        fp_fields.push(line.account_code.value.clone());
        fp_fields.push(line.debit_minor.value.to_string());
        fp_fields.push(line.credit_minor.value.to_string());
    }
    let fingerprint =
        idempotency_body_fingerprint(&fp_fields.iter().map(String::as_str).collect::<Vec<_>>());
    AccountingPayrollPostingAuditEnvelope {
        topic: internal(ACCOUNTING_PAYROLL_POSTING_TOPIC.to_owned()),
        tenant_id: internal(evidence.journal.tenant_id.value.clone()),
        legal_entity_id: internal(evidence.journal.legal_entity_id.value.clone()),
        journal_id: internal(evidence.journal.journal_id.value.clone()),
        source_payroll_digest: financial(evidence.source_payroll_digest.value.clone()),
        approval_evidence_ref: internal(evidence.journal.approval_evidence_ref.value.clone()),
        reversal_path_ref: internal(evidence.reversal_path_ref.value.clone()),
        wage_ledger_refs: internal(evidence.wage_ledger_refs.value.clone()),
        idempotency_key: internal(scoped_idempotency_key(
            tenant,
            "payroll-posted",
            evidence.journal.journal_id.value.value.as_str(),
        )),
        body_fingerprint: internal(fingerprint),
        payload_data_class: internal(DataClass::Financial),
        schema_version: public(1),
    }
}

fn internal<T>(value: T) -> Classified<T> {
    Classified::new(value, PrivacyDataClass::internal_only())
}

fn financial<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::Financial)
}

fn public<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::Public)
}
