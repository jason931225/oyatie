//! Crate-local error type for the trust daemon.

use os_kernel::Error as CoreError;
use std::fmt;

/// Errors raised by the trustd PKI subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustError {
    /// A value failed validation.
    Invalid(String),
    /// PEM / encoding error.
    Pem(String),
    /// A CSR did not satisfy issuance policy.
    CsrRejected(String),
    /// Certificate verification failed (bad signature, wrong CA, etc.).
    VerificationFailed(String),
    /// The presented join token was missing, malformed, or did not match.
    TokenMismatch(String),
    /// The certificate or token has expired.
    Expired(String),
    /// The caller's role is not permitted to perform the action.
    PermissionDenied(String),
    /// A required item was not found.
    NotFound(String),
    /// Catch-all.
    Other(String),
}

impl TrustError {
    /// Construct an [`TrustError::Invalid`].
    pub fn invalid(m: impl Into<String>) -> Self {
        TrustError::Invalid(m.into())
    }
    /// Construct a [`TrustError::Pem`].
    pub fn pem(m: impl Into<String>) -> Self {
        TrustError::Pem(m.into())
    }
    /// Construct a [`TrustError::CsrRejected`].
    pub fn csr_rejected(m: impl Into<String>) -> Self {
        TrustError::CsrRejected(m.into())
    }
    /// Construct a [`TrustError::VerificationFailed`].
    pub fn verification_failed(m: impl Into<String>) -> Self {
        TrustError::VerificationFailed(m.into())
    }
    /// Construct a [`TrustError::TokenMismatch`].
    pub fn token_mismatch(m: impl Into<String>) -> Self {
        TrustError::TokenMismatch(m.into())
    }
    /// Construct an [`TrustError::Expired`].
    pub fn expired(m: impl Into<String>) -> Self {
        TrustError::Expired(m.into())
    }
    /// Construct a [`TrustError::PermissionDenied`].
    pub fn permission_denied(m: impl Into<String>) -> Self {
        TrustError::PermissionDenied(m.into())
    }
    /// Construct a [`TrustError::NotFound`].
    pub fn not_found(m: impl Into<String>) -> Self {
        TrustError::NotFound(m.into())
    }

    /// Short stable kind tag.
    pub fn kind(&self) -> &'static str {
        match self {
            TrustError::Invalid(_) => "invalid",
            TrustError::Pem(_) => "pem",
            TrustError::CsrRejected(_) => "csr_rejected",
            TrustError::VerificationFailed(_) => "verification_failed",
            TrustError::TokenMismatch(_) => "token_mismatch",
            TrustError::Expired(_) => "expired",
            TrustError::PermissionDenied(_) => "permission_denied",
            TrustError::NotFound(_) => "not_found",
            TrustError::Other(_) => "other",
        }
    }
}

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrustError::Invalid(m) => write!(f, "invalid: {m}"),
            TrustError::Pem(m) => write!(f, "pem: {m}"),
            TrustError::CsrRejected(m) => write!(f, "csr rejected: {m}"),
            TrustError::VerificationFailed(m) => write!(f, "verification failed: {m}"),
            TrustError::TokenMismatch(m) => write!(f, "token mismatch: {m}"),
            TrustError::Expired(m) => write!(f, "expired: {m}"),
            TrustError::PermissionDenied(m) => write!(f, "permission denied: {m}"),
            TrustError::NotFound(m) => write!(f, "not found: {m}"),
            TrustError::Other(m) => write!(f, "error: {m}"),
        }
    }
}

impl From<TrustError> for CoreError {
    fn from(e: TrustError) -> Self {
        match e {
            TrustError::Invalid(m) | TrustError::Pem(m) | TrustError::CsrRejected(m) => {
                CoreError::Invalid(m)
            }
            TrustError::VerificationFailed(m)
            | TrustError::TokenMismatch(m)
            | TrustError::PermissionDenied(m) => CoreError::PermissionDenied(m),
            TrustError::Expired(m) => CoreError::InvalidState(m),
            TrustError::NotFound(m) => CoreError::NotFound(m),
            TrustError::Other(m) => CoreError::Other(m),
        }
    }
}

impl From<CoreError> for TrustError {
    fn from(e: CoreError) -> Self {
        TrustError::Other(e.to_string())
    }
}

impl std::error::Error for TrustError {}

/// Crate-local result alias.
pub type Result<T> = std::result::Result<T, TrustError>;
