//! Fail-closed authorization seam for the `cloud.observability.audit.read`
//! surface (ADR-0590). Identity comes from the [`PrincipalVerifier`] port, never
//! from caller-supplied DTO fields; [`AuditReadAuthorizer`] then decides a
//! scope-derived action against the verified principal's tenant. No default-allow.

// ADR-0083 Tier 3: production code stays panic-free.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// The credential a caller presents to prove a real principal identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerCredential {
    /// Raw `Authorization` header value (e.g. `"Bearer abc..."`), if present.
    pub authorization: Option<String>, // data_class: SECRET
    /// The caller-asserted principal id from `x-principal-id` (cross-check input).
    pub claimed_principal_id: String, // data_class: INTERNAL_ONLY
    /// The caller-asserted principal tenant from `x-principal-tenant-id`.
    pub claimed_tenant_id: String, // data_class: INTERNAL_ONLY
}

/// A principal whose identity has been verified from a caller credential.
///
/// Fields are private and [`VerifiedPrincipal::new`] is `pub(crate)`, so external
/// crates must run a [`PrincipalVerifier`] to obtain one. That is structural
/// defense-in-depth, not proof: in-process code can build its own verifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPrincipal {
    principal_id: String, // data_class: INTERNAL_ONLY — private: see unforgeability note above
    tenant_id: String,    // data_class: INTERNAL_ONLY — private: see unforgeability note above
}

impl VerifiedPrincipal {
    pub(crate) fn new(principal_id: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self {
            principal_id: principal_id.into(),
            tenant_id: tenant_id.into(),
        }
    }

    /// The authoritative principal id bound from the verified credential.
    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

    /// The authoritative tenant the principal acts within.
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Test-only constructor that mints a token without a real credential.
    #[cfg(test)]
    pub(crate) fn new_for_test(
        principal_id: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self::new(principal_id, tenant_id)
    }
}

/// Why principal verification refused; every variant is a fail-closed HTTP 401.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrincipalVerificationError {
    /// No credential was presented (no `Authorization` header).
    MissingCredential,
    /// A credential was presented but did not verify. Deliberately opaque so
    /// probing cannot distinguish "wrong token" from "no such principal".
    InvalidCredential,
}

/// Why authorization refused; each variant maps to HTTP 403 (fail-closed).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditReadAuthorizationError {
    /// The PDP returned a deny decision for this principal/action/resource.
    Denied,
    /// The PDP refused to decide (fail-closed: a refusal is treated as deny).
    Refused,
}

/// The action authorized against the PDP, DERIVED from the requested audit-read
/// scope so the broader `all_tenant_audit` scope requires its OWN grant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditReadAction {
    /// Read control-plane mutation audit only.
    ControlPlaneAuditRead,
    /// Read the full per-tenant audit corpus (data-plane security, KMS use,
    /// billing); requires strictly more authority than [`Self::ControlPlaneAuditRead`].
    AllTenantAuditRead,
}

impl AuditReadAction {
    /// The stable action identifier presented to the PDP.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ControlPlaneAuditRead => "cloud.observability.audit.read.control_plane",
            Self::AllTenantAuditRead => "cloud.observability.audit.read.all_tenant",
        }
    }
}

/// The resource an audit-read decision is made against. The tenant is the
/// VERIFIED principal's tenant, never the caller's DTO tenant, so cross-tenant
/// reads are deniable at the PDP.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditReadResource {
    /// The tenant whose audit corpus is read (bound from the verified principal).
    pub tenant_id: String, // data_class: INTERNAL_ONLY
    /// The region the read targets (trusted boundary value).
    pub region: String, // data_class: PUBLIC
    /// The scope-derived action (control-plane vs all-tenant).
    pub action: AuditReadAction, // data_class: INTERNAL_ONLY
    /// A stable hash of the authorized request shape, so the decision cannot be
    /// replayed against a different request body.
    pub request_hash: String, // data_class: INTERNAL_ONLY
}

/// PORT: verify a caller credential into a [`VerifiedPrincipal`]. The verifier —
/// not the request headers — is the source of truth for caller identity.
pub trait PrincipalVerifier: Send + Sync {
    /// Verify `credential` and return the authoritative principal, or refuse.
    ///
    /// # Errors
    /// [`PrincipalVerificationError`] — fail-closed; the caller MUST map to 401.
    fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedPrincipal, PrincipalVerificationError>;
}

/// PORT: decide whether `principal` may perform the audit read described by
/// `resource`. Default-deny; any refusal is treated as deny.
///
/// Adapter contract, unenforceable by the type system: map EVERY internal fault
/// (network, timeout, parse, unavailability) to `Err(Refused)`; enforce your own
/// deadline, since this port enforces none on a synchronous request path; and
/// never panic — the release profile is `panic = "abort"`.
pub trait AuditReadAuthorizer: Send + Sync {
    /// Authorize `principal` to read `resource`, or refuse.
    ///
    /// # Errors
    /// [`AuditReadAuthorizationError`] on deny or any PDP fault — fail-closed 403.
    fn ensure_authorized(
        &self,
        principal: &VerifiedPrincipal,
        resource: &AuditReadResource,
    ) -> Result<(), AuditReadAuthorizationError>;
}

