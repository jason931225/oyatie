//! In-memory quota store fake for tests and single-node bring-up.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use k8s_tenant_quota_api::{QuotaAdminPort, QuotaDecisionPort, QuotaPortError, evaluate};
use k8s_tenant_quota_kernel::{
    ProvisionRequest, QuotaDecision, TenantId, TenantQuota, TenantUsage,
};

/// Test and bring-up only; production wires a Postgres-backed adapter behind
/// the same port.
#[derive(Clone, Default)]
pub struct InMemoryQuotaStore {
    quotas: Arc<Mutex<BTreeMap<String, TenantQuota>>>,
    usages: Arc<Mutex<BTreeMap<String, TenantUsage>>>,
}

impl InMemoryQuotaStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// # Panics
    /// Panics only if the internal Mutex is poisoned (test environment only).
    #[must_use]
    pub fn with_quota(self, quota: TenantQuota) -> Self {
        self.quotas
            .lock()
            .expect("quota lock")
            .insert(quota.tenant_id.as_str().to_string(), quota);
        self
    }

    /// # Panics
    /// Panics only if the internal Mutex is poisoned (test environment only).
    #[must_use]
    pub fn with_usage(self, usage: TenantUsage) -> Self {
        self.usages
            .lock()
            .expect("usage lock")
            .insert(usage.tenant_id.as_str().to_string(), usage);
        self
    }
}

impl QuotaDecisionPort for InMemoryQuotaStore {
    fn check_quota(&self, request: &ProvisionRequest) -> Result<QuotaDecision, QuotaPortError> {
        let tenant_key = request.tenant_id.as_str().to_string();

        let quota = self
            .quotas
            .lock()
            .map_err(|e| QuotaPortError::Persistence(e.to_string()))?
            .get(&tenant_key)
            .cloned()
            .ok_or_else(|| QuotaPortError::NotFound(tenant_key.clone()))?;

        let usage = self
            .usages
            .lock()
            .map_err(|e| QuotaPortError::Persistence(e.to_string()))?
            .get(&tenant_key)
            .cloned()
            .unwrap_or_else(|| TenantUsage {
                tenant_id: request.tenant_id.clone(),
                current_clusters: 0,
                max_nodes_in_any_cluster: 0,
                max_vcpu_in_any_cluster: 0,
                max_ram_gib_in_any_cluster: 0,
            });

        Ok(evaluate(&quota, &usage, request))
    }
}

impl QuotaAdminPort for InMemoryQuotaStore {
    fn set_quota(&self, quota: TenantQuota) -> Result<(), QuotaPortError> {
        self.quotas
            .lock()
            .map_err(|e| QuotaPortError::Persistence(e.to_string()))?
            .insert(quota.tenant_id.as_str().to_string(), quota);
        Ok(())
    }

    fn get_quota(&self, tenant_id: &TenantId) -> Result<TenantQuota, QuotaPortError> {
        self.quotas
            .lock()
            .map_err(|e| QuotaPortError::Persistence(e.to_string()))?
            .get(tenant_id.as_str())
            .cloned()
            .ok_or_else(|| QuotaPortError::NotFound(tenant_id.as_str().to_string()))
    }

    fn get_usage(&self, tenant_id: &TenantId) -> Result<TenantUsage, QuotaPortError> {
        self.usages
            .lock()
            .map_err(|e| QuotaPortError::Persistence(e.to_string()))?
            .get(tenant_id.as_str())
            .cloned()
            .ok_or_else(|| QuotaPortError::NotFound(tenant_id.as_str().to_string()))
    }

    fn set_usage(&self, usage: TenantUsage) -> Result<(), QuotaPortError> {
        self.usages
            .lock()
            .map_err(|e| QuotaPortError::Persistence(e.to_string()))?
            .insert(usage.tenant_id.as_str().to_string(), usage);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k8s_tenant_quota_kernel::{ProvisionRequest, QuotaDecision, TenantQuota, TenantUsage};

    fn quota(tenant: &str) -> TenantQuota {
        TenantQuota::new(tenant, 5, 10, 32, 128).unwrap()
    }

    fn usage(tenant: &str, clusters: u32) -> TenantUsage {
        TenantUsage::new(tenant, clusters, 0, 0, 0).unwrap()
    }

    fn request(tenant: &str) -> ProvisionRequest {
        ProvisionRequest::new(tenant, 1, 3, 8, 32).unwrap()
    }

    #[test]
    fn unseeded_tenant_returns_not_found() {
        let store = InMemoryQuotaStore::new();
        let req = request("ten_acme");
        let err = store.check_quota(&req).unwrap_err();
        assert!(matches!(err, QuotaPortError::NotFound(_)));
    }

    #[test]
    fn seeded_quota_allows_within_limits() {
        let store = InMemoryQuotaStore::new().with_quota(quota("ten_acme"));
        let req = request("ten_acme");
        assert_eq!(store.check_quota(&req).unwrap(), QuotaDecision::Allow);
    }

    #[test]
    fn seeded_quota_with_usage_denies_when_exceeded() {
        let store = InMemoryQuotaStore::new()
            .with_quota(quota("ten_acme"))
            .with_usage(usage("ten_acme", 5));
        let req = request("ten_acme");
        let decision = store.check_quota(&req).unwrap();
        assert!(decision.is_deny());
    }

    #[test]
    fn get_quota_returns_seeded_record() {
        let store = InMemoryQuotaStore::new().with_quota(quota("ten_acme"));
        let tid = TenantId::new("ten_acme").unwrap();
        let got = store.get_quota(&tid).unwrap();
        assert_eq!(got.max_clusters, 5);
    }

    #[test]
    fn set_and_get_usage_round_trip() {
        let store = InMemoryQuotaStore::new();
        let u = usage("ten_acme", 2);
        store.set_usage(u.clone()).unwrap();
        let tid = TenantId::new("ten_acme").unwrap();
        let got = store.get_usage(&tid).unwrap();
        assert_eq!(got.current_clusters, 2);
    }

    #[test]
    fn missing_usage_defaults_to_zero() {
        let store = InMemoryQuotaStore::new().with_quota(quota("ten_acme"));
        let req = request("ten_acme");
        assert_eq!(store.check_quota(&req).unwrap(), QuotaDecision::Allow);
    }
}
