use data_ontology_kernel::PropertyValue;

use crate::conformance::{ProjectionFixture, applied, fail, object};
use crate::keys::KeyDesignations;
use crate::store::{ProjectionStore, ProjectionStoreError};

fn reading_key() -> KeyDesignations {
    KeyDesignations::default().declaring("ety_reading", "serial")
}

fn serial(value: &str) -> Vec<(&'static str, PropertyValue)> {
    vec![("serial", PropertyValue::String(value.to_owned()))]
}

pub fn check_a_duplicate_primary_key_is_refused<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let keys = reading_key();
    store
        .apply(
            applied(
                "ten_a",
                1,
                vec![object("ten_a", "ent_a1", "ety_reading", serial("sn-1"))],
            ),
            &keys,
        )
        .map_err(|error| fail("the first holder of a key applies", format!("{error:?}")))?;

    let clash = store.apply(
        applied(
            "ten_a",
            2,
            vec![object("ten_a", "ent_a2", "ety_reading", serial("sn-1"))],
        ),
        &keys,
    );
    match clash {
        Err(ProjectionStoreError::DuplicatePrimaryKey {
            ref property,
            ref held_by,
        }) if property == "serial" && held_by == "ent_a1" => {}
        other => {
            return Err(fail(
                "a second object may not claim a held key; the refusal names the holder, never the value",
                format!("{other:?}"),
            ));
        }
    }

    let leaked = store
        .get("ten_a", "ent_a2")
        .map_err(|error| fail("get reads", format!("{error:?}")))?;
    if leaked.is_some() {
        return Err(fail(
            "a refused duplicate writes nothing",
            format!("{leaked:?}"),
        ));
    }
    Ok(())
}

pub fn check_an_object_may_keep_its_own_key<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let keys = reading_key();
    for ordinal in 1..=2 {
        let mut properties = serial("sn-1");
        properties.push(("note", PropertyValue::Integer(ordinal as i64)));
        store
            .apply(
                applied(
                    "ten_a",
                    ordinal,
                    vec![object("ten_a", "ent_a1", "ety_reading", properties)],
                ),
                &keys,
            )
            .map_err(|error| {
                fail(
                    "an object updating itself keeps its own key",
                    format!("{error:?}"),
                )
            })?;
    }
    Ok(())
}

pub fn check_keys_are_scoped_to_their_entity_type_and_tenant<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let keys = KeyDesignations::default()
        .declaring("ety_reading", "serial")
        .declaring("ety_gauge", "serial");
    store
        .apply(
            applied(
                "ten_a",
                1,
                vec![object("ten_a", "ent_a1", "ety_reading", serial("sn-1"))],
            ),
            &keys,
        )
        .map_err(|error| fail("seed apply", format!("{error:?}")))?;

    store
        .apply(
            applied(
                "ten_a",
                2,
                vec![object("ten_a", "ent_a2", "ety_gauge", serial("sn-1"))],
            ),
            &keys,
        )
        .map_err(|error| {
            fail(
                "the same value under a DIFFERENT entity type is a different key",
                format!("{error:?}"),
            )
        })?;

    store
        .apply(
            applied(
                "ten_b",
                1,
                vec![object("ten_b", "ent_b1", "ety_reading", serial("sn-1"))],
            ),
            &keys,
        )
        .map_err(|error| {
            fail(
                "another tenant's key space is its own",
                format!("{error:?}"),
            )
        })?;
    Ok(())
}

pub fn check_an_undeclared_key_constrains_nothing<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let keys = KeyDesignations::default();
    for (ordinal, object_ref) in [(1, "ent_a1"), (2, "ent_a2")] {
        store
            .apply(
                applied(
                    "ten_a",
                    ordinal,
                    vec![object("ten_a", object_ref, "ety_reading", serial("sn-1"))],
                ),
                &keys,
            )
            .map_err(|error| {
                fail(
                    "a type that declares no key constrains nothing",
                    format!("{error:?}"),
                )
            })?;
    }
    Ok(())
}

pub fn check_a_missing_key_property_is_refused<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let refusal = store.apply(
        applied(
            "ten_a",
            1,
            vec![object(
                "ten_a",
                "ent_a1",
                "ety_reading",
                vec![("unrelated", PropertyValue::Integer(1))],
            )],
        ),
        &reading_key(),
    );
    match refusal {
        Err(ProjectionStoreError::MissingPrimaryKey { ref property }) if property == "serial" => {
            Ok(())
        }
        other => Err(fail(
            "an object of a keyed type must carry its key property",
            format!("{other:?}"),
        )),
    }
}

pub fn check_two_objects_in_one_entry_cannot_share_a_key<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let clash = store.apply(
        applied(
            "ten_a",
            1,
            vec![
                object("ten_a", "ent_a1", "ety_reading", serial("sn-1")),
                object("ten_a", "ent_a2", "ety_reading", serial("sn-1")),
            ],
        ),
        &reading_key(),
    );
    match clash {
        Err(ProjectionStoreError::DuplicatePrimaryKey {
            ref property,
            ref held_by,
        }) if property == "serial" && held_by == "ent_a1" => {}
        other => {
            return Err(fail(
                "a key clash WITHIN one entry is caught too, not just against stored objects",
                format!("{other:?}"),
            ));
        }
    }
    let head = store
        .applied_head("ten_a")
        .map_err(|error| fail("head reads", format!("{error:?}")))?;
    if head != 0 {
        return Err(fail(
            "the refused entry spent no ordinal",
            format!("head={head}"),
        ));
    }
    Ok(())
}

pub fn check_a_composite_key_value_is_refused<F: ProjectionFixture>(
    fixture: &mut F,
) -> Result<(), String> {
    let store = fixture.store();
    let refusal = store.apply(
        applied(
            "ten_a",
            1,
            vec![object(
                "ten_a",
                "ent_a1",
                "ety_reading",
                vec![(
                    "serial",
                    PropertyValue::Array(vec![PropertyValue::Integer(1)]),
                )],
            )],
        ),
        &reading_key(),
    );
    match refusal {
        Err(ProjectionStoreError::NonScalarPrimaryKey { ref property }) if property == "serial" => {
            Ok(())
        }
        other => Err(fail(
            "identity must be a scalar — a composite key has no index affinity",
            format!("{other:?}"),
        )),
    }
}
