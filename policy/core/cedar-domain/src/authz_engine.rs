//! Pure value types for Cedar-based authorization evaluation. Wire-marshaling
//! crosses the gRPC/HTTP boundary at the adapter layer — this kernel keeps zero
//! external deps beyond serde.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::PolicyEffect;

/// The principal type that is making an authorization request.
///
/// Serialized with Cedar PascalCase names (`"User"`, `"Employee"`, …) so that
/// the wire format matches `as_cedar_str()` and Cedar policy evaluation engines
/// do not require remapping at every boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PrincipalType {
    User,
    Employee,
    System,
    Llm,
    Workflow,
}

impl PrincipalType {
    /// Returns the Cedar entity-type string for this principal.
    pub const fn as_cedar_str(&self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Employee => "Employee",
            Self::System => "System",
            Self::Llm => "Llm",
            Self::Workflow => "Workflow",
        }
    }
}

/// An authorization request routed to the Cedar policy evaluator.
///
/// `principal_id` is `None` for anonymous/system-level requests.
/// `context` carries arbitrary key→value attributes used by Cedar condition clauses.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthzRequest {
    /// Tenant the request is scoped to; matches `PolicyScope::Tenant`.
    pub tenant_id: String,
    /// Cedar principal entity type.
    pub principal_type: PrincipalType,
    /// Optional principal identifier (e.g. user-id, employee-id).
    pub principal_id: Option<String>,
    /// Cedar action string: `"Read"` | `"Write"` | `"Apply"` | …
    pub action: String,
    /// Cedar resource entity type: `"Object"` | `"WorkflowRun"` | …
    pub resource_type: String,
    /// Optional resource identifier.
    pub resource_id: Option<String>,
    /// Arbitrary Cedar context attributes (key → typed value).
    ///
    /// Values use `serde_json::Value` to preserve booleans, numbers, and
    /// nested objects that Cedar policy conditions commonly evaluate.
    /// Coercing to `String` would silently break numeric comparisons and
    /// boolean guards in Cedar policy rules.
    ///
    /// Defaults to an empty map when absent in JSON so minimal authz
    /// payloads that omit `context` deserialize successfully.
    #[serde(default)]
    pub context: std::collections::BTreeMap<String, JsonValue>,
}

/// The outcome of a Cedar policy evaluation.
///
/// `determining_policies` lists the policy IDs that drove the decision;
/// `errors` lists non-fatal evaluation errors (e.g. missing context keys).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthzDecision {
    /// The net effect after evaluating all applicable rule packs.
    pub effect: PolicyEffect,
    /// Policy IDs that contributed to this decision (may be empty for default-deny).
    pub determining_policies: Vec<String>,
    /// Non-fatal evaluation errors encountered during rule-pack evaluation.
    pub errors: Vec<String>,
}

impl AuthzDecision {
    /// Convenience constructor for an explicit allow decision.
    pub fn allow(determining_policies: Vec<String>) -> Self {
        Self {
            effect: PolicyEffect::Allow,
            determining_policies,
            errors: Vec::new(),
        }
    }

    /// Convenience constructor for a default-deny (no matching allow rule).
    pub fn default_deny() -> Self {
        Self {
            effect: PolicyEffect::Deny,
            determining_policies: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Convenience constructor for an explicit deny decision.
    pub fn explicit_deny(determining_policies: Vec<String>) -> Self {
        Self {
            effect: PolicyEffect::Deny,
            determining_policies,
            errors: Vec::new(),
        }
    }

    /// Returns `true` if the evaluation produced an `Allow` effect.
    pub fn is_allowed(&self) -> bool {
        self.effect == PolicyEffect::Allow
    }
}

/// Filter parameters for querying the evaluation log.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvalLogFilter {
    /// Restrict to a specific principal identifier.
    pub principal_id: Option<String>,
    /// Restrict to a specific effect (`Allow` or `Deny`).
    pub effect: Option<PolicyEffect>,
    /// Restrict to a specific Cedar resource type.
    pub resource_type: Option<String>,
    #[serde(default = "EvalLogFilter::default_limit")]
    pub limit: u32,
}

const DEFAULT_EVAL_LOG_LIMIT: u32 = 100;

impl EvalLogFilter {
    fn default_limit() -> u32 {
        DEFAULT_EVAL_LOG_LIMIT
    }
}

impl Default for EvalLogFilter {
    fn default() -> Self {
        Self {
            principal_id: None,
            effect: None,
            resource_type: None,
            limit: DEFAULT_EVAL_LOG_LIMIT,
        }
    }
}
