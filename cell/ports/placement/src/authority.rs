use crate::{
    CellProofEnvelopeV1, CellProofVerifier, Digest32, PlacementOperationKey,
    ProofConstructionError, ProofVerificationError, TenantId,
};

pub const PLACEMENT_INVOCATION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PlacementActionV1 {
    ReadRebalanceRequirements,
    SelectAndReserve,
    ArmReservation,
    FinalizeReservationCommitPermit,
    ScheduleMovement,
    ApplyBindingOutcome,
    ApplySourceReservationRelease,
    GetOperation,
    CancelOperation,
    RepairOperation,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PolicyVersionToken(String);

impl PolicyVersionToken {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
        Err(ProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationDecisionReceiptV1 {
    pub decision_id: String,
    pub policy_version: PolicyVersionToken,
    pub decision_digest: Digest32,
    pub determining_policy_set_digest: Digest32,
    pub obligations_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementInvocationPayloadV1 {
    pub authorization_evidence: Box<crate::SignedPlacementPolicyDecisionV1>,
    pub schema_version: u32,
    pub action: PlacementActionV1,
    pub tenant_id: TenantId,
    pub operation: PlacementOperationKey,
    pub canonical_request_digest: Digest32,
    pub actor_digest: Digest32,
    pub authorization: AuthorizationDecisionReceiptV1,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedPlacementInvocationV1 {
    pub payload: PlacementInvocationPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementInvocationExpectation {
    pub authorization_request: crate::PlacementAuthorizationRequestV1,
    pub authorization_trust: crate::PlacementAuthorizationTrustV1,
    pub action: PlacementActionV1,
    pub tenant_id: TenantId,
    pub operation: PlacementOperationKey,
    pub canonical_request_digest: Digest32,
    pub expected_producer: crate::ProducerId,
    pub expected_audience: crate::ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlacementInvocationVerificationError {
    NotImplemented,
    Proof(ProofVerificationError),
    UnsupportedSchemaVersion {
        observed: u32,
        supported: crate::ProtocolVersionRangeV1,
    },
    RelationMismatch,
    Expired,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedPlacementInvocation {
    signed: SignedPlacementInvocationV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PlacementPersistenceAuthorityV1(SignedPlacementInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct PlacementReadAuthorityV1(SignedPlacementInvocationV1);

impl VerifiedPlacementInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedPlacementInvocationV1 {
        &self.signed
    }

    pub fn into_persistence_authority(
        self,
    ) -> Result<PlacementPersistenceAuthorityV1, PlacementInvocationVerificationError> {
        Err(PlacementInvocationVerificationError::NotImplemented)
    }

    pub fn into_read_authority(
        self,
    ) -> Result<PlacementReadAuthorityV1, PlacementInvocationVerificationError> {
        Err(PlacementInvocationVerificationError::NotImplemented)
    }
}

impl PlacementPersistenceAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedPlacementInvocationV1 {
        &self.0
    }

    /// Derives the READ authority this write authority subsumes.
    ///
    /// THE ORDERING, STATED ONCE. A persistence authority and its read twin are
    /// two newtypes over the SAME verified invocation. Persistence subsumes
    /// read: a party authorized to change a row is authorized to look at it.
    /// The converse never holds, and that is the whole point of the two types —
    /// a read-authorized invocation still cannot reach a write path by type.
    ///
    /// WHY IT HAD TO BE SAID IN TYPES. Sixty-five of the ninety
    /// precondition/authority obligations in these two crates discharge ONLY
    /// through this step: a write set requires a compare-and-set value by
    /// value, and the only surface yielding that value takes the read twin of
    /// the authority the write itself holds. Without this method the caller's
    /// options were to re-verify its own invocation a second time to mint the
    /// other authority — a step no doc in either crate describes — or to invent
    /// the value. `VerifiedPlacementInvocation` is not `Clone` and both
    /// `into_*` constructors consume `self`, so one invocation mints exactly
    /// one authority; that is what made "a read exists" insufficient.
    ///
    /// THIS DOES NOT PUT READS BEHIND WRITE AUTHORITY. A read-only caller still
    /// obtains [`PlacementReadAuthorityV1`] directly from
    /// [`VerifiedPlacementInvocation::into_read_authority`]. This says only that
    /// a writer may read, in one direction, and it is not a licence for a read
    /// surface to DEMAND a persistence authority.
    pub fn read_authority(
        &self,
    ) -> Result<PlacementReadAuthorityV1, PlacementInvocationVerificationError> {
        Err(PlacementInvocationVerificationError::NotImplemented)
    }
}

impl PlacementReadAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedPlacementInvocationV1 {
        &self.0
    }
}

pub fn verify_placement_invocation(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedPlacementInvocationV1,
    _expectation: &PlacementInvocationExpectation,
) -> Result<VerifiedPlacementInvocation, PlacementInvocationVerificationError> {
    Err(PlacementInvocationVerificationError::NotImplemented)
}
