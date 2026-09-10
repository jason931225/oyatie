//! Schema evolution for entity type definitions: strict revision
//! monotonicity and the additive-only backward-compatibility contract.

use crate::definitions::{
    ActionTypeDefinition, ActionTypeId, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, LinkTypeDefinition, LinkTypeId,
};
use crate::error::OntologyEngineError;

use super::{OntologyEngine, ontology_scoped_key};

impl OntologyEngine {
    /// Register or evolve an entity type definition. An id unknown for the
    /// tenant is a first registration and `DuplicateEntityType` is never
    /// returned here.
    ///
    /// # Errors
    ///
    /// | Error | Condition |
    /// |-------|-----------|
    /// | [`OntologyEngineError::DesignatedPropertyNotDeclared`] | A primary-key or title designation names no declared property. |
    /// | [`OntologyEngineError::PrimaryKeyPropertyNotRequired`] | The designated primary-key property is optional. |
    /// | [`OntologyEngineError::BlankDisplayField`] | A present display field is blank. |
    /// | [`OntologyEngineError::InvalidValueType`] | A `value_type` declaration fails validation. |
    /// | [`OntologyEngineError::ValueTypeTierMismatch`] | A `value_type` projection differs from the stated tier. |
    /// | [`OntologyEngineError::NonMonotonicRevision`] | `definition.revision <= stored.revision`. |
    /// | [`OntologyEngineError::PillarChangedOnEvolution`] | The pillar annotation differs from the stored definition's. |
    /// | [`OntologyEngineError::PrimaryKeyChangedOnEvolution`] | An already-set primary-key designation changed or was removed. |
    /// | [`OntologyEngineError::IncompatibleSchemaEvolution`] | A prior property was removed or its frozen quadruple mutated, or a new property is `required`. |
    pub fn evolve_entity_type(
        &mut self,
        definition: EntityTypeDefinition,
    ) -> Result<EntityTypeId, OntologyEngineError> {
        super::check_designation_integrity(&definition)?;
        crate::display::check_display_integrity(definition.display.as_ref())?;
        for property in &definition.properties {
            crate::display::check_display_integrity(property.display.as_ref())?;
        }
        super::check_value_type_integrity(
            definition
                .properties
                .iter()
                .map(|p| (p.name.as_str(), &p.tier, p.value_type.as_ref())),
        )?;
        let key = ontology_scoped_key(&definition.tenant_id, &definition.id.value);
        match self.entity_types.get(&key) {
            None => {
                let id = definition.id.clone();
                self.retain_entity_type_revision(&definition);
                self.entity_types.insert(key, definition);
                Ok(id)
            }
            Some(stored) => {
                if definition.revision <= stored.revision {
                    return Err(OntologyEngineError::NonMonotonicRevision);
                }
                if definition.pillar != stored.pillar {
                    return Err(OntologyEngineError::PillarChangedOnEvolution);
                }
                if stored.primary_key_property.is_some()
                    && definition.primary_key_property != stored.primary_key_property
                {
                    return Err(OntologyEngineError::PrimaryKeyChangedOnEvolution);
                }
                check_property_compatibility(stored, &definition)?;
                let id = definition.id.clone();
                self.retain_entity_type_revision(&definition);
                self.entity_types.insert(key, definition);
                Ok(id)
            }
        }
    }
}

/// Additive-only property compatibility between two revisions of one entity
/// type.
///
/// A new property must be optional because every object projected under
/// `prior` lacks it, so a required one would invalidate the existing
/// population. `value_type` is frozen in both directions: `None -> Some`
/// in-place typing is rejected too, and the blessed idiom for adopting a
/// type is a NEW optional typed property.
pub(crate) fn check_property_compatibility(
    prior: &EntityTypeDefinition,
    candidate: &EntityTypeDefinition,
) -> Result<(), OntologyEngineError> {
    let candidate_map: std::collections::BTreeMap<&str, &EntityTypePropertyDefinition> = candidate
        .properties
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    let prior_names: std::collections::BTreeSet<&str> =
        prior.properties.iter().map(|p| p.name.as_str()).collect();
    for cand_prop in &candidate.properties {
        if cand_prop.required && !prior_names.contains(cand_prop.name.as_str()) {
            return Err(OntologyEngineError::IncompatibleSchemaEvolution);
        }
    }

    for prior_prop in &prior.properties {
        match candidate_map.get(prior_prop.name.as_str()) {
            None => return Err(OntologyEngineError::IncompatibleSchemaEvolution),
            Some(cand_prop) => {
                if cand_prop.tier != prior_prop.tier
                    || cand_prop.data_class != prior_prop.data_class
                    || cand_prop.required != prior_prop.required
                    || cand_prop.value_type != prior_prop.value_type
                {
                    return Err(OntologyEngineError::IncompatibleSchemaEvolution);
                }
            }
        }
    }
    Ok(())
}

