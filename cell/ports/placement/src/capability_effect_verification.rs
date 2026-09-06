//! The only mints for capability-effect evidence.
//!
//! Every wrapper below has a private field and this module holds the sole
//! constructor, so no `From<Signed>`, deserialization or trait implementation
//! outside this crate can produce one. A signature alone establishes no role:
//! each verifier also checks domain, producer, audience, expected identity and
//! the digests it is given.

use crate::{
    CapabilityAdapterAcceptancePayloadV1, CapabilityEffectErrorV1, CapabilityEffectExpectationV1,
    CapabilityReceiptQueryV1, CellProofVerifier, CommittedLocalEffectReceiptClaimV1, ProducerId,
    SignedCapabilityAdapterAcceptanceV1, SignedCapabilityEffectGrantV1,
    SignedCapabilityReceiptRecoveryV1,
};

/// Deployment qualification of one adapter at one transaction domain.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCapabilityAdapterAcceptanceV1(SignedCapabilityAdapterAcceptanceV1);

impl VerifiedCapabilityAdapterAcceptanceV1 {
    #[must_use]
    pub fn signed(&self) -> &SignedCapabilityAdapterAcceptanceV1 {
        &self.0
    }
}

pub fn verify_capability_adapter_acceptance(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCapabilityAdapterAcceptanceV1,
    _expected: &CapabilityAdapterAcceptancePayloadV1,
    _expected_producer: &ProducerId,
    _expected_audience: &ProducerId,
    _now_unix_seconds: u64,
) -> Result<VerifiedCapabilityAdapterAcceptanceV1, CapabilityEffectErrorV1> {
    Err(CapabilityEffectErrorV1::NotImplemented)
}

/// Owner authority to perform one action, at one scope, on one effect.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCapabilityEffectGrantV1(SignedCapabilityEffectGrantV1);

impl VerifiedCapabilityEffectGrantV1 {
    #[must_use]
    pub fn signed(&self) -> &SignedCapabilityEffectGrantV1 {
        &self.0
    }
}

/// Verifies one signed grant against the caller's expectation and the
/// adapter's acceptance.
///
/// `expected.expected_authority_context_digest` is compared against the digest
/// of the CANONICAL ENCODING of `signed.payload.context`, discriminant
/// included. Comparing only the variant tag is not sufficient, and is the
/// exact hole this comparison closes: a Preparation-shaped wire message that
/// carries fabricated installed fields, or a fabricated zero epoch, still
/// presents a Preparation tag and would pass a tag-only check while its bytes
/// say something else. The variant correspondence is checked too, but it is
/// the weaker of the two checks, not a substitute for it.
///
/// A Preparation context authorizes Prepare, PreparationCleanup, and Transfer
/// into non-serving staged data when the acceptance separately admits it, and
/// nothing else. It can never satisfy Activate, Write, Fence or Release.
pub fn verify_capability_effect_grant(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCapabilityEffectGrantV1,
    _expected: &CapabilityEffectExpectationV1,
    _acceptance: &VerifiedCapabilityAdapterAcceptanceV1,
) -> Result<VerifiedCapabilityEffectGrantV1, CapabilityEffectErrorV1> {
    Err(CapabilityEffectErrorV1::NotImplemented)
}

/// Read-only authority to retrieve one already-committed result.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCapabilityReceiptRecoveryV1(SignedCapabilityReceiptRecoveryV1);

impl VerifiedCapabilityReceiptRecoveryV1 {
    #[must_use]
    pub fn signed(&self) -> &SignedCapabilityReceiptRecoveryV1 {
        &self.0
    }
}

pub fn verify_capability_receipt_recovery(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCapabilityReceiptRecoveryV1,
    _expected_query: &CapabilityReceiptQueryV1,
    _expected_producer: &ProducerId,
    _expected_audience: &ProducerId,
    _now_unix_seconds: u64,
) -> Result<VerifiedCapabilityReceiptRecoveryV1, CapabilityEffectErrorV1> {
    Err(CapabilityEffectErrorV1::NotImplemented)
}

/// Evidence that the durable payload is the one the observer actually reread
/// from the committed store. The publisher accepts nothing else.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCommittedLocalEffectReceiptV1(CommittedLocalEffectReceiptClaimV1);

impl VerifiedCommittedLocalEffectReceiptV1 {
    #[must_use]
    pub fn claim(&self) -> &CommittedLocalEffectReceiptClaimV1 {
        &self.0
    }
}

pub fn verify_committed_local_effect_receipt(
    _verifier: &dyn CellProofVerifier,
    _claim: CommittedLocalEffectReceiptClaimV1,
    _expected_query: &CapabilityReceiptQueryV1,
    _expected_storage_observer: &ProducerId,
    _expected_audience: &ProducerId,
    _acceptance: &VerifiedCapabilityAdapterAcceptanceV1,
    _now_unix_seconds: u64,
) -> Result<VerifiedCommittedLocalEffectReceiptV1, CapabilityEffectErrorV1> {
    Err(CapabilityEffectErrorV1::NotImplemented)
}
