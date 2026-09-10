use crate::cdn::{CdnDistribution, CdnDistributionCreate};
use crate::cidr::{ipv4_overlaps, ipv6_overlaps};
use crate::ddos::{DdosProtection, DdosProtectionCreate};
use crate::dns::{DnsZone, DnsZoneCreate};
use crate::error::CloudNetworkError;
use crate::flow_anomaly::{FlowAnomalyEvent, FlowAnomalySeverity};
use crate::identifier::{FlowAnomalyId, InterconnectPartnerId, MeshId};
use crate::interconnect::{
    DirectInterconnect, DirectInterconnectCreate, InterconnectPartner, InterconnectPartnerCreate,
};
use crate::load_balancer::{LoadBalancer, LoadBalancerCreate};
use crate::mesh::{ServiceMeshCell, ServiceMeshCellCreate};
use crate::port::NetworkRepo;
use crate::vpc::{Subnet, SubnetCreate, Vpc, VpcCreate};
use crate::{NETWORK_SCHEMA_VERSION, internal, public};
use compute_resource::ResourceId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CloudNetworkCatalog {
    vpcs: BTreeMap<ResourceId, Vpc>,
    subnets: BTreeMap<ResourceId, Subnet>,
    load_balancers: BTreeMap<ResourceId, LoadBalancer>,
    dns_zones: BTreeMap<ResourceId, DnsZone>,
    cdn_distributions: BTreeMap<ResourceId, CdnDistribution>,
    interconnect_partners: BTreeMap<InterconnectPartnerId, InterconnectPartner>,
    direct_interconnects: BTreeMap<ResourceId, DirectInterconnect>,
    ddos_protections: BTreeMap<ResourceId, DdosProtection>,
    service_meshes: BTreeMap<MeshId, ServiceMeshCell>,
    anomalies: BTreeMap<FlowAnomalyId, FlowAnomalyEvent>,
}

impl NetworkRepo for CloudNetworkCatalog {
    fn create_vpc(&mut self, input: VpcCreate) -> Result<Vpc, CloudNetworkError> {
        let vpc = Vpc::new(input)?;
        if self.vpcs.contains_key(&vpc.resource_id.value) {
            return Err(CloudNetworkError::DuplicateVpc);
        }
        self.vpcs.insert(vpc.resource_id.value.clone(), vpc.clone());
        Ok(vpc)
    }

    fn add_subnet(&mut self, input: SubnetCreate) -> Result<Subnet, CloudNetworkError> {
        let vpc_id = ResourceId::new(input.vpc_id.clone())
            .map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let vpc = self
            .vpcs
            .get(&vpc_id)
            .ok_or(CloudNetworkError::UnknownVpc)?;
        let subnet = Subnet::new(vpc, input)?;
        if self.subnets.contains_key(&subnet.resource_id.value) {
            return Err(CloudNetworkError::DuplicateSubnet);
        }
        for existing in self
            .subnets
            .values()
            .filter(|existing| existing.vpc_id.value == vpc_id)
        {
            if ipv4_overlaps(&existing.cidr_v4.value, &subnet.cidr_v4.value)?
                || ipv6_overlaps(&existing.cidr_v6.value, &subnet.cidr_v6.value)?
            {
                return Err(CloudNetworkError::OverlappingSubnet);
            }
        }
        self.subnets
            .insert(subnet.resource_id.value.clone(), subnet.clone());
        Ok(subnet)
    }

    fn create_load_balancer(
        &mut self,
        input: LoadBalancerCreate,
    ) -> Result<LoadBalancer, CloudNetworkError> {
        let vpc_id = ResourceId::new(input.vpc_id.clone())
            .map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let vpc = self
            .vpcs
            .get(&vpc_id)
            .ok_or(CloudNetworkError::UnknownVpc)?;
        let load_balancer = LoadBalancer::new(vpc, &self.subnets, input)?;
        if self
            .load_balancers
            .contains_key(&load_balancer.resource_id.value)
        {
            return Err(CloudNetworkError::DuplicateLoadBalancer);
        }
        self.load_balancers.insert(
            load_balancer.resource_id.value.clone(),
            load_balancer.clone(),
        );
        Ok(load_balancer)
    }

    fn create_dns_zone(&mut self, input: DnsZoneCreate) -> Result<DnsZone, CloudNetworkError> {
        let vpc = input
            .vpc_id
            .as_ref()
            .and_then(|id| ResourceId::new(id.clone()).ok())
            .and_then(|id| self.vpcs.get(&id));
        let zone = DnsZone::new(vpc, input)?;
        if self.dns_zones.contains_key(&zone.resource_id.value) {
            return Err(CloudNetworkError::DuplicateDnsZone);
        }
        self.dns_zones
            .insert(zone.resource_id.value.clone(), zone.clone());
        Ok(zone)
    }

    fn create_cdn_distribution(
        &mut self,
        input: CdnDistributionCreate,
    ) -> Result<CdnDistribution, CloudNetworkError> {
        let distribution = CdnDistribution::new(&self.load_balancers, &self.dns_zones, input)?;
        if self
            .cdn_distributions
            .contains_key(&distribution.resource_id.value)
        {
            return Err(CloudNetworkError::DuplicateCdnDistribution);
        }
        self.cdn_distributions
            .insert(distribution.resource_id.value.clone(), distribution.clone());
        Ok(distribution)
    }

