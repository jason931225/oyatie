use crate::*;
use data_boundary_kernel::{DataClass, PrivacyDataClass};

use crate::engine::check_property_compatibility;

fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

fn pii() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::PiiIdentifying).unwrap()
}

fn prop(
    name: &str,
    tier: PropertyTier,
    data_class: PrivacyDataClass,
    required: bool,
) -> EntityTypePropertyDefinition {
    EntityTypePropertyDefinition::new(name, tier, data_class, required).unwrap()
}

fn base_def(revision: u32, extra_props: Vec<EntityTypePropertyDefinition>) -> EntityTypeDefinition {
    let mut props = vec![prop("name", PropertyTier::Scalar, internal(), true)];
    props.extend(extra_props);
    EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_thing").unwrap(),
        "Thing",
        props,
        revision,
    )
    .unwrap()
}

mod check_property_compatibility {
    use super::*;

    #[test]
    fn additive_property_is_accepted() {
        let prior = base_def(1, vec![]);
        let candidate = base_def(2, vec![prop("email", PropertyTier::Scalar, pii(), false)]);
        assert_eq!(check_property_compatibility(&prior, &candidate), Ok(()));
    }

    #[test]
    fn tier_mutation_rejected() {
        let prior = base_def(1, vec![]);
        // Change "name" from Scalar → Vector.
        let candidate = EntityTypeDefinition::new(
            "ten_test",
            EntityTypeId::new("ety_thing").unwrap(),
            "Thing",
            vec![prop("name", PropertyTier::Vector, internal(), true)],
            2,
        )
        .unwrap();
        assert_eq!(
            check_property_compatibility(&prior, &candidate),
            Err(OntologyEngineError::IncompatibleSchemaEvolution)
        );
    }

    #[test]
    fn data_class_mutation_rejected() {
        let prior = base_def(1, vec![]);
        // Change "name" from InternalOnly → PiiIdentifying.
        let candidate = EntityTypeDefinition::new(
            "ten_test",
            EntityTypeId::new("ety_thing").unwrap(),
            "Thing",
            vec![prop("name", PropertyTier::Scalar, pii(), true)],
            2,
        )
        .unwrap();
        assert_eq!(
            check_property_compatibility(&prior, &candidate),
            Err(OntologyEngineError::IncompatibleSchemaEvolution)
        );
    }

    #[test]
    fn required_flip_rejected() {
        let prior = base_def(1, vec![]);
        // Flip "name" required: true → false.
        let candidate = EntityTypeDefinition::new(
            "ten_test",
            EntityTypeId::new("ety_thing").unwrap(),
            "Thing",
            vec![prop("name", PropertyTier::Scalar, internal(), false)],
            2,
        )
        .unwrap();
        assert_eq!(
            check_property_compatibility(&prior, &candidate),
            Err(OntologyEngineError::IncompatibleSchemaEvolution)
        );
    }

    #[test]
    fn property_removal_rejected() {
        let prior = base_def(
            1,
            vec![prop("code", PropertyTier::Scalar, internal(), true)],
        );
        // candidate drops "code".
        let candidate = base_def(2, vec![]);
        assert_eq!(
            check_property_compatibility(&prior, &candidate),
            Err(OntologyEngineError::IncompatibleSchemaEvolution)
        );
    }
}

mod evolve_entity_type {
    use super::*;

    #[test]
    fn first_registration_via_evolve_succeeds() {
        let mut engine = OntologyEngine::default();
        let def = base_def(1, vec![]);
        let id = engine.evolve_entity_type(def).unwrap();
        assert_eq!(id.value, "ety_thing");
        assert!(engine.entity_type("ten_test", &id).is_some());
    }

    #[test]
    fn monotonic_additive_evolution_accepted_and_stored() {
        let mut engine = OntologyEngine::default();
        engine.evolve_entity_type(base_def(1, vec![])).unwrap();

        let v2 = base_def(2, vec![prop("email", PropertyTier::Scalar, pii(), false)]);
        let id = engine.evolve_entity_type(v2).unwrap();
        assert_eq!(id.value, "ety_thing");

        let stored = engine.entity_type("ten_test", &id).unwrap();
        assert_eq!(stored.revision, 2);
        assert_eq!(stored.properties.len(), 2);
        assert!(stored.properties.iter().any(|p| p.name == "email"));
    }

    #[test]
    fn equal_revision_rejected_with_non_monotonic() {
        let mut engine = OntologyEngine::default();
        engine.evolve_entity_type(base_def(1, vec![])).unwrap();
        assert_eq!(
            engine.evolve_entity_type(base_def(1, vec![])),
            Err(OntologyEngineError::NonMonotonicRevision)
        );
    }

    #[test]
    fn lower_revision_rejected_with_non_monotonic() {
        let mut engine = OntologyEngine::default();
        engine.evolve_entity_type(base_def(5, vec![])).unwrap();
        assert_eq!(
            engine.evolve_entity_type(base_def(3, vec![])),
            Err(OntologyEngineError::NonMonotonicRevision)
        );
    }

    #[test]
    fn breaking_change_higher_revision_rejected() {
        let mut engine = OntologyEngine::default();
        engine.evolve_entity_type(base_def(1, vec![])).unwrap();

        // revision 1 → 2 but "name" tier is mutated.
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
            Err(OntologyEngineError::IncompatibleSchemaEvolution)
        );
        // Stored definition must be unchanged after the rejection.
        let id = EntityTypeId::new("ety_thing").unwrap();
        let stored = engine.entity_type("ten_test", &id).unwrap();
        assert_eq!(stored.revision, 1);
    }
}
