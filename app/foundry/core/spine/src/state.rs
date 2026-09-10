//! The projection: disposable derived state, rebuildable from the log at
//! any moment. Everything is `BTreeMap`-backed and `Eq`-derivable so two
//! independent folds can be compared for byte-level agreement.

use std::collections::BTreeMap;

use data_ontology_kernel::{ObjectGraph, OntologyEngine};

use crate::fold::PoisonReason;

/// What the projection knows about one object beyond its kernel entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectBinding {
    /// The `ety_`-prefixed entity type the object was created under.
    pub entity_type: String, // data_class: INTERNAL_ONLY
    /// The schema revision stamped by the writer on the last applied
    /// envelope for this object.
    pub schema_revision: u32, // data_class: INTERNAL_ONLY
    /// The per-tenant ordinal of the last applied envelope.
    pub last_ordinal: u64, // data_class: INTERNAL_ONLY
    /// The principal of the last applied envelope — actor attribution
    /// straight from the payload.
    pub last_actor: String, // data_class: INTERNAL_ONLY
}

/// One tenant's projection: the fold's output, never written by anything
/// but the fold. The registry snapshot is a FOLD INPUT seeded at
/// construction; its link-instance store doubles as the projection's
/// kernel-cardinality-checked link state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionState {
    pub tenant_id: String, // data_class: INTERNAL_ONLY
    /// The registry snapshot this projection was seeded with, held
    /// verbatim so a checkpoint can prove it resumes against the SAME
    /// fold input. Definitions never change during a fold, so this stays
    /// the untouched seed while `engine` accumulates link instances.
    pub registry_input: OntologyEngine, // data_class: INTERNAL_ONLY
    /// The registry snapshot plus accumulated link instances. Links are
    /// written ONLY through `register_link_instance`, so kernel
    /// cardinality law holds by construction.
    pub engine: OntologyEngine, // data_class: INTERNAL_ONLY
    pub objects: ObjectGraph, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    /// Per-object bindings, keyed by `object_ref`.
    pub bindings: BTreeMap<String, ObjectBinding>, // data_class: INTERNAL_ONLY
    /// Per-object applied history: `object_ref` -> ordinals, in order.
    pub history: BTreeMap<String, Vec<u64>>, // data_class: INTERNAL_ONLY
    /// The poison ledger: ordinal -> the deterministic reason the entry
    /// could not apply. Poisoned entries advance `applied_ordinal` and
    /// touch nothing else.
    pub poison: BTreeMap<u64, PoisonReason>, // data_class: INTERNAL_ONLY
    /// The highest ordinal the fold has consumed (dense from 1).
    pub applied_ordinal: u64, // data_class: INTERNAL_ONLY
}

impl ProjectionState {
    /// A fresh projection for `tenant_id`, seeded with the registry
    /// snapshot the fold will be replayed against.
    pub fn new(tenant_id: impl Into<String>, registry: &OntologyEngine) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            registry_input: registry.clone(),
            engine: registry.clone(),
            objects: ObjectGraph::default(),
            bindings: BTreeMap::new(),
            history: BTreeMap::new(),
            poison: BTreeMap::new(),
            applied_ordinal: 0,
        }
    }
}
