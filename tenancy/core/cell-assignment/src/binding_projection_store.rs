use crate::{
    BindingDigest32, BindingGeneration, BindingProofConsumptionV1, BindingStoreError,
    BoxTenancyFuture, ProjectionAudienceId, SignedBindingProjectionV1, TenantId,
    VerifiedBindingProjection, WriteAuthorityEpoch,
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProjectionPartitionKey(String);

impl ProjectionPartitionKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, crate::BindingProofConstructionError> {
        Err(crate::BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProjectionSnapshotId(String);

impl ProjectionSnapshotId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(_value: impl Into<String>) -> Result<Self, crate::BindingProofConstructionError> {
        Err(crate::BindingProofConstructionError::NotImplemented)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingProjectionSnapshotV1 {
    pub audience: ProjectionAudienceId,
    pub partition: ProjectionPartitionKey,
    pub partition_digest: BindingDigest32,
    pub snapshot_id: ProjectionSnapshotId,
    pub revision: u64,
    pub ordered_binding_root_digest: BindingDigest32,
    pub binding_count: u64,
    pub created_at_unix_seconds: u64,
    pub record_digest: BindingDigest32,
}

/// Compare-and-set on the binding row this projection is installed for:
/// [`crate::TenantCellBinding`]. It is not a precondition on the snapshot row —
/// that is
/// [`BindingProjectionInstallWriteSetPartsV1::expected_snapshot_revision`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionInstallPreconditionV1 {
    Unmapped,
    Matches {
        generation: BindingGeneration,
        write_authority_epoch: WriteAuthorityEpoch,
        binding_record_digest: BindingDigest32,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingProjectionInstallWriteSetV1 {
    parts: BindingProjectionInstallWriteSetPartsV1,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BindingProjectionInstallWriteSetPartsV1 {
    pub drain_mutations: cell_placement::DrainContributorMutationSetV1,
    /// Compare-and-set on the projection snapshot row for this audience and
    /// partition — [`BindingProjectionSnapshotV1`] — and the only site where
    /// that row may legitimately not exist yet.
    ///
    /// `None` ASSERTS THAT THE STORE MUST FIND NO SNAPSHOT ROW for this
    /// audience and partition. `install` is the only write in either crate
    /// carrying a [`BindingProjectionSnapshotV1`] as a next-state member, so the
    /// row is born by the first install and before it there is nothing to
    /// compare against. `Some(revision)` asserts a row exists at exactly that
    /// revision, read through
    /// [`BindingProjectionLocalStore::current_snapshot`].
    ///
    /// THE STORE MUST REFUSE RATHER THAN PROCEED WHEN THE ASSERTION IS FALSE:
    /// [`crate::BindingStoreError::Conflict`] both when `None` was claimed and
    /// a row exists, and when `Some` was claimed and the row is absent or at a
    /// different revision. Zero is not the absent value; a caller that means
    /// "no snapshot" says so.
    ///
    /// This member is NOT the sibling `precondition` below. That one is a
    /// compare-and-set on the BINDING row, whose generation, write-authority
    /// epoch and record digest it names, and it has had an `Unmapped` arm since
    /// the seed commit. The snapshot row had no absent representation on either
    /// side of the contract until this change.
    pub expected_snapshot_revision: Option<u64>,
    pub precondition: ProjectionInstallPreconditionV1,
    pub projection: VerifiedBindingProjection,
    pub next_snapshot: BindingProjectionSnapshotV1,
    pub local_idempotency_digest: BindingDigest32,
    pub local_audit_record_digest: BindingDigest32,
    pub proof_consumption: BindingProofConsumptionV1,
}

impl BindingProjectionInstallWriteSetV1 {
    pub fn assemble(
        _parts: BindingProjectionInstallWriteSetPartsV1,
    ) -> Result<Self, BindingStoreError> {
        Err(BindingStoreError::NotImplemented)
    }

    #[must_use]
    pub fn parts(&self) -> &BindingProjectionInstallWriteSetPartsV1 {
        &self.parts
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProjectionFreshnessV1 {
    Current,
    RefreshOverdue,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProjectionLookupShapeV1 {
    OneLocalIndexedSnapshotLookup,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalBindingRouteV1 {
    pub projection: SignedBindingProjectionV1,
    pub snapshot_id: ProjectionSnapshotId,
    pub snapshot_revision: u64,
    pub freshness: ProjectionFreshnessV1,
    pub lookup_shape: ProjectionLookupShapeV1,
}

pub trait BindingProjectionLocalStore: Send + Sync {
    fn install<'a>(
        &'a self,
        write_set: &'a BindingProjectionInstallWriteSetV1,
    ) -> BoxTenancyFuture<'a, Result<BindingProjectionSnapshotV1, BindingStoreError>>;

    fn lookup_local<'a>(
        &'a self,
        audience: &'a ProjectionAudienceId,
        partition: &'a ProjectionPartitionKey,
        tenant_id: &'a TenantId,
        now_unix_seconds: u64,
    ) -> BoxTenancyFuture<'a, Result<Option<LocalBindingRouteV1>, BindingStoreError>>;

    /// Reads the installed projection snapshot for one audience and partition.
    ///
    /// `None` MEANS THE STORE LOOKED AND FOUND NO SNAPSHOT ROW. It is not an
    /// error and not an unknown: it is the exact fact a first
    /// [`BindingProjectionLocalStore::install`] needs, and it is the value
    /// [`BindingProjectionInstallWriteSetPartsV1::expected_snapshot_revision`]
    /// must carry at that first install. The read previously could not express
    /// absence at all, so the one state a first install is in had no
    /// representation on either side of the contract.
    fn current_snapshot<'a>(
        &'a self,
        audience: &'a ProjectionAudienceId,
        partition: &'a ProjectionPartitionKey,
    ) -> BoxTenancyFuture<'a, Result<Option<BindingProjectionSnapshotV1>, BindingStoreError>>;
}
