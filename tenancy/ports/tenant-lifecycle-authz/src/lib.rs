//! The authorization DECISION PORT the tenant-lifecycle delivery surface
//! depends on (ADR-0564 D7). The tenancy facade is the Policy Enforcement Point:
//! it authenticates the caller, assembles the query, asks this port, and
//! enforces the answer. Fail-closed and default-deny — the absence of an
//! explicit permit IS a deny, an error is a deny, and a verified bearer
//! principal never on its own grants the tenant axis.
//!
//! ADR-0083 Tier-3: production code carries no unwrap/expect/panic.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;

/// The authenticated caller a PEP presents to the authorizer. Materialized ONLY
/// from a verified credential — never from a URL path segment or a self-asserted
/// header alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallerIdentity {
    /// Stable principal id of the verified caller (e.g. `platform-admin`,
    /// or a tenant operator principal).
    pub principal_id: String,
    /// The tenant axis the caller has proven authority over, when the caller
    /// is tenant-scoped. `None` for a platform-scoped caller. A tenant-scoped
    /// caller can never satisfy a platform-admin action, and vice versa.
    pub tenant_scope: Option<String>,
    /// Whether the caller holds the platform-admin scope (the cross-tenant
    /// control-plane axis required to register or enumerate tenants).
    pub platform_admin: bool,
}

/// A fail-closed membership-resolution fault: any backing-store error or timeout
/// maps here and the PEP DENIES — an outage never grants a tenant axis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipFault {
    detail: String,
}

impl MembershipFault {
    /// Construct a fault with a human-facing detail.
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Borrow the detail string.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for MembershipFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tenant-membership resolution failed: {}", self.detail)
    }
}

impl std::error::Error for MembershipFault {}

/// SERVER-SIDE tenant-membership resolution PORT.
///
/// The tenant-operator bearer is a SHARED credential: on its own it proves only
/// that the caller is *some* operator, never which tenants that operator may act
/// for. A self-attested `x-tenant` header may at most SELECT among the tenants
/// this port returns, never grant one; an unknown operator resolves to an EMPTY
/// set, so every per-tenant op denies.
pub trait TenantMembershipResolver: Send + Sync {
    /// Resolve the tenant ids the VERIFIED operator principal is assigned to.
    ///
    /// # Errors
    /// [`MembershipFault`] on any backing-store failure; the PEP denies
    /// (fail-closed — the operator gets no tenant axis).
    fn assigned_tenants(&self, operator_principal_id: &str)
    -> Result<Vec<String>, MembershipFault>;
}

/// The tenancy control-plane actions guarded by this port. The stable slug —
/// not the Rust variant name — is the contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TenantLifecycleAction {
    /// Register a new tenant (`POST /v1/tenants`) — platform-admin scope; the
    /// caller has no prior tenant to be scoped to.
    Register,
    /// List/enumerate all tenants (`GET /v1/tenants`) — platform-admin scope;
    /// the surface discloses every tenant.
    List,
    /// Read one tenant's state (`GET /v1/tenants/{id}`).
    Read,
    /// Provision a tenant (`POST /v1/tenants/{id}/provision`).
    Provision,
    /// Suspend a tenant (`POST /v1/tenants/{id}/suspend`).
    Suspend,
    /// Resume a tenant (`POST /v1/tenants/{id}/resume`).
    Resume,
    /// Retire a tenant (`DELETE /v1/tenants/{id}`) — terminal, irreversible.
    Retire,
}

impl TenantLifecycleAction {
    /// The stable action slug the backing engine resolves: lowercase dotted
    /// `tenancy.<verb>`, matching the locked PDP-contract slug charset.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Self::Register => "tenancy.register",
            Self::List => "tenancy.list",
            Self::Read => "tenancy.read",
            Self::Provision => "tenancy.provision",
            Self::Suspend => "tenancy.suspend",
            Self::Resume => "tenancy.resume",
            Self::Retire => "tenancy.retire",
        }
    }

    /// Whether this action requires the platform-admin (cross-tenant) scope
    /// rather than authority over one target tenant.
    #[must_use]
    pub fn is_platform_scoped(self) -> bool {
        matches!(self, Self::Register | Self::List)
    }
}

