use cell_region::{AzCode, CellId, RegionCode};
use compute_resource::{K8sFlavor, ResourceId};
use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use network_domain::SecurityGroupId;
use network_residency::ResidencyClass;

use crate::kubernetes_intent::{KubernetesClusterIntent, project_kubernetes_cluster_intent};
use crate::{
    CloudComputeError, ComputeFlavorSpec, ComputeQuotaEnvelope, ControlPlaneVersion, NodePoolId,
    internal, public, public_metadata_class, validate_tenant_id,
};

pub const KUBERNETES_CLUSTER_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum KubernetesClusterState {
    Creating,
    Ready,
    Reconciling,
    Draining,
    Deleted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum KubernetesClusterDesiredState {
    Present,
    Deleted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KubernetesClusterObservation {
    Known(KubernetesClusterState),
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KubernetesClusterReconcileInput {
    pub desired_state: KubernetesClusterDesiredState,
    pub observation: KubernetesClusterObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KubernetesClusterReconcileAction {
    AwaitObservation,
    BeginDraining,
    ActuateDeletion,
    Noop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KubernetesClusterReconcileError {
    UnknownObservation,
    InconsistentLifecycle {
        desired_state: KubernetesClusterDesiredState,
        observed_state: KubernetesClusterState,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KubernetesClusterMutationError {
    UnknownCluster,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KubernetesNodePoolCreate {
    pub id: String,                   // data_class: INTERNAL_ONLY
    pub az: String,                   // data_class: PUBLIC
    pub cell_id: String,              // data_class: PUBLIC
    pub subnet_id: String,            // data_class: INTERNAL_ONLY
    pub security_groups: Vec<String>, // data_class: INTERNAL_ONLY
    pub flavor: ComputeFlavorSpec,    // data_class: PUBLIC
    pub min_nodes: u32,               // data_class: PUBLIC
    pub max_nodes: u32,               // data_class: PUBLIC
    pub autoscaling_enabled: bool,    // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KubernetesNodePool {
    pub id: NodePoolId,                        // data_class: INTERNAL_ONLY
    pub az: AzCode,                            // data_class: PUBLIC
    pub cell_id: CellId,                       // data_class: PUBLIC
    pub subnet_id: ResourceId,                 // data_class: INTERNAL_ONLY
    pub security_groups: Vec<SecurityGroupId>, // data_class: INTERNAL_ONLY
    pub flavor: ComputeFlavorSpec,             // data_class: PUBLIC
    pub min_nodes: u32,                        // data_class: PUBLIC
    pub max_nodes: u32,                        // data_class: PUBLIC
    pub autoscaling_enabled: bool,             // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KubernetesClusterCreate {
    pub resource_id: String,                       // data_class: INTERNAL_ONLY
    pub tenant_id: String,                         // data_class: INTERNAL_ONLY
    pub region: String,                            // data_class: PUBLIC
    pub flavor: K8sFlavor,                         // data_class: PUBLIC
    pub control_plane_version: String,             // data_class: PUBLIC
    pub control_plane_private: bool,               // data_class: PUBLIC
    pub node_pools: Vec<KubernetesNodePoolCreate>, // data_class: INTERNAL_ONLY
    pub quota: ComputeQuotaEnvelope,               // data_class: INTERNAL_ONLY
    pub residency: ResidencyClass,                 // data_class: INTERNAL_ONLY
    pub state: KubernetesClusterState,             // data_class: PUBLIC
    pub data_class: DataClass,                     // data_class: PUBLIC
    pub created_at_epoch_seconds: u64,             // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KubernetesCluster {
    pub resource_id: Classified<ResourceId>,
    pub tenant_id: Classified<String>,
    pub region: Classified<RegionCode>,
    pub flavor: Classified<K8sFlavor>,
    pub control_plane_version: Classified<ControlPlaneVersion>,
    pub control_plane_private: Classified<bool>,
    pub node_pools: Classified<Vec<KubernetesNodePool>>,
    pub residency: Classified<ResidencyClass>,
    pub state: Classified<KubernetesClusterState>,
    pub desired_state: Classified<KubernetesClusterDesiredState>,
    pub data_class: Classified<PrivacyDataClass>,
    pub created_at_epoch_seconds: Classified<u64>,
    pub schema_version: Classified<u32>,
}
impl KubernetesCluster {
    pub fn new(input: KubernetesClusterCreate) -> Result<Self, CloudComputeError> {
        validate_tenant_id(&input.tenant_id)?;
        if input.state != KubernetesClusterState::Creating {
            return Err(CloudComputeError::InvalidKubernetesState);
        }
        let validated = project_kubernetes_cluster_intent(KubernetesClusterIntent {
            resource_id: input.resource_id,
            tenant_id: input.tenant_id,
            region: input.region,
            flavor: input.flavor,
            control_plane_version: input.control_plane_version,
            control_plane_private: input.control_plane_private,
            node_pools: input.node_pools,
            residency: input.residency,
            data_class: input.data_class,
        })?;
        input.quota.admit(validated.requested)?;
        let data_class = public_metadata_class(input.data_class)?;
        Ok(Self {
            resource_id: internal(validated.resource_id),
            tenant_id: internal(validated.tenant_id),
            region: public(validated.region),
            flavor: public(validated.flavor),
            control_plane_version: public(validated.control_plane_version),
            control_plane_private: public(validated.control_plane_private),
            node_pools: internal(validated.node_pools),
            residency: internal(validated.residency),
            state: public(input.state),
            desired_state: public(KubernetesClusterDesiredState::Present),
            data_class: public(data_class),
            created_at_epoch_seconds: internal(input.created_at_epoch_seconds),
            schema_version: public(KUBERNETES_CLUSTER_SCHEMA_VERSION),
        })
    }

    #[must_use]
    pub fn request_deletion(&self) -> Self {
        let mut next = self.clone();
        next.desired_state = public(KubernetesClusterDesiredState::Deleted);
        next
    }
}

pub const fn kubernetes_cluster_state_label(state: KubernetesClusterState) -> &'static str {
    match state {
        KubernetesClusterState::Creating => "creating",
        KubernetesClusterState::Ready => "ready",
        KubernetesClusterState::Reconciling => "reconciling",
        KubernetesClusterState::Draining => "draining",
        KubernetesClusterState::Deleted => "deleted",
    }
}

pub const fn kubernetes_cluster_desired_state_label(
    state: KubernetesClusterDesiredState,
) -> &'static str {
    match state {
        KubernetesClusterDesiredState::Present => "present",
        KubernetesClusterDesiredState::Deleted => "deleted",
    }
}

pub fn reconcile_kubernetes_cluster(
    input: KubernetesClusterReconcileInput,
) -> Result<KubernetesClusterReconcileAction, KubernetesClusterReconcileError> {
    let observed_state = match input.observation {
        KubernetesClusterObservation::Known(state) => state,
        KubernetesClusterObservation::Unknown => {
            return Err(KubernetesClusterReconcileError::UnknownObservation);
        }
    };

    match (input.desired_state, observed_state) {
        (
            KubernetesClusterDesiredState::Present,
            KubernetesClusterState::Creating | KubernetesClusterState::Reconciling,
        ) => Ok(KubernetesClusterReconcileAction::AwaitObservation),
        (KubernetesClusterDesiredState::Present, KubernetesClusterState::Ready) => {
            Ok(KubernetesClusterReconcileAction::Noop)
        }
        (
            KubernetesClusterDesiredState::Present,
            KubernetesClusterState::Draining | KubernetesClusterState::Deleted,
        ) => Err(KubernetesClusterReconcileError::InconsistentLifecycle {
            desired_state: input.desired_state,
            observed_state,
        }),
        (
            KubernetesClusterDesiredState::Deleted,
            KubernetesClusterState::Creating
            | KubernetesClusterState::Ready
            | KubernetesClusterState::Reconciling,
        ) => Ok(KubernetesClusterReconcileAction::BeginDraining),
        (KubernetesClusterDesiredState::Deleted, KubernetesClusterState::Draining) => {
            Ok(KubernetesClusterReconcileAction::ActuateDeletion)
        }
        (KubernetesClusterDesiredState::Deleted, KubernetesClusterState::Deleted) => {
            Ok(KubernetesClusterReconcileAction::Noop)
        }
    }
}
