//! Fail-closed authorization seam for the `cedar.policy.publish` control plane.
//!
//! A caller credential is verified into a [`VerifiedPrincipal`] by a [`PrincipalVerifier`]
//! port; that principal is then authorized for the action against the target policy by a
//! [`PublishAuthorizer`] port. The header-supplied principal id is never the source of truth,
//! and both ports are required: there is no default-allow fallback.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// The credential a caller presents to prove a real principal identity.
///
/// The header-supplied principal id travels alongside as a CROSS-CHECK only, never as proof.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerCredential {
    /// Raw `Authorization` header value (e.g. `"Bearer abc..."`), if present.
    pub authorization: Option<String>, // data_class: SECRET
    /// The caller-asserted principal id from `x-principal-id` (cross-check input).
    pub claimed_principal_id: String, // data_class: INTERNAL_ONLY
    /// The caller-asserted principal tenant from `x-principal-tenant-id` (cross-check input).
    pub claimed_tenant_id: String, // data_class: INTERNAL_ONLY
}

/// A principal whose identity has been verified from a caller credential.
///
/// Fields are private with no public constructor, so an external crate cannot forge one by
/// struct literal. That is STRUCTURAL defense-in-depth, not a cryptographic proof: it shows
/// only that some [`PrincipalVerifier`] ran, and hostile in-process code can still mint one
/// through its own verifier with a known secret.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPrincipal {
    principal_id: String, // data_class: INTERNAL_ONLY — private: see unforgeability note above
    tenant_id: String,    // data_class: INTERNAL_ONLY — private: see unforgeability note above
}

impl VerifiedPrincipal {
    /// Mint a verified principal; callers outside this crate must go through a
    /// [`PrincipalVerifier`].
    pub(crate) fn new(principal_id: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self {
            principal_id: principal_id.into(),
            tenant_id: tenant_id.into(),
        }
    }

    pub fn principal_id(&self) -> &str {
        &self.principal_id
    }

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

/// Why principal verification refused. Every variant is fail-closed: the caller
/// maps it to HTTP 401 and the request never reaches the authorizer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrincipalVerificationError {
    MissingCredential,
    /// A credential was presented but did not verify. Deliberately opaque, so probing cannot
    /// distinguish "wrong token" from "no such principal".
    InvalidCredential,
}

/// Why authorization refused. Each variant maps to HTTP 403 (the principal is
/// authenticated but not permitted for this action/resource/tenant).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublishAuthorizationError {
    Denied,
    /// The PDP refused to decide (fail-closed: a refusal is treated as deny).
    Refused,
}

/// The scope of a publish resource: whether it affects one specific tenant or
/// all tenants. Carried explicitly so the PDP sees the true blast radius: presenting a global
/// policy as tenant-scoped with the caller's own tenant would silently authorize a
/// tenant-admin for platform-wide policy control.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PublishScope {
    /// Scoped to the single tenant identified by `tenant_id` in the enclosing resource.
    Tenant,
    /// Applies to ALL tenants; the PDP must require platform-admin authority, not treat it as
    /// a resource belonging to any individual tenant.
    Global,
}

/// The resource a publish decision is made against: the target policy, its scope, and the
/// tenant when tenant-scoped. The tenant axis is asserted by the authorizer — a verified
/// principal alone never grants it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishResource {
    /// The policy id being published (from the path/body, already bound equal).
    pub policy_id: String, // data_class: INTERNAL_ONLY
    /// Whether this publish affects one tenant or all of them; the PDP MUST distinguish these.
    /// A global publish requires platform-admin, not tenant-admin, authority.
    pub scope: PublishScope, // data_class: INTERNAL_ONLY
    /// The tenant whose policy store the version lands in; empty for [`PublishScope::Global`],
    /// where the PDP must key on the scope rather than on this field.
    pub tenant_id: String, // data_class: INTERNAL_ONLY
}

/// PORT: verify a caller credential into a [`VerifiedPrincipal`].
///
/// The verifier, never the headers, is the source of truth for caller identity.
pub trait PrincipalVerifier: Send + Sync {
    /// Verify `credential` and return the authoritative principal, or refuse.
    ///
    /// # Errors
    /// [`PrincipalVerificationError`] when no credential is presented or it does not verify;
    /// fail-closed, so the caller MUST treat this as 401.
    fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedPrincipal, PrincipalVerificationError>;
}

