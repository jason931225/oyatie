//! kube-rs adapter for the managed-Kubernetes control-plane-host port.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use kube::Client;
use kube::api::ApiResource;
use kube::config::{Config, KubeConfigOptions, Kubeconfig};
use kube::core::GroupVersionKind;

use k8s_control_plane_host_api::{
    BoxFuture, ControlPlaneProvisioning, ControlPlaneRef, ControlPlaneStatusReport,
    ProvisionRequest, ProvisioningError, Unimplemented,
};

pub const CAPI_CONTROL_PLANE_GROUP: &str = "controlplane.cluster.x-k8s.io";
pub const KAMAJI_TENANT_CONTROL_PLANE_KIND: &str = "TenantControlPlane";
pub const KAMAJI_CONTROL_PLANE_KIND: &str = "KamajiControlPlane";
pub const KAMAJI_CONTROL_PLANE_VERSION: &str = "v1alpha1";

#[derive(Clone)]
pub struct CapiControlPlaneHost {
    client: Client,
    tenant_control_plane: ApiResource,
}

impl CapiControlPlaneHost {
    /// `client` must be connected to the MANAGEMENT cluster, never a tenant
    /// cluster. The dynamic [`ApiResource`] is derived from a
    /// [`GroupVersionKind`] so the live path can build a
    /// `kube::Api<DynamicObject>` without a compile-time CRD struct.
    #[must_use]
    pub fn new(client: Client) -> Self {
        let gvk = GroupVersionKind::gvk(
            CAPI_CONTROL_PLANE_GROUP,
            KAMAJI_CONTROL_PLANE_VERSION,
            KAMAJI_TENANT_CONTROL_PLANE_KIND,
        );
        let tenant_control_plane = ApiResource::from_gvk(&gvk);
        Self {
            client,
            tenant_control_plane,
        }
    }

    /// Exists so that kube-rs `Client`/`Config` construction stays inside this
    /// crate and the composition root never imports kube-rs.
    ///
    /// # Errors
    /// Returns a boxed error if the kubeconfig cannot be read/parsed or a client
    /// cannot be constructed from it.
    pub async fn from_kubeconfig_path(
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let kubeconfig = Kubeconfig::read_from(path)?;
        let config =
            Config::from_custom_kubeconfig(kubeconfig, &KubeConfigOptions::default()).await?;
        let client = Client::try_from(config)?;
        Ok(Self::new(client))
    }

    #[must_use]
    pub fn client(&self) -> &Client {
        &self.client
    }

    #[must_use]
    pub fn tenant_control_plane_resource(&self) -> &ApiResource {
        &self.tenant_control_plane
    }

    fn deferred() -> ProvisioningError {
        ProvisioningError::Unimplemented(Unimplemented::KamajiProviderLiveIntegration)
    }
}

impl ControlPlaneProvisioning for CapiControlPlaneHost {
    fn provision<'a>(
        &'a self,
        request: &'a ProvisionRequest,
    ) -> BoxFuture<'a, Result<ControlPlaneRef, ProvisioningError>> {
        // Validate the caller input fail-closed BEFORE reporting the deferred
        // boundary, so a malformed request is still rejected honestly.
        Box::pin(async move {
            if !request.cluster_ref.is_well_formed() {
                return Err(ProvisioningError::InvalidClusterRef {
                    cluster_ref: request.cluster_ref.to_string(),
                });
            }
            Err(Self::deferred())
        })
    }

    fn status<'a>(
        &'a self,
        _control_plane_ref: &'a ControlPlaneRef,
    ) -> BoxFuture<'a, Result<ControlPlaneStatusReport, ProvisioningError>> {
        Box::pin(async move { Err(Self::deferred()) })
    }

    fn teardown<'a>(
        &'a self,
        _control_plane_ref: &'a ControlPlaneRef,
    ) -> BoxFuture<'a, Result<(), ProvisioningError>> {
        Box::pin(async move { Err(Self::deferred()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_api_resource_descriptor_is_kamaji_tenant_control_plane() {
        let gvk = GroupVersionKind::gvk(
            CAPI_CONTROL_PLANE_GROUP,
            KAMAJI_CONTROL_PLANE_VERSION,
            KAMAJI_TENANT_CONTROL_PLANE_KIND,
        );
        let resource = ApiResource::from_gvk(&gvk);
        assert_eq!(resource.group, CAPI_CONTROL_PLANE_GROUP);
        assert_eq!(resource.version, KAMAJI_CONTROL_PLANE_VERSION);
        assert_eq!(resource.kind, KAMAJI_TENANT_CONTROL_PLANE_KIND);
    }

    #[test]
    fn deferred_boundary_cites_placeholder_debt() {
        let err = CapiControlPlaneHost::deferred();
        let rendered = err.to_string();
        assert!(rendered.contains("kamaji-provider-live-integration"));
        assert!(matches!(
            err,
            ProvisioningError::Unimplemented(Unimplemented::KamajiProviderLiveIntegration)
        ));
    }
}
