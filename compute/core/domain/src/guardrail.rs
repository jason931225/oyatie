use std::collections::BTreeSet;

use cell_region::{CellId, RegionCode};
use compute_resource::ResourceId;
use data_boundary_kernel::Classified;
use iam_domain::IamRoleId;

use crate::{
    COMPUTE_SCHEMA_VERSION, CloudComputeError, internal, looks_secret_like, public,
    resource_id_for_kind_label, safe_ref_token, validate_cell_region, validate_tenant_id,
};

pub const COMPUTE_WORKLOAD_IDENTITY_EVIDENCE_PREFIX: &str = "evidence/compute/identity/";
pub const COMPUTE_SCHEDULING_EVIDENCE_PREFIX: &str = "evidence/compute/scheduling/";
pub const COMPUTE_AUDIT_EVIDENCE_PREFIX: &str = "evidence/compute/audit/";
const KUBERNETES_SERVICE_ACCOUNT_KIND: &str = "ksa";
const FUNCTION_SERVICE_ACCOUNT_KIND: &str = "function-sa";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ComputeWorkloadIsolation {
    SharedHost,
    HardenedNode,
    HardwareVirtualizedVm,
    KataMicroVm,
    FirecrackerMicroVm,
    GvisorSandbox,
    WasmSandbox,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeTenantCellGuardrailCreate {
    pub tenant_id: String,                              // data_class: INTERNAL_ONLY
    pub region: String,                                 // data_class: PUBLIC
    pub primary_cell_id: String,                        // data_class: PUBLIC
    pub vm_instance_id: String,                         // data_class: INTERNAL_ONLY
    pub vm_iam_role: String,                            // data_class: INTERNAL_ONLY
    pub vm_runtime_isolation: ComputeWorkloadIsolation, // data_class: PUBLIC
    pub k8s_cluster_id: String,                         // data_class: INTERNAL_ONLY
    pub k8s_service_account_ref: String,                // data_class: INTERNAL_ONLY
    pub k8s_private_control_plane: bool,                // data_class: PUBLIC
    pub k8s_pod_security_restricted: bool,              // data_class: PUBLIC
    pub k8s_topology_spread_required: bool,             // data_class: PUBLIC
    pub k8s_runtime_isolation: ComputeWorkloadIsolation, // data_class: PUBLIC
    pub function_id: String,                            // data_class: INTERNAL_ONLY
    pub function_service_account_ref: String,           // data_class: INTERNAL_ONLY
    pub function_runtime_isolation: ComputeWorkloadIsolation, // data_class: PUBLIC
    pub audit_evidence_ref: String,                     // data_class: INTERNAL_ONLY
    pub scheduling_evidence_ref: String,                // data_class: INTERNAL_ONLY
    pub identity_evidence_refs: Vec<String>,            // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComputeTenantCellGuardrail {
    pub tenant_id: Classified<String>,
    pub region: Classified<RegionCode>,
    pub primary_cell_id: Classified<CellId>,
    pub vm_instance_id: Classified<ResourceId>,
    pub vm_iam_role: Classified<IamRoleId>,
    pub vm_runtime_isolation: Classified<ComputeWorkloadIsolation>,
    pub k8s_cluster_id: Classified<ResourceId>,
    pub k8s_service_account_ref: Classified<String>,
    pub k8s_private_control_plane: Classified<bool>,
    pub k8s_pod_security_restricted: Classified<bool>,
    pub k8s_topology_spread_required: Classified<bool>,
    pub k8s_runtime_isolation: Classified<ComputeWorkloadIsolation>,
    pub function_id: Classified<ResourceId>,
    pub function_service_account_ref: Classified<String>,
    pub function_runtime_isolation: Classified<ComputeWorkloadIsolation>,
    pub audit_evidence_ref: Classified<String>,
    pub scheduling_evidence_ref: Classified<String>,
    pub identity_evidence_refs: Classified<Vec<String>>,
    pub schema_version: Classified<u32>,
}

impl ComputeWorkloadIsolation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SharedHost => "shared_host",
            Self::HardenedNode => "hardened_node",
            Self::HardwareVirtualizedVm => "hardware_virtualized_vm",
            Self::KataMicroVm => "kata_microvm",
            Self::FirecrackerMicroVm => "firecracker_microvm",
            Self::GvisorSandbox => "gvisor_sandbox",
            Self::WasmSandbox => "wasm_sandbox",
        }
    }

    const fn satisfies_tenant_isolation(self) -> bool {
        matches!(
            self,
            Self::HardwareVirtualizedVm
                | Self::KataMicroVm
                | Self::FirecrackerMicroVm
                | Self::GvisorSandbox
                | Self::WasmSandbox
        )
    }
}

