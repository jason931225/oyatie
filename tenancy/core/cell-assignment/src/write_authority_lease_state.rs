use cell_placement::CellId;

use crate::{
    BindingDigest32, BindingGeneration, BindingOperationKey, BindingRevision, TenantId,
    WriteAuthorityEpoch,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WriteAuthorityLeaseStateRevision(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum WriteAuthorityLeaseDispositionV1 {
    Active,
    FrozenForMigration,
    Retired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseStateV1 {
    instance: crate::ServingAuthorityInstanceV1,
    tenant_id: TenantId,
    cell_id: CellId,
    binding_generation: BindingGeneration,
    binding_revision: BindingRevision,
    write_authority_epoch: WriteAuthorityEpoch,
    participant_manifest_digest: BindingDigest32,
    binding_record_digest: BindingDigest32,
    disposition: WriteAuthorityLeaseDispositionV1,
    current_lease_digest: BindingDigest32,
    maximum_issued_lease_expires_at_unix_seconds: u64,
    frozen_by_operation: Option<BindingOperationKey>,
    revision: WriteAuthorityLeaseStateRevision,
    record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseStatePartsV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub tenant_id: TenantId,
    pub cell_id: CellId,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub participant_manifest_digest: BindingDigest32,
    pub binding_record_digest: BindingDigest32,
    pub disposition: WriteAuthorityLeaseDispositionV1,
    pub current_lease_digest: BindingDigest32,
    pub maximum_issued_lease_expires_at_unix_seconds: u64,
    pub frozen_by_operation: Option<BindingOperationKey>,
    pub revision: WriteAuthorityLeaseStateRevision,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteAuthorityLeaseStateConstructionError {
    NotImplemented,
    InvalidRevision,
    InvalidDisposition,
    InvalidLeaseRelation,
    InvalidBindingRelation,
}

impl WriteAuthorityLeaseStateV1 {
    pub fn rehydrate(
        _parts: WriteAuthorityLeaseStatePartsV1,
    ) -> Result<Self, WriteAuthorityLeaseStateConstructionError> {
        Err(WriteAuthorityLeaseStateConstructionError::NotImplemented)
    }

    #[must_use]
    pub fn instance(&self) -> &crate::ServingAuthorityInstanceV1 {
        &self.instance
    }

    #[must_use]
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    #[must_use]
    pub fn cell_id(&self) -> &CellId {
        &self.cell_id
    }

    #[must_use]
    pub fn binding_generation(&self) -> BindingGeneration {
        self.binding_generation
    }

    #[must_use]
    pub fn binding_revision(&self) -> BindingRevision {
        self.binding_revision
    }

    #[must_use]
    pub fn write_authority_epoch(&self) -> WriteAuthorityEpoch {
        self.write_authority_epoch
    }

    #[must_use]
    pub fn participant_manifest_digest(&self) -> BindingDigest32 {
        self.participant_manifest_digest
    }

    #[must_use]
    pub fn binding_record_digest(&self) -> BindingDigest32 {
        self.binding_record_digest
    }

    #[must_use]
    pub fn disposition(&self) -> WriteAuthorityLeaseDispositionV1 {
        self.disposition
    }

    #[must_use]
    pub fn current_lease_digest(&self) -> BindingDigest32 {
        self.current_lease_digest
    }

    #[must_use]
    pub fn maximum_issued_lease_expires_at_unix_seconds(&self) -> u64 {
        self.maximum_issued_lease_expires_at_unix_seconds
    }

    #[must_use]
    pub fn frozen_by_operation(&self) -> Option<&BindingOperationKey> {
        self.frozen_by_operation.as_ref()
    }

    #[must_use]
    pub fn revision(&self) -> WriteAuthorityLeaseStateRevision {
        self.revision
    }

    #[must_use]
    pub fn record_digest(&self) -> BindingDigest32 {
        self.record_digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteAuthorityLeaseStatePreconditionV1 {
    pub instance: crate::ServingAuthorityInstanceV1,
    pub revision: WriteAuthorityLeaseStateRevision,
    pub disposition: WriteAuthorityLeaseDispositionV1,
    pub current_lease_digest: BindingDigest32,
    pub maximum_issued_lease_expires_at_unix_seconds: u64,
    pub record_digest: BindingDigest32,
}
