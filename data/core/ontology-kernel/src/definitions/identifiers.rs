//! Identifier newtypes and shared vocabularies of the type plane.

use crate::error::OntologyEngineError;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EntityTypeId {
    pub value: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LinkTypeId {
    pub value: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ActionTypeId {
    pub value: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LinkCardinality {
    OneToOne,
    OneToMany,
    ManyToMany,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AutonomyTier {
    T0Suggest,
    T1Assist,
    T2ExecuteWithApproval,
    T3Autonomous,
}

impl EntityTypeId {
    pub fn new(value: impl Into<String>) -> Result<Self, OntologyEngineError> {
        prefixed_ontology_id(value.into(), "ety_", OntologyEngineError::InvalidTypeId)
            .map(|value| Self { value })
    }
}
impl LinkTypeId {
    pub fn new(value: impl Into<String>) -> Result<Self, OntologyEngineError> {
        prefixed_ontology_id(value.into(), "lty_", OntologyEngineError::InvalidLinkTypeId)
            .map(|value| Self { value })
    }
}
impl ActionTypeId {
    pub fn new(value: impl Into<String>) -> Result<Self, OntologyEngineError> {
        prefixed_ontology_id(
            value.into(),
            "aty_",
            OntologyEngineError::InvalidActionTypeId,
        )
        .map(|value| Self { value })
    }
}

fn prefixed_ontology_id(
    value: String,
    prefix: &str,
    error: OntologyEngineError,
) -> Result<String, OntologyEngineError> {
    if value.starts_with(prefix) && value.len() > prefix.len() {
        Ok(value)
    } else {
        Err(error)
    }
}
pub(crate) fn validate_ontology_tenant(tenant_id: &str) -> Result<(), OntologyEngineError> {
    if tenant_id.starts_with("ten_") && tenant_id.len() > "ten_".len() {
        Ok(())
    } else {
        Err(OntologyEngineError::InvalidTenantId)
    }
}
