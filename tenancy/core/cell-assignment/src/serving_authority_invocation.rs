use crate::{BindingDigest32, BindingProofEnvelopeV1, ServingAuthorityInstanceV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServingAuthorityActionV1 {
    Install,
    Freeze,
    Renew,
    Read,
    Publish,
    ListPendingIssuances,
    ClaimPendingIssuance,
    CompletePendingIssuance,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityInvocationPayloadV1 {
    pub schema_version: u32,
    pub instance: ServingAuthorityInstanceV1,
    pub action: ServingAuthorityActionV1,
    pub actor_digest: BindingDigest32,
    pub authorization: crate::BindingAuthorizationDecisionReceiptV1,
    pub canonical_request_digest: BindingDigest32,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedServingAuthorityInvocationV1 {
    pub payload: ServingAuthorityInvocationPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityInvocationExpectationV1 {
    pub instance: ServingAuthorityInstanceV1,
    pub action: ServingAuthorityActionV1,
    pub canonical_request_digest: BindingDigest32,
    pub expected_producer: crate::BindingProducerId,
    pub expected_audience: crate::BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedServingAuthorityInvocation(SignedServingAuthorityInvocationV1);

/// Write authority for the serving-authority store, minted from a verified
/// invocation. Distinct from [`ServingAuthorityReadAuthorityV1`] so that a read
/// AUTHORITY cannot reach a write path by type: that newtype declares no method
/// handing back this one, so a holder of the read value cannot become a writer.
///
/// WHAT THIS DOES NOT DO, stated because the sentence above used to say it did.
/// It does not stand in place of a per-call-site check of the invocation's
/// [`ServingAuthorityActionV1`]. [`VerifiedServingAuthorityInvocation`] is ONE
/// type whatever action its signed payload names, and both
/// [`VerifiedServingAuthorityInvocation::into_persistence_authority`] and
/// [`VerifiedServingAuthorityInvocation::into_read_authority`] are available on
/// it unconditionally, so an invocation carrying
/// [`ServingAuthorityActionV1::Read`] converts to this write authority and the
/// compiler raises nothing. The earlier wording asserted that the type
/// separation made the action check unnecessary; the types make no such
/// separation, and asserting one that does not exist is worse than stating the
/// obligation, because it tells an implementer the check is already done.
///
/// THE OBLIGATION THAT REMAINS. Which [`ServingAuthorityActionV1`] each of the
/// two conversions may be performed for is a rule NO SURFACE IN THESE CRATES
/// STATES, and until it is stated an adapter minting this authority must check
/// the action at the call site. That is a deployment obligation on every
/// composition root, not a property of any signature here.
#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityPersistenceAuthorityV1(SignedServingAuthorityInvocationV1);

/// Read authority for the serving-authority store. See
/// [`ServingAuthorityPersistenceAuthorityV1`] for why the two are separate
/// types.
#[derive(Debug, Eq, PartialEq)]
pub struct ServingAuthorityReadAuthorityV1(SignedServingAuthorityInvocationV1);

impl VerifiedServingAuthorityInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedServingAuthorityInvocationV1 {
        &self.0
    }

    pub fn into_persistence_authority(
        self,
    ) -> Result<ServingAuthorityPersistenceAuthorityV1, crate::BindingProofVerificationError> {
        Err(crate::BindingProofVerificationError::NotImplemented)
    }

    pub fn into_read_authority(
        self,
    ) -> Result<ServingAuthorityReadAuthorityV1, crate::BindingProofVerificationError> {
        Err(crate::BindingProofVerificationError::NotImplemented)
    }
}

impl ServingAuthorityPersistenceAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedServingAuthorityInvocationV1 {
        &self.0
    }

    /// Derives the READ authority this write authority subsumes. The ordering
    /// and the reason it had to be said in types are stated once, on
    /// [`crate::BindingPersistenceAuthorityV1::read_authority`]; this is the
    /// same rule on this axis.
    pub fn read_authority(
        &self,
    ) -> Result<ServingAuthorityReadAuthorityV1, crate::BindingProofVerificationError> {
        Err(crate::BindingProofVerificationError::NotImplemented)
    }
}

impl ServingAuthorityReadAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedServingAuthorityInvocationV1 {
        &self.0
    }
}

pub fn verify_serving_authority_invocation(
    _verifier: &dyn crate::BindingProofVerifier,
    _signed: SignedServingAuthorityInvocationV1,
    _expectation: &ServingAuthorityInvocationExpectationV1,
) -> Result<VerifiedServingAuthorityInvocation, crate::BindingProofVerificationError> {
    Err(crate::BindingProofVerificationError::NotImplemented)
}
