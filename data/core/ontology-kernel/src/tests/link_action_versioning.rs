//! Link- and action-type versioning: strict monotonicity, frozen semantic
//! fields, and — for actions — the parameter law mirrored from properties:
//! existing quadruples frozen, new parameters optional-only.

use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn engine_with_types() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    for id in ["ety_org", "ety_worker"] {
        engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    "ten_test",
                    EntityTypeId::new(id).unwrap(),
                    "T",
                    vec![
                        EntityTypePropertyDefinition::new(
                            "name",
                            PropertyTier::Scalar,
                            internal(),
                            true,
                        )
                        .unwrap(),
                    ],
                    1,
                )
                .unwrap(),
            )
            .unwrap();
    }
    engine
}

fn link(cardinality: LinkCardinality) -> LinkTypeDefinition {
    LinkTypeDefinition::new(
        "ten_test",
        LinkTypeId::new("lty_employs").unwrap(),
        EntityTypeId::new("ety_org").unwrap(),
        EntityTypeId::new("ety_worker").unwrap(),
        cardinality,
        false,
    )
    .unwrap()
}

fn action() -> ActionTypeDefinition {
    ActionTypeDefinition::new(
        "ten_test",
        ActionTypeId::new("aty_hire").unwrap(),
        EntityTypeId::new("ety_worker").unwrap(),
        "console",
        AutonomyTier::T1Assist,
        "worker.hired",
    )
    .unwrap()
}

fn param(name: &str, required: bool) -> ActionParameterDefinition {
    ActionParameterDefinition::new(name, PropertyTier::Scalar, internal(), required).unwrap()
}

#[test]
fn link_evolution_is_monotonic_and_known_only() {
    let mut engine = engine_with_types();
    engine
        .register_link_type(link(LinkCardinality::OneToMany))
        .unwrap();
    assert_eq!(
        engine
            .link_type("ten_test", &LinkTypeId::new("lty_employs").unwrap())
            .unwrap()
            .revision,
        1
    );

    engine
        .evolve_link_type(link(LinkCardinality::OneToMany).with_revision(2))
        .expect("revision-only bump must evolve");
    assert_eq!(
        engine.evolve_link_type(link(LinkCardinality::OneToMany).with_revision(2)),
        Err(OntologyEngineError::NonMonotonicRevision)
    );

    let mut unknown = link(LinkCardinality::OneToMany).with_revision(2);
    unknown.id = LinkTypeId::new("lty_ghost").unwrap();
    assert_eq!(
        engine.evolve_link_type(unknown),
        Err(OntologyEngineError::UnknownLinkType)
    );
}

#[test]
fn link_semantic_fields_frozen() {
    let mut engine = engine_with_types();
    engine
        .register_link_type(link(LinkCardinality::OneToMany))
        .unwrap();

    let cases: Vec<(LinkTypeDefinition, &str)> = vec![
        (link(LinkCardinality::ManyToMany), "cardinality"),
        (
            {
                let mut l = link(LinkCardinality::OneToMany);
                l.allow_cross_tenant = true;
                l
            },
            "allow_cross_tenant",
        ),
        (
            {
                let mut l = link(LinkCardinality::OneToMany);
                l.to_entity_type = EntityTypeId::new("ety_org").unwrap();
                l
            },
            "to_entity_type",
        ),
    ];
    for (candidate, field) in cases {
        assert_eq!(
            engine.evolve_link_type(candidate.with_revision(2)),
            Err(OntologyEngineError::FrozenFieldChangedOnEvolution {
                field: field.to_string()
            })
        );
    }
}

#[test]
fn action_parameter_law_mirrored() {
    let mut engine = engine_with_types();
    engine
        .register_action_type(action().with_parameters(vec![param("reason", true)]))
        .unwrap();

    // Frozen semantic field, named.
    let mut retargeted = action().with_parameters(vec![param("reason", true)]);
    retargeted.max_autonomy_tier = AutonomyTier::T3Autonomous;
    assert_eq!(
        engine.evolve_action_type(retargeted.with_revision(2)),
        Err(OntologyEngineError::FrozenFieldChangedOnEvolution {
            field: "max_autonomy_tier".to_string()
        })
    );

    // New required parameter: breaking.
    assert_eq!(
        engine.evolve_action_type(
            action()
                .with_parameters(vec![param("reason", true), param("severity", true)])
                .with_revision(2)
        ),
        Err(OntologyEngineError::IncompatibleSchemaEvolution)
    );

    // Removed parameter: breaking.
    assert_eq!(
        engine.evolve_action_type(action().with_revision(2)),
        Err(OntologyEngineError::IncompatibleSchemaEvolution)
    );

    // Mutated existing quadruple: breaking.
    assert_eq!(
        engine.evolve_action_type(
            action()
                .with_parameters(vec![param("reason", false)])
                .with_revision(2)
        ),
        Err(OntologyEngineError::IncompatibleSchemaEvolution)
    );

    // The blessed change: a new optional parameter.
    engine
        .evolve_action_type(
            action()
                .with_parameters(vec![param("reason", true), param("note", false)])
                .with_revision(2),
        )
        .expect("new optional parameter must be additive");
    assert_eq!(
        engine
            .action_type("ten_test", &ActionTypeId::new("aty_hire").unwrap())
            .unwrap()
            .revision,
        2
    );
}
