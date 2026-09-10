//! QuotaDecision PORT + DTOs for managed-K8s tenant quota.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub use k8s_tenant_quota_kernel::{
    DenyReason, ProvisionRequest, QuotaDecision, QuotaModelError, RbacBinding, RbacRole, TenantId,
    TenantQuota, TenantUsage, evaluate,
};

/// The port cluster-lifecycle calls before provisioning a cluster.
///
/// Implementations MUST be:
/// - **Deny-by-default**: if no quota record exists for the tenant, deny.
/// - **Cross-tenant safe**: an implementation MUST NOT return quota data for a
///   tenant other than the one in the request.
/// - **Fail-closed**: any store error is surfaced as `QuotaPortError`, never as
///   a silent allow.
pub trait QuotaDecisionPort {
    /// # Errors
    /// Returns [`QuotaPortError`] on persistence failure or tenant-not-found.
    fn check_quota(&self, request: &ProvisionRequest) -> Result<QuotaDecision, QuotaPortError>;
}

pub trait QuotaAdminPort {
    /// # Errors
    /// Returns [`QuotaPortError`] on validation or persistence failure.
    fn set_quota(&self, quota: TenantQuota) -> Result<(), QuotaPortError>;

    /// # Errors
    /// Returns [`QuotaPortError::NotFound`] when no record exists,
    /// or [`QuotaPortError::Persistence`] on store failure.
    fn get_quota(&self, tenant_id: &TenantId) -> Result<TenantQuota, QuotaPortError>;

    /// # Errors
    /// Returns [`QuotaPortError::NotFound`] when no record exists,
    /// or [`QuotaPortError::Persistence`] on store failure.
    fn get_usage(&self, tenant_id: &TenantId) -> Result<TenantUsage, QuotaPortError>;

    /// # Errors
    /// Returns [`QuotaPortError`] on persistence failure.
    fn set_usage(&self, usage: TenantUsage) -> Result<(), QuotaPortError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuotaPortError {
    NotFound(String),
    Persistence(String),
    Validation(String),
}

impl std::fmt::Display for QuotaPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "quota record not found for tenant {id}"),
            Self::Persistence(detail) => write!(f, "quota store error: {detail}"),
            Self::Validation(detail) => write!(f, "quota validation error: {detail}"),
        }
    }
}

impl std::error::Error for QuotaPortError {}

impl From<QuotaModelError> for QuotaPortError {
    fn from(e: QuotaModelError) -> Self {
        Self::Validation(e.to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuotaDto {
    pub tenant_id: String,
    pub max_clusters: u32,
    pub max_nodes_per_cluster: u32,
    pub max_vcpu_per_cluster: u32,
    pub max_ram_gib_per_cluster: u32,
}

impl QuotaDto {
    /// # Errors
    /// Returns [`QuotaPortError::Validation`] if the values fail kernel validation.
    pub fn into_quota(self) -> Result<TenantQuota, QuotaPortError> {
        TenantQuota::new(
            self.tenant_id,
            self.max_clusters,
            self.max_nodes_per_cluster,
            self.max_vcpu_per_cluster,
            self.max_ram_gib_per_cluster,
        )
        .map_err(Into::into)
    }
}

impl From<TenantQuota> for QuotaDto {
    fn from(q: TenantQuota) -> Self {
        Self {
            tenant_id: q.tenant_id.as_str().to_string(),
            max_clusters: q.max_clusters,
            max_nodes_per_cluster: q.max_nodes_per_cluster,
            max_vcpu_per_cluster: q.max_vcpu_per_cluster,
            max_ram_gib_per_cluster: q.max_ram_gib_per_cluster,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UsageDto {
    pub tenant_id: String,
    pub current_clusters: u32,
    pub max_nodes_in_any_cluster: u32,
    pub max_vcpu_in_any_cluster: u32,
    pub max_ram_gib_in_any_cluster: u32,
}

impl From<TenantUsage> for UsageDto {
    fn from(u: TenantUsage) -> Self {
        Self {
            tenant_id: u.tenant_id.as_str().to_string(),
            current_clusters: u.current_clusters,
            max_nodes_in_any_cluster: u.max_nodes_in_any_cluster,
            max_vcpu_in_any_cluster: u.max_vcpu_in_any_cluster,
            max_ram_gib_in_any_cluster: u.max_ram_gib_in_any_cluster,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuotaCheckResponse {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny_reason: Option<String>,
}

impl From<QuotaDecision> for QuotaCheckResponse {
    fn from(d: QuotaDecision) -> Self {
        match d {
            QuotaDecision::Allow => Self {
                allowed: true,
                deny_reason: None,
            },
            QuotaDecision::Deny(reason) => Self {
                allowed: false,
                deny_reason: Some(reason.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_dto_round_trip() {
        let dto = QuotaDto {
            tenant_id: "ten_acme".to_string(),
            max_clusters: 5,
            max_nodes_per_cluster: 10,
            max_vcpu_per_cluster: 32,
            max_ram_gib_per_cluster: 128,
        };
        let quota = dto.clone().into_quota().unwrap();
        let back: QuotaDto = quota.into();
        assert_eq!(back, dto);
    }

    #[test]
    fn quota_check_response_allow() {
        let resp: QuotaCheckResponse = QuotaDecision::Allow.into();
        assert!(resp.allowed);
        assert!(resp.deny_reason.is_none());
    }

    #[test]
    fn quota_check_response_deny() {
        let resp: QuotaCheckResponse = QuotaDecision::Deny(DenyReason::ClusterLimitExceeded {
            current: 4,
            requested: 2,
            limit: 5,
        })
        .into();
        assert!(!resp.allowed);
        assert!(resp.deny_reason.is_some());
    }

    #[test]
    fn port_error_display() {
        assert!(
            QuotaPortError::NotFound("ten_x".into())
                .to_string()
                .contains("ten_x")
        );
    }
}
