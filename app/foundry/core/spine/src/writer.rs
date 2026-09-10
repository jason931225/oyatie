//! The ActionWriter: the ONE way anything becomes a log entry.

use data_ontology_kernel::{ActionInvocationReceipt, OntologyEngine};
use foundry_edits::{ActionRecord, OntologyEdit, encode_action_record};
use foundry_records_draft::{ActionEnvelope, Receipt, RecordsLog, RecordsLogError, SealedEnvelope};

use crate::boundary;
use crate::error::{RefusalGate, Refused};
use crate::fold::{FoldOutcome, PoisonReason, apply_sealed};
use crate::state::ProjectionState;

/// One caller submission: what the Action wants to do, and the policy
/// decision that authorizes the caller to want it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionSubmission {
    pub request: data_ontology_kernel::ActionInvocationRequest, // data_class: INTERNAL_ONLY
    pub decision: data_ontology_kernel::ActionPolicyDecision,   // data_class: INTERNAL_ONLY
    pub parameters: Vec<foundry_edits::WireProperty>,           // data_class: PII_IDENTIFYING
    pub edits: foundry_edits::EditSet,                          // data_class: PII_IDENTIFYING
}

/// What became of an accepted submission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplyOutcome {
    Applied {
        receipt: Receipt,
    },
    /// The entry stands in the log but the projection refused it — the
    /// refusal is the projection's, honestly reported, never un-appended.
    Poisoned {
        receipt: Receipt,
        reason: PoisonReason,
    },
}

/// Why a submission produced no applied entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteError {
    /// A gate refused the submission; nothing was appended.
    Refused(Refused),
    /// The log refused the append — a divergent idempotency-key reuse
    /// surfaces loudly here, never as a silent dedup.
    Log(RecordsLogError),
}

/// Submit one Action: gate, encode, append, apply.
pub fn submit(
    submission: ActionSubmission,
    log: &mut dyn RecordsLog,
    denial_log: &mut dyn RecordsLog,
    projection: &mut ProjectionState,
) -> Result<ApplyOutcome, WriteError> {
    match submit_gated(&submission, log, projection) {
        Err(WriteError::Refused(refused)) => {
            crate::audit::record_denial(denial_log, &submission, &refused);
            Err(WriteError::Refused(refused))
        }
        other => other,
    }
}

fn submit_gated(
    submission: &ActionSubmission,
    log: &mut dyn RecordsLog,
    projection: &mut ProjectionState,
) -> Result<ApplyOutcome, WriteError> {
    let registry = projection.registry_input.clone();
    let receipt = authorize_invocation(&registry, submission)?;
    check_parameter_conformance(&registry, submission)?;
    let schema_revision = admit_edits_and_stamp_registered_revision(&registry, submission)?;
    let envelope = encode_envelope(submission, &receipt, schema_revision)?;
    advisory_dry_run_against_scratch_fold(projection, &envelope)?;
    let log_receipt =
        append_with_receipt(receipt, log, envelope.clone()).map_err(WriteError::Log)?;
    if log_receipt.deduplicated {
        return Ok(outcome_of_deduplicated_append(projection, log_receipt));
    }
    Ok(apply_through_fold(projection, envelope, log_receipt))
}

/// Failure appends nothing, anywhere.
fn authorize_invocation(
    registry: &OntologyEngine,
    submission: &ActionSubmission,
) -> Result<ActionInvocationReceipt, WriteError> {
    registry
        .authorize_action_invocation(submission.request.clone(), submission.decision.clone())
        .map_err(|_| {
            refuse(
                RefusalGate::Authorization,
                "policy decision does not cover this invocation",
            )
        })
}

fn check_parameter_conformance(
    registry: &OntologyEngine,
    submission: &ActionSubmission,
) -> Result<(), WriteError> {
    let converted = submission
        .parameters
        .iter()
        .map(boundary::property)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            refuse(
                RefusalGate::Parameters,
                "parameter value unrepresentable in the kernel",
            )
        })?;
    registry
        .check_action_parameter_conformance(
            &submission.request.tenant_id,
            &submission.request.action_id,
            &converted,
        )
        .map_err(|_| {
            refuse(
                RefusalGate::Parameters,
                "parameters fail the declared schema",
            )
        })
}

