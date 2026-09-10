//! Fixtures for the catch-up law: a registry, and log entries whose
//! content can be varied one field at a time so a divergent log is
//! divergent for exactly one reason.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, LinkCardinality, LinkTypeDefinition, LinkTypeId, OntologyEngine,
    PropertyTier,
};
use foundry_edits::{
    ActionRecord, EditSet, OntologyEdit, WireDataClass, WireProperty, WireTier, WireValue,
    encode_action_record,
};
use foundry_projection_draft::{PageRequest, ProjectionStore};
use foundry_records_draft::{ActionEnvelope, Receipt, SealedEnvelope};
use foundry_spine::{ProjectionState, poison_label};

pub(crate) const TENANT: &str = "ten_test";

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

/// `ety_reading` keys on `name`, so a rebuild that reached the store
/// without the registry's key designation would lose identity law —
/// which is why catch-up folds through the projector rather than
/// copying rows.
pub(crate) fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    let definition = EntityTypeDefinition::new(
        TENANT,
        EntityTypeId::new("ety_reading").unwrap(),
        "Reading",
        vec![
            EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal(), true)
                .unwrap(),
        ],
        1,
    )
    .unwrap()
    .with_primary_key_property("name");
    engine.register_entity_type(definition).unwrap();
    engine
        .register_link_type(
            LinkTypeDefinition::new(
                TENANT,
                LinkTypeId::new("lty_measures").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                LinkCardinality::ManyToMany,
                false,
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .register_action_type(
            ActionTypeDefinition::new(
                TENANT,
                ActionTypeId::new("aty_calibrate").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                "ops-console",
                AutonomyTier::T1Assist,
                "reading.calibrated",
            )
            .unwrap(),
        )
        .unwrap();
    engine
}

fn name_property(value: &str) -> WireProperty {
    WireProperty::new(
        "name",
        WireTier::Scalar,
        WireDataClass::InternalOnly,
        WireValue::String(value.into()),
    )
    .unwrap()
}

fn envelope(actor: &str, object_ref: &str, ordinal: u64, name: &str) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    let record = ActionRecord::new(
        actor,
        "dec_1",
        "reading.calibrated",
        &key,
        1_700_000_000_000,
        vec![],
        EditSet::new(vec![
            OntologyEdit::create_object("ety_reading", vec![name_property(name)]).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            TENANT,
            object_ref,
            "aty_calibrate",
            &key,
            1,
            encode_action_record(&record),
            1_700_000_000_000,
        )
        .unwrap(),
        receipt: Receipt {
            ordinal,
            object_sequence: 1,
            deduplicated: false,
        },
    }
}

/// One well-formed entry: object `ent_{ordinal}` named `name`.
pub(crate) fn sealed(ordinal: u64, name: &str) -> SealedEnvelope {
    envelope("prn_alice", &format!("ent_{ordinal}"), ordinal, name)
}

/// The same entry written by a different principal. Differs from
/// [`sealed`] ONLY in `last_actor`, which reaches the store through
/// `ProjectedObject` — so a store built from the other log diverges
/// here without also tripping the primary-key law, and the refusal
/// under test is unambiguous.
pub(crate) fn sealed_by_another_actor(ordinal: u64, name: &str) -> SealedEnvelope {
    envelope("prn_bob", &format!("ent_{ordinal}"), ordinal, name)
}

/// Payload bytes the decoder refuses. Poisons identically on every
/// replay, because a poison derives from (log bytes, registry).
pub(crate) fn corrupt(ordinal: u64) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            TENANT,
            format!("ent_{ordinal}"),
            "aty_calibrate",
            &key,
            1,
            vec![0xFF],
            1_700_000_000_000,
        )
        .unwrap(),
        receipt: Receipt {
            ordinal,
            object_sequence: 1,
            deduplicated: false,
        },
    }
}

/// Stamped with a schema revision the registry never accepted, so the
/// fold poisons it `unknown_revision`. Same shape as [`corrupt`] and a
/// DIFFERENT reason, which is the axis a count — or an ordinal-only
/// comparison — cannot see.
pub(crate) fn sealed_at_unknown_revision(ordinal: u64) -> SealedEnvelope {
    let mut entry = sealed(ordinal, "unreadable revision");
    entry.envelope.schema_revision = 99;
    entry
}

