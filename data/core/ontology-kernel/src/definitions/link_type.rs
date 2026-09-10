//! Link-type definitions: typed, cardinality-bearing edges.

use crate::error::OntologyEngineError;

use super::identifiers::{EntityTypeId, LinkCardinality, LinkTypeId, validate_ontology_tenant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkTypeDefinition {
    pub tenant_id: String,
    pub id: LinkTypeId,
    pub from_entity_type: EntityTypeId,
    pub to_entity_type: EntityTypeId,
    pub cardinality: LinkCardinality,
    pub allow_cross_tenant: bool,
    pub revision: u32,
    pub display: Option<crate::display::DisplayMetadata>,
}

impl LinkTypeDefinition {
    pub fn new(
        tenant_id: impl Into<String>,
        id: LinkTypeId,
        from_entity_type: EntityTypeId,
        to_entity_type: EntityTypeId,
        cardinality: LinkCardinality,
        allow_cross_tenant: bool,
    ) -> Result<Self, OntologyEngineError> {
        let tenant_id = tenant_id.into();
        validate_ontology_tenant(&tenant_id)?;
        Ok(Self {
            tenant_id,
            id,
            from_entity_type,
            to_entity_type,
            cardinality,
            allow_cross_tenant,
            revision: 1,
            display: None,
        })
    }

    pub fn with_display(mut self, display: crate::display::DisplayMetadata) -> Self {
        self.display = Some(display);
        self
    }
}
