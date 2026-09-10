//! Audit-chain sealing domain: seal-record construction, `SealStatus`
//! lifecycle transitions, and `PackEpoch` signing-key coverage checks.
//!
//! ## What this crate owns
//!
//! The pure (I/O-free) `core/*-domain` crate for the sealing capability. It
//! reimplements neither crate it sits on: `audit_chain_domain` supplies the
//! RFC 6962 §2.1 Merkle math, wrapped behind the `MerkleEngine` port
//! ([`merkle_engine::MerkleTreeEngine`]) rather than re-derived here, and
//! `audit_sealing_kernel` supplies the `SigningKeyRef` / `PackEpoch` /
//! `SealStatus` / `SealRecord` types plus the five trait ports this crate's
//! callers compose against.
//!
//! No PKCS#11, S3, Postgres, Mimir or HTTP call happens here; each belongs
//! behind `SignerPort` / `RootPublisher` / `IndexWriter` /
//! `ObjectStoreWriter`, none of which `audit/` implements yet. Every function
//! here is pure, or validates a typed attestation the caller supplies
//! ([`seal_record::PriorPeriod`]): a pure domain crate has no read path of
//! its own to confirm what happened before.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

pub mod epoch;
pub mod merkle_engine;
pub mod seal_record;
pub mod status;

pub use audit_chain_domain::{MerkleTree, Sha256Hash};
pub use audit_sealing_kernel::{PackEpoch, SealRecord, SealStatus, SigningKeyRef};

pub use epoch::verify_epoch_covers_period;
pub use merkle_engine::{MerkleTreeEngine, verify_leaf_inclusion};
pub use seal_record::{PriorPeriod, PriorPeriodLookup, SealRecordInput, build_seal_record};
pub use status::{apply_seal_status_transition, transition_seal_status};

/// Domain-level seal error variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SealingDomainError {
    EmptyPack,
    EmptyTenantPartition,
    InvalidLeafCount,
    InvalidProofPath,
    /// Rejected rather than silently trusting either value: a mismatch means
    /// the caller's batch accumulator and the leaves it actually handed over
    /// have already disagreed before any hash runs.
    LeafCountMismatch {
        declared: u64,
        actual: u64,
    },
    EmptyPriorRoot,
    /// The prior-root string is not shaped like a root this crate's own
    /// `encode_root` could ever emit (`sha256:` followed by exactly 64
    /// lowercase hex characters). This crate cannot verify that a prior-root
    /// string is the REAL prior period's root (it has no read path), but it
    /// can and does reject values that are structurally impossible chain
    /// references.
    MalformedPriorRoot {
        root: String,
    },
    SelfReferentialPriorRoot,
    /// The caller claimed [`PriorPeriod::First`], but the supplied
    /// [`PriorPeriodLookup`] reports a sealed period already exists for
    /// `(pack, tenant_partition)`. A false firstness claim
    /// would otherwise seal a record with `prior_root: None` that is not
    /// actually the start of the chain, defeating tamper-evidence between
    /// periods.
    FalseFirstPeriodClaim {
        pack: String,
        tenant_partition: String,
    },
    IllegalSealStatusTransition {
        from: SealStatus,
        to: SealStatus,
    },
    EpochPackMismatch {
        epoch_pack: String,
        record_pack: String,
    },
    EpochTenantPartitionMismatch {
        epoch_tenant_partition: String,
        record_tenant_partition: String,
    },
    PeriodOutsideEpochWindow {
        period: String,
        period_lo: String,
        period_hi: String,
    },
    RetiringKeyOutsideEpochWindow {
        key_id: String,
        period: String,
        period_lo: String,
        period_hi: String,
    },
    SigningKeyNotInEpoch {
        key_id: String,
    },
}
