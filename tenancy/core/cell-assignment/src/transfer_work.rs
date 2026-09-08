use crate::{
    BindingDigest32, BindingOperationKey, BindingProofConstructionError, CapabilityParticipantId,
    ResidencyTransferEffectV1, SignedTransferExecutionOutcomeV1, SignedTransferExecutionPermitV1,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferWorkPageTokenV1(Vec<u8>);

impl TransferWorkPageTokenV1 {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, BindingProofConstructionError> {
        Err(BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferEffectSnapshotV1 {
    pub operation: BindingOperationKey,
    pub object_authority: String,
    pub repository_id: String,
    pub object_id: String,
    pub object_version: u64,
    pub content_digest: BindingDigest32,
    pub ordered_effect_root_digest: BindingDigest32,
    pub effect_count: u64,
    pub snapshot_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TransferEffectSetV1 {
    snapshot: TransferEffectSnapshotV1,
    sealed_progress: crate::BindingWorkSnapshotProgressV1,
}

impl TransferEffectSetV1 {
    pub fn assemble(
        _snapshot: TransferEffectSnapshotV1,
        _sealed_progress: crate::BindingWorkSnapshotProgressV1,
    ) -> Result<Self, crate::BindingStoreError> {
        Err(crate::BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn snapshot(&self) -> &TransferEffectSnapshotV1 {
        &self.snapshot
    }

    #[must_use]
    pub fn sealed_progress(&self) -> &crate::BindingWorkSnapshotProgressV1 {
        &self.sealed_progress
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferEffectPageRequestV1 {
    pub snapshot: TransferEffectSnapshotV1,
    pub manifest_digest: BindingDigest32,
    pub start_ordinal: u64,
    pub page_size: u32,
    pub page_token: Option<TransferWorkPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferEffectPageV1 {
    pub snapshot: TransferEffectSnapshotV1,
    pub start_ordinal: u64,
    pub effects: Vec<ResidencyTransferEffectV1>,
    pub next_ordinal: u64,
    pub next_page_token: Option<TransferWorkPageTokenV1>,
    pub page_digest: BindingDigest32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransferExecutionItemRevision(pub u64);

/// Where one transfer-execution item row stands.
///
/// The store DERIVES this value; a caller proposing `next_item` restates it and
/// the store refuses a disagreeing proposal, under the ownership rule at the
/// head of `tenancy/binding/v1/serving_authority.proto`. The `next_item` doc on
/// [`crate::TransferExecutionStore::issue_permit`] delegates to that rule
/// without saying `disposition` is one of the values it covers; it is.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TransferExecutionItemDispositionV1 {
    /// NO WRITE IN THESE CRATES CAN PRODUCE AN ITEM ROW IN THIS STATE, and the
    /// variant is kept rather than deleted because it is a live wire value.
    ///
    /// [`crate::TransferExecutionStore::issue_permit`] is the only write
    /// carrying a `next_item` for an item that need not already exist, and its
    /// [`crate::TransferExecutionItemPreconditionV1`] has had an `Absent` arm
    /// since the seed, so the transition it performs is `Absent ->
    /// `[`Self::PermitIssued`]. There is no earlier write to leave a row here.
    ///
    /// A store reading a durable row that carries this disposition is reading
    /// something no conforming write produced: that is
    /// [`crate::BindingStoreError::Conflict`] against the item precondition, not
    /// a state to advance from. It is stated here and at
    /// `TRANSFER_EXECUTION_ITEM_DISPOSITION_V1_PENDING_PERMIT` on the wire
    /// because an adapter author reads the schema and sees a legal value at the
    /// LOWEST non-zero tag -- the one a naive "first state after unspecified"
    /// implementation reaches for -- with nothing saying it is unreachable.
    ///
    /// Not `reserved` on the wire: the tag is in use by this variant, and
    /// reserving a number a live value occupies is a different and false claim.
    PendingPermit,
    PermitIssued,
    OutcomeRecorded,
    SettlementPending,
    Settled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecutionItemV1 {
    pub operation: BindingOperationKey,
    pub effect_ordinal: u64,
    pub effect_fingerprint: BindingDigest32,
    pub participant_id: CapabilityParticipantId,
    pub disposition: TransferExecutionItemDispositionV1,
    pub permit: Option<SignedTransferExecutionPermitV1>,
    pub outcome: Option<SignedTransferExecutionOutcomeV1>,
    pub idempotency_digest: BindingDigest32,
    pub worker_id: String,
    pub worker_lease_epoch: u64,
    pub worker_lease_expires_at_unix_seconds: u64,
    pub settlement_claim_digest: Option<BindingDigest32>,
    pub settlement_digest: Option<BindingDigest32>,
    pub revision: TransferExecutionItemRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferExecutionItemPreconditionV1 {
    Absent,
    Matches {
        revision: TransferExecutionItemRevision,
        disposition: TransferExecutionItemDispositionV1,
        record_digest: BindingDigest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecutionItemPageRequestV1 {
    pub operation: BindingOperationKey,
    pub ledger_revision: crate::TransferExecutionLedgerRevision,
    pub ledger_record_digest: BindingDigest32,
    pub start_ordinal: u64,
    pub page_size: u32,
    pub page_token: Option<TransferWorkPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecutionItemPageV1 {
    pub operation: BindingOperationKey,
    pub items: Vec<TransferExecutionItemV1>,
    pub next_ordinal: u64,
    pub next_page_token: Option<TransferWorkPageTokenV1>,
    pub page_digest: BindingDigest32,
}
