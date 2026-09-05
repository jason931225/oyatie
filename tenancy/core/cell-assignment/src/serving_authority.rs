use crate::{BindingDigest32, BindingGeneration, BindingRevision, TenantId, WriteAuthorityEpoch};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantControlPartitionRefV1 {
    pub tenant_id: TenantId,
    pub shard_id: String,
    pub topology_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellServingPartitionRefV1 {
    pub cell_id: cell_placement::CellId,
    pub shard_id: String,
    pub topology_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityIncarnationV1(Vec<u8>);

impl ServingAuthorityIncarnationV1 {
    pub fn parse(
        _independently_generated: Vec<u8>,
    ) -> Result<Self, crate::ServingAuthorityStoreError> {
        Err(crate::ServingAuthorityStoreError::NotImplemented)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityInstanceV1 {
    pub tenant_id: TenantId,
    pub partition: CellServingPartitionRefV1,
    pub binding_generation: BindingGeneration,
    pub binding_revision: BindingRevision,
    pub binding_record_digest: BindingDigest32,
    pub write_authority_epoch: WriteAuthorityEpoch,
    pub incarnation: ServingAuthorityIncarnationV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstalledServingAuthorityV1 {
    pub instance: ServingAuthorityInstanceV1,
    pub installation_issuance_digest: BindingDigest32,
    pub installed_grant_digest: BindingDigest32,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityRejectionHighWaterV1 {
    pub tenant_id: TenantId,
    pub partition: CellServingPartitionRefV1,
    pub generation: BindingGeneration,
    pub rejected_instance_root_digest: BindingDigest32,
    pub rejected_instance_count: u64,
    pub revision: u64,
    pub record_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServingAuthorityLocalPreconditionV1 {
    Uninstalled {
        instance: ServingAuthorityInstanceV1,
        rejection_high_water: ServingAuthorityRejectionHighWaterV1,
    },
    Installed {
        authority: Box<InstalledServingAuthorityV1>,
        lease_state: Box<crate::WriteAuthorityLeaseStatePreconditionV1>,
        rejection_high_water: ServingAuthorityRejectionHighWaterV1,
    },
    Rejected {
        rejection: crate::ServingAuthorityRejectionV1,
        rejection_high_water: ServingAuthorityRejectionHighWaterV1,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityBusinessIdV1 {
    pub operation: crate::BindingOperationKey,
    pub idempotency_key: crate::BindingIdempotencyKey,
    pub request_digest: BindingDigest32,
    pub handoff_digest: BindingDigest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServingAuthorityHandoffExpectationV1 {
    pub control_partition: TenantControlPartitionRefV1,
    pub instance: ServingAuthorityInstanceV1,
    pub business: ServingAuthorityBusinessIdV1,
    pub expected_producer: crate::BindingProducerId,
    pub expected_audience: crate::BindingProducerId,
    pub now_unix_seconds: u64,
}
