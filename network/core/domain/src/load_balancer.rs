use crate::error::CloudNetworkError;
use crate::identifier::{TargetGroupId, WafPolicyId};
use crate::reference::CertificateRef;
use crate::security_group::{SecurityGroup, SecurityGroupCreate};
use crate::tenancy::{resource_id_for, validate_tenant_id};
use crate::vpc::{Subnet, Vpc};
use crate::{NETWORK_SCHEMA_VERSION, internal, public, public_metadata_class};
use cell_region::RegionCode;
use compute_resource::LbProtocol;
use compute_resource::ResourceId;
use compute_resource::ResourceKind;
use data_boundary_kernel::Classified;
use data_boundary_kernel::DataClass;
use data_boundary_kernel::PrivacyDataClass;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LbKind {
    L4Tcp,
    L4Udp,
    L7Http,
    L7Grpc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LbState {
    Creating,
    Active,
    Draining,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Listener {
    pub port: u16,                               // data_class: PUBLIC
    pub target_group_id: TargetGroupId,          // data_class: INTERNAL_ONLY
    pub tls_certificate: Option<CertificateRef>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetGroup {
    pub id: TargetGroupId,                 // data_class: INTERNAL_ONLY
    pub subnet_ids: Vec<ResourceId>,       // data_class: INTERNAL_ONLY
    pub health_check_path: Option<String>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MtlsClientPolicy {
    RequireVerifiedClientCert,
    ForwardVerifiedIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MtlsConfig {
    pub ca_bundle_ref: CertificateRef,   // data_class: INTERNAL_ONLY
    pub client_policy: MtlsClientPolicy, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListenerCreate {
    pub port: u16,                       // data_class: PUBLIC
    pub target_group_id: String,         // data_class: INTERNAL_ONLY
    pub tls_certificate: Option<String>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetGroupCreate {
    pub id: String,                        // data_class: INTERNAL_ONLY
    pub subnet_ids: Vec<String>,           // data_class: INTERNAL_ONLY
    pub health_check_path: Option<String>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MtlsConfigCreate {
    pub ca_bundle_ref: String,           // data_class: INTERNAL_ONLY
    pub client_policy: MtlsClientPolicy, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadBalancerCreate {
    pub resource_id: String,                   // data_class: INTERNAL_ONLY
    pub tenant_id: String,                     // data_class: INTERNAL_ONLY
    pub vpc_id: String,                        // data_class: INTERNAL_ONLY
    pub region: String,                        // data_class: PUBLIC
    pub kind: LbKind,                          // data_class: PUBLIC
    pub listeners: Vec<ListenerCreate>,        // data_class: INTERNAL_ONLY
    pub target_groups: Vec<TargetGroupCreate>, // data_class: INTERNAL_ONLY
    pub mtls: Option<MtlsConfigCreate>,        // data_class: INTERNAL_ONLY
    pub waf_policy: Option<String>,            // data_class: INTERNAL_ONLY
    pub state: LbState,                        // data_class: PUBLIC
    pub data_class: DataClass,                 // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,         // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadBalancer {
    pub resource_id: Classified<ResourceId>, // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,       // data_class: INTERNAL_ONLY
    pub vpc_id: Classified<ResourceId>,      // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,      // data_class: PUBLIC
    pub kind: Classified<LbKind>,            // data_class: PUBLIC
    pub listeners: Classified<Vec<Listener>>, // data_class: INTERNAL_ONLY
    pub target_groups: Classified<Vec<TargetGroup>>, // data_class: INTERNAL_ONLY
    pub mtls: Classified<Option<MtlsConfig>>, // data_class: INTERNAL_ONLY
    pub waf_policy: Classified<Option<WafPolicyId>>, // data_class: INTERNAL_ONLY
    pub state: Classified<LbState>,          // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,     // data_class: PUBLIC
}

impl LbKind {
    pub const fn resource_protocol(self) -> LbProtocol {
        match self {
            Self::L4Tcp | Self::L4Udp => LbProtocol::L4,
            Self::L7Http | Self::L7Grpc => LbProtocol::L7,
        }
    }

    pub const fn requires_tls(self) -> bool {
        matches!(self, Self::L7Http | Self::L7Grpc)
    }

    pub const fn requires_mtls(self) -> bool {
        matches!(self, Self::L7Grpc)
    }
}

impl LoadBalancer {
    pub fn new(
        vpc: &Vpc,
        known_subnets: &BTreeMap<ResourceId, Subnet>,
        input: LoadBalancerCreate,
    ) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != LbState::Creating {
            return Err(CloudNetworkError::InvalidLbState);
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
        let resource_id = resource_id_for(
            &input.resource_id,
            &input.tenant_id,
            &region,
            ResourceKind::LoadBalancer(input.kind.resource_protocol()),
        )?;
        if input.kind.requires_mtls() && input.mtls.is_none() {
            return Err(CloudNetworkError::GrpcRequiresMtls);
        }
        let target_groups =
            target_groups(input.target_groups, known_subnets, &vpc.resource_id.value)?;
        let listeners = listeners(input.listeners, &target_groups, input.kind)?;
        let mtls = input
            .mtls
            .map(|mtls| {
                Ok(MtlsConfig {
                    ca_bundle_ref: CertificateRef::new(mtls.ca_bundle_ref)?,
                    client_policy: mtls.client_policy,
                })
            })
            .transpose()?;
        let waf_policy = input.waf_policy.map(WafPolicyId::new).transpose()?;
        Ok(Self {
            resource_id: internal(resource_id),
            tenant_id: internal(input.tenant_id),
            vpc_id: internal(vpc_id),
            region: public(region),
            kind: public(input.kind),
            listeners: internal(listeners),
            target_groups: internal(target_groups),
            mtls: internal(mtls),
            waf_policy: internal(waf_policy),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

pub(crate) fn security_groups(
    input: Vec<SecurityGroupCreate>,
) -> Result<Vec<SecurityGroup>, CloudNetworkError> {
    let mut seen = BTreeSet::new();
    let mut groups = Vec::with_capacity(input.len());
    for group in input {
        let group = SecurityGroup::new(group)?;
        if !seen.insert(group.id.clone()) {
            return Err(CloudNetworkError::DuplicateSecurityGroup);
        }
        groups.push(group);
    }
    Ok(groups)
}

pub(crate) fn target_groups(
    input: Vec<TargetGroupCreate>,
    known_subnets: &BTreeMap<ResourceId, Subnet>,
    vpc_id: &ResourceId,
) -> Result<Vec<TargetGroup>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::InvalidTargetGroupId);
    }
    let mut seen = BTreeSet::new();
    let mut groups = Vec::with_capacity(input.len());
    for group in input {
        let id = TargetGroupId::new(group.id)?;
        if !seen.insert(id.clone()) {
            return Err(CloudNetworkError::DuplicateTargetGroup);
        }
        let mut subnet_ids = Vec::with_capacity(group.subnet_ids.len());
        if group.subnet_ids.is_empty() {
            return Err(CloudNetworkError::UnknownSubnet);
        }
        for subnet_id in group.subnet_ids {
            let subnet_id =
                ResourceId::new(subnet_id).map_err(|_| CloudNetworkError::InvalidResourceId)?;
            let subnet = known_subnets
                .get(&subnet_id)
                .ok_or(CloudNetworkError::UnknownSubnet)?;
            if &subnet.vpc_id.value != vpc_id {
                return Err(CloudNetworkError::UnknownSubnet);
            }
            subnet_ids.push(subnet_id);
        }
        if group
            .health_check_path
            .as_ref()
            .is_some_and(|path| !path.starts_with('/'))
        {
            return Err(CloudNetworkError::InvalidTargetGroupId);
        }
        groups.push(TargetGroup {
            id,
            subnet_ids,
            health_check_path: group.health_check_path,
        });
    }
    Ok(groups)
}

pub(crate) fn listeners(
    input: Vec<ListenerCreate>,
    target_groups: &[TargetGroup],
    kind: LbKind,
) -> Result<Vec<Listener>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::InvalidListener);
    }
    let target_ids: BTreeSet<_> = target_groups.iter().map(|group| group.id.clone()).collect();
    let mut seen_ports = BTreeSet::new();
    let mut listeners = Vec::with_capacity(input.len());
    for listener in input {
        if listener.port == 0 || !seen_ports.insert(listener.port) {
            return Err(CloudNetworkError::DuplicateListenerPort);
        }
        let target_group_id = TargetGroupId::new(listener.target_group_id)?;
        if !target_ids.contains(&target_group_id) {
            return Err(CloudNetworkError::ListenerTargetGroupMissing);
        }
        let tls_certificate = listener
            .tls_certificate
            .map(CertificateRef::new)
            .transpose()?;
        if kind.requires_tls() && tls_certificate.is_none() {
            return Err(CloudNetworkError::L7RequiresTls);
        }
        listeners.push(Listener {
            port: listener.port,
            target_group_id,
            tls_certificate,
        });
    }
    Ok(listeners)
}
