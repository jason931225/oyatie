//! The fold: the ONLY writer of projection state, total and
//! deterministic. Applying at write time and replaying from the log run
//! the SAME function, so `projection == fold(log)` holds by construction.
//!
//! Per entry the fold is all-or-nothing: edits land on staged copies and
//! commit together, or the whole entry poisons and nothing changes.

use foundry_edits::{ActionRecord, DecodeError, OntologyEdit, decode_action_record};
use foundry_records_draft::{ActionEnvelope, SealedEnvelope};

use data_ontology_kernel::{
    ActionTypeId, EntityTypeId, LinkTypeId, ObjectEntity, ObjectGraph, ObjectGraphError,
    OntologyEngine, OntologyEngineError,
};

use crate::boundary::{self, BoundaryError};
use crate::state::{ObjectBinding, ProjectionState};

/// Why one log entry could not apply. Derived ONLY from (log bytes,
/// registry snapshot), so the same entry poisons identically on every
/// fold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoisonReason {
    TenantMismatch,
    /// Per-tenant ordinals are dense from 1; a gap or repeat is a log
    /// integrity failure, not something to guess around.
    NonDenseOrdinal {
        expected: u64,
        found: u64,
    },
    Decode(DecodeError),
    /// The embedded receipt disagrees with the envelope it rides in.
    ReceiptMismatch,
    /// The envelope's action type is not even shaped like an action id.
    InvalidActionType,
    /// Submitted parameters failed conformance against the registry.
    Parameters(OntologyEngineError),
    /// A wire shape the kernel refuses to carry.
    Boundary(BoundaryError),
    /// The envelope's schema revision was never accepted for the object's
    /// entity type in this registry snapshot — un-poisons on refold after
    /// the evolution lands.
    UnknownRevision {
        revision: u32,
    },
    MissingObject,
    /// The post-edit object failed instance conformance.
    Conformance(OntologyEngineError),
    /// The kernel refused the link (unknown type, cardinality).
    Link(OntologyEngineError),
    Object(ObjectGraphError),
}

/// What the fold did with one entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FoldOutcome {
    Applied,
    Poisoned(PoisonReason),
}

/// Fold one sealed envelope into the projection.
pub fn apply_sealed(state: &mut ProjectionState, sealed: &SealedEnvelope) -> FoldOutcome {
    match fold_entry(state, sealed) {
        Ok(()) => FoldOutcome::Applied,
        Err(reason) => {
            state.poison.insert(sealed.receipt.ordinal, reason.clone());
            state.applied_ordinal = sealed.receipt.ordinal;
            FoldOutcome::Poisoned(reason)
        }
    }
}

pub fn fold_from_scratch<'a>(
    tenant_id: &str,
    registry: &data_ontology_kernel::OntologyEngine,
    entries: impl IntoIterator<Item = &'a SealedEnvelope>,
) -> ProjectionState {
    let mut state = ProjectionState::new(tenant_id, registry);
    for sealed in entries {
        let _ = apply_sealed(&mut state, sealed);
    }
    state
}

fn fold_entry(state: &mut ProjectionState, sealed: &SealedEnvelope) -> Result<(), PoisonReason> {
    let envelope = &sealed.envelope;
    if envelope.tenant_id != state.tenant_id {
        return Err(PoisonReason::TenantMismatch);
    }
    let expected = state.applied_ordinal + 1;
    let ordinal = sealed.receipt.ordinal;
    if ordinal != expected {
        return Err(PoisonReason::NonDenseOrdinal {
            expected,
            found: ordinal,
        });
    }

    let record = decode_action_record(&envelope.payload).map_err(PoisonReason::Decode)?;
    if record.idempotency_key != envelope.idempotency_key {
        return Err(PoisonReason::ReceiptMismatch);
    }

    let action_id = ActionTypeId::new(envelope.action_type.clone())
        .map_err(|_| PoisonReason::InvalidActionType)?;
    let parameters = record
        .parameters
        .iter()
        .map(boundary::property)
        .collect::<Result<Vec<_>, _>>()
        .map_err(PoisonReason::Boundary)?;
    state
        .engine
        .check_action_parameter_conformance(&state.tenant_id, &action_id, &parameters)
        .map_err(PoisonReason::Parameters)?;

    let staged = stage_edits(state, envelope, &record)?;
    check_revision_binding(
        state,
        staged.entity_type.as_deref(),
        envelope.schema_revision,
    )?;
    commit(state, staged, envelope, &record, ordinal);
    Ok(())
}

