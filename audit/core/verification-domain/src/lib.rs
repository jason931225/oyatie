//! Audit-chain verification domain: the read side of what
//! `audit-sealing-domain` produces.
//!
//! ## What this crate owns
//!
//! One decision and the trust material to reach it. [`verify`] is the entry
//! point; every contract sits on the item that carries it — `verify`'s own
//! named predicates for the checks and their order, `ports` for the traits
//! a caller supplies, [`verification_signing_payload`] for the bytes a
//! signature must cover, [`VerificationFailureReason`] for the closed set
//! of ways a claim can fail.
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
