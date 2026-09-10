//! The migration attestation: the honest V1 completion fence — fixpoint
//! is `pending == 0` over a full rescan by the ONE predicate the runner
//! scans with, and per-object poisons are surfaced, never hidden behind
//! the fixpoint claim.

use data_ontology_kernel::EntityTypeId;

use crate::state::ProjectionState;

use super::MigrationPlan;
use super::runner::{computed_target, pending_objects};

/// Where one plan stands over one tenant's projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationAttestation {
    pub fixpoint: bool, // data_class: INTERNAL_ONLY
    /// The objects still owed, in deterministic order.
    pub pending: Vec<String>, // data_class: INTERNAL_ONLY
    /// Every poisoned ordinal in the projection — listed verbatim so a
    /// fixpoint claim can never launder a poisoned entry.
    pub poisoned: Vec<u64>, // data_class: INTERNAL_ONLY
}

/// Attest the plan against the projection. Pure; recomputable at any
/// moment; the same predicate `run_to_fixpoint` and the pinned view use.
pub fn migration_attestation(
    state: &ProjectionState,
    plan: &MigrationPlan,
) -> MigrationAttestation {
    let pending: Vec<String> = pending_objects(state, plan)
        .into_iter()
        .map(|owed| owed.object_ref)
        .collect();
    MigrationAttestation {
        fixpoint: pending.is_empty(),
        pending,
        poisoned: state.poison.keys().copied().collect(),
    }
}

/// Does the plan still owe this object anything? The pinned view's
/// refinement of `UpcastPending` — the SAME per-object computation the
/// runner scans with.
pub(crate) fn plan_owes(state: &ProjectionState, plan: &MigrationPlan, object_ref: &str) -> bool {
    let Ok(type_id) = EntityTypeId::new(plan.entity_type.clone()) else {
        return false;
    };
    let Some(head) = state.engine.entity_type(&state.tenant_id, &type_id) else {
        return false;
    };
    let Some(entity) = state.objects.get(&state.tenant_id, object_ref) else {
        return false;
    };
    plan.transforms
        .iter()
        .any(|transform| computed_target(transform, entity, head).is_some())
}
