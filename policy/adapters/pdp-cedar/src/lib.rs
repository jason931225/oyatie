//! Embedded Cedar policy decision point.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, RwLock};

use cedar_policy::{Authorizer, EntityUid, PolicySet, Schema};
use shared_pdp_kernel::{DecisionCache, PdpError, PolicyBundle};
use shared_platform_contracts_kernel::pdp::PolicyVersion;
use shared_ulid_id_kernel::IdGenerator;

mod admission;
mod audit;
mod entity;
mod evaluation;
mod overlay;

use admission::compile;
pub use admission::validate_bundle;
pub use audit::{
    AuditChainCedarPdp, PDP_DECISION_AUDIT_SURFACE, PdpAuditChainError, PdpDecisionAuditChainLogger,
};

struct LoadedBundle {
    version: PolicyVersion,
    source_identity: Vec<u8>,
    schema: Schema,
    policy_set: PolicySet,
    /// Cross-tenant GRANT isolation is a separate, stronger guarantee enforced
    /// at runtime by the global `structural-tenant-isolation` forbid present in
    /// every merged set — not by this selection keying.
    tenant_policy_sets: BTreeMap<String, PolicySet>,
    action_map: BTreeMap<String, EntityUid>,
}

impl LoadedBundle {
    fn policy_set_for(&self, tenant_id: &str) -> &PolicySet {
        self.tenant_policy_sets
            .get(tenant_id)
            .unwrap_or(&self.policy_set)
    }
}

pub struct CedarPdp {
    state: RwLock<LoadedBundle>,
    cache: Mutex<DecisionCache>,
    cache_capacity: usize,
    id_gen: Arc<dyn IdGenerator>,
    authorizer: Authorizer,
}

impl CedarPdp {
    /// Compile and strict-validate `bundle`, then serve from it.
    ///
    /// # Errors
    /// [`PdpError::BundleRejected`] when any part of the bundle fails to
    /// parse, link, or strict-validate — nothing is loaded in that case.
    pub fn load(
        bundle: &PolicyBundle,
        id_gen: Arc<dyn IdGenerator>,
        cache_capacity: usize,
    ) -> Result<Self, PdpError> {
        let state = compile(bundle)?;
        Ok(Self {
            state: RwLock::new(state),
            cache: Mutex::new(DecisionCache::new(cache_capacity)),
            cache_capacity,
            id_gen,
            authorizer: Authorizer::new(),
        })
    }

    /// Atomically replace the serving bundle (the revocation path). The new
    /// bundle is fully compiled and strict-validated BEFORE the swap; on any
    /// error the current bundle keeps serving (fail closed). A replacement
    /// invalidates cached content, including content from a previous use of its
    /// opaque version. An identical reload is a no-op.
    ///
    /// # Errors
    /// [`PdpError::BundleRejected`] when admission fails or the current version
    /// is redefined; [`PdpError::Evaluation`] when a serving-state lock is poisoned.
    pub fn swap_bundle(&self, bundle: &PolicyBundle) -> Result<(), PdpError> {
        let next = compile(bundle)?;
        let mut state = self.state.write().map_err(|_| PdpError::Evaluation {
            detail: "policy state lock poisoned".to_owned(),
        })?;
        if state.source_identity == next.source_identity {
            return Ok(());
        }
        if state.version == next.version {
            return Err(PdpError::BundleRejected {
                detail: format!(
                    "serving policy version {} cannot be redefined",
                    state.version.as_str()
                ),
            });
        }
        let mut cache = self.cache.lock().map_err(|_| PdpError::Evaluation {
            detail: "decision cache lock poisoned".to_owned(),
        })?;
        // Authorization retains the state read lock through cache insertion.
        // Holding the write lock makes this invalidation and replacement atomic.
        *cache = DecisionCache::new(self.cache_capacity);
        *state = next;
        Ok(())
    }
}
