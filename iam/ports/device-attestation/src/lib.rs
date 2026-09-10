//! Device and workload attestation port, feeding Cedar context (ADR-0719).
//!
//! Every implementation must fail closed: until a real adapter is wired,
//! [`UnwiredAttestation`] refuses rather than returning a trusted context.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// Closed set: removing a variant needs a recorded decision, because an adapter
/// keyed on it stops being reachable rather than failing to compile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationKind {
    Passkey,
    Mdm,
    ChromeEnterprise,
    SpiffeWorkload,
}

impl AttestationKind {
    pub const CLOSED: [AttestationKind; 4] = [
        AttestationKind::Passkey,
        AttestationKind::Mdm,
        AttestationKind::ChromeEnterprise,
        AttestationKind::SpiffeWorkload,
    ];
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceContext {
    pub kind: AttestationKind,
    pub trusted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestationError {
    AdapterNotWired(AttestationKind),
    TokenEmpty,
}

impl Display for AttestationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::AdapterNotWired(kind) => {
                write!(f, "device attestation adapter {kind:?} is not wired")
            }
            Self::TokenEmpty => write!(f, "attestation token is empty"),
        }
    }
}

impl Error for AttestationError {}

pub trait DeviceAttestation: Send + Sync {
    fn kind(&self) -> AttestationKind;
    fn verify(&self, token: &[u8]) -> Result<DeviceContext, AttestationError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnwiredAttestation {
    pub kind: AttestationKind,
}

impl DeviceAttestation for UnwiredAttestation {
    fn kind(&self) -> AttestationKind {
        self.kind
    }

    fn verify(&self, token: &[u8]) -> Result<DeviceContext, AttestationError> {
        if token.is_empty() {
            return Err(AttestationError::TokenEmpty);
        }
        Err(AttestationError::AdapterNotWired(self.kind))
    }
}

pub fn bind(kind: AttestationKind) -> UnwiredAttestation {
    UnwiredAttestation { kind }
}
