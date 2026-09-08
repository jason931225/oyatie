use crate::{BindingControlContributionError, BindingDigest32, BoxTenancyFuture};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingControlContributionCoverageV1 {
    pub source_partition_root_digest: BindingDigest32,
    pub source_partition_count: u64,
    pub contribution_checkpoint_root_digest: BindingDigest32,
    pub source_manifest: cell_placement::ImmutableEvidenceRefV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCellIndexProjectionWriteSetV1 {
    parts: BindingCellIndexProjectionWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingCellIndexProjectionWriteSetPartsV1 {
    pub authority: crate::BindingReconciliationPersistenceAuthorityV1,
    pub target: crate::BindingControlContributionTargetV1,
    /// Compare-and-set on the cell binding index snapshot row this write
    /// proposes, [`crate::CellBindingIndexSnapshotV1`].
    ///
    /// Every other name below is in single backticks deliberately: an intra-doc
    /// link on a precondition member is read by the law tests as a declaration
    /// of the ROW this compare-and-set is on, so linking a store, an authority
    /// or a sibling write set here would attribute the compare-and-set to them.
    ///
    /// The value is read from `TenantBindingCellIndexStore::acquire_snapshot`,
    /// which takes `BindingReconciliationReadAuthorityV1` -- the read authority
    /// a holder of the `BindingReconciliationPersistenceAuthorityV1` this write
    /// set carries derives from
    /// `BindingReconciliationPersistenceAuthorityV1::read_authority`.
    ///
    /// AN EARLIER VERSION OF THIS SENTENCE SAID THE RECONCILER RECEIVES BOTH
    /// HALVES "IN THE SUBJECT IT IS HANDED", AND THAT IS FALSE. The subject for
    /// this work class is
    /// `BindingReconciliationSubjectV1::ControlContributionProjection`, whose
    /// whole content is the source-side `BindingControlContributionOutboxV1`;
    /// the `revision` and `record_digest` on that outbox are the OUTBOX row's,
    /// guarded by
    /// `BindingControlContributionDeliveryWriteSetPartsV1::expected_outbox_revision`,
    /// and a reconciler following the old sentence would have read one row's
    /// version into a compare-and-set on another. Both halves are `u64` and
    /// `BindingDigest32` on both rows, so nothing would have refused the
    /// mistake. A type-graph walk from that subject reaches neither the snapshot
    /// row nor either field name, and `tests/cas_precondition_reachability.rs`
    /// asserts that non-reachability as a standing control beside the positive
    /// one for the outbox.
    pub expected_projection_revision: u64,
    /// Compare-and-set on [`crate::CellBindingIndexSnapshotV1`]; the digest half
    /// of the pair above.
    pub expected_projection_digest: BindingDigest32,
    pub source_contributions: Vec<crate::VerifiedBindingControlContributionHandoff>,
    /// Compare-and-set on the per-source contribution checkpoints,
    /// [`crate::BindingControlContributionCheckpointV1`], advanced by
    /// `next_source_checkpoints` below. They are carried as a list rather than as
    /// a single row, so no write set proposes one of them as a bare record.
    pub expected_source_checkpoints: Vec<crate::BindingControlContributionCheckpointV1>,
    pub next_source_checkpoints: Vec<crate::BindingControlContributionCheckpointV1>,
    pub application_intents: Vec<crate::BindingControlContributionApplicationIntentV1>,
    pub limits: crate::BindingControlContributionLimitsV1,
    pub proof_consumptions: Vec<crate::BindingProofConsumptionV1>,
    pub next_snapshot: crate::CellBindingIndexSnapshotV1,
    pub audit_outbox: crate::BindingAuditRecordV1,
}

impl BindingCellIndexProjectionWriteSetV1 {
    pub fn assemble(
        _parts: BindingCellIndexProjectionWriteSetPartsV1,
    ) -> Result<Self, BindingControlContributionError> {
        Err(BindingControlContributionError::NotImplemented)
    }
    #[must_use]
    pub fn parts(&self) -> &BindingCellIndexProjectionWriteSetPartsV1 {
        &self.parts
    }
}

pub trait TenantBindingCellIndexProjectionStore: Send + Sync {
    /// Reads back the UNSIGNED durable acknowledgment, or `None` when none was
    /// recorded for that contribution.
    ///
    /// `apply_contributions` writes unsigned
    /// [`crate::BindingControlContributionAcknowledgmentPayloadV1`]s and, as its
    /// own documentation says, does not attest that it applied them. No write
    /// path carries a signed acknowledgment into this store, so a getter typed
    /// to the signed form could only have been satisfied by the store forging
    /// it. The signed form comes from
    /// [`crate::BindingControlContributionAcknowledgmentObserver::observe_acknowledgment`].
    ///
    /// Absence must stay distinct from "result unknown". A reconciler asking
    /// whether the target acknowledged needs to tell "not yet" from "the read
    /// failed", and `BindingControlContributionError` folds neither into the
    /// other only if absence never becomes an error. `MissingAcknowledgment`
    /// exists but names a required member absent from a record that WAS found,
    /// which is a different fact from no record existing.
    fn get_acknowledgment<'a>(
        &'a self,
        authority: &'a crate::BindingReconciliationReadAuthorityV1,
        query: &'a crate::BindingControlContributionAcknowledgmentQueryV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<
            Option<crate::BindingControlContributionAcknowledgmentPayloadV1>,
            BindingControlContributionError,
        >,
    >;

    fn apply_contributions<'a>(
        &'a self,
        write_set: &'a BindingCellIndexProjectionWriteSetV1,
    ) -> BoxTenancyFuture<
        'a,
        Result<crate::BindingCellIndexProjectionResultV1, BindingControlContributionError>,
    >;
}
