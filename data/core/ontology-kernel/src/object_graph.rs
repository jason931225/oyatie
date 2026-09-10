//! Object instance plane: tenant-keyed [`ObjectEntity`] rows in the
//! in-memory [`ObjectGraph`] registry.

use std::collections::BTreeMap;

use data_boundary_kernel::{Classified, DataClass};

use crate::property::{ObjectProperty, validate_property};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectEntity {
    pub tenant_id: String,
    pub id: String,
    pub entity_type: Classified<String>,
    pub properties: BTreeMap<String, ObjectProperty>, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectEntityUpsertOutcome {
    Created,
    Updated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectPropertyUpsertOutcome {
    Inserted,
    Updated,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObjectGraph {
    entities: BTreeMap<ObjectEntityKey, ObjectEntity>, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ObjectEntityKey {
    tenant_id: String,
    id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObjectGraphError {
    InvalidEntityId,
    EmptyEntityType,
    MissingProperties,
    EmptyPropertyName,
    InvalidDataClass,
}

impl ObjectEntity {
    pub fn new(
        tenant_id: String,
        id: String,
        entity_type: String,
        properties: Vec<ObjectProperty>,
    ) -> Result<Self, ObjectGraphError> {
        if !id.starts_with("ent_") {
            return Err(ObjectGraphError::InvalidEntityId);
        }
        if entity_type.trim().is_empty() {
            return Err(ObjectGraphError::EmptyEntityType);
        }
        if properties.is_empty() {
            return Err(ObjectGraphError::MissingProperties);
        }
        let mut by_name = BTreeMap::new();
        for property in properties {
            validate_property(&property)?;
            by_name.insert(property.name.clone(), property);
        }
        Ok(Self {
            tenant_id,
            id,
            entity_type: Classified::new(entity_type, DataClass::InternalOnly),
            properties: by_name,
        })
    }

    pub fn upsert_property(
        &mut self,
        property: ObjectProperty,
    ) -> Result<ObjectPropertyUpsertOutcome, ObjectGraphError> {
        validate_property(&property)?;
        let outcome = if self
            .properties
            .insert(property.name.clone(), property)
            .is_some()
        {
            ObjectPropertyUpsertOutcome::Updated
        } else {
            ObjectPropertyUpsertOutcome::Inserted
        };
        Ok(outcome)
    }
}

impl ObjectGraph {
    pub fn upsert_entity(
        &mut self,
        entity: ObjectEntity,
    ) -> Result<ObjectEntityUpsertOutcome, ObjectGraphError> {
        validate_entity_key(&entity.tenant_id, &entity.id)?;
        if entity.properties.is_empty() {
            return Err(ObjectGraphError::MissingProperties);
        }
        if entity.entity_type.value.trim().is_empty() {
            return Err(ObjectGraphError::EmptyEntityType);
        }
        for property in entity.properties.values() {
            validate_property(property)?;
        }

        let key = ObjectEntityKey {
            tenant_id: entity.tenant_id.clone(),
            id: entity.id.clone(),
        };
        let outcome = if self.entities.insert(key, entity).is_some() {
            ObjectEntityUpsertOutcome::Updated
        } else {
            ObjectEntityUpsertOutcome::Created
        };
        Ok(outcome)
    }

    pub fn get(&self, tenant_id: &str, entity_id: &str) -> Option<&ObjectEntity> {
        self.entities.get(&ObjectEntityKey {
            tenant_id: tenant_id.to_string(),
            id: entity_id.to_string(),
        })
    }

    pub fn entities_for_tenant(&self, tenant_id: &str) -> impl Iterator<Item = &ObjectEntity> {
        self.entities
            .range(
                ObjectEntityKey {
                    tenant_id: tenant_id.to_string(),
                    id: String::new(),
                }..,
            )
            .map_while(move |(key, entity)| (key.tenant_id == tenant_id).then_some(entity))
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

fn validate_entity_key(tenant_id: &str, entity_id: &str) -> Result<(), ObjectGraphError> {
    if tenant_id.trim().is_empty() || !entity_id.starts_with("ent_") {
        return Err(ObjectGraphError::InvalidEntityId);
    }
    Ok(())
}
