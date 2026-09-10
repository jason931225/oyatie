//! Instance-vs-type conformance: validate an [`ObjectEntity`](crate::ObjectEntity)
//! against the [`EntityTypeDefinition`](crate::EntityTypeDefinition) registered
//! for its tenant.

use data_boundary_kernel::DataClassification;

use crate::error::OntologyEngineError;
use crate::object_graph::ObjectEntity;
use crate::property::ObjectProperty;

use super::value_conformance::{ValueCheckSubject, check_declared_value};
use super::{OntologyEngine, ontology_scoped_key};

impl OntologyEngine {
    /// Check that `entity` conforms to the entity type definition registered
    /// for `(entity.tenant_id, entity.entity_type)`.
    ///
    /// Fail-closed contract:
    ///
    /// | Error | Condition |
    /// |-------|-----------|
    /// | [`OntologyEngineError::UnknownEntityType`](crate::OntologyEngineError::UnknownEntityType) | No definition registered under the entity's `(tenant_id, entity_type)`. |
    /// | [`OntologyEngineError::MissingRequiredProperty`](crate::OntologyEngineError::MissingRequiredProperty) | A definition property with `required: true` is absent from the instance. |
    /// | [`OntologyEngineError::UndeclaredProperty`](crate::OntologyEngineError::UndeclaredProperty) | The instance carries a property the definition does not declare. |
    /// | [`OntologyEngineError::PropertyTierMismatch`](crate::OntologyEngineError::PropertyTierMismatch) | An instance property's tier differs from the declared tier. |
    /// | [`OntologyEngineError::PropertyDataClassMismatch`](crate::OntologyEngineError::PropertyDataClassMismatch) | An instance property's data class differs from the declared class. |
    /// | [`OntologyEngineError::PropertyValueTypeMismatch`](crate::OntologyEngineError::PropertyValueTypeMismatch) | The FINAL step: the carrier fails its declared value type; for an untyped (`None`) declaration, the carrier is not the legacy `String`. |
    pub fn check_instance_conformance(
        &self,
        entity: &ObjectEntity,
    ) -> Result<(), OntologyEngineError> {
        let definition = self
            .entity_types
            .get(&ontology_scoped_key(
                &entity.tenant_id,
                &entity.entity_type.value,
            ))
            .ok_or(OntologyEngineError::UnknownEntityType)?;

        for declared in &definition.properties {
            if declared.required && !entity.properties.contains_key(&declared.name) {
                return Err(OntologyEngineError::MissingRequiredProperty {
                    name: declared.name.clone(),
                });
            }
        }

        for (name, property) in &entity.properties {
            let Some(declared) = definition.properties.iter().find(|p| &p.name == name) else {
                return Err(OntologyEngineError::UndeclaredProperty { name: name.clone() });
            };
            if property.tier != declared.tier {
                return Err(OntologyEngineError::PropertyTierMismatch { name: name.clone() });
            }
            if property.value.data_class != DataClassification::from(declared.data_class) {
                return Err(OntologyEngineError::PropertyDataClassMismatch { name: name.clone() });
            }
            check_declared_value(
                declared.value_type.as_ref(),
                property,
                ValueCheckSubject::Property,
            )?;
        }
        Ok(())
    }

    /// Check that submitted parameter `values` conform to the parameter
    /// schema declared by the action type registered for
    /// `(tenant_id, action_id)`.
    ///
    /// Fail-closed contract:
    ///
    /// | Error | Condition |
    /// |-------|-----------|
    /// | [`OntologyEngineError::UnknownActionType`](crate::OntologyEngineError::UnknownActionType) | No action type registered under `(tenant_id, action_id)`. |
    /// | [`OntologyEngineError::MissingRequiredParameter`](crate::OntologyEngineError::MissingRequiredParameter) | A declared `required: true` parameter is absent. |
    /// | [`OntologyEngineError::UndeclaredParameter`](crate::OntologyEngineError::UndeclaredParameter) | A submitted value names no declared parameter. |
    /// | [`OntologyEngineError::ParameterTierMismatch`](crate::OntologyEngineError::ParameterTierMismatch) | A submitted value's tier differs from the declaration. |
    /// | [`OntologyEngineError::ParameterDataClassMismatch`](crate::OntologyEngineError::ParameterDataClassMismatch) | A submitted value's data class differs from the declaration. |
    /// | [`OntologyEngineError::ParameterValueTypeMismatch`](crate::OntologyEngineError::ParameterValueTypeMismatch) | The FINAL step: the carrier fails its declared value type; for an untyped (`None`) declaration, the carrier is not the legacy `String`. |
    pub fn check_action_parameter_conformance(
        &self,
        tenant_id: &str,
        action_id: &crate::definitions::ActionTypeId,
        values: &[ObjectProperty],
    ) -> Result<(), OntologyEngineError> {
        let action = self
            .action_types
            .get(&ontology_scoped_key(tenant_id, &action_id.value))
            .ok_or(OntologyEngineError::UnknownActionType)?;

        for declared in &action.parameters {
            if declared.required && !values.iter().any(|v| v.name == declared.name) {
                return Err(OntologyEngineError::MissingRequiredParameter {
                    name: declared.name.clone(),
                });
            }
        }

        for value in values {
            let Some(declared) = action.parameters.iter().find(|p| p.name == value.name) else {
                return Err(OntologyEngineError::UndeclaredParameter {
                    name: value.name.clone(),
                });
            };
            if value.tier != declared.tier {
                return Err(OntologyEngineError::ParameterTierMismatch {
                    name: value.name.clone(),
                });
            }
            if value.value.data_class != DataClassification::from(declared.data_class) {
                return Err(OntologyEngineError::ParameterDataClassMismatch {
                    name: value.name.clone(),
                });
            }
            check_declared_value(
                declared.value_type.as_ref(),
                value,
                ValueCheckSubject::Parameter,
            )?;
        }
        Ok(())
    }
}
