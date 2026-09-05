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

impl VerifiedServingAuthorityInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedServingAuthorityInvocationV1 {
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
