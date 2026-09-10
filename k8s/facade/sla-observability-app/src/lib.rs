//! Managed-Kubernetes SLA observability composition layer.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use k8s_control_plane_host_api as control_plane_host;

pub use control_plane_host::{ControlPlaneProvisioning, ControlPlaneRef, ProvisioningError};
pub use k8s_sla_observability_adapter_inmemory::InMemorySlaObservabilityStore;
pub use k8s_sla_observability_api::{
    ClusterRef, ControlPlaneSlaSnapshot, SlaObservabilityError, SlaObservabilityPort, StatusWindow,
};
pub use k8s_sla_observability_kernel::{ObservedControlPlaneStatus, SlaPolicy, SlaSummary};

pub struct SlaObservabilityService<S> {
    store: S,
}

impl<S> SlaObservabilityService<S> {
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

impl<S> SlaObservabilityService<S>
where
    S: SlaObservabilityPort,
{
    /// # Errors
    /// Returns typed fail-closed errors for malformed/unknown cluster identity or
    /// invalid observation windows.
    pub async fn ingest_snapshot(
        &self,
        snapshot: ControlPlaneSlaSnapshot,
    ) -> Result<SlaSummary, SlaObservabilityError> {
        self.store.ingest_status_snapshot(snapshot).await
    }

    /// # Errors
    /// Returns [`SlaObservabilityError::UnknownCluster`] if no observation exists.
    pub async fn summarize_cluster(
        &self,
        cluster_ref: &ClusterRef,
    ) -> Result<SlaSummary, SlaObservabilityError> {
        self.store.summarize_cluster(cluster_ref).await
    }

    /// `status_window` and `provisioning_latency_millis` are supplied by the
    /// caller/adapter because this lane intentionally does not couple to live
    /// Prometheus or Kubernetes.
    ///
    /// # Errors
    /// Propagates typed control-plane-host read errors and SLA validation errors.
    pub async fn ingest_from_control_plane<P>(
        &self,
        control_plane: &P,
        control_plane_ref: &ControlPlaneRef,
        status_window: StatusWindow,
        provisioning_latency_millis: Option<u64>,
    ) -> Result<SlaSummary, SlaObservabilityError>
    where
        P: ControlPlaneProvisioning,
    {
        let report = control_plane
            .status(control_plane_ref)
            .await
            .map_err(|err| SlaObservabilityError::ControlPlane {
                detail: err.to_string(),
            })?;
        let snapshot = ControlPlaneSlaSnapshot::new(
            map_cluster_ref(&report.control_plane_ref.cluster_ref),
            map_control_plane_status(report.status),
            status_window,
            provisioning_latency_millis,
        );
        self.ingest_snapshot(snapshot).await
    }
}

#[must_use]
pub fn build_inmemory_service() -> SlaObservabilityService<InMemorySlaObservabilityStore> {
    SlaObservabilityService::new(InMemorySlaObservabilityStore::new(SlaPolicy::default()))
}

#[must_use]
pub fn map_cluster_ref(cluster_ref: &control_plane_host::ClusterRef) -> ClusterRef {
    ClusterRef::new(&cluster_ref.tenant_id, &cluster_ref.cluster_name)
}

#[must_use]
pub const fn map_control_plane_status(
    status: control_plane_host::ControlPlaneStatus,
) -> ObservedControlPlaneStatus {
    match status {
        control_plane_host::ControlPlaneStatus::Requested => ObservedControlPlaneStatus::Requested,
        control_plane_host::ControlPlaneStatus::DatastoreBound => {
            ObservedControlPlaneStatus::DatastoreBound
        }
        control_plane_host::ControlPlaneStatus::MediaFormed => {
            ObservedControlPlaneStatus::MediaFormed
        }
        control_plane_host::ControlPlaneStatus::Provisioning => {
            ObservedControlPlaneStatus::Provisioning
        }
        control_plane_host::ControlPlaneStatus::EndpointReady => {
            ObservedControlPlaneStatus::EndpointReady
        }
        control_plane_host::ControlPlaneStatus::Active => ObservedControlPlaneStatus::Active,
        control_plane_host::ControlPlaneStatus::Draining => ObservedControlPlaneStatus::Draining,
        control_plane_host::ControlPlaneStatus::Deleted => ObservedControlPlaneStatus::Deleted,
        control_plane_host::ControlPlaneStatus::Failed => ObservedControlPlaneStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_control_plane_host_adapter_inmemory::InMemoryControlPlaneHost;
    use k8s_control_plane_host_api::{
        ClusterRef as HostClusterRef, DatastoreClass, ProvisionRequest,
    };
    use k8s_sla_observability_kernel::{AvailabilityState, ProvisioningLatencyState};

    #[tokio::test]
    async fn reads_control_plane_status_port_and_summarizes_available_cluster() {
        let control_plane = InMemoryControlPlaneHost::new();
        let cp = control_plane
            .provision(&ProvisionRequest::new(
                HostClusterRef::new("ten_acme", "prod-a"),
                Default::default(),
                DatastoreClass::EtcdPerTenant,
            ))
            .await
            .unwrap();
        let service = build_inmemory_service();
        let summary = service
            .ingest_from_control_plane(
                &control_plane,
                &cp,
                StatusWindow::new(1_000, 1_000),
                Some(120_000),
            )
            .await
            .unwrap();
        assert_eq!(summary.availability.state, AvailabilityState::Available);
        assert_eq!(
            summary.provisioning_latency.state,
            ProvisioningLatencyState::Met
        );
    }

    #[tokio::test]
    async fn direct_degraded_snapshot_burns_budget() {
        let service = build_inmemory_service();
        let summary = service
            .ingest_snapshot(ControlPlaneSlaSnapshot::new(
                ClusterRef::new("ten_acme", "prod-a"),
                ObservedControlPlaneStatus::Failed,
                StatusWindow::new(100, 80),
                Some(900_000),
            ))
            .await
            .unwrap();
        assert_eq!(summary.availability.state, AvailabilityState::Unavailable);
        assert!(summary.error_budget.exhausted);
        assert_eq!(
            summary.provisioning_latency.state,
            ProvisioningLatencyState::Breached
        );
    }

    #[tokio::test]
    async fn unknown_cluster_fails_closed() {
        let service = build_inmemory_service();
        let err = service
            .summarize_cluster(&ClusterRef::new("ten_acme", "ghost"))
            .await
            .unwrap_err();
        assert!(matches!(err, SlaObservabilityError::UnknownCluster { .. }));
    }

    #[tokio::test]
    async fn summary_serializes_with_stable_slug() {
        let service = build_inmemory_service();
        let summary = service
            .ingest_snapshot(ControlPlaneSlaSnapshot::new(
                ClusterRef::new("TEN Acme!", "Prod_A.01"),
                ObservedControlPlaneStatus::Active,
                StatusWindow::new(10, 10),
                Some(100),
            ))
            .await
            .unwrap();
        assert_eq!(summary.cluster_slug, "ten-acme--prod-a-01");
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"cluster_slug\":\"ten-acme--prod-a-01\""));
    }
}
