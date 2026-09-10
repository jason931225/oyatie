use crate::ddos::validate_resource_scope;
use crate::dns::{DnsName, DnsZone, DnsZoneKind, dns_names};
use crate::error::CloudNetworkError;
use crate::identifier::WafPolicyId;
use crate::load_balancer::LoadBalancer;
use crate::reference::CertificateRef;
use crate::tenancy::{resource_id_for, validate_tenant_id};
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
pub enum CdnOriginKind {
    LoadBalancer,
    DnsZone,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CdnCacheMode {
    StaticAssets,
    ConsoleAssets,
    ApiEdge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CdnState {
    Creating,
    Active,
    Suspended,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdnOriginCreate {
    pub resource_id: String, // data_class: INTERNAL_ONLY
    pub kind: CdnOriginKind, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdnOrigin {
    pub resource_id: ResourceId, // data_class: INTERNAL_ONLY
    pub kind: CdnOriginKind,     // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdnDistributionCreate {
    pub resource_id: String,           // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub hostnames: Vec<String>,        // data_class: PUBLIC
    pub origins: Vec<CdnOriginCreate>, // data_class: INTERNAL_ONLY
    pub tls_certificate: String,       // data_class: INTERNAL_ONLY
    pub waf_policy: String,            // data_class: INTERNAL_ONLY
    pub cache_mode: CdnCacheMode,      // data_class: PUBLIC
    pub state: CdnState,               // data_class: PUBLIC
    pub data_class: DataClass,         // data_class: PUBLIC
    pub created_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdnDistribution {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub hostnames: Classified<Vec<DnsName>>, // data_class: PUBLIC
    pub origins: Classified<Vec<CdnOrigin>>, // data_class: INTERNAL_ONLY
    pub tls_certificate: Classified<CertificateRef>, // data_class: INTERNAL_ONLY
    pub waf_policy: Classified<WafPolicyId>, // data_class: INTERNAL_ONLY
    pub cache_mode: Classified<CdnCacheMode>, // data_class: PUBLIC
    pub state: Classified<CdnState>,         // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

impl CdnDistribution {
    pub fn new(
        known_load_balancers: &BTreeMap<ResourceId, LoadBalancer>,
        known_dns_zones: &BTreeMap<ResourceId, DnsZone>,
        input: CdnDistributionCreate,
    ) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != CdnState::Creating {
            return Err(CloudNetworkError::InvalidCdnState);
        }
        if input.tls_certificate.trim().is_empty() {
            return Err(CloudNetworkError::CdnTlsRequired);
        }
        if input.waf_policy.trim().is_empty() {
            return Err(CloudNetworkError::CdnWafRequired);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::CdnDistribution,
        )?;
        let hostnames = dns_names(input.hostnames)?;
        let origins = cdn_origins(
            input.origins,
            known_load_balancers,
            known_dns_zones,
            &input.tenant_id,
            &region,
        )?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            hostnames: public(hostnames),
            origins: internal(origins),
            tls_certificate: internal(CertificateRef::new(input.tls_certificate)?),
            waf_policy: internal(WafPolicyId::new(input.waf_policy)?),
            cache_mode: public(input.cache_mode),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

fn cdn_origins(
    input: Vec<CdnOriginCreate>,
    known_load_balancers: &BTreeMap<ResourceId, LoadBalancer>,
    known_dns_zones: &BTreeMap<ResourceId, DnsZone>,
    tenant_id: &str,
    region: &RegionCode,
) -> Result<Vec<CdnOrigin>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::InvalidCdnOrigin);
    }
    let mut seen = BTreeSet::new();
    let mut origins = Vec::with_capacity(input.len());
    for origin in input {
        let resource_id = ResourceId::new(origin.resource_id)
            .map_err(|_| CloudNetworkError::InvalidResourceId)?;
        if !seen.insert(resource_id.clone()) {
            return Err(CloudNetworkError::DuplicateCdnOrigin);
        }
        validate_resource_scope(&resource_id, tenant_id, region)?;
        match origin.kind {
            CdnOriginKind::LoadBalancer => {
                known_load_balancers
                    .get(&resource_id)
                    .ok_or(CloudNetworkError::UnknownLoadBalancer)?;
            }
            CdnOriginKind::DnsZone => {
                let zone = known_dns_zones
                    .get(&resource_id)
                    .ok_or(CloudNetworkError::UnknownDnsZone)?;
                if zone.kind.value != DnsZoneKind::Public {
                    return Err(CloudNetworkError::InvalidCdnOrigin);
                }
            }
        }
        origins.push(CdnOrigin {
            resource_id,
            kind: origin.kind,
        });
    }
    Ok(origins)
}
