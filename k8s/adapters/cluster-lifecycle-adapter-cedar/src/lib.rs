//! Cedar RBAC adapter for managed-K8s cluster lifecycle admission.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use iam_identity_workload_authz_cedar::{
    ActionCondition, CedarWorkloadAuthorizer, Policy, PrincipalCondition, ResourceCondition,
    WorkloadAuthorizer,
};
use iam_identity_workload_domain::{
    Action, AuthorizationRequest, ClaimValue, Resource, WorkloadPrincipal,
};

const PLATFORM_CLUSTER_SCOPE: &str = "cluster:platform:write";
const CLUSTER_RESOURCE_TYPE: &str = "ManagedCluster";
const CLUSTER_CREATE_ACTION: &str = "cluster:Create";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RbacAuthzError {
    PolicyBuild(String),
    Denied(String),
}

impl std::fmt::Display for RbacAuthzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyBuild(detail) => {
                write!(f, "cluster-lifecycle rbac policy build failed: {detail}")
            }
            Self::Denied(reason) => write!(f, "cluster-lifecycle rbac denied: {reason}"),
        }
    }
}

impl std::error::Error for RbacAuthzError {}

/// Cedar default-deny guarantees that absent policies = deny.
pub struct ClusterLifecycleRbacAuthorizer {
    inner: CedarWorkloadAuthorizer,
}

impl ClusterLifecycleRbacAuthorizer {
    /// # Errors
    /// Returns [`RbacAuthzError::PolicyBuild`] if any policy fails to compile.
    pub fn with_policies(policies: Vec<Policy>) -> Result<Self, RbacAuthzError> {
        let inner = CedarWorkloadAuthorizer::with_policies(policies)
            .map_err(|e| RbacAuthzError::PolicyBuild(e.to_string()))?;
        Ok(Self { inner })
    }

    /// # Errors
    /// Returns [`RbacAuthzError::PolicyBuild`] if policy compilation fails.
    pub fn new_with_default_policies() -> Result<Self, RbacAuthzError> {
        Self::with_policies(default_cluster_policies())
    }

    /// # Errors
    /// Returns [`RbacAuthzError::Denied`] if Cedar (or the cross-tenant guard)
    /// denies the request.
    pub fn authorize_cluster_create(
        &self,
        principal: &WorkloadPrincipal,
        target_tenant_id: &str,
    ) -> Result<(), RbacAuthzError> {
        deny_cross_tenant_without_platform_scope(principal, target_tenant_id)?;

        let request = AuthorizationRequest::new(
            principal.clone(),
            Action::new(CLUSTER_CREATE_ACTION),
            cluster_resource(target_tenant_id),
        );
        let decision = self.inner.authorize(&request);
        if decision.is_allow() {
            Ok(())
        } else {
            Err(RbacAuthzError::Denied(format!(
                "principal {} denied {CLUSTER_CREATE_ACTION} on tenant {target_tenant_id}",
                principal.workload_id().as_str(),
            )))
        }
    }
}

fn default_cluster_policies() -> Vec<Policy> {
    vec![
        Policy::permit("cluster-create-tenant-admin")
            .when_principal(PrincipalCondition::HasScope("cluster:write".into()))
            .for_action(ActionCondition::Equals(CLUSTER_CREATE_ACTION.into()))
            .for_resource(ResourceCondition::SameTenantAsPrincipal {
                resource_type: CLUSTER_RESOURCE_TYPE.into(),
            }),
        Policy::permit("cluster-create-platform-operator")
            .when_principal(PrincipalCondition::HasScope(PLATFORM_CLUSTER_SCOPE.into()))
            .for_action(ActionCondition::Equals(CLUSTER_CREATE_ACTION.into()))
            .for_resource(ResourceCondition::TypeIs(CLUSTER_RESOURCE_TYPE.into())),
    ]
}

fn deny_cross_tenant_without_platform_scope(
    principal: &WorkloadPrincipal,
    target_tenant_id: &str,
) -> Result<(), RbacAuthzError> {
    if principal.tenant_id().as_str() == target_tenant_id
        || principal.has_scope(PLATFORM_CLUSTER_SCOPE)
    {
        return Ok(());
    }
    Err(RbacAuthzError::Denied(format!(
        "principal {} denied cross-tenant {CLUSTER_CREATE_ACTION} on tenant {target_tenant_id}",
        principal.workload_id().as_str(),
    )))
}

fn cluster_resource(target_tenant_id: &str) -> Resource {
    Resource::new(CLUSTER_RESOURCE_TYPE, target_tenant_id)
        .with_attribute("tenant_id", ClaimValue::Text(target_tenant_id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use iam_identity_workload_domain::{Effect, WorkloadState};

    fn active_principal(tenant: &str, scope: &str) -> WorkloadPrincipal {
        let mut p =
            WorkloadPrincipal::provision(tenant, "wl_admin_01", "cap.k8s.cluster-lifecycle")
                .expect("provision");
        p.transition_to(WorkloadState::Active).expect("activate");
        p.with_scope(scope).expect("scope")
    }

    #[test]
    fn tenant_admin_can_create_own_cluster() {
        let authz = ClusterLifecycleRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "cluster:write");
        assert!(
            authz
                .authorize_cluster_create(&principal, "ten_acme")
                .is_ok()
        );
    }

    #[test]
    fn tenant_admin_cannot_create_other_tenant_cluster() {
        let authz = ClusterLifecycleRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "cluster:write");
        assert!(
            authz
                .authorize_cluster_create(&principal, "ten_globex")
                .is_err()
        );
    }

    #[test]
    fn platform_operator_can_create_any_tenant_cluster() {
        let authz = ClusterLifecycleRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_platform", "cluster:platform:write");
        assert!(
            authz
                .authorize_cluster_create(&principal, "ten_acme")
                .is_ok()
        );
    }

    #[test]
    fn principal_without_scope_denied() {
        let authz = ClusterLifecycleRbacAuthorizer::new_with_default_policies().unwrap();
        let principal = active_principal("ten_acme", "other:scope");
        assert!(
            authz
                .authorize_cluster_create(&principal, "ten_acme")
                .is_err()
        );
    }

    #[test]
    fn default_pdp_policy_denies_cross_tenant_create() {
        let pdp = CedarWorkloadAuthorizer::with_policies(default_cluster_policies()).unwrap();
        let principal = active_principal("ten_acme", "cluster:write");
        let request = AuthorizationRequest::new(
            principal,
            Action::new(CLUSTER_CREATE_ACTION),
            cluster_resource("ten_globex"),
        );
        assert_eq!(pdp.authorize(&request).effect(), Effect::Deny);
    }
}
