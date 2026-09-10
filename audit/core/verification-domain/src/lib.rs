//! Audit-chain verification domain: pure proof + signature verification.
//!
//! ## What this crate owns
//!
//! [`verify`] answers one question about one sealed period's claim: does it
//! hold, and if not, which closed-set reason does it fail with. It resolves
//! the trusted key, verifies the Ed25519 signature over the record's own
//! canonical payload ([`verification_signing_payload`]), verifies the Merkle
//! inclusion proof, confirms the prior published root chains (or that a
//! first-period claim is independently confirmed), rejects identity
//! mismatches across `(pack, tenant_partition, period_id)`, and reports a
//! redacted leaf honestly instead of as a silent pass — mutating neither
//! `request` nor anything reachable through its four ports.
//!
//! It depends only on `audit-chain-domain` (hashes, Ed25519, Merkle proof
//! checking) and `audit-verification-api` (the verdict types), NOT on
//! `audit_verification_kernel`, whose `RootRegistry` / `KeyResolver` /
//! `MerkleVerifier` ports it would otherwise reuse: that dependency edge is
//! out of scope here, so `ports` declares those three shapes independently.
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
