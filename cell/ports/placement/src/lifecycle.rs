use crate::{
    AuthorizationDecisionReceiptV1, CellId, CellProofEnvelopeV1, CellProofVerifier, Digest32,
    PlacementPartitionV1, ProducerId, ProofConstructionError, ProofVerificationError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellLifecycleStateV1 {
    Registered,
    Provisioning,
    Serving,
    Draining,
    Drained,
    Decommissioned,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellReadinessRingV1 {
    Candidate,
    Canary,
    Production,
}

/// The revision of [`CellLifecycleRecordV1`], which is carried inside the cell
/// resource row: a compare-and-set stated at this type is a compare-and-set on
/// [`crate::CellResourceV1`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellLifecycleRevision(pub u64);

/// The admission epoch on [`CellLifecycleRecordV1`], which is carried inside
/// the cell resource row: a compare-and-set stated at this type is a
/// compare-and-set on [`crate::CellResourceV1`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellAdmissionEpoch(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DrainTermV1(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellLifecycleRecordV1 {
    pub cell_id: CellId,
    pub lifecycle: CellLifecycleStateV1,
    pub readiness: CellReadinessRingV1,
    pub revision: CellLifecycleRevision,
    pub admission_epoch: CellAdmissionEpoch,
    pub drain_term: Option<DrainTermV1>,
    pub transition_at_unix_seconds: u64,
    pub record_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellLifecycleTransitionKindV1 {
    Register,
    BeginProvisioning,
    EnterServing,
    PromoteReadiness,
    DemoteReadiness,
    BeginDrain,
    CompleteDrain,
    Decommission,
    UpdateWithoutLifecycleChange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellLifecycleTransitionPartsV1 {
    pub previous: Option<CellLifecycleRecordV1>,
    pub next: CellLifecycleRecordV1,
    pub kind: CellLifecycleTransitionKindV1,
    pub evidence_digest: Digest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellLifecycleTransitionV1(CellLifecycleTransitionPartsV1);

impl VerifiedCellLifecycleTransitionV1 {
    #[must_use]
    pub fn parts(&self) -> &CellLifecycleTransitionPartsV1 {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellLifecycleTransitionErrorV1 {
    CellMismatch,
    InvalidInitialState,
    InvalidLifecycleEdge,
    InvalidReadinessEdge,
    RevisionDidNotAdvance,
    AdmissionEpochDidNotAdvance,
    DrainTermMismatch,
    DecommissionedIdentityCannotReturn,
    EvidenceMismatch,
    RecordDigestMismatch,
    NotImplemented,
}

pub fn verify_cell_lifecycle_transition(
    _parts: CellLifecycleTransitionPartsV1,
) -> Result<VerifiedCellLifecycleTransitionV1, CellLifecycleTransitionErrorV1> {
    Err(CellLifecycleTransitionErrorV1::NotImplemented)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellAdmissionTermV1 {
    pub cell_id: CellId,
    pub lifecycle_revision: CellLifecycleRevision,
    pub admission_epoch: CellAdmissionEpoch,
    pub cell_spec_revision: crate::CellSpecRevision,
    pub cell_resource_digest: Digest32,
    pub capacity_revision: crate::CellCapacityRevision,
    pub capacity_record_digest: Digest32,
    pub lifecycle: CellLifecycleStateV1,
    pub readiness: CellReadinessRingV1,
    pub term_digest: Digest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CellControlActionV1 {
    Create,
    Get,
    List,
    Update,
    CreateRebalanceJob,
    GetRebalanceJob,
    CancelRebalanceJob,
    GetControlOperation,
    CancelControlOperation,
    Promote,
    Demote,
    BeginDrain,
    AppendDrainProof,
    CompleteDrain,
    Decommission,
    RepairControlOperation,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CellControlSubjectV1 {
    Cell(CellId),
    Partition(PlacementPartitionV1),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellControlOperationId(String);

impl CellControlOperationId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
        Err(ProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlInvocationPayloadV1 {
    pub schema_version: u32,
    pub action: CellControlActionV1,
    pub subject: CellControlSubjectV1,
    pub operation_id: CellControlOperationId,
    pub canonical_request_digest: Digest32,
    pub actor_digest: Digest32,
    pub authorization: AuthorizationDecisionReceiptV1,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedCellControlInvocationV1 {
    pub payload: CellControlInvocationPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlInvocationExpectationV1 {
    pub action: CellControlActionV1,
    pub subject: CellControlSubjectV1,
    pub operation_id: CellControlOperationId,
    pub canonical_request_digest: Digest32,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedCellControlInvocation(SignedCellControlInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct CellControlPersistenceAuthorityV1(SignedCellControlInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct CellControlReadAuthorityV1(SignedCellControlInvocationV1);

impl VerifiedCellControlInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedCellControlInvocationV1 {
        &self.0
    }

    pub fn into_persistence_authority(
        self,
    ) -> Result<CellControlPersistenceAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }

    pub fn into_read_authority(self) -> Result<CellControlReadAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }
}

impl CellControlPersistenceAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedCellControlInvocationV1 {
        &self.0
    }
}

impl CellControlReadAuthorityV1 {
    #[must_use]
    pub fn invocation(&self) -> &SignedCellControlInvocationV1 {
        &self.0
    }
}

pub fn verify_cell_control_invocation(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedCellControlInvocationV1,
    _expectation: &CellControlInvocationExpectationV1,
) -> Result<VerifiedCellControlInvocation, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}
