//! Cell-routing kernel: immutable per-tenant per-region cell placement.

use std::collections::BTreeMap;

use data_boundary_kernel::{Classified, PrivacyDataClass};
use network_residency::{RegionRef, ResidencyClass, residency_class_allows_home_region_label};

const CELL_BINDING_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CellTier {
    Shared,
    Pooled,
    Dedicated,
    SovereignAirGapped,
    FoundryRuntime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBindingCreate {
    pub tenant_id: String,
    pub region: RegionRef,
    pub residency_class: ResidencyClass,
    pub az: String,
    pub cell_id: String,
    pub tier: CellTier,
    pub hsm_partition_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBinding {
    pub tenant_id: String,
    pub region: String,
    pub region_ref: Classified<RegionRef>,
    pub residency_class: Classified<ResidencyClass>,
    pub az: Classified<String>,
    pub cell_id: Classified<String>,
    pub tier: Classified<CellTier>,
    pub hsm_partition_ref: Classified<String>,
    pub schema_version: Classified<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CellError {
    AlreadyBound,
    InvalidTenantId,
    EmptyAz,
    EmptyCell,
    EmptyHsmPartition,
    AzRegionMismatch,
    ResidencyRegionMismatch,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CellRouter {
    bindings: BTreeMap<String, CellBinding>,
}

impl CellRouter {
    pub fn bind(&mut self, input: CellBindingCreate) -> Result<CellBinding, CellError> {
        if self.bindings.contains_key(&input.tenant_id) {
            return Err(CellError::AlreadyBound);
        }
        let binding = CellBinding::new(input)?;
        self.bindings
            .insert(binding.tenant_id.clone(), binding.clone());
        Ok(binding)
    }

    pub fn get(&self, tenant_id: &str) -> Option<&CellBinding> {
        self.bindings.get(tenant_id)
    }
}

impl CellBinding {
    pub fn new(input: CellBindingCreate) -> Result<Self, CellError> {
        validate_tenant_id(&input.tenant_id)?;
        validate_non_empty(&input.az, CellError::EmptyAz)?;
        validate_non_empty(&input.cell_id, CellError::EmptyCell)?;
        validate_non_empty(&input.hsm_partition_ref, CellError::EmptyHsmPartition)?;
        let region_id = input.region.region_id.value.clone();
        validate_az_region(&input.az, &region_id)?;
        if !residency_class_allows_home_region_label(&input.residency_class, &region_id) {
            return Err(CellError::ResidencyRegionMismatch);
        }
        Ok(Self {
            tenant_id: input.tenant_id,
            region: region_id,
            region_ref: internal(input.region),
            residency_class: internal(input.residency_class),
            az: internal(input.az),
            cell_id: internal(input.cell_id),
            tier: internal(input.tier),
            hsm_partition_ref: internal(input.hsm_partition_ref),
            schema_version: internal(CELL_BINDING_SCHEMA_VERSION),
        })
    }
}

fn validate_tenant_id(tenant_id: &str) -> Result<(), CellError> {
    if tenant_id.starts_with("ten_") && tenant_id.len() > 4 {
        Ok(())
    } else {
        Err(CellError::InvalidTenantId)
    }
}

fn validate_az_region(az: &str, region_id: &str) -> Result<(), CellError> {
    if az == region_id
        || az
            .strip_prefix(region_id)
            .is_some_and(|suffix| suffix.starts_with('-'))
    {
        Ok(())
    } else {
        Err(CellError::AzRegionMismatch)
    }
}

fn validate_non_empty(value: &str, error: CellError) -> Result<(), CellError> {
    if value.trim().is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

fn internal<T>(value: T) -> Classified<T> {
    Classified::new(value, internal_data_class())
}

fn internal_data_class() -> PrivacyDataClass {
    PrivacyDataClass::internal_only()
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_residency::{
        PerPackResidency, PerPackResidencyCreate, RegionJurisdiction, RegionRefCreate,
        RegulatorOverlay, RegulatorOverlayCreate,
    };

    fn region(region_id: &str, jurisdiction: RegionJurisdiction) -> RegionRef {
        RegionRef::new(RegionRefCreate {
            region_id: region_id.to_string(),
            jurisdiction,
            cell_group_ref: format!("cells/{region_id}"),
        })
        .expect("region fixture is valid")
    }

    fn residency_class() -> ResidencyClass {
        ResidencyClass::PerPack(Box::new(
            PerPackResidency::new(PerPackResidencyCreate {
                allowed_primary_regions: vec!["region-alpha".to_string()],
                allowed_replica_regions: vec!["region-beta".to_string()],
                forbidden_regions: vec!["region-gamma".to_string()],
                regulator_overlay: RegulatorOverlay::new(RegulatorOverlayCreate {
                    regulator_refs: vec!["regulator/global".to_string()],
                    evidence_ref: "evidence/residency/global".to_string(),
                })
                .expect("regulator overlay fixture is valid"),
            })
            .expect("per-pack residency fixture is valid"),
        ))
    }

    fn cell_binding_create() -> CellBindingCreate {
        CellBindingCreate {
            tenant_id: "ten_alpha".to_string(),
            region: region("region-alpha", RegionJurisdiction::Other),
            residency_class: residency_class(),
            az: "region-alpha-a".to_string(),
            cell_id: "cell-region-alpha-a-001".to_string(),
            tier: CellTier::Pooled,
            hsm_partition_ref: "hsm/region-alpha/cell-001".to_string(),
        }
    }

    #[test]
    fn binding_captures_region_residency_tier_and_hsm_partition() {
        let binding = CellBinding::new(cell_binding_create()).expect("cell binding should build");

        assert_eq!(binding.region, "region-alpha");
        assert_eq!(binding.residency_class.value, residency_class());
        assert_eq!(binding.tier.value, CellTier::Pooled);
        assert_eq!(binding.schema_version.value, CELL_BINDING_SCHEMA_VERSION);
    }

    #[test]
    fn router_keeps_tenant_cell_binding_immutable() {
        let mut router = CellRouter::default();
        router
            .bind(cell_binding_create())
            .expect("first binding succeeds");

        let error = router
            .bind(CellBindingCreate {
                az: "region-alpha-b".to_string(),
                cell_id: "cell-region-alpha-b-001".to_string(),
                ..cell_binding_create()
            })
            .expect_err("tenant cannot be rebound to another cell");

        assert_eq!(error, CellError::AlreadyBound);
    }

    #[test]
    fn rejects_az_outside_declared_region() {
        let error = CellBinding::new(CellBindingCreate {
            az: "region-beta-a".to_string(),
            ..cell_binding_create()
        })
        .expect_err("AZ must belong to declared region");

        assert_eq!(error, CellError::AzRegionMismatch);
    }

    #[test]
    fn rejects_residency_region_mismatch() {
        let error = CellBinding::new(CellBindingCreate {
            region: region("region-beta", RegionJurisdiction::Other),
            az: "region-beta-a".to_string(),
            ..cell_binding_create()
        })
        .expect_err("residency fixture only permits the primary region");

        assert_eq!(error, CellError::ResidencyRegionMismatch);
    }

    #[test]
    fn requires_per_cell_hsm_partition_reference() {
        let error = CellBinding::new(CellBindingCreate {
            hsm_partition_ref: "".to_string(),
            ..cell_binding_create()
        })
        .expect_err("per-cell HSM partition is mandatory");

        assert_eq!(error, CellError::EmptyHsmPartition);
    }
}
