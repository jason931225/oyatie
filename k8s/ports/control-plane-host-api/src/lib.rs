//! Managed-Kubernetes control-plane-host API / port layer (ADR-0376).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use core::fmt;
use core::pin::Pin;

use serde::{Deserialize, Serialize};

pub use k8s_control_plane_host_kernel::{
    ControlPlaneStatus, ControlPlaneTier, DatastoreClass, IllegalTransition,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn core::future::Future<Output = T> + Send + 'a>>;

/// Stable, tenant-scoped reference to the tenant CLUSTER whose control plane is
/// being provisioned.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ClusterRef {
    /// Tenant that owns the cluster. Tenant-zero (Oyatie dogfood) is an ordinary
    /// value here — there is NO internal-bypass identity (ADR-0376).
    pub tenant_id: String, // data_class: TENANT_SCOPED
    /// Tenant-unique cluster name.
    pub cluster_name: String, // data_class: TENANT_SCOPED
}

impl ClusterRef {
    #[must_use]
    pub fn new(tenant_id: impl Into<String>, cluster_name: impl Into<String>) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            cluster_name: cluster_name.into(),
        }
    }

    /// Whether both identity components are non-blank.
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

/// Request to provision a tenant control plane. `datastore_class` is meaningful
/// only for [`ControlPlaneTier::HostedKamaji`]; a spoke carries its own etcd.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProvisionRequest {
    pub cluster_ref: ClusterRef,
    pub tier: ControlPlaneTier,
    pub datastore_class: DatastoreClass,
}

impl ProvisionRequest {
    #[must_use]
    pub fn new(
        cluster_ref: ClusterRef,
        tier: ControlPlaneTier,
        datastore_class: DatastoreClass,
    ) -> Self {
        Self {
            cluster_ref,
            tier,
            datastore_class,
        }
    }
}

/// Opaque, adapter-issued handle to a provisioned control plane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlPlaneRef {
    pub cluster_ref: ClusterRef,
    pub tier: ControlPlaneTier,
    /// Adapter-issued opaque handle (e.g. the Kamaji `TenantControlPlane`
    /// namespaced name, or the Talos spoke id). Treated as opaque by callers.
    pub handle: String, // data_class: TENANT_SCOPED
}

impl ControlPlaneRef {
    /// Construct a control-plane reference.
    #[must_use]
    pub fn new(cluster_ref: ClusterRef, tier: ControlPlaneTier, handle: impl Into<String>) -> Self {
        Self {
            cluster_ref,
            tier,
            handle: handle.into(),
        }
    }
}

/// A point-in-time status report for a tenant control plane.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlPlaneStatusReport {
    pub control_plane_ref: ControlPlaneRef,
    pub status: ControlPlaneStatus,
    /// The API-server endpoint, once the control plane reaches
    /// [`ControlPlaneStatus::EndpointReady`] or later. `None` before that.
    pub endpoint: Option<String>, // data_class: TENANT_SCOPED
}

impl ControlPlaneStatusReport {
    #[must_use]
    pub fn new(
        control_plane_ref: ControlPlaneRef,
        status: ControlPlaneStatus,
        endpoint: Option<String>,
    ) -> Self {
        Self {
            control_plane_ref,
            status,
            endpoint,
        }
    }
}

/// Downstream paths an adapter CLAIMS but does NOT implement end-to-end; it returns
/// [`ProvisioningError::Unimplemented`] carrying one rather than a fake `Ok(...)`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unimplemented {
    /// Live reconciliation of a Kamaji `TenantControlPlane` against the CAPI providers.
    KamajiProviderLiveIntegration,
}

impl Unimplemented {
    /// Stable human-facing slug for this boundary.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::KamajiProviderLiveIntegration => "Unimplemented::KamajiProviderLiveIntegration",
        }
    }

    /// Stable placeholder-debt id this boundary maps to.
    #[must_use]
    pub const fn placeholder_debt_id(&self) -> &'static str {
        match self {
            Self::KamajiProviderLiveIntegration => "kamaji-provider-live-integration",
        }
    }
}

impl fmt::Display for Unimplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Failure modes from the [`ControlPlaneProvisioning`] port.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProvisioningError {
    InvalidClusterRef {
        cluster_ref: String,
    },
    NotFound {
        handle: String,
    },
    IllegalTransition(IllegalTransition),
    /// The backend could not be reached or returned an error. Operator-facing
    /// detail only — NEVER carries tenant secrets or kubeconfig material.
    Backend {
        /// Human-facing detail for logs.
        detail: String, // data_class: INTERNAL_ONLY
    },
    Unimplemented(Unimplemented),
}

