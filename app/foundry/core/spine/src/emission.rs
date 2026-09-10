//! Derivation of port-shaped audit events from the spine's own facts:
//! the action log's applied and poisoned entries and the denial trail
//! each become [`FoundryAuditEvent`]s for the foundry audit port.
//!
//! Derivation is pure over (projection, log entries), recomputable at
//! any moment like every other view.

use foundry_audit_draft::{AuditDisposition, AuditPortError, FoundryAuditEvent};
use foundry_edits::{decode_action_record, decode_denial_record};
use foundry_records_draft::SealedEnvelope;

use crate::fold::PoisonReason;
use crate::state::ProjectionState;

/// Filed instead of a declared event type, which names something that
/// happened; a denial is the fact that one did not.
pub const DENIED_AUDIT_EVENT_TYPE: &str = "foundry.submission.denied";

/// Filed instead of a declared event type, for the same reason.
pub const POISONED_AUDIT_EVENT_TYPE: &str = "foundry.entry.poisoned";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Underivable {
    pub ordinal: u64,              // data_class: INTERNAL_ONLY
    pub reason: UnderivableReason, // data_class: INTERNAL_ONLY
}

/// Why derivation could not construct the event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnderivableReason {
    /// The payload decodes as neither wire shape, so attribution is
    /// absent and fail-closed construction cannot be met.
    PayloadUndecodable,
    /// The port refused the constructed event.
    EventRefused(AuditPortError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DerivedEvents {
    pub events: Vec<FoundryAuditEvent>, // data_class: INTERNAL_ONLY
    pub underivable: Vec<Underivable>,  // data_class: INTERNAL_ONLY
}

/// The static audit label for one poison reason — a closed vocabulary,
/// never a classified value or a payload byte.
pub fn poison_label(reason: &PoisonReason) -> &'static str {
    match reason {
        PoisonReason::TenantMismatch => "tenant_mismatch",
        PoisonReason::NonDenseOrdinal { .. } => "non_dense_ordinal",
        PoisonReason::Decode(_) => "payload_decode",
        PoisonReason::ReceiptMismatch => "receipt_mismatch",
        PoisonReason::InvalidActionType => "invalid_action_type",
        PoisonReason::Parameters(_) => "parameter_conformance",
        PoisonReason::Boundary(_) => "wire_boundary",
        PoisonReason::UnknownRevision { .. } => "unknown_revision",
        PoisonReason::MissingObject => "missing_object",
        PoisonReason::Conformance(_) => "instance_conformance",
        PoisonReason::Link(_) => "link_admission",
        PoisonReason::Object(_) => "object_shape",
    }
}

/// Derive one event per entry the fold has consumed, in ordinal order:
/// applied entries under their declared event type, poisoned entries
/// under [`POISONED_AUDIT_EVENT_TYPE`] with attribution kept whenever
/// the payload decodes. The event's tenant is the projection's — a
/// tenant-mismatched poison is still THIS tenant's log fact.
pub fn derive_action_events(state: &ProjectionState, entries: &[SealedEnvelope]) -> DerivedEvents {
    let mut consumed: Vec<&SealedEnvelope> = entries
        .iter()
        .filter(|sealed| sealed.receipt.ordinal <= state.applied_ordinal)
        .collect();
    consumed.sort_by_key(|sealed| sealed.receipt.ordinal);
    let mut derived = DerivedEvents::default();
    for sealed in consumed {
        let ordinal = sealed.receipt.ordinal;
        let Ok(record) = decode_action_record(&sealed.envelope.payload) else {
            derived.underivable.push(Underivable {
                ordinal,
                reason: UnderivableReason::PayloadUndecodable,
            });
            continue;
        };
        let (audit_event_type, disposition) = match state.poison.get(&ordinal) {
            Some(reason) => (
                POISONED_AUDIT_EVENT_TYPE.to_owned(),
                AuditDisposition::Poisoned {
                    ordinal,
                    reason: poison_label(reason).into(),
                },
            ),
            None => (
                record.audit_event_type.clone(),
                AuditDisposition::Applied { ordinal },
            ),
        };
        push(
            &mut derived,
            ordinal,
            FoundryAuditEvent::new(
                state.tenant_id.clone(),
                audit_event_type,
                record.principal_id,
                record.decision_id,
                sealed.envelope.object_ref.clone(),
                disposition,
                record.occurred_at_epoch_ms,
            ),
        );
    }
    derived
}

/// Derive one [`DENIED_AUDIT_EVENT_TYPE`] event per denial-trail entry,
/// in the trail's own ordinal order. The emitted event's decision id is
/// the join key back to the full [`foundry_edits::DenialRecord`].
pub fn derive_denial_events(entries: &[SealedEnvelope]) -> DerivedEvents {
    let mut trail: Vec<&SealedEnvelope> = entries.iter().collect();
    trail.sort_by_key(|sealed| sealed.receipt.ordinal);
    let mut derived = DerivedEvents::default();
    for sealed in trail {
        let ordinal = sealed.receipt.ordinal;
        let Ok(record) = decode_denial_record(&sealed.envelope.payload) else {
            derived.underivable.push(Underivable {
                ordinal,
                reason: UnderivableReason::PayloadUndecodable,
            });
            continue;
        };
        push(
            &mut derived,
            ordinal,
            FoundryAuditEvent::new(
                sealed.envelope.tenant_id.clone(),
                DENIED_AUDIT_EVENT_TYPE,
                record.principal_id,
                record.decision_id,
                record.object_ref,
                AuditDisposition::Denied { gate: record.gate },
                record.occurred_at_epoch_ms,
            ),
        );
    }
    derived
}

fn push(
    derived: &mut DerivedEvents,
    ordinal: u64,
    constructed: Result<FoundryAuditEvent, AuditPortError>,
) {
    match constructed {
        Ok(event) => derived.events.push(event),
        Err(error) => derived.underivable.push(Underivable {
            ordinal,
            reason: UnderivableReason::EventRefused(error),
        }),
    }
}
