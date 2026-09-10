//! Cedar adapter implementing the kernel's [`AuthzGate`] trait (ADR-0384 D7).
//!
//! The principal's role is not carried on [`AuthzRequest`], so
//! [`action_mapping`] derives it from the action. A request that should be
//! admitted under one role and refused under another cannot be expressed
//! until the role travels on the request.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::{HashMap, HashSet};

use cedar_policy::{
    Authorizer, Context, Decision, Entities, Entity, EntityUid, PolicySet, Request,
    RestrictedExpression,
};
use intelligence_kernel::{AuthzAction, AuthzDecision, AuthzGate, AuthzRequest, Provider};

/// Policy text compiled into the crate, so the request path does no file I/O.
pub const DEFAULT_POLICY_TEXT: &str = include_str!("../../../cedar/cloud-intelligence.cedar");

/// Construction-time errors. The request path is total: a malformed id or a
/// missing attribute becomes [`AuthzDecision::Forbid`], never an error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CedarAuthzGateError {
    PolicyParse(String),
}

impl std::fmt::Display for CedarAuthzGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CedarAuthzGateError::PolicyParse(reason) => {
                write!(f, "cedar policy parse failed: {reason}")
            }
        }
    }
}

impl std::error::Error for CedarAuthzGateError {}

/// Cedar [`AuthzGate`] implementation.
pub struct CedarAuthzGate {
    policy_set: PolicySet,
}

impl CedarAuthzGate {
    pub fn from_policy_text(text: &str) -> Result<Self, CedarAuthzGateError> {
        let policy_set: PolicySet = text.parse().map_err(|e: cedar_policy::ParseErrors| {
            CedarAuthzGateError::PolicyParse(e.to_string())
        })?;
        Ok(Self { policy_set })
    }

    /// Build from [`DEFAULT_POLICY_TEXT`].
    pub fn with_default_policy() -> Result<Self, CedarAuthzGateError> {
        Self::from_policy_text(DEFAULT_POLICY_TEXT)
    }

    pub fn policy_count(&self) -> usize {
        self.policy_set.policies().count()
    }
}

impl AuthzGate for CedarAuthzGate {
    fn decide(&self, request: &AuthzRequest<'_>) -> AuthzDecision {
        match try_decide(&self.policy_set, request) {
            Some(d) => d,
            None => AuthzDecision::Forbid,
        }
    }
}

/// Returns `Option` rather than `Result` so every translation failure
/// short-circuits to the same fail-closed answer in [`CedarAuthzGate::decide`].
fn try_decide(policy_set: &PolicySet, request: &AuthzRequest<'_>) -> Option<AuthzDecision> {
    let (cedar_action, role_name) = action_mapping(request.action);

    let role_uid: EntityUid = format!(r#"Role::"{role_name}""#).parse().ok()?;
    let role_entity = Entity::new(role_uid.clone(), HashMap::new(), HashSet::new()).ok()?;

    let principal_uid: EntityUid = cedar_uid("Workload", request.principal_agent.as_str()).ok()?;
    let mut principal_attrs = HashMap::new();
    principal_attrs.insert(
        "tenant_id".to_string(),
        RestrictedExpression::new_string(request.principal_tenant.as_str().to_string()),
    );
    let mut principal_parents = HashSet::new();
    principal_parents.insert(role_uid);
    let principal_entity =
        Entity::new(principal_uid.clone(), principal_attrs, principal_parents).ok()?;

    let resource_id = format!(
        "{}:{}",
        request.resource_tenant.as_str(),
        provider_label(request.resource_provider)
    );
    let resource_uid: EntityUid = cedar_uid("Subscription", &resource_id).ok()?;
    let mut resource_attrs = HashMap::new();
    resource_attrs.insert(
        "tenant_id".to_string(),
        RestrictedExpression::new_string(request.resource_tenant.as_str().to_string()),
    );
    let resource_entity = Entity::new(resource_uid.clone(), resource_attrs, HashSet::new()).ok()?;

    let action_uid: EntityUid = format!(r#"Action::"{cedar_action}""#).parse().ok()?;

    let entities = Entities::empty()
        .add_entities([principal_entity, resource_entity, role_entity], None)
        .ok()?;

    let cedar_request = Request::new(
        principal_uid,
        action_uid,
        resource_uid,
        Context::empty(),
        None,
    )
    .ok()?;

    let authorizer = Authorizer::new();
    let response = authorizer.is_authorized(&cedar_request, policy_set, &entities);
    match response.decision() {
        Decision::Allow => Some(AuthzDecision::Allow),
        Decision::Deny => Some(AuthzDecision::Forbid),
    }
}

fn action_mapping(action: AuthzAction) -> (&'static str, &'static str) {
    match action {
        AuthzAction::SelectSeat => ("InvokeChatCompletion", "IngressRealm"),
        AuthzAction::RefreshToken => ("RefreshKeyPool", "AdminRealm"),
        AuthzAction::InvalidateSeat => ("RefreshKeyPool", "AdminRealm"),
    }
}

fn provider_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "anthropic",
        Provider::Codex => "codex",
        Provider::Gemini => "gemini",
    }
}

/// `ParseErrors` carries a whole diagnostic tree, but every caller reduces it
/// to `None`, so the size never travels.
#[allow(clippy::result_large_err)]
fn cedar_uid(type_name: &str, id: &str) -> Result<EntityUid, cedar_policy::ParseErrors> {
    let literal = format!("{type_name}::{}", escape_cedar_string(id));
    literal.parse()
}

fn escape_cedar_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
