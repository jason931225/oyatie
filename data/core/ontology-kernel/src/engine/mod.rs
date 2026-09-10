//! The tenant-scoped ontology registry engine.

mod conformance;
mod evolution;
mod invocation;
mod links;
mod value_conformance;

pub use links::LinkInstanceOutcome;

#[cfg(test)]
pub(crate) use evolution::check_property_compatibility;

use std::collections::BTreeMap;

use crate::definitions::{
    ActionTypeDefinition, ActionTypeId, EntityTypeDefinition, EntityTypeId, LinkTypeDefinition,
    LinkTypeId,
};
use crate::error::OntologyEngineError;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OntologyEngine {
    entity_types: BTreeMap<OntologyScopedKey, EntityTypeDefinition>,
    link_types: BTreeMap<OntologyScopedKey, LinkTypeDefinition>,
    action_types: BTreeMap<OntologyScopedKey, ActionTypeDefinition>,
    /// Only ACCEPTED definitions are retained; a rejected candidate never
    /// enters the history.
    entity_type_revisions: BTreeMap<EntityTypeRevisionKey, EntityTypeDefinition>,
    link_instances: BTreeMap<LinkInstanceKey, ()>,
    link_outbound: BTreeMap<LinkEndpointKey, ()>,
    link_inbound: BTreeMap<LinkEndpointKey, ()>,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct OntologyScopedKey {
    tenant_id: String,
    id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct EntityTypeRevisionKey {
    tenant_id: String,
    entity_type_id: String,
    revision: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct LinkInstanceKey {
    tenant_id: String,
    link_type_id: String,
    from_entity_id: String,
    to_entity_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct LinkEndpointKey {
    tenant_id: String,
    link_type_id: String,
    entity_id: String,
}

impl EntityTypeRevisionKey {
    fn new(tenant_id: &str, entity_type_id: &str, revision: u32) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            entity_type_id: entity_type_id.to_string(),
            revision,
        }
    }
}

impl LinkInstanceKey {
    pub(super) fn new(
        tenant_id: &str,
        link_type_id: &LinkTypeId,
        from_entity_id: &str,
        to_entity_id: &str,
    ) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            link_type_id: link_type_id.value.clone(),
            from_entity_id: from_entity_id.to_string(),
            to_entity_id: to_entity_id.to_string(),
        }
    }
}

impl LinkEndpointKey {
    pub(super) fn new(tenant_id: &str, link_type_id: &LinkTypeId, entity_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            link_type_id: link_type_id.value.clone(),
            entity_id: entity_id.to_string(),
        }
    }
}

impl OntologyEngine {
    pub fn register_entity_type(
        &mut self,
        definition: EntityTypeDefinition,
    ) -> Result<EntityTypeId, OntologyEngineError> {
        check_designation_integrity(&definition)?;
        crate::display::check_display_integrity(definition.display.as_ref())?;
        for property in &definition.properties {
            crate::display::check_display_integrity(property.display.as_ref())?;
        }
        check_value_type_integrity(
            definition
                .properties
                .iter()
                .map(|p| (p.name.as_str(), &p.tier, p.value_type.as_ref())),
        )?;
        let key = ontology_scoped_key(&definition.tenant_id, &definition.id.value);
        if self.entity_types.contains_key(&key) {
            return Err(OntologyEngineError::DuplicateEntityType);
        }
        let id = definition.id.clone();
        self.retain_entity_type_revision(&definition);
        self.entity_types.insert(key, definition);
        Ok(id)
    }

