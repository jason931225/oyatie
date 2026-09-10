use std::collections::BTreeMap;
use std::sync::RwLockReadGuard;

mod diagnostics;

use cedar_policy::{Context, Decision as CedarDecision, Entities, PolicySet, Request};
use shared_pdp_kernel::{
    CachedDecision, DecisionAuditRecord, DecisionCacheKey, EntitySlice, PdpError, PdpOutcome,
    PolicyDecisionPoint, request_fingerprint,
};
use shared_platform_contracts_kernel::pdp::{
    AuthorizationRequest, AuthorizationResponse, Decision, Obligation, PolicyVersion,
};

use super::entity::{cedar_entity, entity_uid, restricted_expression};
use super::{CedarPdp, LoadedBundle};
use diagnostics::qualification_diagnostic_result;

/// Annotation key whose value names the obligation a permit carries.
/// PEPs MUST enforce obligations or fail closed (locked PDP contract).
const OBLIGATION_ANNOTATION: &str = "obligation";

#[derive(Clone, Copy)]
enum DiagnosticBoundary {
    Serving,
    Qualification,
}

impl CedarPdp {
    fn evaluate(
        &self,
        state: &LoadedBundle,
        policy_set: &PolicySet,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
        diagnostic_boundary: DiagnosticBoundary,
    ) -> Result<CachedDecision, PdpError> {
        let action =
            state
                .action_map
                .get(&request.action)
                .ok_or_else(|| PdpError::UnknownAction {
                    action: request.action.clone(),
                })?;
        let mut context_pairs = Vec::new();
        for (key, value) in &request.context {
            context_pairs.push((
                key.clone(),
                restricted_expression(&format!("context {key}"), value)?,
            ));
        }
        let context = Context::from_pairs(context_pairs).map_err(|e| PdpError::Evaluation {
            detail: format!("context rejected: {e}"),
        })?;
        let cedar_request = Request::new(
            entity_uid(&request.principal)?,
            action.clone(),
            entity_uid(&request.resource)?,
            context,
            Some(&state.schema),
        )
        .map_err(|e| PdpError::Evaluation {
            detail: format!("request rejected by schema: {e}"),
        })?;
        let mut cedar_entities = Vec::new();
        for record in &entities.entities {
            cedar_entities.push(cedar_entity(record)?);
        }
        let cedar_entities =
            Entities::from_entities(cedar_entities, Some(&state.schema)).map_err(|e| {
                PdpError::Evaluation {
                    detail: format!("entity slice rejected by schema: {e}"),
                }
            })?;
        let response = self
            .authorizer
            .is_authorized(&cedar_request, policy_set, &cedar_entities);
        if matches!(diagnostic_boundary, DiagnosticBoundary::Qualification) {
            qualification_diagnostic_result(response.diagnostics().errors())?;
        }
        let decision = match response.decision() {
            CedarDecision::Allow => Decision::Allow,
            CedarDecision::Deny => Decision::Deny,
        };
        let mut determining_policy_ids: Vec<String> = response
            .diagnostics()
            .reason()
            .map(ToString::to_string)
            .collect();
        determining_policy_ids.sort();
        let mut obligations = Vec::new();
        if decision.is_allow() {
            for policy_id in response.diagnostics().reason() {
                let annotation = policy_set
                    .policy(policy_id)
                    .and_then(|p| p.annotation(OBLIGATION_ANNOTATION));
                if let Some(obligation_id) = annotation {
                    obligations.push(Obligation {
                        obligation_id: obligation_id.to_owned(),
                        parameters: BTreeMap::new(),
                    });
                }
            }
            obligations.sort_by(|a, b| a.obligation_id.cmp(&b.obligation_id));
        }
        Ok(CachedDecision {
            decision,
            determining_policy_ids,
            obligations,
        })
    }

