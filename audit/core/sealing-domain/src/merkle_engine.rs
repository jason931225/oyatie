//! `MerkleEngine` port implementation backed by [`audit_chain_domain::MerkleTree`].
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
    )
)]

use audit_chain_domain::{MerkleTree, Sha256Hash};
use audit_sealing_kernel::MerkleEngine;

use crate::SealingDomainError;

/// Concrete [`MerkleEngine`] adapter over [`audit_chain_domain::MerkleTree`].
///
/// Holds no state; every call is a pure function of its arguments. Rejects
/// an empty leaf set with [`SealingDomainError::InvalidLeafCount`] rather
/// than calling into [`MerkleTree::new`], which panics on empty input.
#[derive(Clone, Copy, Debug, Default)]
pub struct MerkleTreeEngine;

impl MerkleEngine for MerkleTreeEngine {
    type Leaf = Sha256Hash;
    type Root = Sha256Hash;
    type Error = SealingDomainError;

    /// # Errors
    /// [`SealingDomainError::InvalidLeafCount`] when `leaves` is empty.
    fn root(&self, leaves: &[Sha256Hash]) -> Result<Sha256Hash, SealingDomainError> {
        if leaves.is_empty() {
            return Err(SealingDomainError::InvalidLeafCount);
        }
        Ok(MerkleTree::new(leaves.to_vec()).build_root())
    }
}

/// Verify that `leaf` at `leaf_index` is included under `root`, via
/// `proof_path`, for a tree built over `leaf_count` leaves.
///
/// A thin, explicit wrapper over [`MerkleTree::verify_proof`] so callers get
/// a typed [`SealingDomainError`] instead of a bare `bool`.
///
/// # Errors
/// [`SealingDomainError::InvalidProofPath`] when the proof does not
/// recompute to `root`: a tampered leaf, a tampered or truncated proof
/// path, an out-of-range `leaf_index`, and a proof whose depth does not
/// match `leaf_count` all fail closed here.
pub fn verify_leaf_inclusion(
    leaf: Sha256Hash,
    leaf_index: usize,
    proof_path: &[Sha256Hash],
    root: Sha256Hash,
    leaf_count: u64,
) -> Result<(), SealingDomainError> {
    let leaf_count = usize::try_from(leaf_count).unwrap_or(usize::MAX);
    if MerkleTree::verify_proof(leaf, leaf_index, proof_path, root, leaf_count) {
        Ok(())
    } else {
        Err(SealingDomainError::InvalidProofPath)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic, distinct-per-`seed` 32-byte stand-in for a real
    /// SHA-256 digest. `MerkleTree`/`MerkleEngine` treat leaves as opaque
    /// 32-byte values (callers are expected to hash their own leaf preimage
    /// before handing it in), so a synthetic value exercises the same tree
    /// math without pulling in a hashing dependency this crate does not have.
    fn leaf(seed: u8) -> Sha256Hash {
        let mut bytes = [0_u8; 32];
        bytes[0] = seed;
        bytes
    }

    #[test]
    fn root_rejects_empty_leaf_set() {
        let engine = MerkleTreeEngine;
        assert_eq!(engine.root(&[]), Err(SealingDomainError::InvalidLeafCount));
    }

    #[test]
    fn root_matches_direct_merkle_tree_construction() {
        let leaves: Vec<Sha256Hash> = (0_u8..5).map(leaf).collect();
        let engine = MerkleTreeEngine;
        let via_engine = engine.root(&leaves).expect("non-empty leaves build a root");
        let via_tree = MerkleTree::new(leaves).build_root();
        assert_eq!(via_engine, via_tree);
    }

    #[test]
    fn verify_leaf_inclusion_accepts_valid_proof() {
        let leaves: Vec<Sha256Hash> = (0_u8..4).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path = tree.proof_path(2);
        assert_eq!(
            verify_leaf_inclusion(leaves[2], 2, &path, root, tree.len() as u64),
            Ok(())
        );
    }

    #[test]
    fn verify_leaf_inclusion_rejects_tampered_leaf() {
        let leaves: Vec<Sha256Hash> = (0_u8..4).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path = tree.proof_path(0);
        let mut bad_leaf = leaves[0];
        bad_leaf[1] ^= 0xff;
        assert_eq!(
            verify_leaf_inclusion(bad_leaf, 0, &path, root, tree.len() as u64),
            Err(SealingDomainError::InvalidProofPath)
        );
    }

    #[test]
    fn verify_leaf_inclusion_rejects_out_of_range_index() {
        let l = leaf(9);
        let tree = MerkleTree::new(vec![l]);
        let root = tree.build_root();
        assert_eq!(
            verify_leaf_inclusion(root, 999, &[], root, 1),
            Err(SealingDomainError::InvalidProofPath)
        );
    }

    #[test]
    fn root_accepts_trailing_duplicate_leaf_and_does_not_collide() {
        let engine = MerkleTreeEngine;
        let three = vec![leaf(1), leaf(2), leaf(3)];
        let four_padded = vec![leaf(1), leaf(2), leaf(3), leaf(3)];
        let root_three = engine.root(&three).expect("3 leaves build a root");
        let root_four = engine
            .root(&four_padded)
            .expect("trailing-duplicate 4th leaf is accepted, not rejected");
        assert_ne!(
            root_three, root_four,
            "RFC 6962's domain-separated k-split must not let a trailing-duplicate \
             leaf collapse a 4-leaf root onto its 3-leaf prefix's root"
        );
    }

    #[test]
    fn root_accepts_leaf_sets_of_either_parity() {
        let engine = MerkleTreeEngine;
        assert!(engine.root(&[leaf(1), leaf(2), leaf(3)]).is_ok());
        assert!(engine.root(&[leaf(1), leaf(2), leaf(3), leaf(4)]).is_ok());
    }

    #[test]
    fn root_accepts_non_trailing_duplicate_leaves() {
        let engine = MerkleTreeEngine;
        assert!(engine.root(&[leaf(1), leaf(1), leaf(2)]).is_ok());
    }

    #[test]
    fn verify_leaf_inclusion_rejects_shortened_proof_for_internal_node() {
        let leaves: Vec<Sha256Hash> = (0_u8..4).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path_for_leaf_two = tree.proof_path(2);
        // A single-sibling proof (depth 1) cannot certify anything in a
        // 4-leaf tree, whose real proof depth is 2.
        let shortened = vec![path_for_leaf_two[0]];
        assert_eq!(
            verify_leaf_inclusion(leaves[0], 0, &shortened, root, tree.len() as u64),
            Err(SealingDomainError::InvalidProofPath)
        );
    }
}
