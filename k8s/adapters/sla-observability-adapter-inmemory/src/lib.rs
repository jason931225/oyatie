//! Deterministic in-memory SLA observability adapter.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use k8s_sla_observability_api::{
    BoxFuture, ClusterKey, ClusterRef, ControlPlaneSlaSnapshot, SlaObservabilityError,
    SlaObservabilityPort, validate_cluster_ref,
};
use k8s_sla_observability_kernel::{SlaObservation, SlaPolicy, SlaSummary, summarize_sla};

#[derive(Debug)]
pub struct InMemorySlaObservabilityStore {
    policy: SlaPolicy,
    observations: Mutex<BTreeMap<ClusterKey, SlaObservation>>, // data_class: TENANT_SCOPED
}

impl Default for InMemorySlaObservabilityStore {
    fn default() -> Self {
        Self::new(SlaPolicy::default())
    }
}

impl InMemorySlaObservabilityStore {
    #[must_use]
    pub fn new(policy: SlaPolicy) -> Self {
        Self {
            policy,
            observations: Mutex::new(BTreeMap::new()),
        }
    }

    /// # Panics
    /// Panics if the supplied fixture is invalid or the mutex is poisoned;
    /// production ingestion uses the fallible port method.
    #[must_use]
    pub fn with_snapshot(self, snapshot: ControlPlaneSlaSnapshot) -> Self {
        let key = ClusterKey::from_ref(&snapshot.cluster_ref);
        self.observations
            .lock()
            .expect("sla observations lock")
            .insert(key, snapshot.into_observation());
        self
    }

    #[cfg(test)]
    fn poison_for_test(&self) {
        let _ = std::panic::catch_unwind(|| {
            let _guard = self.observations.lock().expect("lock before poison");
            panic!("poison SLA observation store for regression coverage");
        });
    }
}

impl SlaObservabilityPort for InMemorySlaObservabilityStore {
    fn ingest_status_snapshot<'a>(
        &'a self,
        snapshot: ControlPlaneSlaSnapshot,
    ) -> BoxFuture<'a, Result<SlaSummary, SlaObservabilityError>> {
        Box::pin(async move {
            validate_cluster_ref(&snapshot.cluster_ref)?;
            let key = ClusterKey::from_ref(&snapshot.cluster_ref);
            let observation = snapshot.into_observation();
            let summary = summarize_sla(&observation, self.policy)?;
            self.observations
                .lock()
                .map_err(|err| SlaObservabilityError::Store {
                    detail: format!("in-memory SLA store poisoned: {err}"),
                })?
                .insert(key, observation);
            Ok(summary)
        })
    }

    fn summarize_cluster<'a>(
        &'a self,
        cluster_ref: &'a ClusterRef,
    ) -> BoxFuture<'a, Result<SlaSummary, SlaObservabilityError>> {
        Box::pin(async move {
            validate_cluster_ref(cluster_ref)?;
            let key = ClusterKey::from_ref(cluster_ref);
            let observation = self
                .observations
                .lock()
                .map_err(|err| SlaObservabilityError::Store {
                    detail: format!("in-memory SLA store poisoned: {err}"),
                })?
                .get(&key)
                .cloned()
                .ok_or_else(|| SlaObservabilityError::UnknownCluster {
                    cluster_ref: cluster_ref.to_string(),
                })?;
            summarize_sla(&observation, self.policy).map_err(Into::into)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_sla_observability_api::{ObservedControlPlaneStatus, StatusWindow};
    use k8s_sla_observability_kernel::{AvailabilityState, ProvisioningLatencyState};

    fn snapshot(status: ObservedControlPlaneStatus) -> ControlPlaneSlaSnapshot {
        ControlPlaneSlaSnapshot::new(
            ClusterRef::new("ten_acme", "prod-a"),
            status,
            StatusWindow::new(100, 100),
            Some(120_000),
        )
    }

    #[tokio::test]
    async fn ingest_and_summarize_healthy_cluster() {
        let store = InMemorySlaObservabilityStore::default();
        let summary = store
            .ingest_status_snapshot(snapshot(ObservedControlPlaneStatus::Active))
            .await
            .unwrap();
        assert_eq!(summary.availability.state, AvailabilityState::Available);
        let read = store
            .summarize_cluster(&ClusterRef::new("ten_acme", "prod-a"))
            .await
            .unwrap();
        assert_eq!(summary, read);
    }

    #[tokio::test]
    async fn degraded_snapshot_burns_budget() {
        let store = InMemorySlaObservabilityStore::default();
        let summary = store
            .ingest_status_snapshot(ControlPlaneSlaSnapshot::new(
                ClusterRef::new("ten_acme", "prod-a"),
                ObservedControlPlaneStatus::Failed,
                StatusWindow::new(100, 80),
                Some(900_000),
            ))
            .await
            .unwrap();
        assert!(summary.error_budget.exhausted);
        assert_eq!(
            summary.provisioning_latency.state,
            ProvisioningLatencyState::Breached
        );
    }

    #[tokio::test]
    async fn unknown_cluster_fails_closed() {
        let store = InMemorySlaObservabilityStore::default();
        let err = store
            .summarize_cluster(&ClusterRef::new("ten_acme", "ghost"))
            .await
            .unwrap_err();
        assert!(matches!(err, SlaObservabilityError::UnknownCluster { .. }));
    }

    #[tokio::test]
    async fn poisoned_store_is_internal_error_not_unknown_cluster() {
        let store = InMemorySlaObservabilityStore::default();
        store.poison_for_test();
        let err = store
            .summarize_cluster(&ClusterRef::new("ten_acme", "prod-a"))
            .await
            .unwrap_err();
        assert!(matches!(err, SlaObservabilityError::Store { .. }));
    }
}
