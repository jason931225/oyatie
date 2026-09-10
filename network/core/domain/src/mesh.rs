use crate::error::CloudNetworkError;
use crate::identifier::MeshId;
use crate::reference::{AuditStreamRef, CedarPolicyRef, HealthAlarmRef};
use crate::tenancy::{validate_cell_region, validate_tenant_id};
use crate::vpc::Vpc;
use crate::{NETWORK_SCHEMA_VERSION, internal, public, public_metadata_class};
use cell_region::CellId;
use cell_region::RegionCode;
use compute_resource::ResourceId;
use data_boundary_kernel::Classified;
use data_boundary_kernel::DataClass;
use data_boundary_kernel::PrivacyDataClass;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MeshNamespace {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ServiceMeshMode {
    IstioAmbient,
    Sidecar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MeshGatewayKind {
    Envoy,
    Nginx,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ServiceMeshState {
    Creating,
    Active,
    Degraded,
    Deleting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceMeshCellCreate {
    pub mesh_id: String,               // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub cell_id: String,               // data_class: PUBLIC
    pub vpc_id: String,                // data_class: INTERNAL_ONLY
    pub namespace: String,             // data_class: INTERNAL_ONLY
    pub mode: ServiceMeshMode,         // data_class: PUBLIC
    pub edge_gateway: MeshGatewayKind, // data_class: PUBLIC
    pub mtls_everywhere: bool,         // data_class: PUBLIC
    pub ext_authz_enabled: bool,       // data_class: PUBLIC
    pub cross_cell_policy_ref: String, // data_class: INTERNAL_ONLY
    pub audit_stream_ref: String,      // data_class: INTERNAL_ONLY
    pub health_alarm_ref: String,      // data_class: INTERNAL_ONLY
    pub control_plane_replicas: u8,    // data_class: PUBLIC
    pub quarterly_upgrade_drill: bool, // data_class: PUBLIC
    pub state: ServiceMeshState,       // data_class: PUBLIC
    pub data_class: DataClass,         // data_class: PUBLIC
    pub created_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceMeshCell {
    pub mesh_id: Classified<MeshId>,          // data_class: INTERNAL_ONLY
    pub tenant_id: Classified<String>,        // data_class: INTERNAL_ONLY
    pub region: Classified<RegionCode>,       // data_class: PUBLIC
    pub cell_id: Classified<CellId>,          // data_class: PUBLIC
    pub vpc_id: Classified<ResourceId>,       // data_class: INTERNAL_ONLY
    pub namespace: Classified<MeshNamespace>, // data_class: INTERNAL_ONLY
    pub mode: Classified<ServiceMeshMode>,    // data_class: PUBLIC
    pub edge_gateway: Classified<MeshGatewayKind>, // data_class: PUBLIC
    pub mtls_everywhere: Classified<bool>,    // data_class: PUBLIC
    pub ext_authz_enabled: Classified<bool>,  // data_class: PUBLIC
    pub cross_cell_policy_ref: Classified<CedarPolicyRef>, // data_class: INTERNAL_ONLY
    pub audit_stream_ref: Classified<AuditStreamRef>, // data_class: INTERNAL_ONLY
    pub health_alarm_ref: Classified<HealthAlarmRef>, // data_class: INTERNAL_ONLY
    pub control_plane_replicas: Classified<u8>, // data_class: PUBLIC
    pub quarterly_upgrade_drill: Classified<bool>, // data_class: PUBLIC
    pub state: Classified<ServiceMeshState>,  // data_class: PUBLIC
    pub data_class: Classified<PrivacyDataClass>, // data_class: PUBLIC
    pub created_at_epoch_seconds: Classified<u64>, // data_class: INTERNAL_ONLY
    pub schema_version: Classified<u32>,      // data_class: PUBLIC
}

impl MeshNamespace {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        let value = value.into();
        if value.starts_with("mesh-")
            && value.len() > "mesh-".len()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && !value.ends_with('-')
        {
            Ok(Self { value })
        } else {
            Err(CloudNetworkError::InvalidMeshNamespace)
        }
    }
}

impl ServiceMeshCell {
    pub fn new(
        vpcs: &BTreeMap<ResourceId, Vpc>,
        input: ServiceMeshCellCreate,
    ) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != ServiceMeshState::Creating {
            return Err(CloudNetworkError::InvalidMeshState);
        }
        if input.mode != ServiceMeshMode::IstioAmbient {
            return Err(CloudNetworkError::InvalidMeshMode);
        }
        if input.edge_gateway != MeshGatewayKind::Envoy {
            return Err(CloudNetworkError::InvalidMeshGateway);
        }
        if !input.mtls_everywhere {
            return Err(CloudNetworkError::MeshMtlsRequired);
        }
        if !input.ext_authz_enabled {
            return Err(CloudNetworkError::MeshExtAuthzRequired);
        }
        if input.control_plane_replicas < 3 {
            return Err(CloudNetworkError::MeshControlPlaneReplicasRequired);
        }
        if !input.quarterly_upgrade_drill {
            return Err(CloudNetworkError::MeshUpgradeDrillRequired);
        }
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let vpc_id =
            ResourceId::new(input.vpc_id).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let vpc = vpcs.get(&vpc_id).ok_or(CloudNetworkError::UnknownVpc)?;
        if vpc.tenant_id.value != input.tenant_id {
            return Err(CloudNetworkError::ResourceTenantMismatch);
        }
        if vpc.region.value != region {
            return Err(CloudNetworkError::ResourceRegionMismatch);
        }
        let cell_id = CellId::new(input.cell_id).map_err(|_| CloudNetworkError::InvalidCellId)?;
        validate_cell_region(&cell_id, &region)?;
        let namespace = MeshNamespace::new(input.namespace)?;
        validate_mesh_namespace_cell(&namespace, &cell_id)?;
        Ok(Self {
            mesh_id: internal(MeshId::new(input.mesh_id)?),
            tenant_id: internal(input.tenant_id),
            region: public(region),
            cell_id: public(cell_id),
            vpc_id: internal(vpc_id),
            namespace: internal(namespace),
            mode: public(input.mode),
            edge_gateway: public(input.edge_gateway),
            mtls_everywhere: public(input.mtls_everywhere),
            ext_authz_enabled: public(input.ext_authz_enabled),
            cross_cell_policy_ref: internal(CedarPolicyRef::new(input.cross_cell_policy_ref)?),
            audit_stream_ref: internal(AuditStreamRef::new(input.audit_stream_ref)?),
            health_alarm_ref: internal(HealthAlarmRef::new(input.health_alarm_ref)?),
            control_plane_replicas: public(input.control_plane_replicas),
            quarterly_upgrade_drill: public(input.quarterly_upgrade_drill),
            state: public(input.state),
            data_class: public(public_metadata_class(input.data_class)?),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }
}

pub(crate) fn validate_mesh_namespace_cell(
    namespace: &MeshNamespace,
    cell_id: &CellId,
) -> Result<(), CloudNetworkError> {
    if namespace.value.contains(&cell_id.value) {
        Ok(())
    } else {
        Err(CloudNetworkError::InvalidMeshNamespace)
    }
}
