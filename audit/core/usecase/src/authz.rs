//! Fail-closed authorization seam for the `audit.event.emit` boundary: verify a
//! producer principal from an unforgeable credential, then authorize the target
//! `{tenant, surface, scope}` at the PDP. Both ports are required (ADR-0588).

// ADR-0083 Tier 3: production code stays panic-free.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

/// The credential a caller presents to prove a real producer principal identity.
/// The `claimed_*` fields are a CROSS-CHECK input only — never proof of identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallerCredential {
    /// Raw `Authorization` header value (e.g. `"Bearer abc..."`), if present.
    pub authorization: Option<String>, // data_class: SECRET
    /// The caller-asserted producer id from the envelope (cross-check input).
    pub claimed_producer_id: String, // data_class: INTERNAL_ONLY
    /// The caller-asserted tenant id from the envelope (cross-check input).
    pub claimed_tenant_id: String, // data_class: INTERNAL_ONLY
}

/// A producer principal whose identity has been verified from a caller
/// credential. Private fields and no public constructor: outside this crate one
/// is obtainable only by running a [`PrincipalVerifier`].
///
/// Structural defense-in-depth, NOT a cryptographic proof — in-process code can
/// still construct its own verifier with a known secret. The guarantee is
/// verification + PDP decision + cross-check together, not this type alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedProducerPrincipal {
    producer_id: String, // data_class: INTERNAL_ONLY — private: see unforgeability note above
    tenant_id: String,   // data_class: INTERNAL_ONLY — private: see unforgeability note above
}

impl VerifiedProducerPrincipal {
    pub(crate) fn new(producer_id: impl Into<String>, tenant_id: impl Into<String>) -> Self {
        Self {
            producer_id: producer_id.into(),
            tenant_id: tenant_id.into(),
        }
    }

    /// The authoritative producer id bound from the verified credential.
    #[must_use]
    pub fn producer_id(&self) -> &str {
        &self.producer_id
    }

    /// The authoritative tenant the producer acts within.
    #[must_use]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    /// Test-only constructor that mints a token without a real credential.
    #[cfg(test)]
    pub(crate) fn new_for_test(
        producer_id: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self::new(producer_id, tenant_id)
    }
}

/// Why principal verification refused. Every variant is fail-closed: the caller
/// maps it to HTTP 401 and the request never reaches the authorizer or the
/// audit chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrincipalVerificationError {
    /// No credential was presented (no `Authorization` header).
    MissingCredential,
    /// A credential was presented but did not verify. Deliberately opaque so
    /// probing cannot distinguish "wrong token" from "no such principal".
    InvalidCredential,
}

/// Why authorization refused (403-class: authenticated but not permitted).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditEmitAuthorizationError {
    /// The PDP returned a deny decision for this principal/action/resource.
    Denied,
    /// The PDP refused to decide (fail-closed: a refusal is treated as deny).
    Refused,
}

/// Whether an audit-emit records for one tenant or for the platform itself.
/// Carried explicitly so the PDP sees the true blast radius: presenting a
/// platform emit as tenant-scoped would let tenant producers forge
/// platform-level audit records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditEmitScope {
    /// Recorded for one tenant, named by the enclosing resource's `tenant_id`.
    Tenant,
    /// Recorded against the platform's own audit lineage. The PDP MUST require
    /// platform-audit authority, not mere tenant-producer authority.
    Platform,
}

/// The resource an audit-emit decision is made against. `tenant_id` and
/// `surface` come from the validated payload's TARGET fields, never flattened to
/// the caller's verified tenant, so a cross-tenant emit is deniable at the PDP.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEmitResource {
    /// The tenant the event is recorded for. Empty for
    /// [`AuditEmitScope::Platform`]: the PDP keys on `scope`, not this field.
    pub tenant_id: String, // data_class: INTERNAL_ONLY
    /// The operational surface the event attests (from the validated payload).
    pub surface: String, // data_class: INTERNAL_ONLY
    /// Whether this emit records for one tenant or for the platform itself.
    pub scope: AuditEmitScope, // data_class: INTERNAL_ONLY
}

/// PORT: verify a caller credential into a [`VerifiedProducerPrincipal`]. The
/// verifier — not the envelope fields — is the source of truth for identity.
pub trait PrincipalVerifier: Send + Sync {
    /// Verify `credential` and return the authoritative principal, or refuse.
    ///
    /// # Errors
    /// [`PrincipalVerificationError`] — fail-closed; caller maps to HTTP 401.
    fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedProducerPrincipal, PrincipalVerificationError>;
}

/// PORT: decide whether `principal` may emit an audit event for `resource`. The
/// default posture is deny.
///
/// Adapter contract: map EVERY internal fault (network, timeout, parse,
/// unavailable) to `Err(Refused)`; enforce your own deadline, because this port
/// enforces none; never panic — the release profile is `panic = "abort"`, so
/// `catch_unwind` is defeated and this crate deliberately does not attempt it.
pub trait AuditEmitAuthorizer: Send + Sync {
    /// Authorize `principal` to emit an audit event for `resource`, or refuse.
    ///
    /// # Errors
    /// [`AuditEmitAuthorizationError`] on deny or any PDP fault (fail-closed).
    fn ensure_authorized(
        &self,
        principal: &VerifiedProducerPrincipal,
        resource: &AuditEmitResource,
    ) -> Result<(), AuditEmitAuthorizationError>;
}

