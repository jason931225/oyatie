#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Additive-only backward compatibility and `evolve_entity_type`.

#[path = "schema_evolution_support/mod.rs"]
mod support;
use support::*;

/// Adding a new property to a definition whose existing properties are all
/// retained unchanged is a valid additive (backward-compatible) evolution.
/// This mirrors Protobuf field-add and Avro reader schema extension.
#[test]
fn additive_new_property_with_higher_revision_is_accepted() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    let v2 = base_def(2, vec![prop("email", PropertyTier::Scalar, pii(), false)]);
    let id = engine
        .evolve_entity_type(v2)
        .expect("additive evolution with higher revision must be accepted");

    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(stored.revision, 2, "stored revision must advance to 2");
    assert_eq!(
        stored.properties.len(),
        2,
        "both properties must be present"
    );
    assert!(
        stored.properties.iter().any(|p| p.name == "email"),
        "new 'email' property must be persisted"
    );
}

/// Adding multiple new properties in a single evolution step is still additive.
#[test]
fn multiple_additive_properties_in_single_evolution_accepted() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    let v2 = base_def(
        2,
        vec![
            prop("email", PropertyTier::Scalar, pii(), false),
            prop("embedding", PropertyTier::Vector, quasi(), false),
            prop("last_seen", PropertyTier::Timeseries, internal(), false),
        ],
    );
    let id = engine
        .evolve_entity_type(v2)
        .expect("multi-property additive evolution must be accepted");

    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(stored.revision, 2);
    assert_eq!(stored.properties.len(), 4, "base + 3 new properties");
}

/// Changing the `PropertyTier` of an existing property is a breaking schema
/// change — equivalent to changing a Protobuf field type. Must be rejected with
/// `IncompatibleSchemaEvolution`.
#[test]
fn tier_mutation_on_existing_property_rejected_with_incompatible() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    // Mutate "name": Scalar → Vector (breaking).
    let breaking = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_thing").unwrap(),
        "Thing",
        vec![prop("name", PropertyTier::Vector, internal(), true)],
        2,
    )
    .unwrap();

    assert_eq!(
        engine.evolve_entity_type(breaking),
        Err(OntologyEngineError::IncompatibleSchemaEvolution),
        "tier mutation must be rejected"
    );
}

/// Changing the `data_class` of an existing property violates the privacy
/// contract — equivalent to tightening/loosening Avro field schema. Must be
/// rejected with `IncompatibleSchemaEvolution`.
#[test]
fn data_class_mutation_on_existing_property_rejected_with_incompatible() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    // Mutate "name": InternalOnly → PiiIdentifying (breaking privacy contract).
    let breaking = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_thing").unwrap(),
        "Thing",
        vec![prop("name", PropertyTier::Scalar, pii(), true)],
        2,
    )
    .unwrap();

    assert_eq!(
        engine.evolve_entity_type(breaking),
        Err(OntologyEngineError::IncompatibleSchemaEvolution),
        "data_class mutation must be rejected"
    );
}

/// Flipping the `required` flag of an existing property is a breaking change
/// (consumers relying on presence will be surprised by absence). Must be
/// rejected with `IncompatibleSchemaEvolution`.
#[test]
fn required_flag_flip_on_existing_property_rejected_with_incompatible() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    // Flip "name" required: true → false.
    let breaking = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_thing").unwrap(),
        "Thing",
        vec![prop("name", PropertyTier::Scalar, internal(), false)],
        2,
    )
    .unwrap();

    assert_eq!(
        engine.evolve_entity_type(breaking),
        Err(OntologyEngineError::IncompatibleSchemaEvolution),
        "required flag flip must be rejected"
    );
}

/// Removing an existing property is the canonical breaking change in both
/// Protobuf and Avro schema evolution. Must be rejected with
/// `IncompatibleSchemaEvolution`.
#[test]
fn property_removal_rejected_with_incompatible() {
    let mut engine = OntologyEngine::default();
    // v1 has "name" + "code".
    engine
        .evolve_entity_type(base_def(
            1,
            vec![prop("code", PropertyTier::Scalar, internal(), true)],
        ))
        .unwrap();

    // v2 drops "code" — backward-incompatible.
    let dropping = base_def(2, vec![]);

    assert_eq!(
        engine.evolve_entity_type(dropping),
        Err(OntologyEngineError::IncompatibleSchemaEvolution),
        "property removal must be rejected"
    );
}

/// If multiple existing properties are mutated simultaneously the checker must
/// still return `IncompatibleSchemaEvolution` (first violation is sufficient).
#[test]
fn multiple_mutations_all_rejected_with_incompatible() {
    let mut engine = OntologyEngine::default();
    engine
        .evolve_entity_type(base_def(
            1,
            vec![prop("code", PropertyTier::Scalar, internal(), true)],
        ))
        .unwrap();

    // Mutate both "name" (tier) and "code" (data_class).
    let breaking = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_thing").unwrap(),
        "Thing",
        vec![
            prop("name", PropertyTier::Vector, internal(), true), // tier changed
            prop("code", PropertyTier::Scalar, pii(), true),      // data_class changed
        ],
        2,
    )
    .unwrap();

    assert_eq!(
        engine.evolve_entity_type(breaking),
        Err(OntologyEngineError::IncompatibleSchemaEvolution),
        "multiple simultaneous mutations must be rejected"
    );
}

// ---------------------------------------------------------------------------
// ST2 – OntologyEngine::evolve_entity_type
// ---------------------------------------------------------------------------

/// First call to `evolve_entity_type` with an unknown id behaves like
/// `register_entity_type`: inserts the definition and returns the id.
/// `DuplicateEntityType` is never surfaced from `evolve_entity_type`.
#[test]
fn first_registration_via_evolve_inserts_and_returns_id() {
    let mut engine = OntologyEngine::default();
    let def = base_def(1, vec![]);
    let id = engine
        .evolve_entity_type(def)
        .expect("first evolve call must succeed as first registration");

    assert_eq!(id.value, "ety_thing");
    assert!(
        engine.entity_type("ten_test", &id).is_some(),
        "definition must be retrievable after first-registration evolve"
    );
}

/// A second call with a strictly higher revision and only additive property
/// changes must succeed, replace the stored definition, and reflect the new
/// revision and property set.
#[test]
fn monotonic_additive_evolution_accepted_updates_stored_revision() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    let v2 = base_def(2, vec![prop("email", PropertyTier::Scalar, pii(), false)]);
    let id = engine
        .evolve_entity_type(v2)
        .expect("monotonic additive evolution must be accepted");

    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(
        stored.revision, 2,
        "stored revision must be 2 after evolution"
    );
    assert_eq!(
        stored.properties.len(),
        2,
        "stored definition must contain both base and new property"
    );
    assert!(
        stored.properties.iter().any(|p| p.name == "email"),
        "new property 'email' must be persisted in stored definition"
    );
}
