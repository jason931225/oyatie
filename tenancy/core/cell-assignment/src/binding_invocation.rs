use crate::{
    BindingActionV1, BindingDigest32, BindingOperationKey, BindingPolicyVersionToken,
    BindingProducerId, BindingProofEnvelopeV1, BindingProofVerificationError, BindingProofVerifier,
    TenantId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingAuthorizationDecisionReceiptV1 {
    pub decision_id: String,
    pub policy_version: BindingPolicyVersionToken,
    pub decision_digest: BindingDigest32,
    pub determining_policy_set_digest: BindingDigest32,
    pub obligations_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingInvocationPayloadV1 {
    pub schema_version: u32,
    pub action: BindingActionV1,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub canonical_request_digest: BindingDigest32,
    pub actor_digest: BindingDigest32,
    pub authorization: BindingAuthorizationDecisionReceiptV1,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedBindingInvocationV1 {
    pub payload: BindingInvocationPayloadV1,
    pub envelope: BindingProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingInvocationExpectation {
    pub action: BindingActionV1,
    pub operation: BindingOperationKey,
    pub tenant_id: TenantId,
    pub canonical_request_digest: BindingDigest32,
    pub expected_producer: BindingProducerId,
    pub expected_audience: BindingProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedBindingInvocation(SignedBindingInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct BindingPersistenceAuthorityV1(SignedBindingInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct BindingReadAuthorityV1(SignedBindingInvocationV1);

impl VerifiedBindingInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedBindingInvocationV1 {
        &self.0
    }

    pub fn into_persistence_authority(
        self,
    ) -> Result<BindingPersistenceAuthorityV1, BindingProofVerificationError> {
        Err(BindingProofVerificationError::NotImplemented)
    }

    pub fn into_read_authority(
        self,
    ) -> Result<BindingReadAuthorityV1, BindingProofVerificationError> {
        Err(BindingProofVerificationError::NotImplemented)
    }
}

impl BindingPersistenceAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedBindingInvocationV1 {
        &self.0
    }

    /// Derives the READ authority this write authority subsumes.
    ///
    /// THE ORDERING, STATED ONCE FOR THIS CRATE. A persistence authority and
    /// its read twin are two newtypes over the SAME verified invocation.
    /// Persistence subsumes read: a party authorized to change a row is
    /// authorized to look at it. The converse never holds, and that is the
    /// whole point of the two types — a read AUTHORITY still cannot reach a
    /// write path by type, because [`BindingReadAuthorityV1`] declares no
    /// method handing back a persistence authority.
    ///
    /// THE SEPARATION IS BETWEEN THE AUTHORITY VALUES AND NOT BETWEEN
    /// INVOCATIONS. This sentence used to say "a read-authorized invocation",
    /// which is a different and false claim: one verified invocation mints
    /// EITHER authority, through `into_*` constructors gated on nothing, so
    /// nothing about the invocation refuses a write. What the two types refuse
    /// is a holder of the read value becoming a writer.
    ///
    /// WHY IT HAD TO BE SAID IN TYPES. Most precondition/authority obligations in
    /// these two crates discharge ONLY through this step: a write set requires a compare-and-set value by
    /// value, and the only surface yielding that value takes the read twin of
    /// the authority the write itself holds. Without this method the caller's
    /// options were to re-verify its own invocation a second time to mint the
    /// other authority — a step no doc in either crate describes — or to invent
    /// the value. [`VerifiedBindingInvocation`] is not `Clone` and both
    /// `into_*` constructors consume `self`, so one invocation mints exactly
    /// one authority; that is what made "a read exists" insufficient, and it is
    /// why `TransferExecutionStore::load_item` saying it "adds no new
    /// authority" was false before this method existed: that read takes
    /// [`BindingReadAuthorityV1`] while `issue_permit`, `publish_permit` and
    /// `record_outcome` take [`BindingPersistenceAuthorityV1`].
    ///
    /// THIS DOES NOT PUT READS BEHIND WRITE AUTHORITY. A read-only caller still
    /// obtains [`BindingReadAuthorityV1`] directly from
    /// [`VerifiedBindingInvocation::into_read_authority`]. This says only that a
    /// writer may read, in one direction, and it is not a licence for a read
    /// surface to DEMAND a persistence authority.
    pub fn read_authority(&self) -> Result<BindingReadAuthorityV1, BindingProofVerificationError> {
        Err(BindingProofVerificationError::NotImplemented)
    }
}

impl BindingReadAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedBindingInvocationV1 {
        &self.0
    }
}

pub fn verify_binding_invocation(
    _verifier: &dyn BindingProofVerifier,
    _signed: SignedBindingInvocationV1,
    _expectation: &BindingInvocationExpectation,
) -> Result<VerifiedBindingInvocation, BindingProofVerificationError> {
    Err(BindingProofVerificationError::NotImplemented)
}
