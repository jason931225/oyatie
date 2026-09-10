use crate::cidr::{Ipv4Cidr, Ipv6Cidr, ipv4_contains, ipv6_contains};
use crate::error::CloudNetworkError;
use crate::load_balancer::security_groups;
use crate::route::{RouteTable, RouteTableCreate};
use crate::security_group::{SecurityGroup, SecurityGroupCreate};
use crate::tenancy::{resource_id_for, validate_az_region, validate_tenant_id};
use crate::{NETWORK_SCHEMA_VERSION, internal, public, public_metadata_class};
use cell_region::AzCode;
use cell_region::RegionCode;
use compute_resource::ResourceId;
use compute_resource::ResourceKind;
use data_boundary_kernel::Classified;
use data_boundary_kernel::DataClass;
use data_boundary_kernel::PrivacyDataClass;
use network_residency::ResidencyClass;
use network_residency::residency_class_allows_home_region_label;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum VpcState {
    Creating,
    Active,
    Suspended,
    Deleting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SubnetState {
    Creating,
    Active,
    Draining,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VpcCreate {
    pub resource_id: String,                       // data_class: INTERNAL_ONLY
    pub tenant_id: String,                         // data_class: INTERNAL_ONLY
    pub region: String,                            // data_class: PUBLIC
    pub cidr_v4: String,                           // data_class: PUBLIC
    pub cidr_v6: String,                           // data_class: PUBLIC
    pub flow_logs_enabled: bool,                   // data_class: PUBLIC
    pub route_table: RouteTableCreate,             // data_class: INTERNAL_ONLY
    pub security_groups: Vec<SecurityGroupCreate>, // data_class: INTERNAL_ONLY
    pub residency: ResidencyClass,                 // data_class: INTERNAL_ONLY
    pub state: VpcState,                           // data_class: PUBLIC
    pub data_class: DataClass,                     // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,             // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vpc {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub cidr_v4: Classified<Ipv4Cidr>,       // data_class: PUBLIC
    pub cidr_v6: Classified<Ipv6Cidr>,       // data_class: PUBLIC
    pub flow_logs_enabled: Classified<bool>, // data_class: PUBLIC
    pub route_table: Classified<RouteTable>, // data_class: INTERNAL_ONLY
    pub security_groups: Classified<Vec<SecurityGroup>>, // data_class: INTERNAL_ONLY
    pub residency: Classified<ResidencyClass>, // data_class: INTERNAL_ONLY
    pub state: Classified<VpcState>,         // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubnetCreate {
    pub resource_id: String,           // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub vpc_id: String,                // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub az: String,                    // data_class: PUBLIC
    pub cidr_v4: String,               // data_class: PUBLIC
    pub cidr_v6: String,               // data_class: PUBLIC
    pub public_ip_on_launch: bool,     // data_class: PUBLIC
    pub state: SubnetState,            // data_class: PUBLIC
    pub data_class: DataClass,         // data_class: PUBLIC
    pub created_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subnet {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub vpc_id: Classified<ResourceId>,      // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub az: Classified<AzCode>,              // data_class: PUBLIC
    pub cidr_v4: Classified<Ipv4Cidr>,       // data_class: PUBLIC
    pub cidr_v6: Classified<Ipv6Cidr>,       // data_class: PUBLIC
    pub public_ip_on_launch: Classified<bool>, // data_class: PUBLIC
    pub state: Classified<SubnetState>,      // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

impl Vpc {
    pub fn new(input: VpcCreate) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != VpcState::Creating {
            return Err(CloudNetworkError::InvalidVpcState);
        }
        if !input.flow_logs_enabled {
            return Err(CloudNetworkError::FlowLogsRequired);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        if !residency_class_allows_home_region_label(&input.residency, &region.value) {
            return Err(CloudNetworkError::ResourceRegionMismatch);
        }
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::Vpc,
        )?;
        let cidr_v4 = Ipv4Cidr::new(input.cidr_v4)?;
        let cidr_v6 = Ipv6Cidr::new(input.cidr_v6)?;
        let route_table = RouteTable::new(input.route_table)?;
        let security_groups = security_groups(input.security_groups)?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            cidr_v4: public(cidr_v4),
            cidr_v6: public(cidr_v6),
            flow_logs_enabled: public(input.flow_logs_enabled),
            route_table: internal(route_table),
            security_groups: internal(security_groups),
            residency: internal(input.residency),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

impl Subnet {
    pub fn new(vpc: &Vpc, input: SubnetCreate) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != SubnetState::Creating {
            return Err(CloudNetworkError::InvalidSubnetState);
        }
        let vpc_id =
            ResourceId::new(input.vpc_id).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        if vpc_id != vpc.resource_id.value {
            return Err(CloudNetworkError::UnknownVpc);
        }
        if input.tenant_id != vpc.tenant_id.value {
            return Err(CloudNetworkError::ResourceTenantMismatch);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        if region != vpc.region.value {
            return Err(CloudNetworkError::ResourceRegionMismatch);
        }
        let az = AzCode::new(input.az).map_err(|_| CloudNetworkError::InvalidAzCode)?;
        validate_az_region(&az, &region)?;
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::Subnet,
        )?;
        let cidr_v4 = Ipv4Cidr::new(input.cidr_v4)?;
        let cidr_v6 = Ipv6Cidr::new(input.cidr_v6)?;
        if !ipv4_contains(&vpc.cidr_v4.value, &cidr_v4)?
            || !ipv6_contains(&vpc.cidr_v6.value, &cidr_v6)?
        {
            return Err(CloudNetworkError::SubnetOutsideVpc);
        }
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            vpc_id: internal(vpc_id),
            region: public(region),
            az: public(az),
            cidr_v4: public(cidr_v4),
            cidr_v6: public(cidr_v6),
            public_ip_on_launch: public(input.public_ip_on_launch),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}