impl ProvisioningError {
    #[must_use]
    pub fn backend(detail: impl Into<String>) -> Self {
        Self::Backend {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidClusterRef { cluster_ref } => {
                write!(f, "invalid cluster reference: {cluster_ref:?}")
            }
            Self::NotFound { handle } => write!(f, "control plane not found: {handle:?}"),
            Self::IllegalTransition(transition) => write!(f, "{transition}"),
            Self::Backend { detail } => write!(f, "control-plane-host backend error: {detail}"),
            Self::Unimplemented(boundary) => write!(
                f,
                "{boundary} — honest-deferred; see registry/placeholder-debt/adr-follow-ups.yaml#{}",
                boundary.placeholder_debt_id()
            ),
        }
    }
}

impl std::error::Error for ProvisioningError {}

impl From<IllegalTransition> for ProvisioningError {
    fn from(value: IllegalTransition) -> Self {
        Self::IllegalTransition(value)
    }
}

/// The shared control-plane provisioning port (ADR-0376).
///
/// Object-safe: every method returns a boxed `Send` future so the composition root
/// can hold `Arc<dyn ControlPlaneProvisioning>` and swap adapters.
pub trait ControlPlaneProvisioning: Send + Sync {
    /// Provision a tenant control plane for `request.cluster_ref` under `request.tier`.
    ///
    /// # Errors
    /// [`ProvisioningError::InvalidClusterRef`] for a malformed ref, `Backend` for a
    /// backend failure, or `Unimplemented` when the live CRD path is deferred.
    fn provision<'a>(
        &'a self,
        request: &'a ProvisionRequest,
    ) -> BoxFuture<'a, Result<ControlPlaneRef, ProvisioningError>>;

    /// Read the current status of the control plane identified by `control_plane_ref`.
    ///
    /// # Errors
    /// [`ProvisioningError::NotFound`] if no control plane is known for the handle,
    /// `Backend` for a backend failure, or `Unimplemented` when the CRD path is deferred.
    fn status<'a>(
        &'a self,
        control_plane_ref: &'a ControlPlaneRef,
    ) -> BoxFuture<'a, Result<ControlPlaneStatusReport, ProvisioningError>>;

    /// Tear down the control plane identified by `control_plane_ref` (drain then delete).
    /// Idempotent: tearing down an already-deleted or unknown control plane is not an error.
    ///
    /// # Errors
    /// [`ProvisioningError::Backend`] for a backend failure, or `Unimplemented` when
    /// the live CRD path is deferred.
    fn teardown<'a>(
        &'a self,
        control_plane_ref: &'a ControlPlaneRef,
    ) -> BoxFuture<'a, Result<(), ProvisioningError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cluster_ref_wellformed_rejects_empty() {
        assert!(ClusterRef::new("ten_zero", "dogfood-a").is_well_formed());
        assert!(!ClusterRef::new("", "c").is_well_formed());
        assert!(!ClusterRef::new("t", "  ").is_well_formed());
    }

    #[test]
    fn provision_request_serde_roundtrip() {
        let req = ProvisionRequest::new(
            ClusterRef::new("ten_zero", "dogfood-a"),
            ControlPlaneTier::HostedKamaji,
            DatastoreClass::EtcdPerTenant,
        );
        let json = serde_json::to_string(&req).expect("serialize");
        let back: ProvisionRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(req, back);
        assert!(json.contains("hosted_kamaji"));
        assert!(json.contains("etcd_per_tenant"));
    }

    #[test]
    fn unimplemented_maps_to_stable_debt_id() {
        assert_eq!(
            Unimplemented::KamajiProviderLiveIntegration.placeholder_debt_id(),
            "kamaji-provider-live-integration"
        );
    }

    #[test]
    fn provisioning_error_display_cites_placeholder_debt() {
        let err = ProvisioningError::Unimplemented(Unimplemented::KamajiProviderLiveIntegration);
        let rendered = err.to_string();
        assert!(rendered.contains("kamaji-provider-live-integration"));
        assert!(rendered.contains("registry/placeholder-debt/adr-follow-ups.yaml"));
    }

    #[test]
    fn illegal_transition_converts_into_provisioning_error() {
        let transition = ControlPlaneStatus::Requested
            .transition(ControlPlaneStatus::Active)
            .expect_err("illegal");
        let err: ProvisioningError = transition.into();
        assert!(matches!(err, ProvisioningError::IllegalTransition(_)));
    }
}