fn admit_edits_and_stamp_registered_revision(
    registry: &OntologyEngine,
    submission: &ActionSubmission,
) -> Result<u32, WriteError> {
    let action = registry
        .action_type(&submission.request.tenant_id, &submission.request.action_id)
        .ok_or_else(|| {
            refuse(
                RefusalGate::Admission,
                "action type vanished from the registry",
            )
        })?;
    for edit in submission.edits.edits() {
        if let OntologyEdit::CreateObject { entity_type, .. } = edit
            && entity_type != &action.entity_type.value
        {
            return Err(refuse(
                RefusalGate::Admission,
                "edit entity type differs from the action's declared type",
            ));
        }
    }
    Ok(registry
        .entity_type(&submission.request.tenant_id, &action.entity_type)
        .map(|definition| definition.revision)
        .unwrap_or(1))
}

fn encode_envelope(
    submission: &ActionSubmission,
    receipt: &ActionInvocationReceipt,
    schema_revision: u32,
) -> Result<ActionEnvelope, WriteError> {
    let occurred_at_epoch_ms = receipt.occurred_at_epoch_seconds.saturating_mul(1000);
    let record = ActionRecord::new(
        receipt.principal_id.clone(),
        receipt.decision_id.clone(),
        receipt.audit_event_type.clone(),
        receipt.idempotency_key.clone(),
        occurred_at_epoch_ms,
        submission.parameters.clone(),
        submission.edits.clone(),
    )
    .map_err(|_| refuse(RefusalGate::Admission, "record identity fields refused"))?;
    ActionEnvelope::new(
        receipt.tenant_id.clone(),
        receipt.entity_id.clone(),
        receipt.action_id.clone(),
        receipt.idempotency_key.clone(),
        schema_revision,
        encode_action_record(&record),
        occurred_at_epoch_ms,
    )
    .map_err(|_| refuse(RefusalGate::Admission, "envelope shape refused"))
}

/// Advisory only: authoritative admission is the fold's own re-check at
/// apply time, so a raced entry poisons deterministically rather than
/// corrupting state.
fn advisory_dry_run_against_scratch_fold(
    projection: &ProjectionState,
    envelope: &ActionEnvelope,
) -> Result<(), WriteError> {
    let mut scratch = projection.clone();
    let probe = SealedEnvelope {
        envelope: envelope.clone(),
        receipt: Receipt {
            ordinal: scratch.applied_ordinal + 1,
            object_sequence: 0,
            deduplicated: false,
        },
    };
    match apply_sealed(&mut scratch, &probe) {
        FoldOutcome::Poisoned(_) => Err(refuse(
            RefusalGate::Admission,
            "edits fail the fold's own admission",
        )),
        FoldOutcome::Applied => Ok(()),
    }
}

fn outcome_of_deduplicated_append(
    projection: &ProjectionState,
    log_receipt: Receipt,
) -> ApplyOutcome {
    match projection.poison.get(&log_receipt.ordinal) {
        Some(reason) => ApplyOutcome::Poisoned {
            receipt: log_receipt,
            reason: reason.clone(),
        },
        None => ApplyOutcome::Applied {
            receipt: log_receipt,
        },
    }
}

fn apply_through_fold(
    projection: &mut ProjectionState,
    envelope: ActionEnvelope,
    log_receipt: Receipt,
) -> ApplyOutcome {
    let sealed = SealedEnvelope {
        envelope,
        receipt: log_receipt.clone(),
    };
    match apply_sealed(projection, &sealed) {
        FoldOutcome::Applied => ApplyOutcome::Applied {
            receipt: log_receipt,
        },
        FoldOutcome::Poisoned(reason) => ApplyOutcome::Poisoned {
            receipt: log_receipt,
            reason,
        },
    }
}

fn refuse(gate: RefusalGate, cause: &'static str) -> WriteError {
    WriteError::Refused(Refused { gate, cause })
}

/// The sole append to the ACTION LOG: no [`ActionInvocationReceipt`] BY
/// VALUE, no append — receipt-gating is structural, not disciplinary.
/// The denial trail is the crate's one other append (`audit::record_denial`),
/// and it is deliberately outside this gate: a refusal never earns a
/// receipt, so requiring one to record it would lose the denial.
fn append_with_receipt(
    receipt: ActionInvocationReceipt,
    log: &mut dyn RecordsLog,
    envelope: ActionEnvelope,
) -> Result<Receipt, RecordsLogError> {
    debug_assert_eq!(receipt.idempotency_key, envelope.idempotency_key);
    log.append(envelope)
}
