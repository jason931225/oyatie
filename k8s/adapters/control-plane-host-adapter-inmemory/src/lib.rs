//! Deterministic in-memory [`ControlPlaneProvisioning`] adapter for tests and
//! single-node bring-up.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use k8s_control_plane_host_api::{
    BoxFuture, ClusterRef, ControlPlaneProvisioning, ControlPlaneRef, ControlPlaneStatusReport,
    ProvisionRequest, ProvisioningError,
};
use k8s_control_plane_host_kernel::{ControlPlaneStatus, ControlPlaneTier};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Record {
    cluster_ref: ClusterRef,
    tier: ControlPlaneTier,
    status: ControlPlaneStatus,
    endpoint: Option<String>,
}

#[derive(Debug, Default)]
pub struct InMemoryControlPlaneHost {
    records: Mutex<BTreeMap<String, Record>>, // keyed by handle
}

impl InMemoryControlPlaneHost {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Deterministic handle for a `(cluster_ref, tier)` pair: stable across
    /// `provision`/`status`/`teardown` so a re-provision is idempotent in tests.
    fn handle_for(cluster_ref: &ClusterRef, tier: ControlPlaneTier) -> String {
        format!(
            "{}:{}:{}",
            tier.as_str(),
            cluster_ref.tenant_id,
            cluster_ref.cluster_name
        )
    }

    /// The synthetic API-server endpoint a fully-provisioned control plane
    /// surfaces (deterministic; never a real address).
    fn endpoint_for(handle: &str) -> String {
        format!("https://{handle}.control-plane.invalid:6443")
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.records.lock().map(|guard| guard.len()).unwrap_or(0)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn walk_to_active(tier: ControlPlaneTier) -> Result<ControlPlaneStatus, ProvisioningError> {
        let mut status = ControlPlaneStatus::initial();
        for next in [
            ControlPlaneStatus::next_after_request(tier),
            ControlPlaneStatus::Provisioning,
            ControlPlaneStatus::EndpointReady,
            ControlPlaneStatus::Active,
        ] {
            status = status.transition(next)?;
        }
        Ok(status)
    }
}

impl ControlPlaneProvisioning for InMemoryControlPlaneHost {
    fn provision<'a>(
        &'a self,
        request: &'a ProvisionRequest,
    ) -> BoxFuture<'a, Result<ControlPlaneRef, ProvisioningError>> {
        Box::pin(async move {
            if !request.cluster_ref.is_well_formed() {
                return Err(ProvisioningError::InvalidClusterRef {
                    cluster_ref: request.cluster_ref.to_string(),
                });
            }
            let handle = Self::handle_for(&request.cluster_ref, request.tier);
            let status = Self::walk_to_active(request.tier)?;
            let record = Record {
                cluster_ref: request.cluster_ref.clone(),
                tier: request.tier,
                status,
                endpoint: Some(Self::endpoint_for(&handle)),
            };
            let mut guard = self
                .records
                .lock()
                .map_err(|_| ProvisioningError::backend("in-memory host mutex poisoned"))?;
            guard.insert(handle.clone(), record);

            Ok(ControlPlaneRef::new(
                request.cluster_ref.clone(),
                request.tier,
                handle,
            ))
        })
    }

    fn status<'a>(
        &'a self,
        control_plane_ref: &'a ControlPlaneRef,
    ) -> BoxFuture<'a, Result<ControlPlaneStatusReport, ProvisioningError>> {
        Box::pin(async move {
            let guard = self
                .records
                .lock()
                .map_err(|_| ProvisioningError::backend("in-memory host mutex poisoned"))?;
            let record = guard.get(&control_plane_ref.handle).ok_or_else(|| {
                ProvisioningError::NotFound {
                    handle: control_plane_ref.handle.clone(),
                }
            })?;
            Ok(ControlPlaneStatusReport::new(
                control_plane_ref.clone(),
                record.status,
                record.endpoint.clone(),
            ))
        })
    }

    fn teardown<'a>(
        &'a self,
        control_plane_ref: &'a ControlPlaneRef,
    ) -> BoxFuture<'a, Result<(), ProvisioningError>> {
        Box::pin(async move {
            let mut guard = self
                .records
                .lock()
                .map_err(|_| ProvisioningError::backend("in-memory host mutex poisoned"))?;
            let Some(record) = guard.get_mut(&control_plane_ref.handle) else {
                // Idempotent: tearing down an unknown/already-deleted control
                // plane is a no-op success, not an error.
                return Ok(());
            };
            if record.status.is_terminal() {
                return Ok(());
            }
            for next in [ControlPlaneStatus::Draining, ControlPlaneStatus::Deleted] {
                record.status = record.status.transition(next)?;
            }
            record.endpoint = None;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_control_plane_host_kernel::DatastoreClass;

    fn req(tier: ControlPlaneTier) -> ProvisionRequest {
        ProvisionRequest::new(
            ClusterRef::new("ten_zero", "dogfood-a"),
            tier,
            DatastoreClass::EtcdPerTenant,
        )
    }

    #[tokio::test]
    async fn hosted_provision_reaches_active_with_endpoint() {
        let host = InMemoryControlPlaneHost::new();
        let cp = host
            .provision(&req(ControlPlaneTier::HostedKamaji))
            .await
            .unwrap();
        assert_eq!(cp.tier, ControlPlaneTier::HostedKamaji);
        let report = host.status(&cp).await.unwrap();
        assert_eq!(report.status, ControlPlaneStatus::Active);
        assert!(report.endpoint.is_some());
        assert_eq!(host.len(), 1);
    }

    #[tokio::test]
    async fn dedicated_provision_reaches_active() {
        let host = InMemoryControlPlaneHost::new();
        let cp = host
            .provision(&req(ControlPlaneTier::DedicatedTalosSpoke))
            .await
            .unwrap();
        assert_eq!(cp.tier, ControlPlaneTier::DedicatedTalosSpoke);
        assert_eq!(
            host.status(&cp).await.unwrap().status,
            ControlPlaneStatus::Active
        );
    }

    #[tokio::test]
    async fn teardown_is_idempotent_and_deletes() {
        let host = InMemoryControlPlaneHost::new();
        let cp = host
            .provision(&req(ControlPlaneTier::HostedKamaji))
            .await
            .unwrap();
        host.teardown(&cp).await.unwrap();
        assert_eq!(
            host.status(&cp).await.unwrap().status,
            ControlPlaneStatus::Deleted
        );
        host.teardown(&cp).await.unwrap();
    }

    #[tokio::test]
    async fn malformed_cluster_ref_is_rejected() {
        let host = InMemoryControlPlaneHost::new();
        let bad = ProvisionRequest::new(
            ClusterRef::new("", ""),
            ControlPlaneTier::HostedKamaji,
            DatastoreClass::PooledRelational,
        );
        let err = host
            .provision(&bad)
            .await
            .expect_err("malformed ref rejected");
        assert!(matches!(err, ProvisioningError::InvalidClusterRef { .. }));
    }

    #[tokio::test]
    async fn status_of_unknown_handle_is_not_found() {
        let host = InMemoryControlPlaneHost::new();
        let phantom = ControlPlaneRef::new(
            ClusterRef::new("ten_zero", "ghost"),
            ControlPlaneTier::HostedKamaji,
            "hosted_kamaji:ten_zero:ghost",
        );
        let err = host.status(&phantom).await.expect_err("unknown handle");
        assert!(matches!(err, ProvisioningError::NotFound { .. }));
    }
}
