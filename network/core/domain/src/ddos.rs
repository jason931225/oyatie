use crate::cdn::CdnDistribution;
use crate::dns::DnsZone;
use crate::error::{CloudNetworkError, map_resource_error};
use crate::load_balancer::LoadBalancer;
use crate::reference::{OnCallGroupRef, RunbookRef};
use crate::tenancy::{resource_id_for, validate_tenant_id};
use crate::vpc::Vpc;
use crate::{NETWORK_SCHEMA_VERSION, internal, public, public_metadata_class};
use cell_region::RegionCode;
use compute_resource::ResourceId;
use compute_resource::ResourceKind;
use data_boundary_kernel::Classified;
use data_boundary_kernel::DataClass;
use data_boundary_kernel::PrivacyDataClass;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DdosProtectionState {
    Creating,
    Active,
    Suspended,
    Deleting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProtectedResourceKind {
    Vpc,
    LoadBalancer,
    DnsZone,
    CdnDistribution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedResourceRef {
    pub resource_id: ResourceId,     // data_class: INTERNAL_ONLY
    pub kind: ProtectedResourceKind, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DdosProtectionCreate {
    pub resource_id: String,                 // data_class: INTERNAL_ONLY
    pub tenant_id: String,                   // data_class: INTERNAL_ONLY
    pub region: String,                      // data_class: PUBLIC
    pub protected_resource_ids: Vec<String>, // data_class: INTERNAL_ONLY
    pub scrubbing_regions: Vec<String>,      // data_class: PUBLIC
    pub line_rate_scrubbing: bool,           // data_class: PUBLIC
    pub always_on: bool,                     // data_class: PUBLIC
    pub mitigation_runbook_ref: String,      // data_class: INTERNAL_ONLY
    pub oncall_group_ref: String,            // data_class: INTERNAL_ONLY
    pub state: DdosProtectionState,          // data_class: PUBLIC
    pub data_class: DataClass,               // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,       // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DdosProtection {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub protected_resources: Classified<Vec<ProtectedResourceRef>>, // data_class: INTERNAL_ONLY
    pub scrubbing_regions: Classified<Vec<RegionCode>>, // data_class: PUBLIC
    pub line_rate_scrubbing: Classified<bool>, // data_class: PUBLIC
    pub always_on: Classified<bool>,         // data_class: PUBLIC
    pub mitigation_runbook_ref: Classified<RunbookRef>, // data_class: INTERNAL_ONLY
    pub oncall_group_ref: Classified<OnCallGroupRef>, // data_class: INTERNAL_ONLY
    pub state: Classified<DdosProtectionState>, // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

impl DdosProtection {
    pub fn new(
        vpcs: &BTreeMap<ResourceId, Vpc>,
        load_balancers: &BTreeMap<ResourceId, LoadBalancer>,
        dns_zones: &BTreeMap<ResourceId, DnsZone>,
        cdn_distributions: &BTreeMap<ResourceId, CdnDistribution>,
        input: DdosProtectionCreate,
    ) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != DdosProtectionState::Creating {
            return Err(CloudNetworkError::InvalidDdosState);
        }
        if !input.line_rate_scrubbing {
            return Err(CloudNetworkError::LineRateScrubbingRequired);
        }
        if !input.always_on {
            return Err(CloudNetworkError::DdosAlwaysOnRequired);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::DdosProtection,
        )?;
        let protected_resources = protected_resources(
            input.protected_resource_ids,
            vpcs,
            load_balancers,
            dns_zones,
            cdn_distributions,
            &input.tenant_id,
            &region,
        )?;
        let scrubbing_regions = scrubbing_regions(input.scrubbing_regions, &region)?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            protected_resources: internal(protected_resources),
            scrubbing_regions: public(scrubbing_regions),
            line_rate_scrubbing: public(input.line_rate_scrubbing),
            always_on: public(input.always_on),
            mitigation_runbook_ref: internal(RunbookRef::new(input.mitigation_runbook_ref)?),
            oncall_group_ref: internal(OnCallGroupRef::new(input.oncall_group_ref)?),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

fn protected_resources(
    input: Vec<String>,
    vpcs: &BTreeMap<ResourceId, Vpc>,
    load_balancers: &BTreeMap<ResourceId, LoadBalancer>,
    dns_zones: &BTreeMap<ResourceId, DnsZone>,
    cdn_distributions: &BTreeMap<ResourceId, CdnDistribution>,
    tenant_id: &str,
    region: &RegionCode,
) -> Result<Vec<ProtectedResourceRef>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::UnknownProtectedResource);
    }
    let mut seen = BTreeSet::new();
    let mut protected = Vec::with_capacity(input.len());
    for resource in input {
        let resource_id =
            ResourceId::new(resource).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        if !seen.insert(resource_id.clone()) {
            return Err(CloudNetworkError::DuplicateProtectedResource);
        }
        validate_resource_scope(&resource_id, tenant_id, region)?;
        let kind = match resource_id
            .kind_label()
            .map_err(map_resource_error)?
            .as_str()
        {
            "vpc" => {
                vpcs.get(&resource_id)
                    .ok_or(CloudNetworkError::UnknownProtectedResource)?;
                ProtectedResourceKind::Vpc
            }
            "lb-v4" | "lb-v7" => {
                load_balancers
                    .get(&resource_id)
                    .ok_or(CloudNetworkError::UnknownProtectedResource)?;
                ProtectedResourceKind::LoadBalancer
            }
            "dns-zone" => {
                dns_zones
                    .get(&resource_id)
                    .ok_or(CloudNetworkError::UnknownProtectedResource)?;
                ProtectedResourceKind::DnsZone
            }
            "cdn-distribution" => {
                cdn_distributions
                    .get(&resource_id)
                    .ok_or(CloudNetworkError::UnknownProtectedResource)?;
                ProtectedResourceKind::CdnDistribution
            }
            _ => return Err(CloudNetworkError::UnknownProtectedResource),
        };
        protected.push(ProtectedResourceRef { resource_id, kind });
    }
    Ok(protected)
}

fn scrubbing_regions(
    input: Vec<String>,
    home_region: &RegionCode,
) -> Result<Vec<RegionCode>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::ScrubbingRegionRequired);
    }
    let mut seen = BTreeSet::new();
    let mut regions = Vec::with_capacity(input.len());
    for region in input {
        let region = RegionCode::new(region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        if !seen.insert(region.clone()) {
            return Err(CloudNetworkError::ScrubbingRegionRequired);
        }
        regions.push(region);
    }
    if !seen.contains(home_region) {
        return Err(CloudNetworkError::ScrubbingRegionRequired);
    }
    Ok(regions)
}

pub(crate) fn validate_resource_scope(
    resource_id: &ResourceId,
    tenant_id: &str,
    region: &RegionCode,
) -> Result<(), CloudNetworkError> {
    if resource_id.tenant_id().map_err(map_resource_error)? != tenant_id {
        return Err(CloudNetworkError::ResourceTenantMismatch);
    }
    if resource_id.region().map_err(map_resource_error)? != *region {
        return Err(CloudNetworkError::ResourceRegionMismatch);
    }
    Ok(())
}
