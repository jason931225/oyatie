use crate::{
    BindingAuthorizationDecisionReceiptV1, BindingDigest32, BindingOperationKey, BindingProducerId,
    BindingProofEnvelopeV1, BindingProofVerificationError, BindingProofVerifier,
    BindingReconciliationPartitionKey, BindingReconciliationWorkClassV1, TenantId,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingReconciliationActionV1 {
    ListCandidates,
    ClaimCandidate,
    CompleteCandidate,
    ReadCandidateWork,
    CheckpointCandidateWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReconciliationScopeV1 {
    pub partition: BindingReconciliationPartitionKey,
    pub work_class: BindingReconciliationWorkClassV1,
    pub candidate_digest: Option<BindingDigest32>,
    pub tenant_id: Option<TenantId>,
    pub operation: Option<BindingOperationKey>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReconciliationInvocationPayloadV1 {
    pub schema_version: u32,
    pub action: BindingReconciliationActionV1,
    pub scope: BindingReconciliationScopeV1,
    pub worker_id: String,
    pub actor_digest: BindingDigest32,
    pub authorization: BindingAuthorizationDecisionReceiptV1,
    pub canonical_request_digest: BindingDigest32,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedBindingReconciliationInvocationV1 {
    pub payload: BindingReconciliationInvocationPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReconciliationInvocationExpectationV1 {
    pub action: BindingReconciliationActionV1,
    pub scope: BindingReconciliationScopeV1,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedBindingReconciliationInvocation(SignedBindingReconciliationInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct BindingReconciliationReadAuthorityV1(SignedBindingReconciliationInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct BindingReconciliationPersistenceAuthorityV1(SignedBindingReconciliationInvocationV1);

impl VerifiedBindingReconciliationInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedBindingReconciliationInvocationV1 {
        &self.0
    }

    pub fn into_read_authority(
        self,
    ) -> Result<BindingReconciliationReadAuthorityV1, BindingProofVerificationError> {
        Err(BindingProofVerificationError::NotImplemented)
    }

    pub fn into_persistence_authority(
        self,
    ) -> Result<BindingReconciliationPersistenceAuthorityV1, BindingProofVerificationError> {
        Err(BindingProofVerificationError::NotImplemented)
    }
}

pub fn verify_binding_reconciliation_invocation(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedBindingReconciliationInvocationV1,
    _expectation: &BindingReconciliationInvocationExpectationV1,
) -> Result<VerifiedBindingReconciliationInvocation, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}

impl BindingReconciliationPersistenceAuthorityV1 {
    /// Derives the READ authority this write authority subsumes. The ordering
    /// and the reason it had to be said in types are stated once, on
    /// [`crate::BindingPersistenceAuthorityV1::read_authority`]; this is the
    /// same rule on this axis.
    pub fn read_authority(
        &self,
    ) -> Result<BindingReconciliationReadAuthorityV1, BindingProofVerificationError> {
        Err(BindingProofVerificationError::NotImplemented)
    }
}