/// The authz provider the boundary depends on: a principal verifier PORT plus an
/// audit-read authorizer PORT. There is no default-allow fallback.
pub struct AuditReadAuthzProvider {
    verifier: std::sync::Arc<dyn PrincipalVerifier>, // data_class: INTERNAL_ONLY
    authorizer: std::sync::Arc<dyn AuditReadAuthorizer>, // data_class: INTERNAL_ONLY
}

impl AuditReadAuthzProvider {
    #[must_use]
    pub fn new(
        verifier: std::sync::Arc<dyn PrincipalVerifier>,
        authorizer: std::sync::Arc<dyn AuditReadAuthorizer>,
    ) -> Self {
        Self {
            verifier,
            authorizer,
        }
    }

    /// Verify the caller principal via the [`PrincipalVerifier`] port.
    ///
    /// # Errors
    /// [`PrincipalVerificationError`] — caller maps to HTTP 401.
    pub fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedPrincipal, PrincipalVerificationError> {
        self.verifier.verify_principal(credential)
    }

    /// Authorize the verified principal via the PDP port. Default-deny.
    ///
    /// The `catch_unwind` below is a TEST-ONLY backstop: the release profile sets
    /// `panic = "abort"`, so in production a panicking adapter aborts the process.
    ///
    /// # Errors
    /// [`AuditReadAuthorizationError`] — caller maps to HTTP 403.
    pub fn ensure_authorized(
        &self,
        principal: &VerifiedPrincipal,
        resource: &AuditReadResource,
    ) -> Result<(), AuditReadAuthorizationError> {
        let authorizer = std::sync::Arc::clone(&self.authorizer);
        let principal = principal.clone();
        let resource = resource.clone();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            authorizer.ensure_authorized(&principal, &resource)
        }))
        .unwrap_or(Err(AuditReadAuthorizationError::Refused))
    }
}

/// Constant-time byte comparison for secret material; NEVER use a naive `==`.
/// Residual: input lengths leak through the `a.len() ^ b.len()` seed.
#[must_use]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max_len = a.len().max(b.len());
    let mut diff = a.len() ^ b.len();
    for index in 0..max_len {
        let left = a.get(index).copied().unwrap_or(0);
        let right = b.get(index).copied().unwrap_or(0);
        diff |= (left ^ right) as usize;
    }
    diff == 0
}

/// Reference [`PrincipalVerifier`] adapter: constant-time bearer compare against
/// a configured secret; identity is bound from the configured mapping, never the
/// caller headers. BREAK-GLASS ONLY — one static `(principal_id, tenant_id)` pair
/// per shared secret, so it cannot distinguish multi-tenant callers.
pub struct ConfiguredBearerPrincipalVerifier {
    bearer_secret: String,      // data_class: SECRET
    bound_principal_id: String, // data_class: INTERNAL_ONLY
    bound_tenant_id: String,    // data_class: INTERNAL_ONLY
}

impl ConfiguredBearerPrincipalVerifier {
    /// Construct, REFUSING an empty bearer secret or empty bound identity.
    ///
    /// # Errors
    /// [`AuthzProviderConfigError`] when the secret or bound identity is empty.
    pub fn new(
        bearer_secret: impl Into<String>,
        bound_principal_id: impl Into<String>,
        bound_tenant_id: impl Into<String>,
    ) -> Result<Self, AuthzProviderConfigError> {
        let bearer_secret = bearer_secret.into();
        let bound_principal_id = bound_principal_id.into();
        let bound_tenant_id = bound_tenant_id.into();
        if bearer_secret.trim().is_empty() {
            return Err(AuthzProviderConfigError::EmptyBearerSecret);
        }
        if bound_principal_id.trim().is_empty() || bound_tenant_id.trim().is_empty() {
            return Err(AuthzProviderConfigError::EmptyBoundIdentity);
        }
        Ok(Self {
            bearer_secret,
            bound_principal_id,
            bound_tenant_id,
        })
    }
}

impl PrincipalVerifier for ConfiguredBearerPrincipalVerifier {
    fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedPrincipal, PrincipalVerificationError> {
        let Some(authorization) = credential.authorization.as_deref() else {
            return Err(PrincipalVerificationError::MissingCredential);
        };
        let Some(presented) = authorization.strip_prefix("Bearer ") else {
            return Err(PrincipalVerificationError::InvalidCredential);
        };
        if !constant_time_eq(presented.as_bytes(), self.bearer_secret.as_bytes()) {
            return Err(PrincipalVerificationError::InvalidCredential);
        }
        Ok(VerifiedPrincipal::new(
            self.bound_principal_id.clone(),
            self.bound_tenant_id.clone(),
        ))
    }
}

/// Why the authz provider refused construction. Boot-fatal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthzProviderConfigError {
    /// The bearer secret was empty/whitespace (no provable credential root).
    EmptyBearerSecret,
    /// The bound principal/tenant identity was empty.
    EmptyBoundIdentity,
}

impl std::fmt::Display for AuthzProviderConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBearerSecret => {
                write!(f, "authz provider bearer secret must be non-empty")
            }
            Self::EmptyBoundIdentity => {
                write!(f, "authz provider bound principal/tenant must be non-empty")
            }
        }
    }
}

impl std::error::Error for AuthzProviderConfigError {}
