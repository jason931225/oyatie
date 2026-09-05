use crate::{
    BindingAuditRecordV1, BindingDigest32, BindingOperationKey, BindingOperationRevision,
    BindingOperationV1, BindingReconciliationPersistenceAuthorityV1,
    BindingReconciliationReadAuthorityV1, BindingReservationAttemptRevision,
    BindingReservationAttemptV1, BindingStoreError, BoxTenancyFuture,
    SourceReservationReleaseIssuanceRecordV1, SourceReservationReleaseIssuanceRevision, TenantId,
    TransferExecutionLedgerRevision, TransferExecutionLedgerV1,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingReconciliationPartitionKey(String);

impl BindingReconciliationPartitionKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, crate::BindingProofConstructionError> {
        Err(crate::BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReconciliationPageTokenV1(Vec<u8>);

impl BindingReconciliationPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, crate::BindingContractError> {
        Err(crate::BindingContractError::NotImplemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingReconciliationWorkClassV1 {
    UnsettledReservationAttempt,
    ForwardOnlyMigration,
    PendingProjectionConvergence,
    PendingSourceRelease,
    NonterminalOperation,
    PendingServingAuthorityHandoff,
    PendingControlContributionProjection,
    PendingTransferExecutionSettlement,
    PendingWorkSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReconciliationQueryV1 {
    pub partition: BindingReconciliationPartitionKey,
    pub work_class: BindingReconciliationWorkClassV1,
    pub changed_before_unix_seconds: u64,
    pub page_size: u32,
    pub page_token: Option<BindingReconciliationPageTokenV1>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BindingReconciliationSubjectV1 {
    ControlContributionProjection {
        outbox: Box<crate::BindingControlContributionOutboxV1>,
    },
    WorkSnapshot {
        expected_revision: crate::BindingWorkSnapshotRevision,
        progress: Box<crate::BindingWorkSnapshotProgressV1>,
    },
    ReservationAttempt {
        expected_revision: BindingReservationAttemptRevision,
        attempt: Box<BindingReservationAttemptV1>,
    },
    Operation {
        expected_revision: BindingOperationRevision,
        operation: Box<BindingOperationV1>,
    },
    ServingAuthorityHandoff {
        expected_revision: u64,
        handoff: Box<crate::ServingAuthorityHandoffRecordV1>,
    },
    TransferExecutionLedger {
        expected_revision: TransferExecutionLedgerRevision,
        ledger: Box<TransferExecutionLedgerV1>,
    },
    SourceReservationReleaseIssuance {
        expected_revision: SourceReservationReleaseIssuanceRevision,
        issuance: Box<SourceReservationReleaseIssuanceRecordV1>,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingReconciliationCandidateV1 {
    pub tenant_id: TenantId,
    pub operation: BindingOperationKey,
    pub work_class: BindingReconciliationWorkClassV1,
    pub subject: BindingReconciliationSubjectV1,
    pub durable_checkpoint_digest: BindingDigest32,
    pub candidate_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingReconciliationPageV1 {
    pub candidates: Vec<BindingReconciliationCandidateV1>,
    pub next_page_token: Option<BindingReconciliationPageTokenV1>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingReconciliationWorkerId(String);

impl BindingReconciliationWorkerId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, crate::BindingProofConstructionError> {
        Err(crate::BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingReconciliationLeaseV1 {
    pub candidate_digest: BindingDigest32,
    pub worker_id: BindingReconciliationWorkerId,
    pub lease_epoch: u64,
    pub not_before_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub lease_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ClaimBindingReconciliationWriteSetV1 {
    parts: ClaimBindingReconciliationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ClaimBindingReconciliationWriteSetPartsV1 {
    pub authority: BindingReconciliationPersistenceAuthorityV1,
    pub candidate: BindingReconciliationCandidateV1,
    pub expected_lease_epoch: Option<u64>,
    pub lease: BindingReconciliationLeaseV1,
    pub audit_outbox: BindingAuditRecordV1,
}

impl ClaimBindingReconciliationWriteSetV1 {
    pub fn assemble(
        _parts: ClaimBindingReconciliationWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &ClaimBindingReconciliationWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteBindingReconciliationV1 {
    pub candidate_digest: BindingDigest32,
    pub lease_digest: BindingDigest32,
    pub applied_checkpoint_digest: BindingDigest32,
    pub result_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CompleteBindingReconciliationWriteSetV1 {
    parts: CompleteBindingReconciliationWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CompleteBindingReconciliationWriteSetPartsV1 {
    pub authority: BindingReconciliationPersistenceAuthorityV1,
    pub expected_lease: BindingReconciliationLeaseV1,
    pub completion: CompleteBindingReconciliationV1,
    pub audit_outbox: BindingAuditRecordV1,
}

impl CompleteBindingReconciliationWriteSetV1 {
    pub fn assemble(
        _parts: CompleteBindingReconciliationWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &CompleteBindingReconciliationWriteSetPartsV1 {
        &self.parts
    }
}

pub trait BindingReconciliationStore: Send + Sync {
    fn list_candidates<'a>(
        &'a self,
        authority: &'a BindingReconciliationReadAuthorityV1,
        query: &'a BindingReconciliationQueryV1,
    ) -> BoxTenancyFuture<'a, Result<BindingReconciliationPageV1, BindingStoreError>>;

    fn claim<'a>(
        &'a self,
        write_set: &'a ClaimBindingReconciliationWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingReconciliationLeaseV1, BindingStoreError>>;

    fn complete<'a>(
        &'a self,
        write_set: &'a CompleteBindingReconciliationWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<CompleteBindingReconciliationV1, BindingStoreError>>;
}
