use audit_chain_domain::{Ed25519Signature, Sha256Hash};

/// A bundle of the three RFC 6962 §2.1 values
/// `audit_chain_domain::MerkleTree::verify_proof` needs beyond the leaf and
/// root themselves. Opaque to [`crate::ports::MerkleVerifier`] — plain
/// public fields, no invariant claimed here either: `verify_proof` itself
/// already fails closed on every malformed combination (see its own doc).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleInclusionProof {
    /// The audit path from the leaf up to the root, leaf-to-root order —
    /// see `audit_chain_domain::MerkleTree::proof_path`.
    pub audit_path: Vec<Sha256Hash>, // data_class: INTERNAL_ONLY
    /// The leaf's position among `leaf_count` leaves.
    pub leaf_index: u64, // data_class: PUBLIC
    /// The total number of leaves the committed tree covered.
    pub leaf_count: u64, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PriorRootClaim {
    First,
    Preceding { root: Sha256Hash },
}

/// Two identity triples travel together here, and [`crate::verify`]
/// deliberately keeps them distinct instead of collapsing them into one:
///
/// - `context_pack` / `context_tenant_partition` / `context_period_id` —
///   the `(pack, tenant_partition, period_id)` this verification call is
///   actually being invoked for. This is the identity every port
///   ([`crate::ports::KeyResolver`], [`crate::ports::RootRegistry`]) is
///   resolved against — the trust anchor.
/// - `record_pack` / `record_tenant_partition` / `record_period_id` — the
///   sealed artifact's OWN self-asserted identity. Nothing in the Ed25519
///   signature bytes or the Merkle proof authenticates `context_*` against
///   `record_*` on its own (the signing payload
///   [`crate::verification_signing_payload`] builds is keyed off
///   `record_*`, precisely so a genuinely valid signature and a genuinely
///   valid Merkle proof for pack A tell you nothing about whether they were
///   submitted under a pack A or pack B verification request) — so
///   [`crate::verify`] checks the two triples against each other
///   explicitly and reports any leg's mismatch as
///   `VerificationFailureReason::PackMismatch`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequest {
    pub context_pack: String,             // data_class: PUBLIC
    pub context_tenant_partition: String, // data_class: INTERNAL_ONLY
    pub context_period_id: String,        // data_class: INTERNAL_ONLY

    pub record_pack: String,             // data_class: PUBLIC
    pub record_tenant_partition: String, // data_class: INTERNAL_ONLY
    pub record_period_id: String,        // data_class: INTERNAL_ONLY

    /// The individual leaf under verification (e.g. one audit event's
    /// commitment hash within the sealed period's tree).
    pub leaf: Sha256Hash, // data_class: INTERNAL_ONLY
    pub proof: MerkleInclusionProof, // data_class: INTERNAL_ONLY
    /// The sealed period's own Merkle root, as attested by the signature
    /// (mirrors `audit_sealing_kernel::SealRecord::merkle_root`).
    pub merkle_root: Sha256Hash, // data_class: INTERNAL_ONLY
    pub prior_root: PriorRootClaim,  // data_class: INTERNAL_ONLY
    pub signature: Ed25519Signature, // data_class: INTERNAL_ONLY

    pub redacted: bool, // data_class: PUBLIC
}
