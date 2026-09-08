use crate::{
    AuthorizationDecisionReceiptV1, CellControlReconciliationPartitionKey,
    CellControlReconciliationWorkClassV1, CellId, CellProofEnvelopeV1, CellProofVerifier,
    CellReconciliationPartitionKey, CellReconciliationWorkClassV1, Digest32, ProducerId,
    ProofVerificationError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReconciliationActionV1 {
    ClaimRebalanceJob,
    ReadRebalanceJob,
    PersistRebalanceEvaluation,
    IssueRebalanceAction,
    PublishRebalanceAction,
    AdvanceRebalanceAction,
    CloseRebalanceAction,
    ObserveRebalanceResult,
    ListPlacementCandidates,
    ClaimPlacementCandidate,
    CompletePlacementCandidate,
    ListCellControlCandidates,
    ClaimCellControlCandidate,
    CompleteCellControlCandidate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationScopeV1 {
    RebalanceJob {
        partition: crate::RebalanceJobPartitionV1,
        job_id: crate::RebalanceJobId,
        action: Option<crate::PlacementBusinessActionKeyV1>,
    },
    Placement {
        partition: CellReconciliationPartitionKey,
        work_class: CellReconciliationWorkClassV1,
        candidate_digest: Option<Digest32>,
        cell_id: Option<CellId>,
    },
    CellControl {
        partition: CellControlReconciliationPartitionKey,
        work_class: CellControlReconciliationWorkClassV1,
        candidate_digest: Option<Digest32>,
        cell_id: Option<CellId>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationInvocationPayloadV1 {
    pub schema_version: u32,
    pub action: ReconciliationActionV1,
    pub scope: ReconciliationScopeV1,
    pub worker_id: String,
    pub actor_digest: Digest32,
    pub authorization: AuthorizationDecisionReceiptV1,
    pub canonical_request_digest: Digest32,
    pub deadline_unix_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedReconciliationInvocationV1 {
    pub payload: ReconciliationInvocationPayloadV1,
    pub envelope: CellProofEnvelopeV1,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationInvocationExpectationV1 {
    pub action: ReconciliationActionV1,
    pub scope: ReconciliationScopeV1,
    pub expected_producer: ProducerId,
    pub expected_audience: ProducerId,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedReconciliationInvocation(SignedReconciliationInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct PlacementReconciliationReadAuthorityV1(SignedReconciliationInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct PlacementReconciliationPersistenceAuthorityV1(SignedReconciliationInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct CellControlReconciliationReadAuthorityV1(SignedReconciliationInvocationV1);

#[derive(Debug, Eq, PartialEq)]
pub struct CellControlReconciliationPersistenceAuthorityV1(SignedReconciliationInvocationV1);

impl VerifiedReconciliationInvocation {
    #[must_use]
    pub fn signed(&self) -> &SignedReconciliationInvocationV1 {
        &self.0
    }

    pub fn into_placement_read(
        self,
    ) -> Result<PlacementReconciliationReadAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }

    pub fn into_placement_persistence(
        self,
    ) -> Result<PlacementReconciliationPersistenceAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }

    pub fn into_cell_control_read(
        self,
    ) -> Result<CellControlReconciliationReadAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }

    pub fn into_cell_control_persistence(
        self,
    ) -> Result<CellControlReconciliationPersistenceAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }
}

pub fn verify_reconciliation_invocation(
    _verifier: &dyn CellProofVerifier,
    _signed: SignedReconciliationInvocationV1,
    _expectation: &ReconciliationInvocationExpectationV1,
) -> Result<VerifiedReconciliationInvocation, ProofVerificationError> {
    Err(ProofVerificationError::NotImplemented)
}

impl PlacementReconciliationPersistenceAuthorityV1 {
    /// Derives the READ authority this write authority subsumes. The ordering
    /// and the reason it had to be said in types are stated once, on
    /// [`crate::PlacementPersistenceAuthorityV1::read_authority`]; this is the same rule
    /// on this axis.
    pub fn read_authority(
        &self,
    ) -> Result<PlacementReconciliationReadAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }
}

impl CellControlReconciliationPersistenceAuthorityV1 {
    /// Derives the READ authority this write authority subsumes. The ordering
    /// and the reason it had to be said in types are stated once, on
    /// [`crate::PlacementPersistenceAuthorityV1::read_authority`]; this is the same rule
    /// on this axis.
    pub fn read_authority(
        &self,
    ) -> Result<CellControlReconciliationReadAuthorityV1, ProofVerificationError> {
        Err(ProofVerificationError::NotImplemented)
    }
}