impl ComputeTenantCellGuardrail {
    pub fn new(input: ComputeTenantCellGuardrailCreate) -> Result<Self, CloudComputeError> {
        validate_tenant_id(&input.tenant_id)?;
        let region =
            RegionCode::new(input.region).map_err(|_| CloudComputeError::InvalidResourceId)?;
        let primary_cell_id =
            CellId::new(input.primary_cell_id).map_err(|_| CloudComputeError::InvalidCellId)?;
        validate_cell_region(&primary_cell_id, &region)?;

        let vm_instance_id = resource_id_for_kind_label(
            &input.vm_instance_id,
            &input.tenant_id,
            &region,
            "instance",
        )?;
        let k8s_cluster_id =
            resource_id_for_kind_label(&input.k8s_cluster_id, &input.tenant_id, &region, "k8s")?;
        let function_id =
            resource_id_for_kind_label(&input.function_id, &input.tenant_id, &region, "function")?;

        let vm_iam_role = IamRoleId::new(input.vm_iam_role)
            .map_err(|_| CloudComputeError::InvalidWorkloadIdentityPolicy)?;
        let k8s_service_account_ref = validate_workload_identity_ref(
            input.k8s_service_account_ref,
            KUBERNETES_SERVICE_ACCOUNT_KIND,
            &input.tenant_id,
            &primary_cell_id,
        )?;
        let function_service_account_ref = validate_workload_identity_ref(
            input.function_service_account_ref,
            FUNCTION_SERVICE_ACCOUNT_KIND,
            &input.tenant_id,
            &primary_cell_id,
        )?;

        validate_runtime_isolation(input.vm_runtime_isolation)?;
        validate_runtime_isolation(input.k8s_runtime_isolation)?;
        validate_runtime_isolation(input.function_runtime_isolation)?;

        if !input.k8s_private_control_plane || !input.k8s_pod_security_restricted {
            return Err(CloudComputeError::InvalidRuntimeIsolationPolicy);
        }
        if !input.k8s_topology_spread_required {
            return Err(CloudComputeError::InvalidSchedulingPolicy);
        }

        let audit_evidence_ref = validate_metadata_ref(
            input.audit_evidence_ref,
            COMPUTE_AUDIT_EVIDENCE_PREFIX,
            CloudComputeError::InvalidAuditEvidenceRef,
        )?;
        let scheduling_evidence_ref = validate_metadata_ref(
            input.scheduling_evidence_ref,
            COMPUTE_SCHEDULING_EVIDENCE_PREFIX,
            CloudComputeError::InvalidSchedulingPolicy,
        )?;
        let identity_evidence_refs = validate_metadata_refs(
            input.identity_evidence_refs,
            COMPUTE_WORKLOAD_IDENTITY_EVIDENCE_PREFIX,
            CloudComputeError::InvalidWorkloadIdentityPolicy,
        )?;

        Ok(Self {
            tenant_id: internal(input.tenant_id),
            region: public(region),
            primary_cell_id: public(primary_cell_id),
            vm_instance_id: internal(vm_instance_id),
            vm_iam_role: internal(vm_iam_role),
            vm_runtime_isolation: public(input.vm_runtime_isolation),
            k8s_cluster_id: internal(k8s_cluster_id),
            k8s_service_account_ref: internal(k8s_service_account_ref),
            k8s_private_control_plane: public(input.k8s_private_control_plane),
            k8s_pod_security_restricted: public(input.k8s_pod_security_restricted),
            k8s_topology_spread_required: public(input.k8s_topology_spread_required),
            k8s_runtime_isolation: public(input.k8s_runtime_isolation),
            function_id: internal(function_id),
            function_service_account_ref: internal(function_service_account_ref),
            function_runtime_isolation: public(input.function_runtime_isolation),
            audit_evidence_ref: internal(audit_evidence_ref),
            scheduling_evidence_ref: internal(scheduling_evidence_ref),
            identity_evidence_refs: internal(identity_evidence_refs),
            schema_version: public(COMPUTE_SCHEMA_VERSION),
        })
    }
}
pub const fn compute_workload_isolation_label(isolation: ComputeWorkloadIsolation) -> &'static str {
    isolation.label()
}
fn validate_runtime_isolation(
    isolation: ComputeWorkloadIsolation,
) -> Result<(), CloudComputeError> {
    if isolation.satisfies_tenant_isolation() {
        Ok(())
    } else {
        Err(CloudComputeError::InvalidRuntimeIsolationPolicy)
    }
}

fn validate_workload_identity_ref(
    value: String,
    expected_kind: &str,
    tenant_id: &str,
    cell_id: &CellId,
) -> Result<String, CloudComputeError> {
    if value.trim() != value
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
        || looks_secret_like(&value)
    {
        return Err(CloudComputeError::InvalidWorkloadIdentityPolicy);
    }

    let mut segments = value.split('/');
    let kind = segments.next();
    let tenant = segments.next();
    let cell = segments.next();
    let name = segments.next();
    if kind != Some(expected_kind)
        || tenant != Some(tenant_id)
        || cell != Some(cell_id.value.as_str())
        || !name.is_some_and(safe_ref_token)
        || segments.next().is_some()
    {
        return Err(CloudComputeError::InvalidWorkloadIdentityPolicy);
    }
    Ok(value)
}

fn validate_metadata_refs(
    input: Vec<String>,
    prefix: &str,
    error: CloudComputeError,
) -> Result<Vec<String>, CloudComputeError> {
    if input.is_empty() {
        return Err(error);
    }
    let mut seen = BTreeSet::new();
    let mut refs = Vec::with_capacity(input.len());
    for value in input {
        let value = validate_metadata_ref(value, prefix, error.clone())?;
        if !seen.insert(value.clone()) {
            return Err(error);
        }
        refs.push(value);
    }
    Ok(refs)
}

fn validate_metadata_ref(
    value: String,
    prefix: &str,
    error: CloudComputeError,
) -> Result<String, CloudComputeError> {
    if value.trim() != value
        || !value.starts_with(prefix)
        || value.len() == prefix.len()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || matches!(byte, b' ' | b'\\' | b'?' | b'#'))
        || value
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || looks_secret_like(&value)
    {
        return Err(error);
    }
    Ok(value)
}
