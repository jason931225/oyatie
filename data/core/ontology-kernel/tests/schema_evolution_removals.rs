#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

//! Property removal and revision-history retention under evolution.

#[path = "schema_evolution_support/mod.rs"]
mod support;
use support::*;

/// A large revision jump (e.g. 1 → 100) is permitted by strict-monotonicity;
/// only ordering is enforced, not continuity.
#[test]
fn large_revision_jump_accepted() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    let v100 = base_def(
        100,
        vec![prop("tag", PropertyTier::Scalar, internal(), false)],
    );
    let id = engine
        .evolve_entity_type(v100)
        .expect("large revision jump must be accepted if additive");

    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(stored.revision, 100);
}

/// Submitting a candidate with `revision == stored.revision` (same version)
/// must be rejected with `NonMonotonicRevision`. This mirrors Confluent Schema
/// Registry FORWARD compat: you cannot re-register the same version.
#[test]
fn equal_revision_rejected_with_non_monotonic_revision() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    assert_eq!(
        engine.evolve_entity_type(base_def(1, vec![])),
        Err(OntologyEngineError::NonMonotonicRevision),
        "equal revision must be rejected"
    );
}

/// Submitting a candidate with `revision < stored.revision` (downgrade) must
/// be rejected with `NonMonotonicRevision`.
#[test]
fn lower_revision_rejected_with_non_monotonic_revision() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(5, vec![])).unwrap();

    assert_eq!(
        engine.evolve_entity_type(base_def(3, vec![])),
        Err(OntologyEngineError::NonMonotonicRevision),
        "lower revision must be rejected"
    );
}

/// A breaking change (tier mutation) submitted with a higher revision must be
/// rejected with `IncompatibleSchemaEvolution`, not `NonMonotonicRevision`.
/// This confirms the compatibility check runs after the monotonicity gate.
#[test]
fn breaking_change_higher_revision_rejected_with_incompatible() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

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
        "breaking change with higher revision must yield IncompatibleSchemaEvolution"
    );
}

/// After a rejected evolution the stored definition must remain at its prior
/// revision and property set — atomic rollback on rejection.
#[test]
fn stored_definition_unchanged_after_rejected_evolution() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    // Attempt a breaking evolution (revision is higher but tier mutated).
    let breaking = EntityTypeDefinition::new(
        "ten_test",
        EntityTypeId::new("ety_thing").unwrap(),
        "Thing",
        vec![prop("name", PropertyTier::Vector, internal(), true)],
        2,
    )
    .unwrap();
    let _ = engine.evolve_entity_type(breaking);

    // Stored definition must still be at revision 1 with the original property.
    let id = EntityTypeId::new("ety_thing").unwrap();
    let stored = engine
        .entity_type("ten_test", &id)
        .expect("definition must still be registered after rejection");
    assert_eq!(
        stored.revision, 1,
        "revision must not change after rejected evolution"
    );
    assert_eq!(
        stored.properties.len(),
        1,
        "property count must not change after rejected evolution"
    );
    assert_eq!(
        stored.properties[0].tier,
        PropertyTier::Scalar,
        "property tier must not change after rejected evolution"
    );
}

/// `evolve_entity_type` must never return `DuplicateEntityType`. A second
/// call with a higher revision and additive changes returns `Ok(id)`.
#[test]
fn evolve_does_not_return_duplicate_entity_type_error_on_second_call() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();

    let result = engine.evolve_entity_type(base_def(
        2,
        vec![prop("tag", PropertyTier::Scalar, internal(), false)],
    ));

    assert!(
        result != Err(OntologyEngineError::DuplicateEntityType),
        "evolve_entity_type must never return DuplicateEntityType"
    );
    assert!(result.is_ok(), "valid evolution must return Ok(id)");
}

/// Entity type registrations are scoped per-tenant. Registering `ety_thing`
/// under `ten_other` and then evolving it under `ten_test` must be treated as
/// a first registration for `ten_test` (no cross-tenant leakage).
#[test]
fn tenant_isolation_evolve_does_not_see_other_tenant_registration() {
    let mut engine = OntologyEngine::default();

    // Register ety_thing under ten_other at revision 5.
    engine.evolve_entity_type(other_tenant_def(5)).unwrap();

    // Evolve ety_thing under ten_test at revision 1 — must be a first registration.
    let id = engine
        .evolve_entity_type(base_def(1, vec![]))
        .expect("evolve under ten_test must not see ten_other's registration");

    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(
        stored.revision, 1,
        "ten_test definition must start at revision 1, not inherit ten_other's 5"
    );

    // ten_other definition must be unaffected.
    let other_id = EntityTypeId::new("ety_thing").unwrap();
    let other_stored = engine.entity_type("ten_other", &other_id).unwrap();
    assert_eq!(
        other_stored.revision, 5,
        "ten_other definition must be unchanged"
    );
}

/// The revision field of the stored definition must be readable via the
/// `entity_type` accessor and reflect the latest successful evolution.
#[test]
fn entity_type_accessor_reflects_latest_revision_after_evolution() {
    let mut engine = OntologyEngine::default();
    engine.evolve_entity_type(base_def(1, vec![])).unwrap();
    engine
        .evolve_entity_type(base_def(
            2,
            vec![prop("code", PropertyTier::Scalar, internal(), false)],
        ))
        .unwrap();
    engine
        .evolve_entity_type(base_def(
            3,
            vec![
                prop("code", PropertyTier::Scalar, internal(), false),
                prop("tag", PropertyTier::Scalar, internal(), false),
            ],
        ))
        .unwrap();

    let id = EntityTypeId::new("ety_thing").unwrap();
    let stored = engine.entity_type("ten_test", &id).unwrap();
    assert_eq!(stored.revision, 3, "accessor must return latest revision");
    assert_eq!(
        stored.properties.len(),
        3,
        "all three properties must be present"
    );
}
