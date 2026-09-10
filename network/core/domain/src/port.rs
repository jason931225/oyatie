use crate::cdn::{CdnDistribution, CdnDistributionCreate};
use crate::ddos::{DdosProtection, DdosProtectionCreate};
use crate::dns::{DnsZone, DnsZoneCreate};
use crate::error::CloudNetworkError;
use crate::flow_anomaly::{FlowAnomalyEvent, FlowAnomalySeverity};
use crate::interconnect::{
    DirectInterconnect, DirectInterconnectCreate, InterconnectPartner, InterconnectPartnerCreate,
};
use crate::load_balancer::{LoadBalancer, LoadBalancerCreate};
use crate::mesh::{ServiceMeshCell, ServiceMeshCellCreate};
use crate::provider::NetworkProviderKind;
use crate::provider::direct_interconnect::{
    NetworkProviderDirectInterconnectCreateRequest, NetworkProviderDirectInterconnectError,
    NetworkProviderDirectInterconnectReceipt,
};
use crate::provider::dns_zone::{
    NetworkProviderDnsZoneCreateRequest, NetworkProviderDnsZoneError, NetworkProviderDnsZoneReceipt,
};
use crate::provider::load_balancer::{
    NetworkProviderLoadBalancerCreateRequest, NetworkProviderLoadBalancerError,
    NetworkProviderLoadBalancerReceipt,
};
use crate::provider::vpc::{
    NetworkProviderVpcCreateRequest, NetworkProviderVpcError, NetworkProviderVpcReceipt,
};
use crate::vpc::{Subnet, SubnetCreate, Vpc, VpcCreate};
pub trait NetworkProviderVpcPort {
    fn provider_kind(&self) -> NetworkProviderKind;

    fn create_vpc(
        &self,
        input: NetworkProviderVpcCreateRequest,
    ) -> Result<NetworkProviderVpcReceipt, NetworkProviderVpcError>;
}

pub trait NetworkProviderLoadBalancerPort {
    fn provider_kind(&self) -> NetworkProviderKind;

    fn create_load_balancer(
        &self,
        input: NetworkProviderLoadBalancerCreateRequest,
    ) -> Result<NetworkProviderLoadBalancerReceipt, NetworkProviderLoadBalancerError>;
}

pub trait NetworkProviderDnsZonePort {
    fn provider_kind(&self) -> NetworkProviderKind;

    fn create_dns_zone(
        &self,
        input: NetworkProviderDnsZoneCreateRequest,
    ) -> Result<NetworkProviderDnsZoneReceipt, NetworkProviderDnsZoneError>;
}

pub trait NetworkProviderDirectInterconnectPort {
    fn provider_kind(&self) -> NetworkProviderKind;

    fn create_direct_interconnect(
        &self,
        input: NetworkProviderDirectInterconnectCreateRequest,
    ) -> Result<NetworkProviderDirectInterconnectReceipt, NetworkProviderDirectInterconnectError>;
}

pub trait NetworkRepo {
    fn create_vpc(&mut self, input: VpcCreate) -> Result<Vpc, CloudNetworkError>;
    fn add_subnet(&mut self, input: SubnetCreate) -> Result<Subnet, CloudNetworkError>;
    fn create_load_balancer(
        &mut self,
        input: LoadBalancerCreate,
    ) -> Result<LoadBalancer, CloudNetworkError>;
    fn create_dns_zone(&mut self, input: DnsZoneCreate) -> Result<DnsZone, CloudNetworkError>;
    fn create_cdn_distribution(
        &mut self,
        input: CdnDistributionCreate,
    ) -> Result<CdnDistribution, CloudNetworkError>;
    fn add_interconnect_partner(
        &mut self,
        input: InterconnectPartnerCreate,
    ) -> Result<InterconnectPartner, CloudNetworkError>;
    fn create_direct_interconnect(
        &mut self,
        input: DirectInterconnectCreate,
    ) -> Result<DirectInterconnect, CloudNetworkError>;
    fn create_ddos_protection(
        &mut self,
        input: DdosProtectionCreate,
    ) -> Result<DdosProtection, CloudNetworkError>;
    fn create_service_mesh_cell(
        &mut self,
        input: ServiceMeshCellCreate,
    ) -> Result<ServiceMeshCell, CloudNetworkError>;
    fn record_flow_anomaly(
        &mut self,
        id: String,
        vpc_id: String,
        severity: FlowAnomalySeverity,
        flow_pattern: String,
        detected_at_epoch_seconds: u64,
    ) -> Result<FlowAnomalyEvent, CloudNetworkError>;
}
