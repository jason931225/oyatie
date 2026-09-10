//! Cedar RBAC adapter for managed-K8s tenant quota: quota admin authorization
//! layered on `iam-identity-workload-authz-cedar`'s `CedarWorkloadAuthorizer`.
//! Cedar default-deny, plus an adapter-side cross-tenant guard (ADR-0376).

// ADR-0083 Tier-3: panic-free on the request path.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use iam_identity_workload_authz_cedar::{
    ActionCondition, CedarWorkloadAuthorizer, Policy, PrincipalCondition, ResourceCondition,
    WorkloadAuthorizer,
};
use iam_identity_workload_domain::{
    Action, AuthorizationRequest, ClaimValue, Resource, WorkloadPrincipal,
};
use k8s_tenant_quota_kernel::{RbacRole, TenantId};

const PLATFORM_QUOTA_SCOPE: &str = "quota:platform:write";

/// Errors from the Cedar RBAC authorizer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RbacAuthzError {
    PolicyBuild(String),
    Denied(String),
}

impl std::fmt::Display for RbacAuthzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyBuild(detail) => write!(f, "quota rbac policy build failed: {detail}"),
            Self::Denied(reason) => write!(f, "quota rbac denied: {reason}"),
        }
    }
}

impl std::error::Error for RbacAuthzError {}

/// Cedar-backed RBAC authorizer for quota admin operations. Cedar default-deny
/// means an absent policy denies.
pub struct QuotaRbacAuthorizer {
    inner: CedarWorkloadAuthorizer,
}

impl QuotaRbacAuthorizer {
    /// Build with explicit policies (test / custom policy injection path).
    ///
    /// # Errors
    /// Returns [`RbacAuthzError::PolicyBuild`] if any policy fails to compile.
    pub fn with_policies(policies: Vec<Policy>) -> Result<Self, RbacAuthzError> {
        let inner = CedarWorkloadAuthorizer::with_policies(policies)
            .map_err(|e| RbacAuthzError::PolicyBuild(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Build with the default quota RBAC policies (production path).
    ///
    /// # Errors
    /// Returns [`RbacAuthzError::PolicyBuild`] if policy compilation fails.
    pub fn new_with_default_policies() -> Result<Self, RbacAuthzError> {
        Self::with_policies(default_quota_policies())
    }

    /// Authorize a quota write operation for `target_tenant_id`.
    ///
    /// # Errors
    /// Returns [`RbacAuthzError::Denied`] if Cedar denies the request.
    pub fn authorize_quota_write(
        &self,
        principal: &WorkloadPrincipal,
        target_tenant_id: &TenantId,
    ) -> Result<(), RbacAuthzError> {
        deny_cross_tenant_without_platform_scope(principal, target_tenant_id, "quota:Write")?;

        let request = AuthorizationRequest::new(
            principal.clone(),
            Action::new("quota:Write"),
            quota_resource(target_tenant_id),
        );
        let decision = self.inner.authorize(&request);
        if decision.is_allow() {
            Ok(())
        } else {
            Err(RbacAuthzError::Denied(format!(
                "principal {} denied quota:Write on tenant {}",
                principal.workload_id().as_str(),
                target_tenant_id.as_str()
            )))
        }
    }

    /// Authorize a quota read operation for `target_tenant_id`.
    ///
    /// # Errors
    /// Returns [`RbacAuthzError::Denied`] if Cedar denies the request.
    pub fn authorize_quota_read(
        &self,
        principal: &WorkloadPrincipal,
        target_tenant_id: &TenantId,
    ) -> Result<(), RbacAuthzError> {
        deny_cross_tenant_without_platform_scope(principal, target_tenant_id, "quota:Read")?;

        let request = AuthorizationRequest::new(
            principal.clone(),
            Action::new("quota:Read"),
            quota_resource(target_tenant_id),
        );
        let decision = self.inner.authorize(&request);
        if decision.is_allow() {
            Ok(())
        } else {
            Err(RbacAuthzError::Denied(format!(
                "principal {} denied quota:Read on tenant {}",
                principal.workload_id().as_str(),
                target_tenant_id.as_str()
            )))
        }
    }

    /// The Cedar scope string an RBAC role must hold.
    #[must_use]
    pub fn scope_for_role(role: &RbacRole) -> &'static str {
        match role {
            RbacRole::TenantAdmin => "quota:write",
            RbacRole::TenantViewer => "quota:read",
            RbacRole::PlatformOperator => PLATFORM_QUOTA_SCOPE,
        }
    }
}

fn default_quota_policies() -> Vec<Policy> {
    vec![
        Policy::permit("quota-write-tenant-admin")
            .when_principal(PrincipalCondition::HasScope("quota:write".into()))
            .for_action(ActionCondition::Equals("quota:Write".into()))
            .for_resource(ResourceCondition::SameTenantAsPrincipal {
                resource_type: "QuotaRecord".into(),
            }),
        Policy::permit("quota-read-tenant")
            .when_principal(PrincipalCondition::HasScope("quota:read".into()))
            .for_action(ActionCondition::Equals("quota:Read".into()))
            .for_resource(ResourceCondition::SameTenantAsPrincipal {
                resource_type: "QuotaRecord".into(),
            }),
        Policy::permit("quota-write-platform-operator")
            .when_principal(PrincipalCondition::HasScope(PLATFORM_QUOTA_SCOPE.into()))
            .for_action(ActionCondition::Equals("quota:Write".into()))
            .for_resource(ResourceCondition::TypeIs("QuotaRecord".into())),
        Policy::permit("quota-read-platform-operator")
            .when_principal(PrincipalCondition::HasScope(PLATFORM_QUOTA_SCOPE.into()))
            .for_action(ActionCondition::Equals("quota:Read".into()))
            .for_resource(ResourceCondition::TypeIs("QuotaRecord".into())),
    ]
}

fn deny_cross_tenant_without_platform_scope(
    principal: &WorkloadPrincipal,
    target_tenant_id: &TenantId,
    action: &str,
) -> Result<(), RbacAuthzError> {
    if principal.tenant_id().as_str() == target_tenant_id.as_str()
        || principal.has_scope(PLATFORM_QUOTA_SCOPE)
    {
        return Ok(());
    }

    Err(RbacAuthzError::Denied(format!(
        "principal {} denied cross-tenant {action} on tenant {}",
        principal.workload_id().as_str(),
        target_tenant_id.as_str()
    )))
}

fn quota_resource(target_tenant_id: &TenantId) -> Resource {
    Resource::new("QuotaRecord", target_tenant_id.as_str()).with_attribute(
        "tenant_id",
        ClaimValue::Text(target_tenant_id.as_str().to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use iam_identity_workload_domain::{Effect, WorkloadState};

    fn active_principal(tenant: &str, scope: &str) -> WorkloadPrincipal {
        let mut p = WorkloadPrincipal::provision(tenant, "wl_admin_01", "cap.quota.admin")
            .expect("provision");
        p.transition_to(WorkloadState::Active).expect("activate");
        p.with_scope(scope).expect("scope")
    }

    #[test]
    fn tenant_admin_can_write_own_quota() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "quota:write");
        let tenant_id = TenantId::new("ten_acme").unwrap();
        assert!(authz.authorize_quota_write(&principal, &tenant_id).is_ok());
    }

    #[test]
    fn tenant_admin_can_read_own_quota() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "quota:read");
        let tenant_id = TenantId::new("ten_acme").unwrap();
        assert!(authz.authorize_quota_read(&principal, &tenant_id).is_ok());
    }

