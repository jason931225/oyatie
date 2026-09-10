use std::collections::BTreeMap;
use std::fmt;

use shared_platform_contracts_kernel::ContractViolation;
use shared_platform_contracts_kernel::pdp::{
    AuthorizationRequest, Decision, EntityRef, PolicyVersion,
};

/// Request shape for PEP-side decision authorization before it is projected
/// into the canonical PDP PARC contract.
///
/// This is intentionally narrower than [`AuthorizationRequest`]: tenant-rbac
/// and later central PBAC/ReBAC integrations name the caller tenant and target
/// tenant separately, then [`DecisionAuthzRequest::to_authorization_request`]
/// performs the one stable projection into the shared PDP port. The PDP
/// evaluates against the TARGET tenant (`tenant_id`) while the caller/target
/// tenancy axes stay visible in ABAC context for policy conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionAuthzRequest<'a> {
    pub caller_tenant: &'a str,     // data_class: TENANT_SCOPED
    pub caller_id: &'a str,         // data_class: TENANT_SCOPED
    pub target_tenant: &'a str,     // data_class: TENANT_SCOPED
    pub target_subject_id: &'a str, // data_class: TENANT_SCOPED
    pub action: &'a str,            // data_class: INTERNAL_ONLY
    pub resource_type: &'a str,     // data_class: INTERNAL_ONLY
    pub resource_id: &'a str,       // data_class: TENANT_SCOPED
}

impl DecisionAuthzRequest<'_> {
    /// The caller principal entity id projected into the target-tenant PDP.
    ///
    /// Principal ids can be tenant-local. Encoding the verified caller tenant
    /// with the caller id makes the principal uid structurally tenant-qualified
    /// before the request enters a target-tenant policy graph, so `acme/alice`
    /// cannot collide with `globex/alice` and accidentally match a target-local
    /// principal. The JSON tuple is intentionally opaque and unambiguous.
    #[must_use]
    pub fn qualified_caller_principal_id(&self) -> String {
        serde_json::json!([self.caller_tenant, self.caller_id]).to_string()
    }

    /// Project this decision-authorization request into the canonical PDP
    /// request shape.
    ///
    /// The target tenant becomes `AuthorizationRequest::tenant_id` so embedded
    /// PDP engines evaluate against the tenant whose resource/policy is being
    /// mutated. The caller tenant remains in context rather than being
    /// collapsed into `tenant_id`; that keeps cross-tenant/platform-admin cases
    /// representable for central PBAC/ReBAC policies without changing this port.
    pub fn to_authorization_request(
        &self,
        request_id: impl Into<String>,
        min_policy_version: Option<PolicyVersion>,
    ) -> Result<AuthorizationRequest, DecisionAuthzError> {
        self.validate_for_decision()?;
        let request = AuthorizationRequest {
            request_id: request_id.into(),
            tenant_id: self.target_tenant.to_owned(),
            principal: EntityRef {
                entity_type: "OyaPlatform::Principal".to_owned(),
                entity_id: self.qualified_caller_principal_id(),
            },
            action: self.action.to_owned(),
            resource: EntityRef {
                entity_type: self.resource_type.to_owned(),
                entity_id: self.resource_id.to_owned(),
            },
            context: BTreeMap::from([
                (
                    "caller_tenant".to_owned(),
                    serde_json::Value::String(self.caller_tenant.to_owned()),
                ),
                (
                    "caller_id".to_owned(),
                    serde_json::Value::String(self.caller_id.to_owned()),
                ),
                (
                    "target_tenant".to_owned(),
                    serde_json::Value::String(self.target_tenant.to_owned()),
                ),
                (
                    "target_subject_id".to_owned(),
                    serde_json::Value::String(self.target_subject_id.to_owned()),
                ),
            ]),
            min_policy_version,
        };
        request
            .validate()
            .map_err(DecisionAuthzError::InvalidProjectedRequest)?;
        Ok(request)
    }

    fn validate_for_decision(&self) -> Result<(), DecisionAuthzError> {
        for (field, value) in [
            ("caller_tenant", self.caller_tenant),
            ("caller_id", self.caller_id),
            ("target_tenant", self.target_tenant),
            ("target_subject_id", self.target_subject_id),
            ("action", self.action),
            ("resource_type", self.resource_type),
            ("resource_id", self.resource_id),
        ] {
            if value.trim().is_empty() {
                return Err(DecisionAuthzError::MissingValue { field });
            }
        }
        Ok(())
    }
}

/// Why the decision authorizer refused to decide. Every variant is
/// fail-closed: callers MUST treat errors as deny/refusal, never as allow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionAuthzError {
    /// A required trusted/decision field was empty.
    MissingValue { field: &'static str },
    /// Projection into the locked PDP PARC contract failed validation.
    InvalidProjectedRequest(Vec<ContractViolation>),
    /// A downstream PDP refused to decide.
    PdpRefused { detail: String },
}

impl fmt::Display for DecisionAuthzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingValue { field } => {
                write!(f, "decision authorization field {field} is required")
            }
            Self::InvalidProjectedRequest(violations) => {
                write!(f, "invalid projected decision PDP request: ")?;
                for (i, v) in violations.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "{v}")?;
                }
                Ok(())
            }
            Self::PdpRefused { detail } => write!(f, "decision PDP refused: {detail}"),
        }
    }
}

impl std::error::Error for DecisionAuthzError {}

/// PORT: decide whether the verified caller may perform a decision-affecting
/// tenant-rbac/PBAC/ReBAC operation.
///
/// Adapters call a central/embedded PDP by projecting [`DecisionAuthzRequest`]
/// through [`DecisionAuthzRequest::to_authorization_request`]. Default posture
/// is fail-closed: [`Decision::Deny`] or [`DecisionAuthzError`] both stop the
/// caller.
pub trait DecisionAuthorizer: Send + Sync {
    fn decide(&self, request: &DecisionAuthzRequest<'_>) -> Result<Decision, DecisionAuthzError>;
}

/// Fail-closed placeholder used only when a composition root has not injected a
/// PDP-backed decision authorizer.
///
/// It validates the trusted request shape, then refuses every decision. This is
/// deliberately NOT a same-tenant fallback: same-tenant equality alone does not
/// prove tenant-rbac route scope, PBAC policy, ReBAC reachability, MFA/step-up,
/// or zookie freshness. Production composition must inject a real PDP-backed
/// authorizer to produce an allow.
#[derive(Debug, Clone, Copy, Default)]
pub struct FailClosedDecisionAuthorizer;

impl FailClosedDecisionAuthorizer {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl DecisionAuthorizer for FailClosedDecisionAuthorizer {
    fn decide(&self, request: &DecisionAuthzRequest<'_>) -> Result<Decision, DecisionAuthzError> {
        request.validate_for_decision()?;
        Err(DecisionAuthzError::PdpRefused {
            detail: "no PDP-backed decision authorizer configured".to_owned(),
        })
    }
}
