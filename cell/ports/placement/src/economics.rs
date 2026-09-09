use crate::{
    BoxCellFuture, CapacityVectorV1, CellId, Digest32, PlacementLocationV1,
    PlacementReadAuthorityV1, ProofConstructionError, TenantId,
};

macro_rules! opaque_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn parse(_value: impl Into<String>) -> Result<Self, ProofConstructionError> {
                Err(ProofConstructionError::NotImplemented)
            }
        }
    };
}

opaque_id!(CurrencyCode);
opaque_id!(PriceSnapshotId);
opaque_id!(CommercialAuthorityId);
opaque_id!(CommercialEntryId);
opaque_id!(CommercialPartitionKey);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialPageToken(Vec<u8>);

impl CommercialPageToken {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn parse(_value: Vec<u8>) -> Result<Self, crate::PlacementContractError> {
        Err(crate::PlacementContractError::NotImplemented)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CommitmentTreatmentV1 {
    Exclude,
    AmortizedEffectiveCost,
    IncrementalCommittedCost,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SpotTreatmentV1 {
    Exclude,
    RiskAdjustedExpectedCost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MoneyMicrounitsV1 {
    pub currency: CurrencyCode,
    pub microunits: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CommercialAvailabilityStateV1 {
    Available,
    Draining,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialSourceRecordRefV1 {
    pub authority_id: CommercialAuthorityId,
    pub repository_id: String,
    pub object_id: String,
    pub object_version: u64,
    pub source_schema_version: u32,
    pub content_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StableSupplyV1 {
    pub reserved_capacity_id: Option<String>,
    pub commitment_id: Option<String>,
    pub term_months: Option<u32>,
    pub committed_units: Option<u64>,
    pub discount_basis_points: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotSupplyV1 {
    pub pool_id: String,
    pub state: CommercialAvailabilityStateV1,
    pub available_capacity: CapacityVectorV1,
    pub current_unit_price: MoneyMicrounitsV1,
    pub interruption_notice_seconds: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapacitySupplyV1 {
    OnDemand(StableSupplyV1),
    Reserved(StableSupplyV1),
    CommittedUse(StableSupplyV1),
    Spot(SpotSupplyV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialSnapshotEntryV1 {
    pub entry_id: CommercialEntryId,
    pub source_schema_version: u32,
    pub source_record_digest: Digest32,
    pub source_record: CommercialSourceRecordRefV1,
    pub location: PlacementLocationV1,
    pub cell_id: CellId,
    pub capacity_class: String,
    pub offer_id: String,
    pub unit_capacity: CapacityVectorV1,
    pub list_unit_price: MoneyMicrounitsV1,
    pub rate_period_seconds: u64,
    pub priced_quantity: u64,
    pub normalized_capacity_unit_digest: Digest32,
    pub valuation_horizon_seconds: u64,
    pub price_calculation_policy_version: String,
    pub price_calculation_policy_digest: Digest32,
    pub supply: CapacitySupplyV1,
    pub valid_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub entry_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialSnapshotV1 {
    pub authority_id: CommercialAuthorityId,
    pub snapshot_id: PriceSnapshotId,
    pub revision: u64,
    pub partition: CommercialPartitionKey,
    pub ordered_entry_root_digest: Digest32,
    pub entry_count: u64,
    pub valid_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub snapshot_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialSnapshotPageRequestV1 {
    pub partition: CommercialPartitionKey,
    pub cell_id: CellId,
    pub page_size: u32,
    pub page_token: Option<CommercialPageToken>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialSnapshotPageV1 {
    pub snapshot: CommercialSnapshotV1,
    pub entries: Vec<CommercialSnapshotEntryV1>,
    pub next_page_token: Option<CommercialPageToken>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SpotDataLossToleranceV1 {
    NoDataLoss,
    CheckpointBounded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpotAcceptanceV1 {
    pub maximum_unit_price: MoneyMicrounitsV1,
    pub minimum_interruption_notice_seconds: u32,
    pub maximum_interruptions_per_horizon: u32,
    pub tolerance: SpotDataLossToleranceV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapacityPurchasePreferenceV1 {
    StableOnly,
    SpotAllowed(SpotAcceptanceV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialAllocationAttributionV1 {
    pub tenant_id: TenantId,
    pub snapshot_id: PriceSnapshotId,
    pub snapshot_revision: u64,
    pub entry_id: CommercialEntryId,
    pub entry_digest: Digest32,
    pub supply: CapacitySupplyV1,
    pub allocated_quantity: u64,
    pub valuation_horizon_seconds: u64,
    pub normalized_charge_basis_digest: Digest32,
    pub formula_input_root_digest: Digest32,
    pub formula_input_count: u64,
    pub calculation_policy_version: String,
    pub calculation_policy_digest: Digest32,
    pub attributed_cost: MoneyMicrounitsV1,
    pub attribution_digest: Digest32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialSourceRecordReadRequestV1 {
    pub source_record: CommercialSourceRecordRefV1,
    pub maximum_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalCommercialSourceRecordV1 {
    pub source_record: CommercialSourceRecordRefV1,
    pub canonical_encoding: String,
    pub canonical_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommercialPlacementBasisV1 {
    pub currency: CurrencyCode,
    pub valuation_horizon_seconds: u64,
    pub price_snapshot: PriceSnapshotId,
    pub price_snapshot_revision: u64,
    pub valid_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub commitment_treatment: CommitmentTreatmentV1,
    pub spot_treatment: SpotTreatmentV1,
    pub minimum_spot_interruption_notice_seconds: u32,
    pub capacity_evidence_digest: Digest32,
    pub commitment_evidence_digest: Digest32,
    pub spot_evidence_digest: Digest32,
    pub stable_headroom_basis_points: u32,
    pub maximum_rebalance_basis_points: u32,
    pub commercial_basis_digest: Digest32,
    pub commercial_authority_id: CommercialAuthorityId,
    pub snapshot_partition: CommercialPartitionKey,
    pub ordered_entry_root_digest: Digest32,
    pub snapshot_entry_count: u64,
    pub calculation_policy_version: String,
    pub calculation_policy_digest: Digest32,
}

pub trait CommercialSnapshotResolver: Send + Sync {
    fn resolve_page<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        request: &'a CommercialSnapshotPageRequestV1,
    ) -> BoxCellFuture<'a, Result<CommercialSnapshotPageV1, crate::PlacementContractError>>;

    /// Reads the canonical commercial source record the request names.
    ///
    /// `None` MEANS THE RESOLVER LOOKED AND FOUND NO RECORD for that request.
    /// It is an outcome, not a failure, and it is deliberately not
    /// [`crate::PlacementContractError::NotFoundOrNotAuthorized`]: that variant
    /// conflates absence with an authorization refusal, which is right at an
    /// unauthenticated edge and wrong for a method that already runs under an
    /// explicit [`PlacementReadAuthorityV1`].
    fn read_source_record<'a>(
        &'a self,
        authority: &'a PlacementReadAuthorityV1,
        request: &'a CommercialSourceRecordReadRequestV1,
    ) -> BoxCellFuture<
        'a,
        Result<Option<CanonicalCommercialSourceRecordV1>, crate::PlacementContractError>,
    >;
}
