//! Guards on entity-type evolution beyond property-shape compatibility:
//! a new property must be optional (every existing instance lacks it), and
//! the pillar annotation is immutable (links were validated under it).

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn prop(name: &str, required: bool) -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), required).unwrap()
}

fn def(revision: u32, extra: Vec<EntityTypePropertyDefinition>) -> EntityTypeDefinition {
    let mut properties = vec![prop("name", true)];
    properties.extend(extra);
    EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_profile").unwrap(),
        "Profile",
        properties,
        revision,
    )
    .unwrap()
}

#[test]
fn new_required_property_rejected_as_incompatible() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(def(1, vec![])).unwrap();

    assert_eq!(
        engine.evolve_entity_type(def(2, vec![prop("email", true)])),
        Err(OntologyEngineError::IncompatibleSchemaEvolution),
        "a new required property is a breaking change, not an additive one"
    );
}

#[test]
fn new_optional_property_still_accepted() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(def(1, vec![])).unwrap();

    let id = engine
        .evolve_entity_type(def(2, vec![prop("email", false)]))
        .expect("optional new property must stay additive");
    assert_eq!(
        engine
            .entity_type("ten_test", &id)
            .unwrap()
            .properties
            .len(),
        2
    );
}

#[test]
fn stored_definition_unchanged_after_required_property_rejection() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(def(1, vec![])).unwrap();
    let id = EntityTypeId::new("ety_profile").unwrap();

    let _ = engine.evolve_entity_type(def(2, vec![prop("email", true)]));

    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(stored.revision, 1);
    assert_eq!(stored.properties.len(), 1);
}

fn pillar_def(revision: u32, pillar: Option<OntologyPillar>) -> EntityTypeDefinition {
    let base = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_person_record").unwrap(),
        "Person Record",
        vec![prop("name", true)],
        revision,
    )
    .unwrap();
    match pillar {
        Some(p) => base.with_pillar(p),
        None => base,
    }
}

#[test]
fn pillar_change_on_evolve_rejected() {
    for (from, to) in [
        (Some(OntologyPillar::Person), Some(OntologyPillar::Org)),
        (Some(OntologyPillar::Org), Some(OntologyPillar::Person)),
        (None, Some(OntologyPillar::Person)),
        (Some(OntologyPillar::Person), None),
    ] {
        let mut engine = OntologyEngine::default();
        engine.evolve_entity_type(pillar_def(1, from)).unwrap();

        assert_eq!(
            engine.evolve_entity_type(pillar_def(2, to)),
            Err(OntologyEngineError::PillarChangedOnEvolution),
            "pillar change {from:?} -> {to:?} must be rejected"
        );
    }
}

#[test]
fn unchanged_pillar_evolves_freely() {
    for pillar in [
        None,
        Some(OntologyPillar::Person),
        Some(OntologyPillar::Org),
    ] {
        let mut engine = OntologyEngine::default();
        engine.evolve_entity_type(pillar_def(1, pillar)).unwrap();
        engine
            .evolve_entity_type(pillar_def(2, pillar))
            .expect("same-pillar evolution must succeed");
    }
}