    pub fn register_link_type(
        &mut self,
        definition: LinkTypeDefinition,
    ) -> Result<LinkTypeId, OntologyEngineError> {
        let from_def =
            self.registered_endpoint(&definition.tenant_id, &definition.from_entity_type)?;
        let to_def = self.registered_endpoint(&definition.tenant_id, &definition.to_entity_type)?;
        crate::display::check_display_integrity(definition.display.as_ref())?;
        check_pillar_consistency(from_def, to_def)?;
        let key = ontology_scoped_key(&definition.tenant_id, &definition.id.value);
        if self.link_types.contains_key(&key) {
            return Err(OntologyEngineError::DuplicateLinkType);
        }
        let id = definition.id.clone();
        self.link_types.insert(key, definition);
        Ok(id)
    }
    pub fn register_action_type(
        &mut self,
        definition: ActionTypeDefinition,
    ) -> Result<ActionTypeId, OntologyEngineError> {
        self.registered_endpoint(&definition.tenant_id, &definition.entity_type)?;
        crate::display::check_display_integrity(definition.display.as_ref())?;
        check_parameter_names_unique(&definition)?;
        check_value_type_integrity(
            definition
                .parameters
                .iter()
                .map(|p| (p.name.as_str(), &p.tier, p.value_type.as_ref())),
        )?;
        let key = ontology_scoped_key(&definition.tenant_id, &definition.id.value);
        if self.action_types.contains_key(&key) {
            return Err(OntologyEngineError::DuplicateActionType);
        }
        let id = definition.id.clone();
        self.action_types.insert(key, definition);
        Ok(id)
    }
    pub fn entity_type(&self, tenant_id: &str, id: &EntityTypeId) -> Option<&EntityTypeDefinition> {
        self.entity_types
            .get(&ontology_scoped_key(tenant_id, &id.value))
    }
    pub fn entity_type_at_revision(
        &self,
        tenant_id: &str,
        id: &EntityTypeId,
        revision: u32,
    ) -> Option<&EntityTypeDefinition> {
        self.entity_type_revisions
            .get(&EntityTypeRevisionKey::new(tenant_id, &id.value, revision))
    }
    pub(crate) fn retain_entity_type_revision(&mut self, definition: &EntityTypeDefinition) {
        self.entity_type_revisions.insert(
            EntityTypeRevisionKey::new(
                &definition.tenant_id,
                &definition.id.value,
                definition.revision,
            ),
            definition.clone(),
        );
    }
    pub fn link_type(&self, tenant_id: &str, id: &LinkTypeId) -> Option<&LinkTypeDefinition> {
        self.link_types
            .get(&ontology_scoped_key(tenant_id, &id.value))
    }
    pub fn action_type(&self, tenant_id: &str, id: &ActionTypeId) -> Option<&ActionTypeDefinition> {
        self.action_types
            .get(&ontology_scoped_key(tenant_id, &id.value))
    }

    fn registered_endpoint(
        &self,
        tenant_id: &str,
        id: &EntityTypeId,
    ) -> Result<&EntityTypeDefinition, OntologyEngineError> {
        self.entity_types
            .get(&ontology_scoped_key(tenant_id, &id.value))
            .ok_or(OntologyEngineError::UnknownEntityTypeEndpoint)
    }
}

fn check_pillar_consistency(
    from_def: &EntityTypeDefinition,
    to_def: &EntityTypeDefinition,
) -> Result<(), OntologyEngineError> {
    if let (Some(from_pillar), Some(to_pillar)) = (from_def.pillar, to_def.pillar)
        && from_pillar != to_pillar
    {
        return Err(OntologyEngineError::CrossPillarLink);
    }
    Ok(())
}

fn check_parameter_names_unique(
    definition: &ActionTypeDefinition,
) -> Result<(), OntologyEngineError> {
    let mut seen = std::collections::BTreeSet::new();
    for parameter in &definition.parameters {
        if !seen.insert(parameter.name.as_str()) {
            return Err(OntologyEngineError::DuplicateParameterName {
                name: parameter.name.clone(),
            });
        }
    }
    Ok(())
}

/// Requiring the projection to equal the stated tier rejects a `Some` on
/// any tier the projection never yields (Timeseries/Geo/Ciphertext).
pub(crate) fn check_value_type_integrity<'a>(
    declarations: impl Iterator<
        Item = (
            &'a str,
            &'a crate::PropertyTier,
            Option<&'a crate::ValueTypeDeclaration>,
        ),
    >,
) -> Result<(), OntologyEngineError> {
    for (name, tier, value_type) in declarations {
        if let Some(declaration) = value_type {
            declaration
                .validate()
                .map_err(|cause| OntologyEngineError::InvalidValueType {
                    name: name.to_string(),
                    cause,
                })?;
            if &declaration.tier() != tier {
                return Err(OntologyEngineError::ValueTypeTierMismatch {
                    name: name.to_string(),
                });
            }
        }
    }
    Ok(())
}

/// The key property must be `required`: a key absent from a conformant
/// instance is a contradiction.
pub(crate) fn check_designation_integrity(
    definition: &EntityTypeDefinition,
) -> Result<(), OntologyEngineError> {
    for name in [
        definition.primary_key_property.as_deref(),
        definition.title_property.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !definition.properties.iter().any(|p| p.name == name) {
            return Err(OntologyEngineError::DesignatedPropertyNotDeclared {
                name: name.to_string(),
            });
        }
    }
    if let Some(key_name) = definition.primary_key_property.as_deref()
        && definition
            .properties
            .iter()
            .any(|p| p.name == key_name && !p.required)
    {
        return Err(OntologyEngineError::PrimaryKeyPropertyNotRequired {
            name: key_name.to_string(),
        });
    }
    Ok(())
}

fn ontology_scoped_key(tenant_id: &str, id: &str) -> OntologyScopedKey {
    OntologyScopedKey {
        tenant_id: tenant_id.to_string(),
        id: id.to_string(),
    }
}
