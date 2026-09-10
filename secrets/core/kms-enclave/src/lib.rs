#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

//! Crypto-enclave kernel for kms: the type-system one-way door for key
//! material. Keys live inside the crypto boundary and key material never
//! leaves it, following the AWS KMS domain model.
//!
//! The separate enclave *process* boundary ships as a `[[bin]]` wrapper
//! later; every key-material invariant it must uphold is enforced here, in
//! types, so the binary cannot weaken them.
//!
//! [`KekMaterial`], [`DekMaterial`] and [`EnclaveRoot`] have ingress doors
//! (generation from the CSPRNG, [`EnclaveRoot::from_key_bytes`] for the
//! unseal ceremony) and no egress doors. That absence is the enforcement,
//! and no test can pin it.

pub mod chain;
pub mod dek_cache;
pub mod material;
pub mod mlocked;
pub mod provenance;
pub mod shred;
pub mod token;

pub use chain::{DecryptOnlyKek, KekVersionChain};
pub use dek_cache::{
    BoundedTtlDekCache, ClockSource, ControlPlaneUnavailable, DekCacheError, DekCacheKey,
    FetchSource, SystemClockSource,
};
pub use material::{DekMaterial, EnclaveRoot, KekMaterial, KekVersion, SealingRootId};
pub use provenance::RootProvenance;
pub use shred::{
    CancelEvidence, MIN_WAITING_WINDOW_SECONDS, PendingDeletionChain, QuorumPolicy,
    ScheduledKeyDeletion, ShredAction, ShredAuthorizationPort, ShredAuthorizationRequest,
    ShredDecision, ShredDecisionEvidence, ShredError, ShredProof,
};
pub use token::{TokenError, WrappedDek, WrappedKekToken};

// Re-exported so enclave callers name envelope identifiers without a direct
// dependency on the domain crate.
pub use secrets_kms_domain::envelope_keys::{DekId, EnvelopeKeyError, KekId};

use std::fmt;

/// AEAD failures are deliberately collapsed into [`EnclaveError::CryptoRejected`]
/// without distinguishing tamper, wrong-key, or AAD mismatch — distinguishing
/// them would hand an oracle to a caller probing the crypto boundary.
#[derive(Debug)]
pub enum EnclaveError {
    /// `mlock(2)` refused to pin the key page; key material must never touch
    /// swap, so construction fails closed instead of degrading silently.
    MemoryLockFailed {
        errno: i32,
    },
    RandomSourceFailed,
    CryptoRejected,
    TokenMalformed(TokenError),
    KeyBindingMismatch {
        expected: String,
        found: String,
    },
    UnknownKekVersion {
        version: u32,
    },
    VersionOverflow,
    ZeroVersion,
    InvalidIdentifier(EnvelopeKeyError),
}

impl fmt::Display for EnclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemoryLockFailed { errno } => {
                write!(
                    f,
                    "enclave: mlock failed (errno {errno}); refusing unpinned key material"
                )
            }
            Self::RandomSourceFailed => f.write_str("enclave: CSPRNG failure"),
            Self::CryptoRejected => f.write_str("enclave: AEAD rejected the operation"),
            Self::TokenMalformed(err) => write!(f, "enclave: malformed wrapped token: {err}"),
            Self::KeyBindingMismatch { expected, found } => {
                write!(
                    f,
                    "enclave: token bound to '{found}', unwrapping key is '{expected}'"
                )
            }
            Self::UnknownKekVersion { version } => {
                write!(f, "enclave: no KEK material held for version {version}")
            }
            Self::VersionOverflow => f.write_str("enclave: KEK version overflow"),
            Self::ZeroVersion => f.write_str("enclave: KEK versions are 1-based; zero is invalid"),
            Self::InvalidIdentifier(err) => {
                write!(f, "enclave: invalid envelope identifier: {err}")
            }
        }
    }
}

impl std::error::Error for EnclaveError {}

impl From<TokenError> for EnclaveError {
    fn from(err: TokenError) -> Self {
        Self::TokenMalformed(err)
    }
}

impl From<EnvelopeKeyError> for EnclaveError {
    fn from(err: EnvelopeKeyError) -> Self {
        Self::InvalidIdentifier(err)
    }
}
