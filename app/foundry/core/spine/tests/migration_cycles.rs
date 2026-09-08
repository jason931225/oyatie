use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, OntologyEngine, PropertyTier,
};
use foundry_spine::{MigrationPlan, PlanError, UpcastTransform};

fn registry() -> OntologyEngine {
    let mut registry = OntologyEngine::default();
    for revision in [1, 2] {
        let properties = ["a", "b", "c", "d"]
            .into_iter()
            .map(|name| {
                EntityTypePropertyDefinition::new(
                    name,
                    PropertyTier::Scalar,
                    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap(),
                    false,
                )
                .unwrap()
            })
            .collect();
        let definition = EntityTypeDefinition::new(
            "ten_test",
            EntityTypeId::new("ety_reading").unwrap(),
            "Reading",
            properties,
            revision,
        )
        .unwrap();
        if revision == 1 {
            registry.register_entity_type(definition).unwrap();
        } else {
            registry.evolve_entity_type(definition).unwrap();
        }
    }
    // The plan's action, as the seed registers one. `validate` requires it to
    // EXIST and not merely to parse, so a registry without it is a fixture
    // less faithful than production rather than a smaller one.
    registry
        .register_action_type(
            ActionTypeDefinition::new(
                "ten_test",
                ActionTypeId::new("aty_upcast").unwrap(),
                EntityTypeId::new("ety_reading").unwrap(),
                "ops-console",
                AutonomyTier::T1Assist,
                "reading.upcast",
            )
            .unwrap(),
        )
        .unwrap();
    registry
}

fn plan(edges: &[(&str, &str)]) -> MigrationPlan {
    MigrationPlan {
        tenant_id: "ten_test".into(),
        entity_type: "ety_reading".into(),
        from_revision: 1,
        to_revision: 2,
        action_type: "aty_upcast".into(),
        audit_event_type: "reading.upcast".into(),
        declared_at_epoch_seconds: 1,
        transforms: edges
            .iter()
            .map(|(from, to)| UpcastTransform::CopyAs {
                from: (*from).into(),
                to: (*to).into(),
            })
            .collect(),
    }
}

#[test]
fn cyclic_cross_property_dependencies_are_refused() {
    for edges in [
        vec![("a", "b"), ("b", "a")],
        vec![("a", "b"), ("b", "c"), ("c", "a"), ("c", "d")],
    ] {
        assert_eq!(
            plan(&edges).validate(&registry()),
            Err(PlanError::CyclicTransforms)
        );
        let mut reversed = edges;
        reversed.reverse();
        assert_eq!(
            plan(&reversed).validate(&registry()),
            Err(PlanError::CyclicTransforms)
        );
    }
}

#[test]
fn acyclic_chains_fanout_and_same_field_copies_are_admitted() {
    for edges in [
        vec![("a", "b"), ("b", "c"), ("a", "d")],
        vec![("a", "a"), ("a", "b"), ("b", "c")],
        vec![],
    ] {
        assert_eq!(plan(&edges).validate(&registry()), Ok(()));
    }
}
