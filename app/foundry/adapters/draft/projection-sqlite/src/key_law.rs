//! Primary-key enforcement against the property index.

use data_ontology_kernel::PropertyValue;
use foundry_projection_draft::{KeyDesignations, ProjectedObject, ProjectionStoreError};
use rusqlite::{OptionalExtension, params};

use crate::scan;
use crate::storage;

const HELD_BY: &str = "SELECT object_ref FROM projection_property_index
     WHERE tenant_id = ?1 AND entity_type = ?2 AND property = ?3
       AND value_kind = ?4 AND object_ref <> ?5";

pub(crate) fn check(
    transaction: &rusqlite::Transaction<'_>,
    tenant_id: &str,
    object: &ProjectedObject,
    keys: &KeyDesignations,
    earlier_in_entry: &[ProjectedObject],
) -> Result<(), ProjectionStoreError> {
    let entity_type = object.entity.entity_type.value.as_str();
    let Some(property) = keys.property_for(entity_type) else {
        return Ok(());
    };
    let Some(held) = object.entity.properties.get(property) else {
        return Err(ProjectionStoreError::MissingPrimaryKey {
            property: property.to_owned(),
        });
    };
    let value = &held.value.value;
    if matches!(value, PropertyValue::Array(_) | PropertyValue::Struct(_)) {
        return Err(ProjectionStoreError::NonScalarPrimaryKey {
            property: property.to_owned(),
        });
    }

    if let Some(clash) = earlier_in_entry.iter().find(|candidate| {
        candidate.entity.id != object.entity.id
            && candidate.entity.entity_type.value == entity_type
            && candidate
                .entity
                .properties
                .get(property)
                .is_some_and(|stored| &stored.value.value == value)
    }) {
        return Err(ProjectionStoreError::DuplicatePrimaryKey {
            property: property.to_owned(),
            held_by: clash.entity.id.clone(),
        });
    }

    let kind = value.type_label();
    let object_ref = object.entity.id.as_str();
    let held_by: Option<String> = if let Some(key) = scan::int_key(value) {
        transaction
            .query_row(
                &format!("{HELD_BY} AND int_value = ?6 LIMIT 1"),
                params![tenant_id, entity_type, property, kind, object_ref, key],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
    } else if let Some(key) = scan::real_key(value) {
        transaction
            .query_row(
                &format!("{HELD_BY} AND real_value = ?6 LIMIT 1"),
                params![tenant_id, entity_type, property, kind, object_ref, key],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
    } else if let Some(key) = value.as_str() {
        transaction
            .query_row(
                &format!("{HELD_BY} AND text_value = ?6 LIMIT 1"),
                params![tenant_id, entity_type, property, kind, object_ref, key],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?
    } else {
        None
    };

    match held_by {
        Some(held_by) => Err(ProjectionStoreError::DuplicatePrimaryKey {
            property: property.to_owned(),
            held_by,
        }),
        None => Ok(()),
    }
}
