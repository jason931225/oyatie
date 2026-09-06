use crate::{
    AuthorizationDecisionReceiptV1, BoxCellFuture, CellControlActionV1, CellControlOperationKeyV1,
    CellControlPersistenceAuthorityV1, CellControlReadAuthorityV1, CellControlSubjectV1, Digest32,
    ImmutableAuditStorageRequirementV1, ImmutableEvidenceRefV1, PlacementContractError,
    PlacementIdempotencyKey, PlacementPartitionV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlAuditPolicyV1 {
    pub policy_evidence: ImmutableEvidenceRefV1,
    pub minimum_retention_seconds: u64,
    pub immutable_storage: ImmutableAuditStorageRequirementV1,
    pub policy_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellControlAuditEffectV1 {
    CellCreated {
        cell_resource_digest: Digest32,
    },
    CellUpdated {
        cell_resource_digest: Digest32,
    },
    ReadinessMutated {
        evidence_digest: Digest32,
    },
    DrainStarted {
        manifest_digest: Digest32,
    },
    DrainProofAppended {
        proof_digest: Digest32,
    },
    DrainCompleted {
        completion_digest: Digest32,
    },
    CellDecommissioned {
        completion_digest: Digest32,
    },
    RebalanceCreated {
        job_digest: Digest32,
    },
    RebalanceCancelled {
        job_digest: Digest32,
    },
    OperationCancelled {
        operation: CellControlOperationKeyV1,
    },
    ReconciliationClaimed {
        candidate_digest: Digest32,
    },
    ReconciliationCompleted {
        result_digest: Digest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlAuditRecordPartsV1 {
    pub audit_event_id: String,
    pub subject: CellControlSubjectV1,
    pub action: CellControlActionV1,
    pub actor_digest: Digest32,
    pub authorization: AuthorizationDecisionReceiptV1,
    pub idempotency_key: PlacementIdempotencyKey,
    pub canonical_request_digest: Digest32,
    pub effect: CellControlAuditEffectV1,
    pub result_digest: Digest32,
    pub occurred_at_unix_seconds: u64,
    pub audit_policy: CellControlAuditPolicyV1,
    pub record_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlAuditRecordV1 {
    parts: CellControlAuditRecordPartsV1,
}

impl CellControlAuditRecordV1 {
    pub fn assemble_reconciliation(
        _authority: &crate::CellControlReconciliationPersistenceAuthorityV1,
        _parts: CellControlAuditRecordPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }
    pub fn assemble(
        _authority: &CellControlPersistenceAuthorityV1,
        _parts: CellControlAuditRecordPartsV1,
    ) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CellControlAuditRecordPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlAuditPageTokenV1(Vec<u8>);

impl CellControlAuditPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, PlacementContractError> {
        Err(PlacementContractError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlAuditPageRequestV1 {
    pub partition: PlacementPartitionV1,
    pub page_size: u32,
    pub page_token: Option<CellControlAuditPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellControlAuditPageV1 {
    pub records: Vec<CellControlAuditRecordV1>,
    pub next_page_token: Option<CellControlAuditPageTokenV1>,
}

pub trait CellControlAuditReader: Send + Sync {
    fn read_page<'a>(
        &'a self,
        authority: &'a CellControlReadAuthorityV1,
        request: &'a CellControlAuditPageRequestV1,
    ) -> BoxCellFuture<'a, Result<CellControlAuditPageV1, PlacementContractError>>;
}
