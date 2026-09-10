//! The denial trail: refused submissions become durable
//! [`DenialRecord`]s on a SEPARATE audit log instance — denials never
//! consume a tenant object ordinal and never forge an object ref.
//!
//! The idempotency key derives from the denial's own canonical bytes
//! (no clocks, no minted ids), so the identical refusal retried
//! deduplicates while divergent denials never conflict. An audit-append
//! failure is deliberately swallowed: the refusal returned to the
//! caller is the truth that must survive.

use foundry_edits::{DenialRecord, encode_denial_record};
use foundry_records_draft::{ActionEnvelope, RecordsLog};

use crate::error::Refused;
use crate::writer::ActionSubmission;

pub(crate) fn record_denial(
    denial_log: &mut dyn RecordsLog,
    submission: &ActionSubmission,
    refused: &Refused,
) {
    let occurred_at_epoch_ms = submission
        .request
        .requested_at_epoch_seconds
        .saturating_mul(1000);
    let Ok(record) = DenialRecord::new(
        refused.gate.label(),
        refused.cause,
        submission.request.principal_id.clone(),
        submission.decision.decision_id.clone(),
        submission.request.action_id.value.clone(),
        submission.request.entity_id.clone(),
        occurred_at_epoch_ms,
    ) else {
        // A submission too malformed to describe still keeps its refusal.
        return;
    };
    let payload = encode_denial_record(&record);
    let key = format!("deny_{:016x}", fnv1a(&payload));
    let Ok(envelope) = ActionEnvelope::new(
        submission.request.tenant_id.clone(),
        submission.request.entity_id.clone(),
        submission.request.action_id.value.clone(),
        key,
        1,
        payload,
        occurred_at_epoch_ms,
    ) else {
        return;
    };
    let _ = denial_log.append(envelope);
}

/// FNV-1a, 64-bit: a tiny deterministic content key with no new
/// dependency. Not cryptographic and not required to be — divergent
/// denials that ever collided would surface as a loud
/// IdempotencyConflict, never silent loss.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
