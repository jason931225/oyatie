//! The vendor-neutral embedded-PDP PORT: the [`PolicyDecisionPoint`] trait
//! over the locked PDP contract family in
//! `shared-platform-contracts-kernel::pdp`, plus the value types every
//! engine adapter consumes — [`PolicyBundle`] (version-bearing policy bundle
//! as pushed by the policy store), [`EntitySlice`] (the PIP entity slice a
//! PEP assembles per request), [`DecisionCache`] keyed on
//! `(request-fingerprint, policy-version)`, and [`DecisionAuditRecord`]
//! (audit record per decision — every decision, allow or deny, cached or
//! evaluated, is attributable).
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

mod bundle;
mod cache;
mod decision_authz;
mod entity;
mod error;
mod port;
mod runtime;

pub use bundle::{PolicyBundle, TemplateLink, TemplateSrc};
pub use cache::{CachedDecision, DecisionCache, DecisionCacheKey, request_fingerprint};
pub use decision_authz::{
    DecisionAuthorizer, DecisionAuthzError, DecisionAuthzRequest, FailClosedDecisionAuthorizer,
};
pub use entity::{EntityRecord, EntitySlice};
pub use error::PdpError;
pub use port::{DecisionAuditRecord, PdpOutcome, PolicyDecisionPoint};
pub use runtime::{
    PdpCircuitState, PdpRuntimeConfig, PdpRuntimeGuard, PdpRuntimeMetrics,
    PdpRuntimeMetricsSnapshot,
};

#[cfg(test)]
mod tests;
