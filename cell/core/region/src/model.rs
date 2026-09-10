use std::collections::BTreeMap;

use cell_location::{AzCode, CellId, CellLocationError, RegionCode};
use cell_routing::CellError;
use data_boundary_kernel::Classified;
use network_residency::{RegionRef, ResidencyClass, ResidencyError};

pub(crate) const CLOUD_REGION_SCHEMA_VERSION: u32 = 1;
pub(crate) const CLOUD_AZ_SCHEMA_VERSION: u32 = 1;
pub(crate) const CLOUD_CELL_SCHEMA_VERSION: u32 = 1;
pub(crate) const REGIONAL_PACK_ID_PREFIX: &str = "pack-";
pub(crate) const HSM_PARTITION_PREFIX: &str = "hsm/";
pub(crate) const TENANT_ID_PREFIX: &str = "ten_";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RegionState {
    Planned,
    Preview,
    Ga,
    Retiring,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AzState {
    Planned,
    Active,
    DrOnly,
    Retiring,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CloudCellState {
    Planned,
    Active,
    DrOnly,
    Draining,
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TenantDensityClass {
    Shared,
    Dedicated,
    Sovereign,
    AirGapped,
    FoundryRuntime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegionCreate {
    pub code: String,
    pub display_name: String,
    pub regulatory_packs: Vec<String>,
    pub state: RegionState,
    pub provider_facing: bool,
    pub residency_strictness: ResidencyClass,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudRegion {
    pub code: Classified<RegionCode>,
    pub display_name: Classified<String>,
    pub regulatory_packs: Classified<Vec<String>>,
    pub azs: Classified<Vec<AzCode>>,
    pub state: Classified<RegionState>,
    pub provider_facing: Classified<bool>,
    pub residency_strictness: Classified<ResidencyClass>,
    pub region_ref: Classified<RegionRef>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudAzCreate {
    pub code: String,
    pub region_code: String,
    pub physical_ref: String,
    pub power_zones: Vec<String>,
    pub state: AzState,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudAz {
    pub code: Classified<AzCode>,
    pub region_code: Classified<RegionCode>,
    pub physical_ref: Classified<String>,
    pub power_zones: Classified<Vec<String>>,
    pub cells: Classified<Vec<CellId>>,
    pub state: Classified<AzState>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CellCapacity {
    pub compute_vcpu: u64,
    pub memory_gb: u64,
    pub ssd_tb: u64,
    pub gpu_count: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct CellUtilization {
    pub compute_vcpu_used: u64,
    pub memory_gb_used: u64,
    pub ssd_tb_used: u64,
    pub gpu_count_used: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCellCreate {
    pub id: String,
    pub region_code: String,
    pub az_code: String,
    pub state: CloudCellState,
    pub tenant_density: TenantDensityClass,
    pub allowed_residency: Vec<ResidencyClass>,
    pub capacity: CellCapacity,
    pub utilization: CellUtilization,
    pub hsm_partition_ref: String,
    pub created_at_epoch_seconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudCell {
    pub id: Classified<CellId>,
    pub region_code: Classified<RegionCode>,
    pub az_code: Classified<AzCode>,
    pub state: Classified<CloudCellState>,
    pub tenant_density: Classified<TenantDensityClass>,
    pub allowed_residency: Classified<Vec<ResidencyClass>>,
    pub capacity: Classified<CellCapacity>,
    pub utilization: Classified<CellUtilization>,
    pub hsm_partition_ref: Classified<String>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TenantCellRouteRequest {
    pub tenant_id: String,
    pub home_region_code: String,
    pub residency_class: ResidencyClass,
    pub required_density: Option<TenantDensityClass>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudRegionError {
    InvalidRegionCode,
    InvalidAzCode,
    InvalidCellId,
    InvalidDisplayName,
    InvalidRegulatoryPack,
    EmptyRegulatoryPackSet,
    DuplicateRegulatoryPack,
    InvalidPhysicalRef,
    InvalidPowerZone,
    EmptyPowerZoneSet,
    DuplicatePowerZone,
    InvalidHsmPartitionRef,
    InvalidTenantId,
    InvalidCapacity,
    UtilizationExceedsCapacity,
    RegionResidencyMismatch,
    EmptyAllowedResidencySet,
    DuplicateAllowedResidencyClass,
    CellResidencyNotAllowedInRegion,
    CellResidencyDenied,
    DuplicateRegion,
    DuplicateAz,
    DuplicateCell,
    UnknownRegion,
    UnknownAz,
    UnknownCell,
    AzRegionMismatch,
    CellRegionMismatch,
    CellAzMismatch,
    NoCompatibleCell,
    CellBindingRejected(CellError),
    ResidencyReferenceRejected(ResidencyError),
}

impl From<CellLocationError> for CloudRegionError {
    fn from(error: CellLocationError) -> Self {
        match error {
            CellLocationError::InvalidRegionCode => Self::InvalidRegionCode,
            CellLocationError::InvalidAzCode => Self::InvalidAzCode,
            CellLocationError::InvalidCellId => Self::InvalidCellId,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CloudRegionCatalog {
    pub(crate) regions: BTreeMap<RegionCode, CloudRegion>,
    pub(crate) azs: BTreeMap<AzCode, CloudAz>,
    pub(crate) cells: BTreeMap<CellId, CloudCell>,
}
