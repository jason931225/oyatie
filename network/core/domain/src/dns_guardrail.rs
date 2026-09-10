use crate::error::CloudNetworkError;
use crate::mesh::{MeshNamespace, validate_mesh_namespace_cell};
use crate::reference::validate_evidence_ref;
use crate::tenancy::{validate_cell_region, validate_tenant_id};
use crate::{NETWORK_SCHEMA_VERSION, internal, public};
use cell_region::CellId;
use cell_region::RegionCode;
use data_boundary_kernel::Classified;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NetworkCniProvider {
    Cilium,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum EdgeGatewayProvider {
    Envoy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CoreDnsPodMode {
    Disabled,
    Verified,
    Insecure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkDnsCellGuardrailCreate {
    pub tenant_id: String,                        // data_class: INTERNAL_ONLY
    pub region: String,                           // data_class: PUBLIC
    pub cell_id: String,                          // data_class: PUBLIC
    pub namespace: String,                        // data_class: INTERNAL_ONLY
    pub cni_provider: NetworkCniProvider,         // data_class: PUBLIC
    pub edge_gateway: EdgeGatewayProvider,        // data_class: PUBLIC
    pub default_deny_ingress: bool,               // data_class: PUBLIC
    pub default_deny_egress: bool,                // data_class: PUBLIC
    pub dns_egress_explicitly_allowed: bool,      // data_class: PUBLIC
    pub cross_cell_default_traffic_allowed: bool, // data_class: PUBLIC
    pub envoy_external_authorization: bool,       // data_class: PUBLIC
    pub envoy_failure_mode_allow: bool,           // data_class: PUBLIC
    pub mtls_required: bool,                      // data_class: PUBLIC
    pub coredns_pod_mode: CoreDnsPodMode,         // data_class: PUBLIC
    pub evidence_ref: String,                     // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkDnsCellGuardrail {
    tenant_id: Classified<String>,        // data_class: INTERNAL_ONLY
    region: Classified<RegionCode>,       // data_class: PUBLIC
    cell_id: Classified<CellId>,          // data_class: PUBLIC
    namespace: Classified<MeshNamespace>, // data_class: INTERNAL_ONLY
    cni_provider: Classified<NetworkCniProvider>, // data_class: PUBLIC
    edge_gateway: Classified<EdgeGatewayProvider>, // data_class: PUBLIC
    default_deny_ingress: Classified<bool>, // data_class: PUBLIC
    default_deny_egress: Classified<bool>, // data_class: PUBLIC
    dns_egress_explicitly_allowed: Classified<bool>, // data_class: PUBLIC
    cross_cell_default_traffic_allowed: Classified<bool>, // data_class: PUBLIC
    envoy_external_authorization: Classified<bool>, // data_class: PUBLIC
    envoy_failure_mode_allow: Classified<bool>, // data_class: PUBLIC
    mtls_required: Classified<bool>,      // data_class: PUBLIC
    coredns_pod_mode: Classified<CoreDnsPodMode>, // data_class: PUBLIC
    evidence_ref: Classified<String>,     // data_class: INTERNAL_ONLY
    schema_version: Classified<u32>,      // data_class: PUBLIC
}

impl NetworkDnsCellGuardrail {
    pub fn new(input: NetworkDnsCellGuardrailCreate) -> Result<Self, CloudNetworkError> {
        validate_tenant_id(&input.tenant_id)?;
        let region =
            RegionCode::new(input.region).map_err(|_| CloudNetworkError::InvalidResourceId)?;
        let cell_id = CellId::new(input.cell_id).map_err(|_| CloudNetworkError::InvalidCellId)?;
        validate_cell_region(&cell_id, &region)?;
        let namespace = MeshNamespace::new(input.namespace)?;
        validate_mesh_namespace_cell(&namespace, &cell_id)?;

        if !input.default_deny_ingress {
            return Err(CloudNetworkError::DefaultDenyIngressRequired);
        }
        if !input.default_deny_egress {
            return Err(CloudNetworkError::DefaultDenyEgressRequired);
        }
        if !input.dns_egress_explicitly_allowed {
            return Err(CloudNetworkError::DnsEgressExceptionRequired);
        }
        if input.cross_cell_default_traffic_allowed {
            return Err(CloudNetworkError::CrossCellDefaultTrafficForbidden);
        }
        if !input.envoy_external_authorization {
            return Err(CloudNetworkError::EnvoyExtAuthzRequired);
        }
        if input.envoy_failure_mode_allow {
            return Err(CloudNetworkError::EnvoyFailClosedRequired);
        }
        if !input.mtls_required {
            return Err(CloudNetworkError::MeshMtlsRequired);
        }
        if input.coredns_pod_mode == CoreDnsPodMode::Insecure {
            return Err(CloudNetworkError::CoreDnsInsecurePodModeForbidden);
        }
        let evidence_ref = validate_evidence_ref(input.evidence_ref)?;

        Ok(Self {
            tenant_id: internal(input.tenant_id),
            region: public(region),
            cell_id: public(cell_id),
            namespace: internal(namespace),
            cni_provider: public(input.cni_provider),
            edge_gateway: public(input.edge_gateway),
            default_deny_ingress: public(input.default_deny_ingress),
            default_deny_egress: public(input.default_deny_egress),
            dns_egress_explicitly_allowed: public(input.dns_egress_explicitly_allowed),
            cross_cell_default_traffic_allowed: public(input.cross_cell_default_traffic_allowed),
            envoy_external_authorization: public(input.envoy_external_authorization),
            envoy_failure_mode_allow: public(input.envoy_failure_mode_allow),
            mtls_required: public(input.mtls_required),
            coredns_pod_mode: public(input.coredns_pod_mode),
            evidence_ref: internal(evidence_ref),
            schema_version: public(NETWORK_SCHEMA_VERSION),
        })
    }

    pub fn tenant_id(&self) -> &str {
        &self.tenant_id.value
    }

    pub fn region(&self) -> &str {
        &self.region.value.value
    }

    pub fn cell_id(&self) -> &str {
        &self.cell_id.value.value
    }

    pub fn namespace(&self) -> &str {
        &self.namespace.value.value
    }

    pub fn cni_provider(&self) -> NetworkCniProvider {
        self.cni_provider.value
    }

    pub fn edge_gateway(&self) -> EdgeGatewayProvider {
        self.edge_gateway.value
    }

    pub fn default_deny_ingress(&self) -> bool {
        self.default_deny_ingress.value
    }

    pub fn default_deny_egress(&self) -> bool {
        self.default_deny_egress.value
    }

    pub fn dns_egress_explicitly_allowed(&self) -> bool {
        self.dns_egress_explicitly_allowed.value
    }

    pub fn cross_cell_default_traffic_allowed(&self) -> bool {
        self.cross_cell_default_traffic_allowed.value
    }

    pub fn envoy_external_authorization(&self) -> bool {
        self.envoy_external_authorization.value
    }

    pub fn envoy_failure_mode_allow(&self) -> bool {
        self.envoy_failure_mode_allow.value
    }

    pub fn mtls_required(&self) -> bool {
        self.mtls_required.value
    }

    pub fn coredns_pod_mode(&self) -> CoreDnsPodMode {
        self.coredns_pod_mode.value
    }

    pub fn evidence_ref(&self) -> &str {
        &self.evidence_ref.value
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version.value
    }
}