    fn add_interconnect_partner(
        &mut self,
        input: InterconnectPartnerCreate,
    ) -> Result<InterconnectPartner, CloudNetworkError> {
        let partner = InterconnectPartner::new(input)?;
        if self.interconnect_partners.contains_key(&partner.id.value) {
            return Err(CloudNetworkError::DuplicateInterconnectPartner);
        }
        self.interconnect_partners
            .insert(partner.id.value.clone(), partner.clone());
        Ok(partner)
    }

    fn create_direct_interconnect(
        &mut self,
        input: DirectInterconnectCreate,
    ) -> Result<DirectInterconnect, CloudNetworkError> {
        let interconnect = DirectInterconnect::new(&self.interconnect_partners, input)?;
        if self
            .direct_interconnects
            .contains_key(&interconnect.resource_id.value)
        {
            return Err(CloudNetworkError::DuplicateDirectInterconnect);
        }
        self.direct_interconnects
            .insert(interconnect.resource_id.value.clone(), interconnect.clone());
        Ok(interconnect)
    }

    fn create_ddos_protection(
        &mut self,
        input: DdosProtectionCreate,
    ) -> Result<DdosProtection, CloudNetworkError> {
        let protection = DdosProtection::new(
            &self.vpcs,
            &self.load_balancers,
            &self.dns_zones,
            &self.cdn_distributions,
            input,
        )?;
        if self
            .ddos_protections
            .contains_key(&protection.resource_id.value)
        {
            return Err(CloudNetworkError::DuplicateDdosProtection);
        }
        self.ddos_protections
            .insert(protection.resource_id.value.clone(), protection.clone());
        Ok(protection)
    }

    fn create_service_mesh_cell(
        &mut self,
        input: ServiceMeshCellCreate,
    ) -> Result<ServiceMeshCell, CloudNetworkError> {
        let mesh = ServiceMeshCell::new(&self.vpcs, input)?;
        if self.service_meshes.contains_key(&mesh.mesh_id.value) {
            return Err(CloudNetworkError::DuplicateServiceMesh);
        }
        self.service_meshes
            .insert(mesh.mesh_id.value.clone(), mesh.clone());
        Ok(mesh)
    }

    fn record_flow_anomaly(
        &mut self,
        id: String,
        vpc_id: String,
        severity: FlowAnomalySeverity,
        flow_pattern: String,
        detected_at_epoch_seconds: u64,
    ) -> Result<FlowAnomalyEvent, CloudNetworkError> {
        let id = FlowAnomalyId::new(id)?;
        let vpc_id = ResourceId::new(vpc_id).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let vpc = self
            .vpcs
            .get(&vpc_id)
            .ok_or(CloudNetworkError::UnknownVpc)?;
        if !vpc.flow_logs_enabled.value {
            return Err(CloudNetworkError::FlowLogsRequired);
        }
        if self.anomalies.contains_key(&id) {
            return Err(CloudNetworkError::DuplicateFlowAnomaly);
        }
        if flow_pattern.trim().is_empty() {
            return Err(CloudNetworkError::InvalidRoute);
        }
        let event = FlowAnomalyEvent {
            id: internal(id.clone()),
            tenant_id: internal(vpc.tenant_id.value.clone()),
            vpc_id: internal(vpc_id),
            severity: public(severity),
            flow_pattern: internal(flow_pattern),
            detected_at_epoch_seconds: internal(detected_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        };
        self.anomalies.insert(id, event.clone());
        Ok(event)
    }
}

impl CloudNetworkCatalog {
    pub fn vpcs(&self) -> impl Iterator<Item = &Vpc> {
        self.vpcs.values()
    }

    pub fn subnets(&self) -> impl Iterator<Item = &Subnet> {
        self.subnets.values()
    }

    pub fn load_balancers(&self) -> impl Iterator<Item = &LoadBalancer> {
        self.load_balancers.values()
    }

    pub fn dns_zones(&self) -> impl Iterator<Item = &DnsZone> {
        self.dns_zones.values()
    }

    pub fn cdn_distributions(&self) -> impl Iterator<Item = &CdnDistribution> {
        self.cdn_distributions.values()
    }

    pub fn interconnect_partners(&self) -> impl Iterator<Item = &InterconnectPartner> {
        self.interconnect_partners.values()
    }

    pub fn direct_interconnects(&self) -> impl Iterator<Item = &DirectInterconnect> {
        self.direct_interconnects.values()
    }

    pub fn ddos_protections(&self) -> impl Iterator<Item = &DdosProtection> {
        self.ddos_protections.values()
    }

    pub fn service_meshes(&self) -> impl Iterator<Item = &ServiceMeshCell> {
        self.service_meshes.values()
    }

    pub fn anomalies(&self) -> impl Iterator<Item = &FlowAnomalyEvent> {
        self.anomalies.values()
    }
}
