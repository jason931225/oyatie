//! The seed is CODE, not configuration, and that is forced rather than
//! chosen: a handwritten yaml or json is inadmissible at a capability root
//! (ADR-0719), and `OntologyEngine` carries no serialization. The durable,
//! operator-authored registry is the Ontology Manager vertical's charter;
//! this module is the seam it will replace and must not quietly grow into it.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    ActionTypeDefinition, ActionTypeId, AutonomyTier, EntityTypeDefinition, EntityTypeId,
    EntityTypePropertyDefinition, OntologyEngine, OntologyEngineError, PropertyTier,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeedError {
    Refused(OntologyEngineError),
    /// The label this seed classifies its properties with is not a privacy
    /// class. Unreachable while the seed names `InternalOnly`, and typed
    /// rather than unwrapped so it stays that way.
    DataClassRefused,
}

impl std::fmt::Display for SeedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(error) => write!(formatter, "the registry refused the seed: {error:?}"),
            Self::DataClassRefused => {
                write!(formatter, "the seed's data class is not a privacy class")
            }
        }
    }
}

impl From<OntologyEngineError> for SeedError {
    fn from(error: OntologyEngineError) -> Self {
        Self::Refused(error)
    }
}

fn internal() -> Result<PrivacyDataClass, SeedError> {
    PrivacyDataClass::try_from(DataClass::InternalOnly).map_err(|_| SeedError::DataClassRefused)
}

pub fn registry_for(tenant_id: &str) -> Result<OntologyEngine, SeedError> {
    build(tenant_id)
}

const SEEDED_ENTITY_TYPE: &str = "ety_record";

fn build(tenant_id: &str) -> Result<OntologyEngine, SeedError> {
    let mut engine = OntologyEngine::default();
    let record = EntityTypeId::new(SEEDED_ENTITY_TYPE)?;
    engine.register_entity_type(
        EntityTypeDefinition::new(
            tenant_id,
            record.clone(),
            "Record",
            vec![
                EntityTypePropertyDefinition::new("name", PropertyTier::Scalar, internal()?, true)?,
                EntityTypePropertyDefinition::new(
                    "note",
                    PropertyTier::Scalar,
                    internal()?,
                    false,
                )?,
            ],
            1,
        )?
        .with_title_property("name"),
    )?;
    engine.register_action_type(ActionTypeDefinition::new(
        tenant_id,
        ActionTypeId::new("aty_record_write")?,
        record,
        "ops-console",
        AutonomyTier::T1Assist,
        "record.written",
    )?)?;
    Ok(engine)
}

pub fn declared_entity_types<'a>(
    engine: &'a OntologyEngine,
    tenant_id: &str,
) -> Vec<&'a EntityTypeDefinition> {
    [EntityTypeId::new(SEEDED_ENTITY_TYPE)]
        .into_iter()
        .flatten()
        .filter_map(|id| engine.entity_type(tenant_id, &id))
        .collect()
}
