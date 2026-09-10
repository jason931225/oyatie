use crate::error::CloudNetworkError;
use crate::reference::DnssecKeyRef;
use crate::tenancy::{resource_id_for, validate_tenant_id};
use crate::vpc::Vpc;
use crate::{NETWORK_SCHEMA_VERSION, internal, public, public_metadata_class};
use cell_region::RegionCode;
use compute_resource::ResourceId;
use compute_resource::ResourceKind;
use data_boundary_kernel::Classified;
use data_boundary_kernel::DataClass;
use data_boundary_kernel::PrivacyDataClass;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DnsName {
    pub value: String, // data_class: PUBLIC
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DnsZoneKind {
    Public,
    Private,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DnsZoneState {
    Creating,
    Active,
    Suspended,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DnsZoneCreate {
    pub resource_id: String,            // data_class: INTERNAL_ONLY
    pub tenant_id: String,              // data_class: INTERNAL_ONLY
    pub region: String,                 // data_class: PUBLIC
    pub name: String,                   // data_class: PUBLIC
    pub kind: DnsZoneKind,              // data_class: PUBLIC
    pub vpc_id: Option<String>,         // data_class: INTERNAL_ONLY
    pub dnssec_key_ref: Option<String>, // data_class: INTERNAL_ONLY
    pub state: DnsZoneState,            // data_class: PUBLIC
    pub data_class: DataClass,          // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,  // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DnsZone {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub name: Classified<DnsName>,           // data_class: PUBLIC
    pub kind: Classified<DnsZoneKind>,       // data_class: PUBLIC
    pub vpc_id: Classified<Option<ResourceId>>, // data_class: INTERNAL_ONLY
    pub dnssec_key_ref: Classified<Option<DnssecKeyRef>>, // data_class: INTERNAL_ONLY
    pub state: Classified<DnsZoneState>,     // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

impl DnsName {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        let value = value.into();
        let trimmed = value.trim_end_matches('.').to_ascii_lowercase();
        if trimmed.len() < 3
            || trimmed.len() > 253
            || trimmed.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || label.starts_with('-')
                    || label.ends_with('-')
                    || !label.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            })
        {
            return Err(CloudNetworkError::InvalidDnsName);
        }
        Ok(Self { value: trimmed })
    }
}

impl DnsZone {
    pub fn new(vpc: Option<&Vpc>, input: DnsZoneCreate) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != DnsZoneState::Creating {
            return Err(CloudNetworkError::InvalidDnsZoneState);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::DnsZone,
        )?;
        let vpc_id = match (input.kind, input.vpc_id) {
            (DnsZoneKind::Private, Some(vpc_id)) => {
                let vpc_id =
                    ResourceId::new(vpc_id).map_err(|_| CloudNetworkError::InvalidResourceId)?;
                let Some(vpc) = vpc else {
                    return Err(CloudNetworkError::UnknownVpc);
                };
                if vpc_id != vpc.resource_id.value {
                    return Err(CloudNetworkError::UnknownVpc);
                }
                if vpc.tenant_id.value != input.tenant_id {
                    return Err(CloudNetworkError::ResourceTenantMismatch);
                }
                if vpc.region.value != region {
                    return Err(CloudNetworkError::ResourceRegionMismatch);
                }
                Some(vpc_id)
            }
            (DnsZoneKind::Private, None) => return Err(CloudNetworkError::PrivateZoneRequiresVpc),
            (DnsZoneKind::Public, Some(_)) => {
                return Err(CloudNetworkError::PublicZoneMustNotBindVpc);
            }
            (DnsZoneKind::Public, None) => None,
        };
        let dnssec_key_ref = input.dnssec_key_ref.map(DnssecKeyRef::new).transpose()?;
        if matches!(input.kind, DnsZoneKind::Public) && dnssec_key_ref.is_none() {
            return Err(CloudNetworkError::DnssecRequired);
        }
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            name: public(DnsName::new(input.name)?),
            kind: public(input.kind),
            vpc_id: internal(vpc_id),
            dnssec_key_ref: internal(dnssec_key_ref),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

pub(crate) fn dns_names(input: Vec<String>) -> Result<Vec<DnsName>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::InvalidDnsName);
    }
    let mut seen = BTreeSet::new();
    let mut names = Vec::with_capacity(input.len());
    for hostname in input {
        let name = DnsName::new(hostname)?;
        if !seen.insert(name.clone()) {
            return Err(CloudNetworkError::DuplicateCdnHostname);
        }
        names.push(name);
    }
    Ok(names)
}