    fn preflight<'a>(
        &'a self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<RwLockReadGuard<'a, LoadedBundle>, PdpError> {
        request.validate().map_err(PdpError::InvalidRequest)?;
        entities.validate().map_err(PdpError::InvalidRequest)?;
        let state = self.state.read().map_err(|_| PdpError::Evaluation {
            detail: "policy state lock poisoned".to_owned(),
        })?;
        if let Some(required) = &request.min_policy_version {
            // Zookie semantics: equality is the only comparison consumers
            // may rely on (the contract makes ordering store-owned).
            if required != &state.version {
                return Err(PdpError::StalePolicyVersion {
                    required: required.clone(),
                    loaded: state.version.clone(),
                });
            }
        }
        Ok(state)
    }

    /// Unlike ordinary serving, this refuses any per-policy Cedar evaluation
    /// diagnostic, and it bypasses the serving cache in BOTH directions so a
    /// permissive aggregate decision cannot mask a diagnostic and qualification
    /// cannot perturb serving state.
    ///
    /// # Errors
    /// [`PdpError::Evaluation`] when Cedar reports any per-policy evaluation
    /// diagnostic, on top of ordinary authorization's refusals.
    pub fn authorize_for_qualification(
        &self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<PdpOutcome, PdpError> {
        let state = self.preflight(request, entities)?;
        let policy_set = state.policy_set_for(&request.tenant_id);
        let content = self.evaluate(
            &state,
            policy_set,
            request,
            entities,
            DiagnosticBoundary::Qualification,
        )?;
        self.outcome(request, &state.version, &content, false)
    }

    fn outcome(
        &self,
        request: &AuthorizationRequest,
        version: &PolicyVersion,
        content: &CachedDecision,
        cache_hit: bool,
    ) -> Result<PdpOutcome, PdpError> {
        let decision_id = self
            .id_gen
            .new_ulid()
            .map_err(|e| PdpError::DecisionIdUnavailable {
                detail: e.to_string(),
            })?
            .as_str()
            .to_lowercase();
        let response = AuthorizationResponse {
            decision_id: decision_id.clone(),
            request_id: request.request_id.clone(),
            decision: content.decision,
            policy_version: version.clone(),
            determining_policy_ids: content.determining_policy_ids.clone(),
            obligations: content.obligations.clone(),
        };
        response
            .validate()
            .map_err(|violations| PdpError::Evaluation {
                detail: format!("decision violates the PDP contract: {violations:?}"),
            })?;
        let audit = DecisionAuditRecord {
            decision_id,
            request_id: request.request_id.clone(),
            tenant_id: request.tenant_id.clone(),
            principal: request.principal.clone(),
            action: request.action.clone(),
            resource: request.resource.clone(),
            decision: content.decision,
            policy_version: version.clone(),
            determining_policy_ids: content.determining_policy_ids.clone(),
            cache_hit,
        };
        Ok(PdpOutcome {
            response,
            audit,
            cache_hit,
        })
    }
}

impl PolicyDecisionPoint for CedarPdp {
    fn authorize(
        &self,
        request: &AuthorizationRequest,
        entities: &EntitySlice,
    ) -> Result<PdpOutcome, PdpError> {
        let state = self.preflight(request, entities)?;
        let key = DecisionCacheKey {
            request_fingerprint: request_fingerprint(request, entities),
            policy_version: state.version.as_str().to_owned(),
        };
        let cached = {
            let cache = self.cache.lock().map_err(|_| PdpError::Evaluation {
                detail: "decision cache lock poisoned".to_owned(),
            })?;
            cache.get(&key).cloned()
        };
        if let Some(content) = cached {
            return self.outcome(request, &state.version, &content, true);
        }
        let policy_set = state.policy_set_for(&request.tenant_id);
        let content = self.evaluate(
            &state,
            policy_set,
            request,
            entities,
            DiagnosticBoundary::Serving,
        )?;
        {
            let mut cache = self.cache.lock().map_err(|_| PdpError::Evaluation {
                detail: "decision cache lock poisoned".to_owned(),
            })?;
            cache.insert(key, content.clone());
        }
        self.outcome(request, &state.version, &content, false)
    }

    fn loaded_policy_version(&self) -> PolicyVersion {
        match self.state.read() {
            Ok(state) => state.version.clone(),
            // A poisoned lock still names the version it held; PolicyVersion
            // is immutable after load so the clone below cannot observe a
            // torn write.
            Err(poisoned) => poisoned.into_inner().version.clone(),
        }
    }
}
