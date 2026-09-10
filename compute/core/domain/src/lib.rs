#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod capacity;
mod catalog;
mod error;
mod function;
mod guardrail;
mod identity;
mod instance;
mod kubernetes;
mod kubernetes_intent;
mod provider;
mod validation;

pub use capacity::instance_flavor_label;
pub use capacity::{ComputeFlavorSpec, ComputeQuotaEnvelope};
pub use catalog::{CloudComputeCatalog, ComputeRepo};
pub use error::CloudComputeError;
pub use function::{
    FunctionDeployment, FunctionDeploymentCreate, FunctionDeploymentState,
    FunctionInvocationReceipt, FunctionInvocationRequest,
};
pub use guardrail::{
    COMPUTE_AUDIT_EVIDENCE_PREFIX, COMPUTE_SCHEDULING_EVIDENCE_PREFIX,
    COMPUTE_WORKLOAD_IDENTITY_EVIDENCE_PREFIX, ComputeTenantCellGuardrail,
    ComputeTenantCellGuardrailCreate, ComputeWorkloadIsolation, compute_workload_isolation_label,
};
pub use identity::{
    ControlPlaneVersion, FunctionName, IdempotencyKey, ImageRef, ImageRefKind, InvocationId,
    KeyPairId, NodePoolId, UserDataUri, image_ref_kind_label,
};
pub use instance::{Instance, InstanceCreate, InstanceState, instance_state_label};
pub use kubernetes::{
    KUBERNETES_CLUSTER_SCHEMA_VERSION, KubernetesCluster, KubernetesClusterCreate,
    KubernetesClusterDesiredState, KubernetesClusterMutationError, KubernetesClusterObservation,
    KubernetesClusterReconcileAction, KubernetesClusterReconcileError,
    KubernetesClusterReconcileInput, KubernetesClusterState, KubernetesNodePool,
    KubernetesNodePoolCreate, kubernetes_cluster_desired_state_label,
    kubernetes_cluster_state_label, reconcile_kubernetes_cluster,
};
pub use kubernetes_intent::{KubernetesClusterIntent, validate_kubernetes_cluster_intent};
pub use provider::{
    ComputeProviderKind, ComputeProviderVmCreateRequest, ComputeProviderVmError,
    ComputeProviderVmPort, ComputeProviderVmReceipt,
};

pub const MAX_FUNCTION_COLD_START_BUDGET_MS: u32 = 1_000;
pub(crate) const COMPUTE_SCHEMA_VERSION: u32 = 1;

pub(crate) use validation::{
    internal, looks_secret_like, map_resource_error, prefixed_token, privacy_classes, public,
    public_metadata_class, region_for, resource_id_for, resource_id_for_kind_label,
    resource_ref_for, safe_ref_token, security_groups, validate_az_region, validate_cell_az,
    validate_cell_region, validate_tenant_id,
};

#[cfg(test)]
mod tests;
