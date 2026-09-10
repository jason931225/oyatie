use shared_platform_contracts_kernel::pdp::{
    AuthorizationRequest, AuthorizationResponse, Decision, EntityRef, PolicyVersion,
};

use crate::{EntitySlice, PdpError, PolicyBundle};

/// Every decision — allow or deny, cached or freshly evaluated — produces one
/// of these, keyed by `decision_id`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionAuditRecord {
    pub decision_id: String,                 // data_class: INTERNAL_ONLY
    pub request_id: String,                  // data_class: INTERNAL_ONLY
    pub tenant_id: String,                   // data_class: TENANT_SCOPED
    pub principal: EntityRef,                // data_class: TENANT_SCOPED
    pub action: String,                      // data_class: INTERNAL_ONLY
    pub resource: EntityRef,                 // data_class: TENANT_SCOPED
    pub decision: Decision,                  // data_class: INTERNAL_ONLY
    pub policy_version: PolicyVersion,       // data_class: INTERNAL_ONLY
    pub determining_policy_ids: Vec<String>, // data_class: INTERNAL_ONLY
    /// Whether the decision content was served from the decision cache.
    pub cache_hit: bool, // data_class: INTERNAL_ONLY
}

/// One authorization outcome: the contract response plus its audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdpOutcome {
    pub response: AuthorizationResponse, // data_class: INTERNAL_ONLY
    pub audit: DecisionAuditRecord,      // data_class: INTERNAL_ONLY
    pub cache_hit: bool,                 // data_class: INTERNAL_ONLY
}

/// The embedded-PDP port. Implementations evaluate in-process against the
/// loaded [`PolicyBundle`] — never over the network — with deny-by-default
/// and forbid-overrides-permit semantics (the locked contract restates the
/// engine semantics; adapters must satisfy them).
pub trait PolicyDecisionPoint: Send + Sync {
    /// Decide one PARC request against the supplied entity slice. Every
    /// error is fail-closed: the PEP MUST treat it as deny.
    fn authorize(
        &self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<PdpOutcome, PdpError>;

    /// The version token of the currently loaded bundle.
    fn loaded_policy_version(&self) -> PolicyVersion;
}
