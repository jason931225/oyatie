//! Revision-history retention: every ACCEPTED entity-type definition is
//! retrievable at the exact revision it landed as; rejected candidates
//! never enter history.

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn prop(name: &str, required: bool) -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), required).unwrap()
}

fn def(
    tenant: &str,
    revision: u32,
    extra: Vec<EntityTypePropertyDefinition>,
) -> EntityTypeDefinition {
    let mut properties = vec![prop("name", true)];
    properties.extend(extra);
    EntityTypeDefinition::new(
        tenant,
        EntityTypeId::new("ety_profile").unwrap(),
        "Profile",
        properties,
        revision,
    )
    .unwrap()
}

fn ety() -> EntityTypeId {
    EntityTypeId::new("ety_profile").unwrap()
}

#[test]
fn accepted_revisions_are_retained_and_addressable() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(def("ten_test", 1, vec![]))
        .unwrap();
    engine
        .evolve_entity_type(def("ten_test", 2, vec![prop("email", false)]))
        .unwrap();
    engine
        .evolve_entity_type(def(
            "ten_test",
            4,
            vec![prop("email", false), prop("phone", false)],
        ))
        .unwrap();

    let v1 = engine
        .entity_type_at_revision("ten_test", &ety(), 1)
        .unwrap();
    assert_eq!(v1.properties.len(), 1);
    let v2 = engine
        .entity_type_at_revision("ten_test", &ety(), 2)
        .unwrap();
    assert_eq!(v2.properties.len(), 2);
    let v4 = engine
        .entity_type_at_revision("ten_test", &ety(), 4)
        .unwrap();
    assert_eq!(v4.properties.len(), 3);
    assert_eq!(
        engine.entity_type_at_revision("ten_test", &ety(), 3),
        None,
        "an unlanded revision has no history entry"
    );
    // The current view equals the highest retained revision.
    assert_eq!(engine.entity_type("ten_test", &ety()), Some(v4));
}

#[test]
fn rejected_candidates_never_enter_history() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(def("ten_test", 1, vec![prop("email", false)]))
        .unwrap();

    // Breaking: drops the prior "email" property.
    assert!(
        engine
            .evolve_entity_type(def("ten_test", 3, vec![]))
            .is_err()
    );
    assert_eq!(engine.entity_type_at_revision("ten_test", &ety(), 3), None);

    // Non-monotonic: equal revision.
    assert!(
        engine
            .evolve_entity_type(def("ten_test", 1, vec![prop("email", false)]))
            .is_err()
    );
    assert_eq!(
        engine
            .entity_type_at_revision("ten_test", &ety(), 1)
            .unwrap()
            .properties
            .len(),
        2,
        "the retained revision 1 is the originally accepted one"
    );
}

#[test]
fn first_registration_via_evolve_is_retained() {
    let mut engine = OntologyEngine::default();
    engine
        .evolve_entity_type(def("ten_test", 7, vec![]))
        .unwrap();
    assert!(
        engine
            .entity_type_at_revision("ten_test", &ety(), 7)
            .is_some()
    );
}

#[test]
fn revision_history_is_tenant_isolated() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(def("ten_a", 1, vec![]))
        .unwrap();
    engine
        .register_entity_type(def("ten_b", 1, vec![prop("email", false)]))
        .unwrap();

    assert_eq!(
        engine
            .entity_type_at_revision("ten_a", &ety(), 1)
            .unwrap()
            .properties
            .len(),
        1
    );
    assert_eq!(
        engine
            .entity_type_at_revision("ten_b", &ety(), 1)
            .unwrap()
            .properties
            .len(),
        2
    );
}
