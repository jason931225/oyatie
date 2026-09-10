use crate::contract::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeGraphLinkInstance {
    pub tenant_id: String,              // data_class: INTERNAL_ONLY
    pub from_entity_id: String,         // data_class: INTERNAL_ONLY
    pub to_entity_id: String,           // data_class: INTERNAL_ONLY
    pub edge_type_id: String,           // data_class: INTERNAL_ONLY
    pub observed_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

impl KnowledgeGraphLinkInstance {
    pub fn new(
        tenant_id: impl Into<String>,
        from_entity_id: impl Into<String>,
        to_entity_id: impl Into<String>,
        edge_type_id: impl Into<String>,
        observed_at_epoch_seconds: u64,
    ) -> Result<Self, KnowledgeGraphQueryError> {
        let link = Self {
            tenant_id: tenant_id.into(),
            from_entity_id: from_entity_id.into(),
            to_entity_id: to_entity_id.into(),
            edge_type_id: edge_type_id.into(),
            observed_at_epoch_seconds,
        };
        validate_tenant_id(&link.tenant_id)?;
        validate_entity_id(&link.from_entity_id)?;
        validate_entity_id(&link.to_entity_id)?;
        validate_edge_type_id(&link.edge_type_id)?;
        Ok(link)
    }

    pub fn as_contract_edge(&self) -> KnowledgeGraphEdge {
        KnowledgeGraphEdge {
            from_entity_id: self.from_entity_id.clone(),
            to_entity_id: self.to_entity_id.clone(),
            edge_type_id: self.edge_type_id.clone(),
        }
    }
}
