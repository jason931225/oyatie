#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Acceptance tests for the ontology-kernel-link-instance-cardinality-enforcement slice.
//!
//! Exercises `OntologyEngine::register_link_instance` across all cardinality variants,
//! the unknown-link-type rejection, and idempotent re-insert behaviour.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, LinkCardinality,
    LinkInstanceOutcome, LinkTypeDefinition, LinkTypeId, OntologyEngine, OntologyEngineError,
    PropertyTier,
};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn prop(name: &str) -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new(name, PropertyTier::Scalar, internal(), true).unwrap()
}

fn entity(tenant: &str, id: &str) -> EntityTypeDefinition {
    EntityTypeDefinition::new(
        tenant,
        EntityTypeId::new(id).unwrap(),
        id,
        vec![prop("name")],
        1,
    )
    .unwrap()
}

fn engine_with_link_type(cardinality: LinkCardinality) -> (OntologyEngine, LinkTypeId) {
    let mut engine = OntologyEngine::default();
    engine
        .register_entity_type(entity("ten_t", "ety_from"))
        .unwrap();
    engine
        .register_entity_type(entity("ten_t", "ety_to"))
        .unwrap();
    let link_id = engine
        .register_link_type(
            LinkTypeDefinition::new(
                "ten_t",
                LinkTypeId::new("lty_edge").unwrap(),
                EntityTypeId::new("ety_from").unwrap(),
                EntityTypeId::new("ety_to").unwrap(),
                cardinality,
                false,
            )
            .unwrap(),
        )
        .unwrap();
    (engine, link_id)
}

#[derive(Clone, Copy, Debug)]
enum SecondEdge {
    Outbound,
    Inbound,
}

/// Calling `register_link_instance` with a `LinkTypeId` that was never registered
/// for the tenant must return `UnknownLinkType`.
#[test]
fn unknown_link_type_rejected() {
    let mut engine = OntologyEngine::default();
    let phantom = LinkTypeId::new("lty_phantom").unwrap();

    assert_eq!(
        engine.register_link_instance("ten_t", &phantom, "ent_a", "ent_b"),
        Err(OntologyEngineError::UnknownLinkType),
        "unregistered link type must be rejected"
    );
}

/// Every cardinality against both shapes of a second edge laid beside the
/// first `(ent_a, ent_b)`: a second OUTBOUND edge from `ent_a`, and a second
/// INBOUND edge into `ent_b`.
#[test]
fn cardinality_governs_the_second_edge() {
    let violation = |cardinality| Err(OntologyEngineError::CardinalityViolation { cardinality });
    let registered = || Ok(LinkInstanceOutcome::Registered);
    let rows = [
        (
            LinkCardinality::OneToOne,
            SecondEdge::Outbound,
            violation(LinkCardinality::OneToOne),
        ),
        (
            LinkCardinality::OneToOne,
            SecondEdge::Inbound,
            violation(LinkCardinality::OneToOne),
        ),
        (
            LinkCardinality::OneToMany,
            SecondEdge::Outbound,
            registered(),
        ),
        (
            LinkCardinality::OneToMany,
            SecondEdge::Inbound,
            violation(LinkCardinality::OneToMany),
        ),
        (
            LinkCardinality::ManyToMany,
            SecondEdge::Outbound,
            registered(),
        ),
        (
            LinkCardinality::ManyToMany,
            SecondEdge::Inbound,
            registered(),
        ),
    ];

    for (cardinality, second, expected) in rows {
        let (mut engine, link_id) = engine_with_link_type(cardinality);
        assert_eq!(
            engine.register_link_instance("ten_t", &link_id, "ent_a", "ent_b"),
            registered(),
            "{cardinality:?}: the first edge must always register"
        );
        let (from, to) = match second {
            SecondEdge::Outbound => ("ent_a", "ent_c"),
            SecondEdge::Inbound => ("ent_c", "ent_b"),
        };
        assert_eq!(
            engine.register_link_instance("ten_t", &link_id, from, to),
            expected,
            "{cardinality:?} with a second {second:?} edge ({from}, {to})"
        );
    }
}

/// Re-inserting the identical `(link_type_id, from_entity_id, to_entity_id)` tuple
/// must return `AlreadyExists` without mutation and without a cardinality error.
#[test]
fn idempotent_reinsert_returns_already_exists() {
    let (mut engine, link_id) = engine_with_link_type(LinkCardinality::OneToOne);

    assert_eq!(
        engine.register_link_instance("ten_t", &link_id, "ent_a", "ent_b"),
        Ok(LinkInstanceOutcome::Registered)
    );
    assert_eq!(
        engine.register_link_instance("ten_t", &link_id, "ent_a", "ent_b"),
        Ok(LinkInstanceOutcome::AlreadyExists),
        "re-inserting the identical edge tuple must return AlreadyExists"
    );
    assert_eq!(
        engine.register_link_instance("ten_t", &link_id, "ent_a", "ent_c"),
        Err(OntologyEngineError::CardinalityViolation {
            cardinality: LinkCardinality::OneToOne
        }),
        "cardinality must still be enforced after an idempotent re-insert"
    );
}
