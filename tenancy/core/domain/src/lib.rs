#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod design_partner_status;
pub use design_partner_status::{DesignPartnerStatus, DesignPartnerStatusError};

use data_boundary_kernel::{Classified, DataClass};
use network_residency::{ResidencyClass, residency_class_allows_home_region_label};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tenant {
    pub id: String,
    pub legal_name: Classified<String>,
    pub home_region: Classified<String>,
    pub residency_class: Classified<ResidencyClass>,
    pub regulatory_packs: Classified<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TenantError {
    InvalidTenantId,
    EmptyLegalName,
    EmptyHomeRegion,
    HomeRegionNotAllowedForResidency,
    MissingRegionalPack,
}

impl Tenant {
    pub fn new(
        id: String,
        legal_name: String,
        home_region: String,
        residency_class: ResidencyClass,
        regulatory_packs: Vec<String>,
    ) -> Result<Self, TenantError> {
        if !id.starts_with("ten_") || id.len() <= 4 {
            return Err(TenantError::InvalidTenantId);
        }
        if legal_name.trim().is_empty() {
            return Err(TenantError::EmptyLegalName);
        }
        if home_region.trim().is_empty() {
            return Err(TenantError::EmptyHomeRegion);
        }
        if !residency_class_allows_home_region_label(&residency_class, &home_region) {
            return Err(TenantError::HomeRegionNotAllowedForResidency);
        }
        if regulatory_packs.is_empty() {
            return Err(TenantError::MissingRegionalPack);
        }

        Ok(Self {
            id,
            legal_name: Classified::new(legal_name, DataClass::InternalOnly),
            home_region: Classified::new(home_region, DataClass::InternalOnly),
            residency_class: Classified::new(residency_class, DataClass::InternalOnly),
            regulatory_packs: Classified::new(regulatory_packs, DataClass::InternalOnly),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_residency::{
        PerPackResidency, PerPackResidencyCreate, RegulatorOverlay, RegulatorOverlayCreate,
    };

    fn per_pack_residency(allowed_primary_regions: Vec<&str>) -> ResidencyClass {
        let regulator_overlay = RegulatorOverlay::new(RegulatorOverlayCreate {
            regulator_refs: vec!["regulator-alpha".to_string()],
            evidence_ref: "evidence/residency-alpha".to_string(),
        })
        .expect("regulator overlay fixture is valid");

        ResidencyClass::PerPack(Box::new(
            PerPackResidency::new(PerPackResidencyCreate {
                allowed_primary_regions: allowed_primary_regions
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                allowed_replica_regions: vec!["region-replica".to_string()],
                forbidden_regions: Vec::new(),
                regulator_overlay,
            })
            .expect("per-pack residency fixture is valid"),
        ))
    }

    #[test]
    fn tenant_identity_includes_residency_class() {
        let tenant = Tenant::new(
            "ten_alpha".to_string(),
            "Alpha Tenant".to_string(),
            "region-home".to_string(),
            ResidencyClass::StrictHomeRegion,
            vec!["pack-alpha".to_string()],
        )
        .expect("home-region tenant residency is valid");

        assert_eq!(
            tenant.residency_class.value.label(),
            Some("strict_home_region")
        );
    }

    #[test]
    fn tenant_rejects_residency_home_region_mismatch() {
        let error = Tenant::new(
            "ten_beta".to_string(),
            "Beta Tenant".to_string(),
            "region-recovery".to_string(),
            ResidencyClass::StrictHomeRegion,
            vec!["pack-alpha".to_string()],
        )
        .expect_err("strict home-region tenants require home-region primary");

        assert_eq!(error, TenantError::HomeRegionNotAllowedForResidency);
    }
}
