use crate::{
    BindingAuditRecordV1, BindingDigest32, BindingIdempotencyRecordV1, BindingOperationKey,
    BindingPersistenceAuthorityV1, BindingReadAuthorityV1, BindingReconciliationLeaseV1,
    BindingReconciliationPersistenceAuthorityV1, BindingReconciliationReadAuthorityV1,
    BindingStoreError, BoxTenancyFuture, ParticipantManifestMemberSnapshotV1,
    ParticipantManifestMemberV1, ResidencyTransferEffectV1, TransferEffectSnapshotV1,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingWorkSnapshotKindV1 {
    ParticipantMembers,
    TransferEffects,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BindingWorkSnapshotStateV1 {
    Open,
    Sealed,
    Published,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BindingWorkSnapshotRevision(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingWorkSnapshotKeyV1 {
    pub operation: BindingOperationKey,
    pub kind: BindingWorkSnapshotKindV1,
    pub snapshot_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingWorkSnapshotProgressV1 {
    pub key: BindingWorkSnapshotKeyV1,
    pub descriptor: BindingWorkSnapshotDescriptorV1,
    pub publication_intent: BindingWorkSnapshotPublicationIntentV1,
    pub state: BindingWorkSnapshotStateV1,
    pub next_ordinal: u64,
    pub committed_item_root_digest: BindingDigest32,
    pub revision: BindingWorkSnapshotRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingWorkSnapshotDescriptorV1 {
    ParticipantMembers(ParticipantManifestMemberSnapshotV1),
    TransferEffects(TransferEffectSnapshotV1),
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParticipantManifestPublicationIntentV1 {
    pub expected_attempt_revision: crate::BindingReservationAttemptRevision,
    pub expected_operation_revision: crate::BindingOperationRevision,
    pub operation: crate::BindingOperationV1,
    pub placement_decision: cell_placement::SignedPlacementDecisionV1,
    pub manifest: crate::SignedParticipantManifestV1,
    pub idempotency_key: crate::BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TransferManifestPublicationIntentV1 {
    pub expected_operation_revision: crate::BindingOperationRevision,
    pub operation: crate::BindingOperationV1,
    pub manifest: crate::SignedTransferEffectManifestV1,
    pub idempotency_key: crate::BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BindingWorkSnapshotPublicationIntentV1 {
    ParticipantManifest(Box<ParticipantManifestPublicationIntentV1>),
    TransferManifest(Box<TransferManifestPublicationIntentV1>),
}

/// Compare-and-set on the binding work snapshot progress row:
/// [`BindingWorkSnapshotProgressV1`]. `Absent` asserts the store must find no
/// progress row, which is the state a first snapshot is in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingWorkSnapshotPreconditionV1 {
    Absent,
    Matches {
        revision: BindingWorkSnapshotRevision,
        record_digest: BindingDigest32,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum BindingWorkSnapshotMutationAuthorityV1 {
    Request(BindingPersistenceAuthorityV1),
    Reconciliation {
        authority: BindingReconciliationPersistenceAuthorityV1,
        expected_lease: BindingReconciliationLeaseV1,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum BindingWorkSnapshotReadAuthorityV1 {
    Request(BindingReadAuthorityV1),
    Reconciliation {
        authority: BindingReconciliationReadAuthorityV1,
        expected_lease: BindingReconciliationLeaseV1,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingWorkSnapshotPageBoundsV1 {
    pub policy: cell_placement::ImmutableEvidenceRefV1,
    pub maximum_items_per_write: u32,
    pub maximum_encoded_bytes_per_write: u64,
    pub maximum_inclusion_path_depth: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingWorkSnapshotMutationV1 {
    AppendParticipantMembers {
        snapshot: ParticipantManifestMemberSnapshotV1,
        start_ordinal: u64,
        members: Vec<ParticipantManifestMemberV1>,
    },
    AppendTransferEffects {
        snapshot: TransferEffectSnapshotV1,
        start_ordinal: u64,
        effects: Vec<ResidencyTransferEffectV1>,
    },
    SealParticipantMembers(ParticipantManifestMemberSnapshotV1),
    SealTransferEffects(TransferEffectSnapshotV1),
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingWorkSnapshotWriteSetV1 {
    parts: BindingWorkSnapshotWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingWorkSnapshotWriteSetPartsV1 {
    pub authority: BindingWorkSnapshotMutationAuthorityV1,
    pub precondition: BindingWorkSnapshotPreconditionV1,
    pub mutation: BindingWorkSnapshotMutationV1,
    pub next: BindingWorkSnapshotProgressV1,
    pub bounds: BindingWorkSnapshotPageBoundsV1,
    pub idempotency: BindingIdempotencyRecordV1,
    pub audit_outbox: BindingAuditRecordV1,
}

impl BindingWorkSnapshotWriteSetV1 {
    pub fn assemble(_parts: BindingWorkSnapshotWriteSetPartsV1) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &BindingWorkSnapshotWriteSetPartsV1 {
        &self.parts
    }
}

pub trait BindingWorkSnapshotStore: Send + Sync {
    fn apply_snapshot_mutation<'a>(
        &'a self,
        write_set: &'a BindingWorkSnapshotWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingWorkSnapshotProgressV1, BindingStoreError>>;

    fn get_snapshot_progress<'a>(
        &'a self,
        authority: &'a BindingWorkSnapshotReadAuthorityV1,
        key: &'a BindingWorkSnapshotKeyV1,
    ) -> BoxTenancyFuture<'a, Result<Option<BindingWorkSnapshotProgressV1>, BindingStoreError>>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParticipantSnapshotSourcePageRequestV1 {
    pub snapshot: ParticipantManifestMemberSnapshotV1,
    pub start_ordinal: u64,
    pub page_size: u32,
    pub page_token: Option<crate::ParticipantWorkPageTokenV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferSnapshotSourcePageRequestV1 {
    pub snapshot: TransferEffectSnapshotV1,
    pub start_ordinal: u64,
    pub page_size: u32,
    pub page_token: Option<crate::TransferWorkPageTokenV1>,
}

pub trait BindingWorkSnapshotSourceReader: Send + Sync {
    fn read_participant_source_page<'a>(
        &'a self,
        authority: &'a BindingWorkSnapshotReadAuthorityV1,
        request: &'a ParticipantSnapshotSourcePageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<crate::ParticipantManifestMemberPageV1, BindingStoreError>>;

    fn read_transfer_source_page<'a>(
        &'a self,
        authority: &'a BindingWorkSnapshotReadAuthorityV1,
        request: &'a TransferSnapshotSourcePageRequestV1,
    ) -> BoxTenancyFuture<'a, Result<crate::TransferEffectPageV1, BindingStoreError>>;
}
