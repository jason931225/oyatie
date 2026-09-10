//! Read views over the log and its projection: per-object history with
//! payload-embedded actor attribution, and the governance audit view —
//! applied entries and poisons alike, nothing hidden.
//!
//! Views are derived purely from (log entries, projection); they hold no
//! state of their own and can be recomputed at any moment.

use foundry_edits::{EditTag, decode_action_record};
use foundry_records_draft::SealedEnvelope;

use crate::fold::PoisonReason;
use crate::state::ProjectionState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry {
    pub ordinal: u64,              // data_class: INTERNAL_ONLY
    pub object_sequence: u64,      // data_class: INTERNAL_ONLY
    pub principal_id: String,      // data_class: INTERNAL_ONLY
    pub decision_id: String,       // data_class: INTERNAL_ONLY
    pub action_type: String,       // data_class: INTERNAL_ONLY
    pub audit_event_type: String,  // data_class: INTERNAL_ONLY
    pub occurred_at_epoch_ms: u64, // data_class: INTERNAL_ONLY
    pub schema_revision: u32,      // data_class: INTERNAL_ONLY
    /// The edit kinds the entry applied, in payload order.
    pub edits: Vec<EditTag>, // data_class: INTERNAL_ONLY
}

/// The APPLIED history of one object, oldest first — exactly the
/// projection's applied index joined back to the log's payload facts.
/// Poisoned entries never appear here; they are the audit view's job.
pub fn object_history(
    state: &ProjectionState,
    entries: &[SealedEnvelope],
    object_ref: &str,
) -> Vec<HistoryEntry> {
    let Some(applied) = state.history.get(object_ref) else {
        return Vec::new();
    };
    let mut view = Vec::new();
    for ordinal in applied {
        let Some(sealed) = entries
            .iter()
            .find(|sealed| sealed.receipt.ordinal == *ordinal)
        else {
            continue;
        };
        let Ok(record) = decode_action_record(&sealed.envelope.payload) else {
            continue;
        };
        view.push(HistoryEntry {
            ordinal: *ordinal,
            object_sequence: sealed.receipt.object_sequence,
            principal_id: record.principal_id,
            decision_id: record.decision_id,
            action_type: sealed.envelope.action_type.clone(),
            audit_event_type: record.audit_event_type,
            occurred_at_epoch_ms: record.occurred_at_epoch_ms,
            schema_revision: sealed.envelope.schema_revision,
            edits: record.edits.edits().iter().map(|edit| edit.tag()).collect(),
        });
    }
    view
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditDisposition {
    Applied,
    Poisoned(PoisonReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntry {
    pub ordinal: u64,            // data_class: INTERNAL_ONLY
    pub object_ref: String,      // data_class: INTERNAL_ONLY
    pub action_type: String,     // data_class: INTERNAL_ONLY
    pub idempotency_key: String, // data_class: INTERNAL_ONLY
    /// Payload attribution when the payload decodes; a poisoned entry
    /// whose bytes do not decode still appears, attribution absent.
    pub principal_id: Option<String>, // data_class: INTERNAL_ONLY
    pub decision_id: Option<String>, // data_class: INTERNAL_ONLY
    pub disposition: AuditDisposition, // data_class: INTERNAL_ONLY
}

/// Every consumed log entry, in ordinal order, applied and poisoned
/// alike — the governance answer to "what happened", with nothing
/// silently dropped.
pub fn audit_view(state: &ProjectionState, entries: &[SealedEnvelope]) -> Vec<AuditEntry> {
    let mut view: Vec<AuditEntry> = entries
        .iter()
        .filter(|sealed| sealed.receipt.ordinal <= state.applied_ordinal)
        .map(|sealed| {
            let attribution = decode_action_record(&sealed.envelope.payload).ok();
            AuditEntry {
                ordinal: sealed.receipt.ordinal,
                object_ref: sealed.envelope.object_ref.clone(),
                action_type: sealed.envelope.action_type.clone(),
                idempotency_key: sealed.envelope.idempotency_key.clone(),
                principal_id: attribution.as_ref().map(|r| r.principal_id.clone()),
                decision_id: attribution.as_ref().map(|r| r.decision_id.clone()),
                disposition: match state.poison.get(&sealed.receipt.ordinal) {
                    Some(reason) => AuditDisposition::Poisoned(reason.clone()),
                    None => AuditDisposition::Applied,
                },
            }
        })
        .collect();
    view.sort_by_key(|entry| entry.ordinal);
    view
}