/// PORT: decide whether `principal` may publish `resource`.
///
/// Default posture is deny. An adapter MUST map every internal fault — network error, timeout,
/// parse failure, unavailability, expiry — to `Err(PublishAuthorizationError::Refused)`, MUST
/// enforce its own deadline (this port enforces none, and a hung PDP hangs the caller
/// thread), and MUST NOT panic: the release profile uses `panic = "abort"`, so the wrapper's
/// `catch_unwind` is a test-only backstop and no production fault isolation.
pub trait PublishAuthorizer: Send + Sync {
    /// Authorize `principal` to publish `resource`, or refuse.
    ///
    /// # Errors
    /// [`PublishAuthorizationError`] on an explicit deny or any PDP fault; caller maps to 403.
    fn ensure_authorized(
        &self,
        principal: &VerifiedPrincipal,
        resource: &PublishResource,
    ) -> Result<(), PublishAuthorizationError>;
}

/// The authz provider the router depends on: a [`PrincipalVerifier`] port plus a
/// [`PublishAuthorizer`] port. The router REFUSES to serve without both configured.
pub struct CedarPolicyAuthzProvider {
    verifier: std::sync::Arc<dyn PrincipalVerifier>, // data_class: INTERNAL_ONLY
    authorizer: std::sync::Arc<dyn PublishAuthorizer>, // data_class: INTERNAL_ONLY
}

impl CedarPolicyAuthzProvider {
    #[must_use]
    pub fn new(
        verifier: std::sync::Arc<dyn PrincipalVerifier>,
        authorizer: std::sync::Arc<dyn PublishAuthorizer>,
    ) -> Self {
        Self {
            verifier,
            authorizer,
        }
    }

    /// Verify the caller principal via the [`PrincipalVerifier`] port; headers are never
    /// trusted as identity.
    ///
    /// # Errors
    /// [`PrincipalVerificationError`] — caller maps to 401.
    pub fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedPrincipal, PrincipalVerificationError> {
        self.verifier.verify_principal(credential)
    }

    /// Authorize the verified principal for the publish resource via the PDP port.
    /// Default-deny / fail-closed.
    ///
    /// The `catch_unwind` here catches only test panics, so the router tests can check the
    /// panicking-authorizer-to-403 property; under `panic = "abort"` it has no effect.
    ///
    /// # Errors
    /// [`PublishAuthorizationError`] — caller maps to 403.
    pub fn ensure_authorized(
        &self,
        principal: &VerifiedPrincipal,
        resource: &PublishResource,
    ) -> Result<(), PublishAuthorizationError> {
        // No-op under `panic = "abort"`; not production fault isolation.
        let authorizer = std::sync::Arc::clone(&self.authorizer);
        let principal = principal.clone();
        let resource = resource.clone();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            authorizer.ensure_authorized(&principal, &resource)
        }))
        .unwrap_or(Err(PublishAuthorizationError::Refused))
    }
}

/// Constant-time byte comparison (no early-exit) so a bearer compare cannot be timing-probed.
/// NEVER use a naive `==` on secret material.
///
/// Residual: the length of both inputs is visible from the XOR seed. Accepted here because
/// bearer tokens are fixed-length; use a MAC if length-hiding is required.
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

/// A reference [`PrincipalVerifier`] adapter that verifies a bearer token by a
/// constant-time compare against a configured secret, then binds the principal
/// identity from the configured mapping (NOT from the caller headers).
///
/// BREAK-GLASS ONLY, not multi-tenant production: it binds ONE static `(principal_id,
/// tenant_id)` pair to a single shared secret, so distinct callers would all be granted the
/// same identity. Construction REFUSES an empty bearer secret or bound identity, so a
/// process that cannot prove a credential root can never authenticate a caller.
pub struct ConfiguredBearerPrincipalVerifier {
    bearer_secret: String,      // data_class: SECRET
    bound_principal_id: String, // data_class: INTERNAL_ONLY
    bound_tenant_id: String,    // data_class: INTERNAL_ONLY
}

impl ConfiguredBearerPrincipalVerifier {
    /// Construct, REFUSING an empty bearer secret or bound identity.
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

/// Why the authz provider refused construction. Boot-fatal: the composition root MUST refuse
/// to serve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthzProviderConfigError {
    EmptyBearerSecret,
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
