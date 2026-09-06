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

/// Why an independently generated incarnation cannot be accepted.
///
/// Separate from [`crate::ServingAuthorityStoreError`] because that taxonomy
/// describes what a STORE did with a write, and has no variant for malformed
/// input: the closest, `Conflict`, is documented as a precondition failure
/// against durable state, which a parse has not yet touched. Every sibling
/// constructor in this crate takes the same shape --
/// `BindingProofConstructionError`, `BindingConstructionError`,
/// `BindingOperationConstructionError`,
/// `SourceReservationReleaseIssuanceConstructionErrorV1` and
/// `TransferExecutionPermitIssuanceConstructionErrorV1`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ServingAuthorityIncarnationConstructionErrorV1 {
    /// No implementation exists yet.
    NotImplemented,
    /// The value is empty. An incarnation must distinguish one lifetime of a
    /// serving authority from the next, and an empty value cannot.
    Empty,
    /// The value exceeds the permitted length.
    TooLong,
    /// The value was not independently generated to the required strength --
    /// for instance a counter or a caller-chosen constant. An incarnation that
    /// a peer can predict does not fence anything.
    NotIndependentlyGenerated,
}

impl ServingAuthorityIncarnationV1 {
    pub fn parse(
        _independently_generated: Vec<u8>,
    ) -> Result<Self, ServingAuthorityIncarnationConstructionErrorV1> {
        Err(ServingAuthorityIncarnationConstructionErrorV1::NotImplemented)
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

impl ServingAuthorityBusinessIdV1 {
    pub fn for_migration_claim(
        _identity: &crate::MigrationClaimIdentityV1,
    ) -> Result<Self, crate::BindingProofConstructionError> {
        Err(crate::BindingProofConstructionError::NotImplemented)
    }
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