/// The audit-emit boundary's authz provider: a [`PrincipalVerifier`] port plus
/// an [`AuditEmitAuthorizer`] port. Required; there is no default-allow path.
pub struct AuditEmitAuthzProvider {
    verifier: std::sync::Arc<dyn PrincipalVerifier>, // data_class: INTERNAL_ONLY
    authorizer: std::sync::Arc<dyn AuditEmitAuthorizer>, // data_class: INTERNAL_ONLY
}

impl AuditEmitAuthzProvider {
    /// Assemble the provider. There is no `Default` and no allow-all constructor.
    #[must_use]
    pub fn new(
        verifier: std::sync::Arc<dyn PrincipalVerifier>,
        authorizer: std::sync::Arc<dyn AuditEmitAuthorizer>,
    ) -> Self {
        Self {
            verifier,
            authorizer,
        }
    }

    /// Verify the caller principal; envelope fields are never identity.
    ///
    /// # Errors
    /// [`PrincipalVerificationError`] — caller maps to HTTP 401.
    pub fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedProducerPrincipal, PrincipalVerificationError> {
        self.verifier.verify_principal(credential)
    }

    /// Authorize the verified principal at the PDP port. Default-deny.
    ///
    /// # Errors
    /// [`AuditEmitAuthorizationError`] — caller maps to HTTP 403.
    pub fn ensure_authorized(
        &self,
        principal: &VerifiedProducerPrincipal,
        resource: &AuditEmitResource,
    ) -> Result<(), AuditEmitAuthorizationError> {
        self.authorizer.ensure_authorized(principal, resource)
    }
}

/// Constant-time byte comparison (no early-exit) for secret material; never use
/// a naive `==`.
///
/// Residual: `a.len() ^ b.len()` leaks whether the lengths match. Accepted for
/// fixed-length bearer tokens; use an HMAC if length-hiding is required.
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

/// A reference [`PrincipalVerifier`] adapter: constant-time bearer compare
/// against a configured secret, then bind the producer identity from the
/// configured mapping (NOT from the envelope fields).
///
/// BREAK-GLASS ONLY. It binds ONE static `(producer_id, tenant_id)` to one
/// shared secret, so in multi-tenant production every caller would be granted
/// the same identity. Production uses the mTLS/SPIFFE peer-SVID verifier.
pub struct ConfiguredBearerPrincipalVerifier {
    bearer_secret: String,     // data_class: SECRET
    bound_producer_id: String, // data_class: INTERNAL_ONLY
    bound_tenant_id: String,   // data_class: INTERNAL_ONLY
}

impl ConfiguredBearerPrincipalVerifier {
    /// Construct, REFUSING an empty bearer secret or empty bound identity.
    ///
    /// # Errors
    /// [`AuthzProviderConfigError`] when the secret or bound identity is empty.
    pub fn new(
        bearer_secret: impl Into<String>,
        bound_producer_id: impl Into<String>,
        bound_tenant_id: impl Into<String>,
    ) -> Result<Self, AuthzProviderConfigError> {
        let bearer_secret = bearer_secret.into();
        let bound_producer_id = bound_producer_id.into();
        let bound_tenant_id = bound_tenant_id.into();
        if bearer_secret.trim().is_empty() {
            return Err(AuthzProviderConfigError::EmptyBearerSecret);
        }
        if bound_producer_id.trim().is_empty() || bound_tenant_id.trim().is_empty() {
            return Err(AuthzProviderConfigError::EmptyBoundIdentity);
        }
        Ok(Self {
            bearer_secret,
            bound_producer_id,
            bound_tenant_id,
        })
    }
}

impl PrincipalVerifier for ConfiguredBearerPrincipalVerifier {
    fn verify_principal(
        &self,
        credential: &CallerCredential,
    ) -> Result<VerifiedProducerPrincipal, PrincipalVerificationError> {
        let Some(authorization) = credential.authorization.as_deref() else {
            return Err(PrincipalVerificationError::MissingCredential);
        };
        let Some(presented) = authorization.strip_prefix("Bearer ") else {
            return Err(PrincipalVerificationError::InvalidCredential);
        };
        if !constant_time_eq(presented.as_bytes(), self.bearer_secret.as_bytes()) {
            return Err(PrincipalVerificationError::InvalidCredential);
        }
        Ok(VerifiedProducerPrincipal::new(
            self.bound_producer_id.clone(),
            self.bound_tenant_id.clone(),
        ))
    }
}

/// Why the authz provider refused construction. Boot-fatal: refuse to serve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthzProviderConfigError {
    /// The bearer secret was empty/whitespace (no provable credential root).
    EmptyBearerSecret,
    /// The bound producer/tenant identity was empty.
    EmptyBoundIdentity,
}

impl std::fmt::Display for AuthzProviderConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyBearerSecret => {
                write!(
                    f,
                    "audit-emit authz provider bearer secret must be non-empty"
                )
            }
            Self::EmptyBoundIdentity => {
                write!(
                    f,
                    "audit-emit authz provider bound producer/tenant must be non-empty"
                )
            }
        }
    }
}

impl std::error::Error for AuthzProviderConfigError {}
