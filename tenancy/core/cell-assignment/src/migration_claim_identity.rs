use crate::{BindingDigest32, BindingProofConstructionError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationClaimIdentityPreimageV1 {
    pub schema_version: u32,
    pub control_partition: crate::TenantControlPartitionRefV1,
    pub operation: crate::BindingOperationKey,
    pub idempotency_key: crate::BindingIdempotencyKey,
    pub canonical_request_digest: BindingDigest32,
    pub source_authority: crate::ServingAuthorityInstanceV1,
    pub source_installation_issuance_digest: BindingDigest32,
    pub target_cell_id: cell_placement::CellId,
    pub successor_generation: crate::BindingGeneration,
    pub successor_write_authority_epoch: crate::WriteAuthorityEpoch,
    pub binding_attempt_digest: BindingDigest32,
    pub allocation_basis: MigrationClaimAllocationBasisV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MigrationClaimAllocationBasisV1 {
    InitialAllocation,
    Retarget {
        superseded_identity_digest: BindingDigest32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationClaimIdentityV1 {
    preimage: MigrationClaimIdentityPreimageV1,
    identity_digest: BindingDigest32,
}

impl MigrationClaimIdentityV1 {
    pub fn from_preimage(
        _preimage: MigrationClaimIdentityPreimageV1,
    ) -> Result<Self, BindingProofConstructionError> {
        Err(BindingProofConstructionError::NotImplemented)
    }

    pub fn rehydrate(
        _preimage: MigrationClaimIdentityPreimageV1,
        _identity_digest: BindingDigest32,
    ) -> Result<Self, BindingProofConstructionError> {
        Err(BindingProofConstructionError::NotImplemented)
    }

    #[must_use]
    pub fn preimage(&self) -> &MigrationClaimIdentityPreimageV1 {
        &self.preimage
    }

    #[must_use]
    pub fn identity_digest(&self) -> BindingDigest32 {
        self.identity_digest
    }
}
