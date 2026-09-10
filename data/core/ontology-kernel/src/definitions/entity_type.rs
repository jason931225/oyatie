//! Entity-type definitions: the property schema plane.

use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};

use crate::error::OntologyEngineError;
use crate::pillar::OntologyPillar;
use crate::property::PropertyTier;
use crate::value_type::ValueTypeDeclaration;

use super::identifiers::{EntityTypeId, validate_ontology_tenant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityTypePropertyDefinition {
    pub name: String,
    pub tier: PropertyTier,
    pub data_class: PrivacyDataClass,
    pub required: bool,
    /// Declared value type; `None` is the legacy string contract. Once
    /// `Some`, immutable across revisions (see `check_property_compatibility`).
    pub value_type: Option<ValueTypeDeclaration>,
    pub display: Option<crate::display::DisplayMetadata>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityTypeDefinition {
    pub tenant_id: String,
    pub id: EntityTypeId,
    pub display_name: Classified<String>,
    pub properties: Vec<EntityTypePropertyDefinition>,
    pub revision: u32,
    /// Optional pillar annotation for org/person isolation (Bominal-ADR-0132).
    /// `None` means the entity type is pillar-agnostic and does not
    /// participate in cross-pillar link rejection.
    pub pillar: Option<OntologyPillar>,
    /// Name of the property that identifies instances of this type. Must
    /// name a declared `required` property; immutable once set (re-keying a
    /// population is a breaking change). `None` means instances are keyed
    /// only by their `ent_`-prefixed id.
    pub primary_key_property: Option<String>,
    pub display: Option<crate::display::DisplayMetadata>,
    /// Name of the property rendered as the default human-readable label.
    /// Must name a declared property; freely changeable across revisions.
    pub title_property: Option<String>,
}

impl EntityTypePropertyDefinition {
    pub fn new(
        name: impl Into<String>,
        tier: PropertyTier,
        data_class: PrivacyDataClass,
        required: bool,
    ) -> Result<Self, OntologyEngineError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(OntologyEngineError::EmptyPropertyName);
        }
        Ok(Self {
            name,
            tier,
            data_class,
            required,
            value_type: None,
            display: None,
        })
    }

    pub fn with_display(mut self, display: crate::display::DisplayMetadata) -> Self {
        self.display = Some(display);
        self
    }
}
impl EntityTypeDefinition {
    pub fn new(
        tenant_id: impl Into<String>,
        id: EntityTypeId,
        display_name: impl Into<String>,
        properties: Vec<EntityTypePropertyDefinition>,
        revision: u32,
    ) -> Result<Self, OntologyEngineError> {
        let tenant_id = tenant_id.into();
        validate_ontology_tenant(&tenant_id)?;
        let display_name = display_name.into();
        if display_name.trim().is_empty() {
            return Err(OntologyEngineError::EmptyDisplayName);
        }
        if properties.is_empty() {
            return Err(OntologyEngineError::EmptyProperties);
        }
        Ok(Self {
            tenant_id,
            id,
            display_name: Classified::new(display_name, DataClass::InternalOnly),
            properties,
            revision,
            pillar: None,
            primary_key_property: None,
            title_property: None,
            display: None,
        })
    }

    /// Annotate this entity type with an [`OntologyPillar`] for org/person
    /// isolation enforcement (Bominal-ADR-0132). Returns `self` for chaining.
    /// Entity types without a pillar annotation are pillar-agnostic and do not
    /// participate in cross-pillar link rejection.
    pub fn with_pillar(mut self, pillar: OntologyPillar) -> Self {
        self.pillar = Some(pillar);
        self
    }

    /// Designate the primary-key property. Integrity (declared, required,
    /// immutable across revisions) is enforced at registration/evolution.
    pub fn with_primary_key_property(mut self, name: impl Into<String>) -> Self {
        self.primary_key_property = Some(name.into());
        self
    }

    /// Designate the title property. Integrity (declared) is enforced at
    /// registration/evolution.
    pub fn with_title_property(mut self, name: impl Into<String>) -> Self {
        self.title_property = Some(name.into());
        self
    }

    pub fn with_display(mut self, display: crate::display::DisplayMetadata) -> Self {
        self.display = Some(display);
        self
    }
}
