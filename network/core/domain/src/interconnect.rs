use crate::bgp::{BgpSession, BgpSessionCreate, advertised_prefixes, bgp_sessions};
use crate::error::CloudNetworkError;
use crate::identifier::{InterconnectPartnerId, InterconnectPortId};
use crate::route::RouteDestination;
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

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PeeringLocation {
    pub value: String, // data_class: PUBLIC
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DirectInterconnectState {
    Creating,
    Provisioned,
    Suspended,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterconnectPartnerCreate {
    pub id: String,                     // data_class: INTERNAL_ONLY
    pub name: String,                   // data_class: PUBLIC
    pub region: String,                 // data_class: PUBLIC
    pub peering_locations: Vec<String>, // data_class: PUBLIC
    pub per_link_sla_basis_points: u16, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterconnectPartner {
    pub id: Classified<InterconnectPartnerId>, // data_class: INTERNAL_ONLY
    pub name: Classified<String>,              // data_class: PUBLIC
    pub region: Classified<RegionCode>,        // data_class: PUBLIC
    pub peering_locations: Classified<Vec<PeeringLocation>>, // data_class: PUBLIC
    pub per_link_sla_basis_points: Classified<u16>, // data_class: PUBLIC
    pub schema_version: Classified<u32>,       // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectInterconnectCreate {
    pub resource_id: String,                 // data_class: INTERNAL_ONLY
    pub tenant_id: String,                   // data_class: INTERNAL_ONLY
    pub region: String,                      // data_class: PUBLIC
    pub partner_id: String,                  // data_class: INTERNAL_ONLY
    pub peering_location: String,            // data_class: PUBLIC
    pub physical_port_id: String,            // data_class: INTERNAL_ONLY
    pub vlan_tag: u16,                       // data_class: INTERNAL_ONLY
    pub bandwidth_mbps: u32,                 // data_class: PUBLIC
    pub redundant_port_count: u8,            // data_class: PUBLIC
    pub bgp_sessions: Vec<BgpSessionCreate>, // data_class: INTERNAL_ONLY
    pub advertised_prefixes: Vec<String>,    // data_class: INTERNAL_ONLY
    pub per_link_sla_basis_points: u16,      // data_class: PUBLIC
    pub state: DirectInterconnectState,      // data_class: PUBLIC
    pub data_class: DataClass,               // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,       // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectInterconnect {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub partner_id: Classified<InterconnectPartnerId>, // data_class: INTERNAL_ONLY
    pub peering_location: Classified<PeeringLocation>, // data_class: PUBLIC
    pub physical_port_id: Classified<InterconnectPortId>, // data_class: INTERNAL_ONLY
    pub vlan_tag: Classified<u16>,           // data_class: INTERNAL_ONLY
    pub bandwidth_mbps: Classified<u32>,     // data_class: PUBLIC
    pub redundant_port_count: Classified<u8>, // data_class: PUBLIC
    pub bgp_sessions: Classified<Vec<BgpSession>>, // data_class: INTERNAL_ONLY
    pub advertised_prefixes: Classified<Vec<RouteDestination>>, // data_class: INTERNAL_ONLY
    pub per_link_sla_basis_points: Classified<u16>, // data_class: PUBLIC
    pub state: Classified<DirectInterconnectState>, // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

impl PeeringLocation {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        canonical_location(value.into()).map(|value| Self { value })
    }
}

impl InterconnectPartner {
    pub fn new(input: InterconnectPartnerCreate) -> Result<Self, CloudNetworkError> {
        let id = InterconnectPartnerId::new(input.id)?;
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        validate_interconnect_sla(input.per_link_sla_basis_points)?;
        if input.name.trim().is_empty() {
            return Err(CloudNetworkError::InvalidInterconnectPartnerId);
        }
        let peering_locations = peering_locations(input.peering_locations)?;
        Ok(Self {
            id: internal(id),
            name: public(input.name),
            region: public(region),
            peering_locations: public(peering_locations),
            per_link_sla_basis_points: public(input.per_link_sla_basis_points),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

impl DirectInterconnect {
    pub fn new(
        known_partners: &BTreeMap<InterconnectPartnerId, InterconnectPartner>,
        input: DirectInterconnectCreate,
    ) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != DirectInterconnectState::Creating {
            return Err(CloudNetworkError::InvalidInterconnectState);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::DirectInterconnect,
        )?;
        let partner_id = InterconnectPartnerId::new(input.partner_id)?;
        let partner = known_partners
            .get(&partner_id)
            .ok_or(CloudNetworkError::UnknownInterconnectPartner)?;
        if partner.region.value != region {
            return Err(CloudNetworkError::ResourceRegionMismatch);
        }
        validate_regional_interconnect_diversity(known_partners, &region)?;
        let peering_location = PeeringLocation::new(input.peering_location)?;
        if !partner.peering_locations.value.contains(&peering_location) {
            return Err(CloudNetworkError::InvalidPeeringLocation);
        }
        validate_interconnect_sla(input.per_link_sla_basis_points)?;
        if input.bandwidth_mbps == 0 {
            return Err(CloudNetworkError::InvalidBandwidth);
        }
        if input.redundant_port_count < 2 {
            return Err(CloudNetworkError::InterconnectRedundancyRequired);
        }
        if !(1..=4094).contains(&input.vlan_tag) {
            return Err(CloudNetworkError::InvalidVlanTag);
        }
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            partner_id: internal(partner_id),
            peering_location: public(peering_location),
            physical_port_id: internal(InterconnectPortId::new(input.physical_port_id)?),
            vlan_tag: internal(input.vlan_tag),
            bandwidth_mbps: public(input.bandwidth_mbps),
            redundant_port_count: public(input.redundant_port_count),
            bgp_sessions: internal(bgp_sessions(input.bgp_sessions)?),
            advertised_prefixes: internal(advertised_prefixes(input.advertised_prefixes)?),
            per_link_sla_basis_points: public(input.per_link_sla_basis_points),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

fn peering_locations(input: Vec<String>) -> Result<Vec<PeeringLocation>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::InvalidPeeringLocation);
    }
    let mut seen = BTreeSet::new();
    let mut locations = Vec::with_capacity(input.len());
    for location in input {
        let location = PeeringLocation::new(location)?;
        if !seen.insert(location.clone()) {
            return Err(CloudNetworkError::InvalidPeeringLocation);
        }
        locations.push(location);
    }
    Ok(locations)
}

fn validate_interconnect_sla(value: u16) -> Result<(), CloudNetworkError> {
    if (9_999..=10_000).contains(&value) {
        Ok(())
    } else {
        Err(CloudNetworkError::InterconnectSlaRequired)
    }
}

fn validate_regional_interconnect_diversity(
    known_partners: &BTreeMap<InterconnectPartnerId, InterconnectPartner>,
    region: &RegionCode,
) -> Result<(), CloudNetworkError> {
    let regional_partners = known_partners
        .values()
        .filter(|partner| partner.region.value == *region)
        .count();
    if regional_partners >= 2 {
        Ok(())
    } else {
        Err(CloudNetworkError::RegionalInterconnectDiversityRequired)
    }
}

fn canonical_location(value: String) -> Result<String, CloudNetworkError> {
    if (3..=96).contains(&value.len())
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Ok(value)
    } else {
        Err(CloudNetworkError::InvalidPeeringLocation)
    }
}
