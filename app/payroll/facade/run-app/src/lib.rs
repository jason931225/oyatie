//! Payroll run application layer.
//!
//! This crate turns pure payroll-run domain outcomes into metadata-only audit
//! and accounting integration envelopes for later cloud/runtime adapters. It
//! does not persist data, call accounting, file with regulators, disburse funds,
//! or perform network I/O.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use payroll_run_domain::{
    EmployeeId, EvidenceDigest, EvidenceRef, HrLeaveImpactIntake, HrLeaveImpactIntakeInput,
    HrLeaveImpactKind, JournalId, LeaveRequestId, LegalEntityId, PayeeId, PayrollDomainError,
    PayrollJournalDraft, PayrollJournalInput, PayrollRun, PayrollRunId, PayrollTrialCloseInput,
    TenantId, build_payroll_journal, ingest_hr_leave_impact, trial_close,
};

const PAYROLL_CLOSE_TOPIC: &str = "audit.payroll.run.close";
const PAYROLL_ACCOUNTING_TOPIC: &str = "tenant_rbac.payroll.accounting.journal_draft";
const PAYROLL_HR_LEAVE_IMPACT_TOPIC: &str = "integration.payroll.hr.leave-impact-intake";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayrollAuditEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub run_id: Classified<PayrollRunId>,
    pub evidence_ref: Classified<EvidenceRef>,
    pub evidence_digest: Classified<EvidenceDigest>,
    pub idempotency_key: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayrollHrLeaveImpactEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub run_id: Classified<PayrollRunId>,
    pub payroll_period: Classified<String>, // data_class: FINANCIAL
    pub payee_id: Classified<PayeeId>,
    pub employee_id: Classified<EmployeeId>,
    pub leave_request_id: Classified<LeaveRequestId>,
    pub impact_kind: Classified<HrLeaveImpactKind>, // data_class: FINANCIAL
    pub source_topic: Classified<String>,
    pub source_hr_idempotency_key: Classified<String>,
    pub payroll_impact_evidence_ref: Classified<EvidenceRef>, // data_class: FINANCIAL
    pub payroll_intake_evidence_ref: Classified<EvidenceRef>,
    pub idempotency_key: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayrollAccountingDispatchEnvelope {
    pub topic: Classified<String>,
    pub tenant_id: Classified<TenantId>,
    pub legal_entity_id: Classified<LegalEntityId>,
    pub run_id: Classified<PayrollRunId>,
    pub journal_id: Classified<JournalId>,
    pub source_payroll_digest: Classified<EvidenceDigest>, // data_class: FINANCIAL
    pub approval_evidence_ref: Classified<EvidenceRef>,
    pub reversal_required_ref: Classified<EvidenceRef>,
    pub idempotency_key: Classified<String>,
    pub payload_data_class: Classified<DataClass>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrialCloseOutcome {
    pub run: PayrollRun, // data_class: PII_IDENTIFYING
    pub audit_envelope: PayrollAuditEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountingBridgeOutcome {
    pub journal: PayrollJournalDraft, // data_class: FINANCIAL
    pub dispatch_envelope: PayrollAccountingDispatchEnvelope,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HrLeaveImpactIntakeOutcome {
    pub intake: HrLeaveImpactIntake, // data_class: FINANCIAL
    pub intake_envelope: PayrollHrLeaveImpactEnvelope, // data_class: FINANCIAL
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayrollAppError {
    Domain(PayrollDomainError),
}

impl From<PayrollDomainError> for PayrollAppError {
    fn from(error: PayrollDomainError) -> Self {
        Self::Domain(error)
    }
}

pub fn close_trial_run(
    input: PayrollTrialCloseInput,
) -> Result<TrialCloseOutcome, PayrollAppError> {
    let run = trial_close(input)?;
    let audit_envelope = close_audit_envelope(&run);
    Ok(TrialCloseOutcome {
        run,
        audit_envelope,
    })
}

pub fn prepare_accounting_dispatch(
    input: PayrollJournalInput,
) -> Result<AccountingBridgeOutcome, PayrollAppError> {
    let journal = build_payroll_journal(input)?;
    let dispatch_envelope = accounting_dispatch_envelope(&journal);
    Ok(AccountingBridgeOutcome {
        journal,
        dispatch_envelope,
    })
}

pub fn prepare_hr_leave_impact_intake(
    input: HrLeaveImpactIntakeInput,
) -> Result<HrLeaveImpactIntakeOutcome, PayrollAppError> {
    let intake = ingest_hr_leave_impact(input)?;
    let intake_envelope = hr_leave_impact_envelope(&intake);
    Ok(HrLeaveImpactIntakeOutcome {
        intake,
        intake_envelope,
    })
}

fn close_audit_envelope(run: &PayrollRun) -> PayrollAuditEnvelope {
    PayrollAuditEnvelope {
        topic: internal(PAYROLL_CLOSE_TOPIC.to_owned()),
        tenant_id: internal(run.tenant_id.value.clone()),
        legal_entity_id: internal(run.legal_entity_id.value.clone()),
        run_id: internal(run.run_id.value.clone()),
        evidence_ref: internal(run.approval_evidence_ref.value.clone()),
        evidence_digest: financial(run.evidence_digest.value.clone()),
        idempotency_key: internal(run.idempotency_key.value.clone()),
        payload_data_class: internal(DataClass::Financial),
        schema_version: public(1),
    }
}

fn hr_leave_impact_envelope(intake: &HrLeaveImpactIntake) -> PayrollHrLeaveImpactEnvelope {
    PayrollHrLeaveImpactEnvelope {
        topic: internal(PAYROLL_HR_LEAVE_IMPACT_TOPIC.to_owned()),
        tenant_id: internal(intake.tenant_id.value.clone()),
        legal_entity_id: internal(intake.legal_entity_id.value.clone()),
        run_id: internal(intake.run_id.value.clone()),
        payroll_period: financial(intake.payroll_period.value.clone()),
        payee_id: internal(intake.payee_id.value.clone()),
        employee_id: internal(intake.employee_id.value.clone()),
        leave_request_id: internal(intake.leave_request_id.value.clone()),
        impact_kind: financial(intake.impact_kind.value),
        source_topic: internal(intake.source_topic.value.clone()),
        source_hr_idempotency_key: internal(intake.source_hr_idempotency_key.value.clone()),
        payroll_impact_evidence_ref: financial(intake.payroll_impact_evidence_ref.value.clone()),
        payroll_intake_evidence_ref: internal(intake.payroll_intake_evidence_ref.value.clone()),
        idempotency_key: internal(intake.idempotency_key.value.clone()),
        payload_data_class: internal(DataClass::Financial),
        schema_version: public(1),
    }
}

fn accounting_dispatch_envelope(
    journal: &PayrollJournalDraft,
) -> PayrollAccountingDispatchEnvelope {
    PayrollAccountingDispatchEnvelope {
        topic: internal(PAYROLL_ACCOUNTING_TOPIC.to_owned()),
        tenant_id: internal(journal.tenant_id.value.clone()),
        legal_entity_id: internal(journal.legal_entity_id.value.clone()),
        run_id: internal(journal.run_id.value.clone()),
        journal_id: internal(journal.journal_id.value.clone()),
        source_payroll_digest: financial(journal.source_payroll_digest.value.clone()),
        approval_evidence_ref: internal(journal.approval_evidence_ref.value.clone()),
        reversal_required_ref: internal(journal.reversal_required_ref.value.clone()),
        idempotency_key: internal(format!(
            "{}:{}:accounting-dispatch",
            journal.run_id.value.value, journal.journal_id.value.value
        )),
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