/// An entry that registers an outbound edge. FROM is the envelope's own
/// object, per spine law.
pub(crate) fn sealed_link(ordinal: u64, from_ref: &str, to_ref: &str) -> SealedEnvelope {
    let key = format!("idem_{ordinal}");
    let record = ActionRecord::new(
        "prn_alice",
        "dec_1",
        "reading.calibrated",
        &key,
        1_700_000_000_000,
        vec![],
        EditSet::new(vec![
            OntologyEdit::create_link("lty_measures", to_ref).unwrap(),
        ])
        .unwrap(),
    )
    .unwrap();
    SealedEnvelope {
        envelope: ActionEnvelope::new(
            TENANT,
            from_ref,
            "aty_calibrate",
            &key,
            1,
            encode_action_record(&record),
            1_700_000_000_000,
        )
        .unwrap(),
        receipt: Receipt {
            ordinal,
            object_sequence: 1,
            deduplicated: false,
        },
    }
}

/// A well-formed entry belonging to a DIFFERENT tenant. The fold would
/// poison it `TenantMismatch`, spending an ordinal in the wrong ledger.
pub(crate) fn sealed_for_another_tenant(ordinal: u64) -> SealedEnvelope {
    let mut foreign = sealed(ordinal, "theirs");
    foreign.envelope.tenant_id = "ten_other".to_owned();
    foreign
}

/// A three-entry log, dense from ordinal 1.
pub(crate) fn log() -> Vec<SealedEnvelope> {
    vec![sealed(1, "one"), sealed(2, "two"), sealed(3, "three")]
}

/// A log that exercises objects, an edge, and a poison together — so an
/// equivalence claim over it is not vacuous on any of the three.
pub(crate) fn mixed_log() -> Vec<SealedEnvelope> {
    vec![
        sealed(1, "one"),
        sealed(2, "two"),
        sealed_link(3, "ent_1", "ent_2"),
        corrupt(4),
        sealed(5, "five"),
    ]
}

/// The oracle both planes are held to: `fold(log)` is the definition of
/// correct, not a second hand-written expectation that could drift.
///
/// The poison ledger is compared by ordinal AND reason rather than by
/// count, and the object set for EQUALITY: iterating only the fold's own
/// bindings proves the store holds everything it should and never that
/// it holds nothing MORE, so a row retained from another log would be
/// invisible.
///
/// Edges are not compared here: the fold keeps link instances in the
/// kernel engine rather than in a set this module can cheaply
/// enumerate. Tests that exercise edges assert them directly, and the
/// durability test compares them after dropping the connection — the
/// gap is covered by name, not left to be noticed.
pub(crate) fn assert_agrees_with_fold(store: &dyn ProjectionStore, state: &ProjectionState) {
    let held: Vec<String> = store
        .objects_of_type(TENANT, "ety_reading", &PageRequest::first(1000))
        .unwrap()
        .objects
        .iter()
        .map(|object| object.entity.id.clone())
        .collect();
    let folded: Vec<String> = state.bindings.keys().cloned().collect();
    assert_eq!(
        held, folded,
        "the store must hold exactly the fold's objects — no more"
    );
    assert_eq!(
        store.applied_head(TENANT).unwrap(),
        state.applied_ordinal,
        "the store's head must be the fold's ordinal"
    );
    let expected: Vec<(u64, String)> = state
        .poison
        .iter()
        .map(|(ordinal, reason)| (*ordinal, poison_label(reason).to_owned()))
        .collect();
    assert_eq!(
        store.poisoned(TENANT).unwrap(),
        expected,
        "the poison ledger must match the fold's by ordinal AND reason"
    );
    for (object_ref, binding) in &state.bindings {
        let projected = store
            .get(TENANT, object_ref)
            .unwrap()
            .unwrap_or_else(|| panic!("the store is missing {object_ref}"));
        assert_eq!(
            &projected.entity,
            state.objects.get(TENANT, object_ref).unwrap(),
            "{object_ref} differs from the fold"
        );
        assert_eq!(projected.last_ordinal, binding.last_ordinal);
        assert_eq!(projected.last_actor, binding.last_actor);
        assert_eq!(projected.schema_revision, binding.schema_revision);
    }
}