/// What the PEP asks the authorizer to decide. The target tenant id is the URL
/// `{id}`, which by itself authorizes NOTHING.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationQuery<'a> {
    pub caller: &'a CallerIdentity,
    pub action: TenantLifecycleAction,
    /// The target tenant id for a per-tenant action; `None` for a
    /// platform-scoped action (register/list).
    pub target_tenant_id: Option<&'a str>,
}

/// The decision the authorizer reached. Exactly two outcomes (deny-by-default).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationDecision {
    Allow,
    Deny,
}

impl AuthorizationDecision {
    /// Whether the decision permits the request.
    #[must_use]
    pub fn is_allow(self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// The attributable outcome: the decision plus the forensic audit fields. The
/// PEP MUST emit a structured audit record (`"tenancy.authz.decision"`) from
/// EVERY outcome — allow and deny alike; discarding the fields is a policy
/// violation (AC-W-13).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationOutcome {
    pub decision: AuthorizationDecision,
    /// Opaque, globally unique id for this decision (PDP-minted ULID).
    /// Non-empty on every successful call. Key the audit trail on this id.
    pub decision_id: String,
    /// The Cedar policy ids that determined the outcome.
    pub determining_policy_ids: Vec<String>,
}

/// Why the authorizer could not return a clean decision. EVERY variant is
/// fail-closed: a PEP MUST treat an [`AuthzError`] as a deny.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthzError {
    /// The query was malformed (e.g. a per-tenant action with no target, or a
    /// target id that is not a valid slug). The PEP denies.
    InvalidQuery(String),
    /// The backing decision engine refused to decide (bundle/evaluation
    /// failure). The PEP denies (fail-closed), never bypasses.
    EngineRefused(String),
}

impl fmt::Display for AuthzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidQuery(detail) => write!(f, "invalid authorization query: {detail}"),
            Self::EngineRefused(detail) => write!(f, "authorization engine refused: {detail}"),
        }
    }
}

impl std::error::Error for AuthzError {}

/// The authorization decision port. The facade (PEP) depends on this trait and
/// is wired to a concrete adapter at the composition root.
///
/// Implementations evaluate deny-by-default and forbid-overrides-permit: a
/// cross-tenant request MUST Deny regardless of any matching permit.
pub trait TenantLifecycleAuthorizer: Send + Sync {
    /// Decide one authorization query; the PEP MUST emit an audit record from
    /// the returned outcome. `Ok(outcome)` with `Deny` and `Err(_)` are BOTH
    /// refusals.
    ///
    /// # Errors
    /// [`AuthzError`] when the query is malformed or the engine refuses — the
    /// PEP treats either as a deny (fail-closed).
    fn authorize(&self, query: &AuthorizationQuery<'_>)
    -> Result<AuthorizationOutcome, AuthzError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_slugs_are_stable_and_lowercase_dotted() {
        for action in [
            TenantLifecycleAction::Register,
            TenantLifecycleAction::List,
            TenantLifecycleAction::Read,
            TenantLifecycleAction::Provision,
            TenantLifecycleAction::Suspend,
            TenantLifecycleAction::Resume,
            TenantLifecycleAction::Retire,
        ] {
            let slug = action.slug();
            assert!(slug.starts_with("tenancy."));
            assert!(
                slug.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.'),
                "slug {slug:?} must be lowercase dotted",
            );
        }
    }

    #[test]
    fn register_and_list_are_platform_scoped_others_are_not() {
        assert!(TenantLifecycleAction::Register.is_platform_scoped());
        assert!(TenantLifecycleAction::List.is_platform_scoped());
        for action in [
            TenantLifecycleAction::Read,
            TenantLifecycleAction::Provision,
            TenantLifecycleAction::Suspend,
            TenantLifecycleAction::Resume,
            TenantLifecycleAction::Retire,
        ] {
            assert!(!action.is_platform_scoped());
        }
    }

    #[test]
    fn decision_is_allow_only_for_allow() {
        assert!(AuthorizationDecision::Allow.is_allow());
        assert!(!AuthorizationDecision::Deny.is_allow());
    }

    #[test]
    fn authz_error_messages_are_legible() {
        assert!(
            AuthzError::EngineRefused("bundle rejected".to_owned())
                .to_string()
                .contains("bundle rejected")
        );
    }
}
