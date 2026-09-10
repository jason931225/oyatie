//! Composition root for the runnable policy-decision-point service: it loads one
//! declarative policy bundle through [`iam_pdp_kernel::PolicyBundleStore`],
//! compiles it into the embedded Cedar engine, and serves decisions over gRPC and
//! REST. Authorization posture is recorded in ADR-0702.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::sync::Arc;

use iam_pdp_cedar::CedarPdp;
use iam_pdp_kernel::DecisionAuditSink;
use shared_pdp_kernel::{EntitySlice, PdpError, PolicyDecisionPoint};
use shared_platform_contracts_kernel::pdp::{
    AuthorizationRequest, AuthorizationResponse, PolicyVersion,
};

pub mod audit;
pub mod client_cert_verifier;
pub mod grpc;
pub mod idgen;
pub mod mtls;
pub mod mtls_transport;
pub mod observability;
pub mod rest;
pub mod server;

pub use iam_pdp_kernel::{
    ENV_BUNDLE_PATH, ENV_DECISION_CACHE_CAPACITY, ENV_GRPC_ADDR, ENV_REST_ADDR, PdpConfig,
};

/// REST and gRPC both decide through [`PdpState::decide`], so the protocols
/// cannot drift and audit-per-decision holds at exactly one place.
pub struct PdpState {
    pdp: CedarPdp,
    audit: Arc<dyn DecisionAuditSink>,
}

impl PdpState {
    #[must_use]
    pub fn new(pdp: CedarPdp, audit: Arc<dyn DecisionAuditSink>) -> Self {
        Self { pdp, audit }
    }

    #[must_use]
    pub fn loaded_policy_version(&self) -> PolicyVersion {
        self.pdp.loaded_policy_version()
    }

    /// An `Err` here is a REFUSAL, not a deny decision: the PDP declined to
    /// decide at all, and a PEP must still fail closed on it.
    ///
    /// # Errors
    /// [`PdpError`] when the PDP refuses to decide.
    pub fn decide(
        &self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<AuthorizationResponse, PdpError> {
        let outcome = self.pdp.authorize(request, entities)?;
        // Infallible by port contract: a sink failure must never become an
        // allow or a refusal, so the sink swallows its own errors.
        self.audit.record(&outcome.audit);
        Ok(outcome.response)
    }

    /// Refusals never enter the decision-audit chain, so this log line is the
    /// only trace a probing caller leaves.
    pub(crate) fn log_refusal(request_id: &str, error: &PdpError) {
        tracing::warn!(
            target: "cloud_iam_pdp::refusal",
            request_id,
            error = %error,
            "authorization request refused (fail-closed: PEPs must treat as deny)",
        );
    }
}