    #[test]
    fn principal_without_scope_denied_write() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "other:scope");
        let tenant_id = TenantId::new("ten_acme").unwrap();
        assert!(authz.authorize_quota_write(&principal, &tenant_id).is_err());
    }

    #[test]
    fn platform_operator_can_write_any_tenant() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_platform", "quota:platform:write");
        let tenant_id = TenantId::new("ten_acme").unwrap();
        assert!(authz.authorize_quota_write(&principal, &tenant_id).is_ok());
    }

    #[test]
    fn tenant_admin_cannot_write_other_tenant_quota() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "quota:write");
        let tenant_id = TenantId::new("ten_globex").unwrap();
        assert!(authz.authorize_quota_write(&principal, &tenant_id).is_err());
    }

    #[test]
    fn tenant_viewer_cannot_read_other_tenant_quota() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "quota:read");
        let tenant_id = TenantId::new("ten_globex").unwrap();
        assert!(authz.authorize_quota_read(&principal, &tenant_id).is_err());
    }

    #[test]
    fn default_pdp_policy_denies_cross_tenant_quota_read() {
        let pdp = CedarWorkloadAuthorizer::with_policies(default_quota_policies()).unwrap();
        let principal = active_principal("ten_acme", "quota:read");
        let tenant_id = TenantId::new("ten_globex").unwrap();
        let request = AuthorizationRequest::new(
            principal,
            Action::new("quota:Read"),
            quota_resource(&tenant_id),
        );

        let decision = pdp.authorize(&request);

        assert_eq!(decision.effect(), Effect::Deny);
    }

    #[test]
    fn default_pdp_policy_denies_cross_tenant_quota_write() {
        let pdp = CedarWorkloadAuthorizer::with_policies(default_quota_policies()).unwrap();
        let principal = active_principal("ten_acme", "quota:write");
        let tenant_id = TenantId::new("ten_globex").unwrap();
        let request = AuthorizationRequest::new(
            principal,
            Action::new("quota:Write"),
            quota_resource(&tenant_id),
        );

        let decision = pdp.authorize(&request);

        assert_eq!(decision.effect(), Effect::Deny);
    }

    #[test]
    fn platform_operator_can_read_any_tenant() {
        let authz = QuotaRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_platform", "quota:platform:write");
        let tenant_id = TenantId::new("ten_acme").unwrap();
        assert!(authz.authorize_quota_read(&principal, &tenant_id).is_ok());
    }

    #[test]
    fn scope_for_role_returns_correct_strings() {
        assert_eq!(
            QuotaRbacAuthorizer::scope_for_role(&RbacRole::TenantAdmin),
            "quota:write"
        );
        assert_eq!(
            QuotaRbacAuthorizer::scope_for_role(&RbacRole::TenantViewer),
            "quota:read"
        );
        assert_eq!(
            QuotaRbacAuthorizer::scope_for_role(&RbacRole::PlatformOperator),
            "quota:platform:write"
        );
    }
}
