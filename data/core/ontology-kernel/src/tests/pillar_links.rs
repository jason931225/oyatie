use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

use super::support::property;

fn patient_type_with_pillar(pillar: OntologyPillar) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_hr",
        EntityTypeId::new("ety_person").unwrap(),
        "Person",
        vec![property("name")],
        1,
    )
    .unwrap()
    .with_pillar(pillar)
}

fn org_type_with_pillar(pillar: OntologyPillar) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_hr",
        EntityTypeId::new("ety_company").unwrap(),
        "Company",
        vec![property("name")],
        1,
    )
    .unwrap()
    .with_pillar(pillar)
}

fn agnostic_entity_type(id: &str) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        "ten_hr",
        EntityTypeId::new(id).unwrap(),
        "Agnostic",
        vec![property("name")],
        1,
    )
    .unwrap()
    // no with_pillar call — pillar: None
}

#[test]
fn cross_pillar_link_org_to_person_rejected() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(org_type_with_pillar(OntologyPillar::Org))
        .unwrap();
    engine
        .register_entity_type(patient_type_with_pillar(OntologyPillar::Person))
        .unwrap();
    let link = LinkTypeDefinition::new(
        "ten_hr",
        LinkTypeId::new("lty_org_person").unwrap(),
        EntityTypeId::new("ety_company").unwrap(),
        EntityTypeId::new("ety_person").unwrap(),
        LinkCardinality::OneToMany,
        false,
    )
    .unwrap();
    assert_eq!(
        engine.register_link_type(link),
        Err(OntologyEngineError::CrossPillarLink)
    );
}

#[test]
fn cross_pillar_link_person_to_org_rejected() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(patient_type_with_pillar(OntologyPillar::Person))
        .unwrap();
    engine
        .register_entity_type(org_type_with_pillar(OntologyPillar::Org))
        .unwrap();
    let link = LinkTypeDefinition::new(
        "ten_hr",
        LinkTypeId::new("lty_person_org").unwrap(),
        EntityTypeId::new("ety_person").unwrap(),
        EntityTypeId::new("ety_company").unwrap(),
        LinkCardinality::ManyToMany,
        false,
    )
    .unwrap();
    assert_eq!(
        engine.register_link_type(link),
        Err(OntologyEngineError::CrossPillarLink)
    );
}

#[test]
fn same_pillar_link_org_to_org_accepted() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(org_type_with_pillar(OntologyPillar::Org))
        .unwrap();
    let subsidiary = EntityTypeDefinition::new(
        "ten_hr",
        EntityTypeId::new("ety_subsidiary").unwrap(),
        "Subsidiary",
        vec![property("name")],
        1,
    )
    .unwrap()
    .with_pillar(OntologyPillar::Org);
    engine.register_entity_type(subsidiary).unwrap();
    let link = LinkTypeDefinition::new(
        "ten_hr",
        LinkTypeId::new("lty_parent_subsidiary").unwrap(),
        EntityTypeId::new("ety_company").unwrap(),
        EntityTypeId::new("ety_subsidiary").unwrap(),
        LinkCardinality::OneToMany,
        false,
    )
    .unwrap();
    assert!(engine.register_link_type(link).is_ok());
}

#[test]
fn same_pillar_link_person_to_person_accepted() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(patient_type_with_pillar(OntologyPillar::Person))
        .unwrap();
    let contact = EntityTypeDefinition::new(
        "ten_hr",
        EntityTypeId::new("ety_contact").unwrap(),
        "Contact",
        vec![property("email")],
        1,
    )
    .unwrap()
    .with_pillar(OntologyPillar::Person);
    engine.register_entity_type(contact).unwrap();
    let link = LinkTypeDefinition::new(
        "ten_hr",
        LinkTypeId::new("lty_person_contact").unwrap(),
        EntityTypeId::new("ety_person").unwrap(),
        EntityTypeId::new("ety_contact").unwrap(),
        LinkCardinality::OneToOne,
        false,
    )
    .unwrap();
    assert!(engine.register_link_type(link).is_ok());
}

#[test]
fn pillar_agnostic_link_both_none_accepted() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(agnostic_entity_type("ety_agnostic_a"))
        .unwrap();
    engine
        .register_entity_type(agnostic_entity_type("ety_agnostic_b"))
        .unwrap();
    let link = LinkTypeDefinition::new(
        "ten_hr",
        LinkTypeId::new("lty_agnostic").unwrap(),
        EntityTypeId::new("ety_agnostic_a").unwrap(),
        EntityTypeId::new("ety_agnostic_b").unwrap(),
        LinkCardinality::ManyToMany,
        false,
    )
    .unwrap();
    assert!(engine.register_link_type(link).is_ok());
}

#[test]
fn one_pillar_agnostic_endpoint_accepted() {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(org_type_with_pillar(OntologyPillar::Org))
        .unwrap();
    engine
        .register_entity_type(agnostic_entity_type("ety_agnostic_b"))
        .unwrap();
    let link = LinkTypeDefinition::new(
        "ten_hr",
        LinkTypeId::new("lty_org_agnostic").unwrap(),
        EntityTypeId::new("ety_company").unwrap(),
        EntityTypeId::new("ety_agnostic_b").unwrap(),
        LinkCardinality::OneToMany,
        false,
    )
    .unwrap();
    assert!(engine.register_link_type(link).is_ok());
}

#[test]
fn all_link_cardinality_variants_accepted_same_pillar() {
    for (link_id, cardinality) in [
        ("lty_one_one", LinkCardinality::OneToOne),
        ("lty_one_many", LinkCardinality::OneToMany),
        ("lty_many_many", LinkCardinality::ManyToMany),
    ] {
        let mut engine = OntologyEngine::default();
        engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    "ten_hr",
                    EntityTypeId::new("ety_from").unwrap(),
                    "From",
                    vec![property("x")],
                    1,
                )
                .unwrap()
                .with_pillar(OntologyPillar::Org),
            )
            .unwrap();
        engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    "ten_hr",
                    EntityTypeId::new("ety_to").unwrap(),
                    "To",
                    vec![property("y")],
                    1,
                )
                .unwrap()
                .with_pillar(OntologyPillar::Org),
            )
            .unwrap();
        let link = LinkTypeDefinition::new(
            "ten_hr",
            LinkTypeId::new(link_id).unwrap(),
            EntityTypeId::new("ety_from").unwrap(),
            EntityTypeId::new("ety_to").unwrap(),
            cardinality,
            false,
        )
        .unwrap();
        assert!(
            engine.register_link_type(link).is_ok(),
            "cardinality {:?} should be accepted",
            cardinality
        );
    }
}
