#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

mod merkle_adapter;
mod ports;
mod request;
mod verify;

pub use audit_verification_api::{VerificationFailureReason, VerificationVerdict};

pub use merkle_adapter::ChainMerkleVerifier;
pub use ports::{KeyResolver, MerkleVerifier, RedactionRegistry, RootRegistry};
pub use request::{MerkleInclusionProof, PriorRootClaim, VerificationRequest};
pub use verify::{verification_signing_payload, verify};
