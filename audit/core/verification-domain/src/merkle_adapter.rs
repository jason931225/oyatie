//! This is pure math (no I/O), so — unlike [`crate::ports::KeyResolver`] and
//! [`crate::ports::RootRegistry`], which need a real read path this crate
//! cannot provide — this crate supplies its own concrete adapter directly,
//! the same way `audit_sealing_domain::merkle_engine::MerkleTreeEngine`
//! supplies a concrete `MerkleEngine` for sealing.

use audit_chain_domain::{MerkleTree, Sha256Hash};

use crate::ports::MerkleVerifier;
use crate::request::MerkleInclusionProof;

/// A `leaf_count` above this bound is rejected before ever reaching
/// `audit_chain_domain::MerkleTree::verify_proof` — see [`ChainMerkleVerifier::verify`]'s
/// doc for why. `2^40` (a little over one trillion) is already many orders
/// of magnitude beyond any tree this system could plausibly ever build (a
/// trillion 32-byte leaves alone is 32 terabytes of raw leaf data before
/// even hashing).
const MAX_PLAUSIBLE_LEAF_COUNT: u64 = 1 << 40;

/// Concrete, stateless [`MerkleVerifier`] over
/// `audit_chain_domain::MerkleTree::verify_proof`.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChainMerkleVerifier;

impl MerkleVerifier for ChainMerkleVerifier {
    /// Delegates to `MerkleTree::verify_proof`, which already fails closed
    /// on an out-of-range index, a wrong-length path, and a zero leaf count
    /// (see its own doc). This adapter refuses to widen two further things,
    /// rejecting rather than truncating or saturating:
    ///
    /// - `MerkleInclusionProof`'s `leaf_index` / `leaf_count` are `u64` but
    ///   `verify_proof` takes `usize`. On a 32-bit target, a `u64` value that
    ///   does not fit in `usize` returns `false` here — a value that cannot
    ///   even be represented can never be verified as in-range. (On the
    ///   64-bit targets this crate ships on, `usize` and `u64` have the same
    ///   range, so this conversion never fails there; it is for portability.)
    /// - a `leaf_count` near `u64::MAX` clears that conversion on a 64-bit
    ///   target, so `MAX_PLAUSIBLE_LEAF_COUNT` bounds it here before
    ///   `MerkleTree`'s split-point arithmetic ever sees it. That arithmetic
    ///   is overflow-safe at the source today; this bound is a ceiling kept
    ///   in front of it, not the only thing standing between untrusted input
    ///   and a panic.
    fn verify(&self, leaf: &Sha256Hash, proof: &MerkleInclusionProof, root: &Sha256Hash) -> bool {
        if proof.leaf_count > MAX_PLAUSIBLE_LEAF_COUNT {
            return false;
        }
        let Ok(leaf_index) = usize::try_from(proof.leaf_index) else {
            return false;
        };
        let Ok(leaf_count) = usize::try_from(proof.leaf_count) else {
            return false;
        };
        MerkleTree::verify_proof(*leaf, leaf_index, &proof.audit_path, *root, leaf_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(seed: u8) -> Sha256Hash {
        let mut bytes = [0_u8; 32];
        bytes[0] = seed;
        bytes
    }

    #[test]
    fn accepts_a_real_inclusion_proof() {
        let leaves: Vec<Sha256Hash> = (0_u8..5).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path = tree.proof_path(2);
        let proof = MerkleInclusionProof {
            audit_path: path,
            leaf_index: 2,
            leaf_count: tree.len() as u64,
        };
        assert!(ChainMerkleVerifier.verify(&leaves[2], &proof, &root));
    }

    #[test]
    fn rejects_a_tampered_leaf() {
        let leaves: Vec<Sha256Hash> = (0_u8..5).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path = tree.proof_path(2);
        let proof = MerkleInclusionProof {
            audit_path: path,
            leaf_index: 2,
            leaf_count: tree.len() as u64,
        };
        let mut bad_leaf = leaves[2];
        bad_leaf[0] ^= 0xff;
        assert!(!ChainMerkleVerifier.verify(&bad_leaf, &proof, &root));
    }

    #[test]
    fn rejects_an_absurd_leaf_count_instead_of_ever_passing() {
        let leaves: Vec<Sha256Hash> = (0_u8..3).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path = tree.proof_path(0);
        let proof = MerkleInclusionProof {
            audit_path: path,
            leaf_index: 0,
            leaf_count: 1_000_000_000_000,
        };
        assert!(!ChainMerkleVerifier.verify(&leaves[0], &proof, &root));
    }

    #[test]
    fn rejects_a_leaf_count_near_u64_max_without_panicking() {
        let leaves: Vec<Sha256Hash> = (0_u8..3).map(leaf).collect();
        let tree = MerkleTree::new(leaves.clone());
        let root = tree.build_root();
        let path = tree.proof_path(0);
        let proof = MerkleInclusionProof {
            audit_path: path,
            leaf_index: 0,
            leaf_count: u64::MAX,
        };
        let outcome =
            std::panic::catch_unwind(|| ChainMerkleVerifier.verify(&leaves[0], &proof, &root));
        assert!(
            matches!(outcome, Ok(false)),
            "a leaf_count near u64::MAX must be rejected cleanly, never panic"
        );
    }
}
