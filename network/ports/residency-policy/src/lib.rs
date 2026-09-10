//! Agreed cross-owner surface for residency-policy values and predicates.
//!
//! The port intentionally excludes residency registries, tenant bindings,
//! change plans, transfer permits, jurisdiction inference, and data-class
//! conversion.
//!
//! The legacy residency core remains the defining crate until a dedicated
//! Network structural lane decomposes it.

#![forbid(unsafe_code)]

pub use data_classification::Classified;
pub use network_residency::{
    PerPackResidency, PerPackResidencyCreate, RegionJurisdiction, RegionRef, RegionRefCreate,
    RegulatorOverlay, RegulatorOverlayCreate, ResidencyClass, ResidencyError,
    parse_residency_class_label, residency_class_allows_home_region_label,
};
