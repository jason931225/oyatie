//! Managed-Kubernetes SLA observability API / port layer.
//!
//! This crate is the transport-neutral seam for the SLA observability bounded
//! context. It owns only SLA DTOs and an object-safe observation port; the app
//! layer performs any adaptation from the sibling control-plane-host API seam.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use core::fmt;
use core::pin::Pin;

use serde::{Deserialize, Serialize};

pub use k8s_sla_observability_kernel::{
    AvailabilityState, ErrorBudgetSummary, ObservedControlPlaneStatus, ProvisioningLatencyState,
    ProvisioningLatencySummary, SlaKernelError, SlaObservation, SlaPolicy, SlaSummary,
    summarize_sla,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn core::future::Future<Output = T> + Send + 'a>>;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ClusterRef {
    pub tenant_id: String,
    pub cluster_name: String,
}

impl ClusterRef {
    #[must_use]
    pub fn new(tenant_id: impl Into<String>, cluster_name: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            cluster_name: cluster_name.into(),
        }
    }

    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        !self.tenant_id.trim().is_empty() && !self.cluster_name.trim().is_empty()
    }
}

impl fmt::Display for ClusterRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.tenant_id, self.cluster_name)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SlaObservabilityError {
    InvalidClusterRef {
        cluster_ref: String,
    },
    UnknownCluster {
        cluster_ref: String,
    },
    Kernel(SlaKernelError),
    /// `detail` is normalized by the app layer so the API crate does not depend
    /// on a sibling API crate.
    ControlPlane {
        detail: String,
    },
    Store {
        detail: String,
    },
}

impl fmt::Display for SlaObservabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidClusterRef { cluster_ref } => {
                write!(f, "invalid cluster reference: {cluster_ref}")
            }
            Self::UnknownCluster { cluster_ref } => {
                write!(f, "SLA observation not found for cluster {cluster_ref}")
            }
            Self::Kernel(err) => write!(f, "SLA kernel error: {err}"),
            Self::ControlPlane { detail } => {
                write!(f, "control-plane status read failed: {detail}")
            }
            Self::Store { detail } => write!(f, "SLA observation store failed: {detail}"),
        }
    }
}

impl std::error::Error for SlaObservabilityError {}

impl From<SlaKernelError> for SlaObservabilityError {
    fn from(value: SlaKernelError) -> Self {
        Self::Kernel(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StatusWindow {
    pub total_status_samples: u64,
    pub healthy_status_samples: u64,
}

impl StatusWindow {
    #[must_use]
    pub const fn new(total_status_samples: u64, healthy_status_samples: u64) -> Self {
        Self {
            total_status_samples,
            healthy_status_samples,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlPlaneSlaSnapshot {
    pub cluster_ref: ClusterRef,
    pub control_plane_status: ObservedControlPlaneStatus,
    pub status_window: StatusWindow,
    pub provisioning_latency_millis: Option<u64>,
}

impl ControlPlaneSlaSnapshot {
    #[must_use]
    pub fn new(
        cluster_ref: ClusterRef,
        control_plane_status: ObservedControlPlaneStatus,
        status_window: StatusWindow,
        provisioning_latency_millis: Option<u64>,
    ) -> Self {
        Self {
            cluster_ref,
            control_plane_status,
            status_window,
            provisioning_latency_millis,
        }
    }

    #[must_use]
    pub fn into_observation(self) -> SlaObservation {
        SlaObservation::new(
            self.cluster_ref.tenant_id,
            self.cluster_ref.cluster_name,
            self.control_plane_status,
            self.status_window.total_status_samples,
            self.status_window.healthy_status_samples,
            self.provisioning_latency_millis,
        )
    }
}

/// Object-safe SLA observability port.
pub trait SlaObservabilityPort: Send + Sync {
    /// Ingest or replace the latest SLA snapshot for a cluster.
    fn ingest_status_snapshot<'a>(
        &'a self,
        snapshot: ControlPlaneSlaSnapshot,
    ) -> BoxFuture<'a, Result<SlaSummary, SlaObservabilityError>>;

    /// Read a deterministic SLA summary for a cluster.
    fn summarize_cluster<'a>(
        &'a self,
        cluster_ref: &'a ClusterRef,
    ) -> BoxFuture<'a, Result<SlaSummary, SlaObservabilityError>>;
}

pub fn validate_cluster_ref(cluster_ref: &ClusterRef) -> Result<(), SlaObservabilityError> {
    if cluster_ref.is_well_formed() {
        Ok(())
    } else {
        Err(SlaObservabilityError::InvalidClusterRef {
            cluster_ref: cluster_ref.to_string(),
        })
    }
}

/// Collision-free map key for cluster observations.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct ClusterKey {
    tenant_id: String,
    cluster_name: String,
}

impl ClusterKey {
    #[must_use]
    pub fn from_ref(cluster_ref: &ClusterRef) -> Self {
        Self {
            tenant_id: cluster_ref.tenant_id.clone(),
            cluster_name: cluster_ref.cluster_name.clone(),
        }
    }
}

impl fmt::Display for ClusterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.tenant_id, self.cluster_name)
    }
}

/// Stable display key for diagnostics only. Storage must use [`ClusterKey`].
#[must_use]
pub fn cluster_key(cluster_ref: &ClusterRef) -> String {
    ClusterKey::from_ref(cluster_ref).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serialization_is_deterministic() {
        let snapshot = ControlPlaneSlaSnapshot::new(
            ClusterRef::new("ten_acme", "prod-a"),
            ObservedControlPlaneStatus::Failed,
            StatusWindow::new(10, 8),
            None,
        );
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"control_plane_status\":\"failed\""));
        let back: ControlPlaneSlaSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snapshot, back);
    }

    #[test]
    fn invalid_cluster_ref_fails_closed() {
        let err = validate_cluster_ref(&ClusterRef::new("", "prod-a")).unwrap_err();
        assert!(matches!(
            err,
            SlaObservabilityError::InvalidClusterRef { .. }
        ));
    }
}