impl OntologyEngine {
    /// Evolve a link type: strict revision monotonicity; the semantic
    /// fields (endpoints, cardinality, cross-tenant flag) are frozen —
    /// today only the revision itself may advance, ready to loosen when
    /// mutable metadata lands. First registration is not served here; use
    /// `register_link_type`.
    pub fn evolve_link_type(
        &mut self,
        definition: LinkTypeDefinition,
    ) -> Result<LinkTypeId, OntologyEngineError> {
        crate::display::check_display_integrity(definition.display.as_ref())?;
        let key = ontology_scoped_key(&definition.tenant_id, &definition.id.value);
        let stored = self
            .link_types
            .get(&key)
            .ok_or(OntologyEngineError::UnknownLinkType)?;
        if definition.revision <= stored.revision {
            return Err(OntologyEngineError::NonMonotonicRevision);
        }
        for (label, changed) in [
            (
                "from_entity_type",
                definition.from_entity_type != stored.from_entity_type,
            ),
            (
                "to_entity_type",
                definition.to_entity_type != stored.to_entity_type,
            ),
            ("cardinality", definition.cardinality != stored.cardinality),
            (
                "allow_cross_tenant",
                definition.allow_cross_tenant != stored.allow_cross_tenant,
            ),
        ] {
            if changed {
                return Err(OntologyEngineError::FrozenFieldChangedOnEvolution {
                    field: label.to_string(),
                });
            }
        }
        let id = definition.id.clone();
        self.link_types.insert(key, definition);
        Ok(id)
    }

    /// Evolve an action type: strict revision monotonicity; entity type,
    /// surface, autonomy ceiling, and audit event type are frozen; every
    /// EXISTING parameter's quadruple (tier, data class, required, value
    /// type) is frozen; NEW parameters must be optional — the property
    /// law, mirrored, so writer-schema interpretability of logged payloads
    /// is preserved. First registration is not served here; use
    /// `register_action_type`.
    pub fn evolve_action_type(
        &mut self,
        definition: ActionTypeDefinition,
    ) -> Result<ActionTypeId, OntologyEngineError> {
        crate::display::check_display_integrity(definition.display.as_ref())?;
        let key = ontology_scoped_key(&definition.tenant_id, &definition.id.value);
        let stored = self
            .action_types
            .get(&key)
            .ok_or(OntologyEngineError::UnknownActionType)?;
        if definition.revision <= stored.revision {
            return Err(OntologyEngineError::NonMonotonicRevision);
        }
        for (label, changed) in [
            ("entity_type", definition.entity_type != stored.entity_type),
            ("surface", definition.surface != stored.surface),
            (
                "max_autonomy_tier",
                definition.max_autonomy_tier != stored.max_autonomy_tier,
            ),
            (
                "audit_event_type",
                definition.audit_event_type != stored.audit_event_type,
            ),
        ] {
            if changed {
                return Err(OntologyEngineError::FrozenFieldChangedOnEvolution {
                    field: label.to_string(),
                });
            }
        }
        super::check_value_type_integrity(
            definition
                .parameters
                .iter()
                .map(|p| (p.name.as_str(), &p.tier, p.value_type.as_ref())),
        )?;
        let candidate_names: std::collections::BTreeSet<&str> = {
            let mut seen = std::collections::BTreeSet::new();
            for parameter in &definition.parameters {
                if !seen.insert(parameter.name.as_str()) {
                    return Err(OntologyEngineError::DuplicateParameterName {
                        name: parameter.name.clone(),
                    });
                }
            }
            seen
        };
        let stored_names: std::collections::BTreeSet<&str> =
            stored.parameters.iter().map(|p| p.name.as_str()).collect();
        for new_name in candidate_names.difference(&stored_names) {
            let new_parameter = definition
                .parameters
                .iter()
                .find(|p| p.name == *new_name)
                .expect("name came from the candidate set");
            if new_parameter.required {
                return Err(OntologyEngineError::IncompatibleSchemaEvolution);
            }
        }
        for prior in &stored.parameters {
            match definition.parameters.iter().find(|p| p.name == prior.name) {
                None => return Err(OntologyEngineError::IncompatibleSchemaEvolution),
                Some(candidate) => {
                    if candidate.tier != prior.tier
                        || candidate.data_class != prior.data_class
                        || candidate.required != prior.required
                        || candidate.value_type != prior.value_type
                    {
                        return Err(OntologyEngineError::IncompatibleSchemaEvolution);
                    }
                }
            }
        }
        let id = definition.id.clone();
        self.action_types.insert(key, definition);
        Ok(id)
    }
}
