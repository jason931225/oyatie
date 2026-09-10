//! Typed action parameters: the declared input schema of an
//! [`ActionTypeDefinition`](crate::ActionTypeDefinition), reusing the
//! property-tier and privacy-data-class vocabulary of the type plane.

use data_boundary_kernel::PrivacyDataClass;

use crate::error::OntologyEngineError;
use crate::property::PropertyTier;
use crate::value_type::ValueTypeDeclaration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionParameterDefinition {
    pub name: String,
    pub tier: PropertyTier,
    pub data_class: PrivacyDataClass,
    pub required: bool,
    /// Declared value type; `None` is the legacy string contract.
    /// Action types are register-once, so a declaration is immutable.
    pub value_type: Option<ValueTypeDeclaration>,
}

impl ActionParameterDefinition {
    pub fn new(
        name: impl Into<String>,
        tier: PropertyTier,
        data_class: PrivacyDataClass,
        required: bool,
    ) -> Result<Self, OntologyEngineError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(OntologyEngineError::EmptyParameterName);
        }
        Ok(Self {
            name,
            tier,
            data_class,
            required,
            value_type: None,
        })
    }

    /// The tier is DERIVED from the declaration's projection, so tier/type
    /// incoherence is unrepresentable here.
    pub fn typed(
        name: impl Into<String>,
        value_type: ValueTypeDeclaration,
        data_class: PrivacyDataClass,
        required: bool,
    ) -> Result<Self, OntologyEngineError> {
        let tier = value_type.tier();
        let mut parameter = Self::new(name, tier, data_class, required)?;
        parameter.value_type = Some(value_type);
        Ok(parameter)
    }
}