/// Every edit lands on copies, committed together or not at all.
struct StagedEntry {
    objects: ObjectGraph,
    engine: OntologyEngine,
    entity_type: Option<String>,
}

fn stage_edits(
    state: &ProjectionState,
    envelope: &ActionEnvelope,
    record: &ActionRecord,
) -> Result<StagedEntry, PoisonReason> {
    let mut staged_objects = state.objects.clone();
    let mut staged_engine = state.engine.clone();
    let mut entity_type: Option<String> = state
        .bindings
        .get(&envelope.object_ref)
        .map(|binding| binding.entity_type.clone());

    for edit in record.edits.edits() {
        match edit {
            OntologyEdit::CreateObject {
                entity_type: declared,
                properties,
            } => {
                let converted = properties
                    .iter()
                    .map(boundary::property)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(PoisonReason::Boundary)?;
                let entity = ObjectEntity::new(
                    state.tenant_id.clone(),
                    envelope.object_ref.clone(),
                    declared.clone(),
                    converted,
                )
                .map_err(PoisonReason::Object)?;
                staged_engine
                    .check_instance_conformance(&entity)
                    .map_err(PoisonReason::Conformance)?;
                staged_objects
                    .upsert_entity(entity)
                    .map_err(PoisonReason::Object)?;
                entity_type = Some(declared.clone());
            }
            OntologyEdit::UpsertProperties { set } => {
                let mut entity = staged_objects
                    .get(&state.tenant_id, &envelope.object_ref)
                    .cloned()
                    .ok_or(PoisonReason::MissingObject)?;
                for wire in set {
                    let property = boundary::property(wire).map_err(PoisonReason::Boundary)?;
                    entity
                        .upsert_property(property)
                        .map_err(PoisonReason::Object)?;
                }
                staged_engine
                    .check_instance_conformance(&entity)
                    .map_err(PoisonReason::Conformance)?;
                staged_objects
                    .upsert_entity(entity)
                    .map_err(PoisonReason::Object)?;
            }
            OntologyEdit::CreateLink {
                link_type,
                to_entity_id,
            } => {
                let id = LinkTypeId::new(link_type.clone()).map_err(PoisonReason::Link)?;
                staged_engine
                    .register_link_instance(
                        &state.tenant_id,
                        &id,
                        &envelope.object_ref,
                        to_entity_id,
                    )
                    .map_err(PoisonReason::Link)?;
            }
        }
    }

    Ok(StagedEntry {
        objects: staged_objects,
        engine: staged_engine,
        entity_type,
    })
}

/// The envelope's stamped revision must have been ACCEPTED for the
/// object's entity type in this registry snapshot.
fn check_revision_binding(
    state: &ProjectionState,
    entity_type: Option<&str>,
    schema_revision: u32,
) -> Result<(), PoisonReason> {
    let Some(declared) = entity_type else {
        return Ok(());
    };
    let revision_known = EntityTypeId::new(declared.to_owned())
        .ok()
        .and_then(|id| {
            state
                .engine
                .entity_type_at_revision(&state.tenant_id, &id, schema_revision)
        })
        .is_some();
    if revision_known {
        Ok(())
    } else {
        Err(PoisonReason::UnknownRevision {
            revision: schema_revision,
        })
    }
}

fn commit(
    state: &mut ProjectionState,
    staged: StagedEntry,
    envelope: &ActionEnvelope,
    record: &ActionRecord,
    ordinal: u64,
) {
    state.objects = staged.objects;
    state.engine = staged.engine;
    if let Some(declared) = staged.entity_type {
        state.bindings.insert(
            envelope.object_ref.clone(),
            ObjectBinding {
                entity_type: declared,
                schema_revision: envelope.schema_revision,
                last_ordinal: ordinal,
                last_actor: record.principal_id.clone(),
            },
        );
    }
    state
        .history
        .entry(envelope.object_ref.clone())
        .or_default()
        .push(ordinal);
    state.applied_ordinal = ordinal;
}
