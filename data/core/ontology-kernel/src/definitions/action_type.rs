//! Action-type definitions and the invocation records they exchange.

use crate::action_parameters::ActionParameterDefinition;
use crate::error::OntologyEngineError;

use super::identifiers::{ActionTypeId, AutonomyTier, EntityTypeId, validate_ontology_tenant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTypeDefinition {
    pub tenant_id: String,
    pub id: ActionTypeId,
    pub entity_type: EntityTypeId,
    pub surface: String,
    pub max_autonomy_tier: AutonomyTier,
    pub audit_event_type: String,
    pub revision: u32,
    pub display: Option<crate::display::DisplayMetadata>,
    /// Declared parameter schema. Empty means the action takes no
    /// parameters; submissions carrying any value are then non-conformant.
    pub parameters: Vec<ActionParameterDefinition>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionPolicyDecision {
    pub decision_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub allowed_surfaces: Vec<String>,
    pub autonomy_tier: AutonomyTier,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionInvocationRequest {
    pub tenant_id: String,
    pub principal_id: String,
    pub action_id: ActionTypeId,
    pub entity_id: String,
    pub idempotency_key: String,
    pub requested_at_epoch_seconds: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionInvocationReceipt {
    pub decision_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub action_id: String,
    pub entity_id: String,
    pub idempotency_key: String,
    pub audit_event_type: String,
    pub occurred_at_epoch_seconds: u64,
    pub schema_version: u32,
}

impl ActionTypeDefinition {
    pub fn new(
        tenant_id: impl Into<String>,
        id: ActionTypeId,
        entity_type: EntityTypeId,
        surface: impl Into<String>,
        max_autonomy_tier: AutonomyTier,
        audit_event_type: impl Into<String>,
    ) -> Result<Self, OntologyEngineError> {
        let tenant_id = tenant_id.into();
        validate_ontology_tenant(&tenant_id)?;
        let surface = surface.into();
        if surface.trim().is_empty() {
            return Err(OntologyEngineError::EmptySurface);
        }
        let audit_event_type = audit_event_type.into();
        if audit_event_type.trim().is_empty() {
            return Err(OntologyEngineError::EmptyAuditEventType);
        }
        Ok(Self {
            tenant_id,
            id,
            entity_type,
            surface,
            max_autonomy_tier,
            audit_event_type,
            revision: 1,
            display: None,
            parameters: Vec::new(),
        })
    }

    pub fn with_display(mut self, display: crate::display::DisplayMetadata) -> Self {
        self.display = Some(display);
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<ActionParameterDefinition>) -> Self {
        self.parameters = parameters;
        self
    }
}
