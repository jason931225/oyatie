//! Primary-key designations: which property identifies an object of a
//! given entity type.
//!
//! This is REGISTRY knowledge, not projection state. The registry is
//! fold input, so the designation arrives as an apply parameter rather
//! than as a stored field — which keeps the canonical entry bytes, and
//! therefore dedup identity, exactly what they were before keys
//! existed. A store enforces uniqueness without ever owning a
//! definition.

use std::collections::BTreeMap;

use data_ontology_kernel::PropertyValue;

use crate::store::{ProjectedObject, ProjectionStoreError};

/// The declared key property per entity type, stamped by the projector.
/// A type absent from this map declares no key and is unconstrained.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KeyDesignations {
    by_entity_type: BTreeMap<String, String>,
}

impl KeyDesignations {
    pub fn declaring(
        mut self,
        entity_type: impl Into<String>,
        property: impl Into<String>,
    ) -> Self {
        self.by_entity_type
            .insert(entity_type.into(), property.into());
        self
    }

    pub fn property_for(&self, entity_type: &str) -> Option<&str> {
        self.by_entity_type
            .get(entity_type)
            .map(|property| property.as_str())
    }

    /// Whether any type declares a key — lets a store skip the whole
    /// uniqueness pass when nothing is keyed.
    pub fn is_empty(&self) -> bool {
        self.by_entity_type.is_empty()
    }
}

/// Refuse an object that cannot be identified, or whose key is
/// already held — by a stored object OR by an earlier object in the
/// same entry, which a store-only scan would miss.
pub(crate) fn check_unique(
    objects: &BTreeMap<(String, String), ProjectedObject>,
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
    if matches!(
        value,
        data_ontology_kernel::PropertyValue::Array(_)
            | data_ontology_kernel::PropertyValue::Struct(_)
    ) {
        return Err(ProjectionStoreError::NonScalarPrimaryKey {
            property: property.to_owned(),
        });
    }

    let clash = earlier_in_entry
        .iter()
        .map(|candidate| (candidate.entity.id.as_str(), candidate))
        .chain(
            objects
                .range((object.entity.tenant_id.clone(), String::new())..)
                .take_while(|((tenant, _), _)| tenant == &object.entity.tenant_id)
                .map(|((_, object_ref), candidate)| (object_ref.as_str(), candidate)),
        )
        .find(|(object_ref, candidate)| {
            *object_ref != object.entity.id
                && candidate.entity.entity_type.value == entity_type
                && candidate
                    .entity
                    .properties
                    .get(property)
                    .is_some_and(|stored| &stored.value.value == value)
        });
    if let Some((held_by, _)) = clash {
        return Err(ProjectionStoreError::DuplicatePrimaryKey {
            property: property.to_owned(),
            held_by: held_by.to_owned(),
        });
    }
    Ok(())
}
