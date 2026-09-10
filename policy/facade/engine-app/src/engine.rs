use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use policy_bundle_content::PolicySource;
use policy_cedar_domain::rebac::RebacTupleStore;
use policy_decision_domain::{DecisionError, DecisionInputs, JoinedDecision};
use policy_decision_service_kernel::{BundleStoreError, PolicyBundleStore};
use policy_pdp_cedar::CedarPdp;
use policy_pdp_kernel::{
    EntityRecord, EntitySlice, PdpError, PdpOutcome, PolicyBundle, PolicyDecisionPoint,
};
use shared_platform_contracts_kernel::pdp::{AuthorizationRequest, PolicyVersion};
use shared_ulid_id_kernel::IdGenerator;

#[derive(Debug)]
pub enum EngineLoadError {
    Store(BundleStoreError),
    Admission(PdpError),
    ContentIdentity {
        declared: PolicyVersion,
        computed: PolicyVersion,
    },
    ReloadUnavailable,
}

pub struct PolicyEngine {
    pdp: CedarPdp,
    reload: Mutex<()>,
}

impl PolicyEngine {
    pub fn load(
        store: &dyn PolicyBundleStore,
        id_gen: Arc<dyn IdGenerator>,
        cache_capacity: usize,
    ) -> Result<Self, EngineLoadError> {
        let bundle = store.load().map_err(EngineLoadError::Store)?;
        verify_content_identity(&bundle)?;
        let pdp =
            CedarPdp::load(&bundle, id_gen, cache_capacity).map_err(EngineLoadError::Admission)?;
        Ok(Self {
            pdp,
            reload: Mutex::new(()),
        })
    }

    /// Serializes store-read and swap as one reload operation without blocking
    /// serving reads while loading and compiling. A failed reload leaves the
    /// previous bundle serving.
    ///
    /// # Errors
    /// Preserves store, content-identity and admission refusals; refuses a
    /// poisoned reload coordinator.
    pub fn reload(&self, store: &dyn PolicyBundleStore) -> Result<PolicyVersion, EngineLoadError> {
        let _reload = self
            .reload
            .lock()
            .map_err(|_| EngineLoadError::ReloadUnavailable)?;
        let bundle = store.load().map_err(EngineLoadError::Store)?;
        verify_content_identity(&bundle)?;
        self.pdp
            .swap_bundle(&bundle)
            .map_err(EngineLoadError::Admission)?;
        Ok(bundle.version)
    }

    /// The caller supplies its admitted graph-to-Cedar candidate mapping; this
    /// does not infer it.
    ///
    /// # Errors
    /// Returns complete typed graph/identity/PDP refusals, never a substituted deny.
    pub fn decide<S: RebacTupleStore>(
        &self,
        inputs: &DecisionInputs<'_, S>,
        request: &AuthorizationRequest,
        principal_attributes: BTreeMap<String, serde_json::Value>,
        context_entities: Vec<EntityRecord>,
    ) -> Result<JoinedDecision, DecisionError> {
        policy_decision_domain::decide(
            &self.pdp,
            inputs,
            request,
            principal_attributes,
            context_entities,
        )
    }
}

fn verify_content_identity(bundle: &PolicyBundle) -> Result<(), EngineLoadError> {
    let computed = PolicySource::from(bundle)
        .content_version()
        .map_err(|error| {
            EngineLoadError::Admission(PdpError::BundleRejected {
                detail: format!("content identity could not be computed: {error:?}"),
            })
        })?;
    if computed != bundle.version {
        return Err(EngineLoadError::ContentIdentity {
            declared: bundle.version.clone(),
            computed,
        });
    }
    Ok(())
}

impl PolicyDecisionPoint for PolicyEngine {
    fn authorize(
        &self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<PdpOutcome, PdpError> {
        self.pdp.authorize(request, entities)
    }

    fn loaded_policy_version(&self) -> PolicyVersion {
        self.pdp.loaded_policy_version()
    }
}
