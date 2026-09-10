//! Idempotent, policy-checked execution over the durable projection.

use std::collections::BTreeMap;

use foundry_ontology_query_domain::{
    KnowledgeGraphQueryError, KnowledgeGraphQueryRequest, query_graph_slice_from_store,
};
use foundry_projection_draft::ProjectionStore;

use crate::types::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OntologyQueryExecutionUsecase {
    receipts_by_idempotency_key: BTreeMap<String, OntologyQueryExecutionReceipt>,
    fingerprints_by_idempotency_key: BTreeMap<String, OntologyQueryIntentFingerprint>,
    audit_events: Vec<OntologyQueryAuditEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OntologyQueryIntentFingerprint {
    tenant_id: String,
    principal_id: String,
    query_surface: String,
    root_entity_id: String,
    edge_type_ids: Vec<String>,
    max_depth: u32,
    freshness_floor_epoch_seconds: u64,
    policy_decision_id: String,
}

impl OntologyQueryExecutionUsecase {
    /// Execute one policy-checked query against the DURABLE projection.
    pub fn execute(
        &mut self,
        store: &dyn ProjectionStore,
        input: OntologyQueryExecutionInput,
    ) -> OntologyQueryExecutionReceipt {
        if input.idempotency_key.trim().is_empty() {
            return invalid_input_receipt(input, Vec::new());
        }

        let fingerprint = OntologyQueryIntentFingerprint::from(&input);
        if let Some(existing) = self.receipts_by_idempotency_key.get(&input.idempotency_key) {
            if self
                .fingerprints_by_idempotency_key
                .get(&input.idempotency_key)
                == Some(&fingerprint)
            {
                return existing.clone();
            }
            return self.conflict_receipt(input);
        }

        let evidence_refs = canonical_evidence_refs(&input);
        if input.principal_id.trim().is_empty()
            || input.query_surface.trim().is_empty()
            || input.request_evidence_ref.trim().is_empty()
            || input.trace_context_ref.trim().is_empty()
            || input.policy_decision.decision_id.trim().is_empty()
            || input.policy_decision.evidence_ref.trim().is_empty()
        {
            return invalid_input_receipt(input, evidence_refs);
        }

        self.record_event(
            OntologyQueryAuditEventKind::QueryRequestReceived,
            &input,
            evidence_refs.clone(),
        );

        if let Some(receipt) = self.policy_denial(&input, &evidence_refs) {
            self.cache_receipt(&input.idempotency_key, fingerprint, &receipt);
            return receipt;
        }

        match query_graph_slice_from_store(store, input.request.clone()) {
            Ok(response) => {
                let receipt = OntologyQueryExecutionReceipt {
                    idempotency_key: input.idempotency_key.clone(),
                    tenant_id: input.request.tenant_id.clone(),
                    principal_id: input.principal_id.clone(),
                    status: OntologyQueryExecutionStatus::Completed,
                    denial_kind: None,
                    failure: None,
                    response: Some(response),
                    evidence_refs: evidence_refs.clone(),
                };
                self.record_event(
                    OntologyQueryAuditEventKind::QueryCompleted,
                    &input,
                    evidence_refs,
                );
                self.cache_receipt(&input.idempotency_key, fingerprint, &receipt);
                receipt
            }
            Err(error) => {
                let receipt = OntologyQueryExecutionReceipt {
                    idempotency_key: input.idempotency_key.clone(),
                    tenant_id: input.request.tenant_id.clone(),
                    principal_id: input.principal_id.clone(),
                    status: OntologyQueryExecutionStatus::Failed,
                    denial_kind: None,
                    failure: Some(error),
                    response: None,
                    evidence_refs: evidence_refs.clone(),
                };
                self.record_event(
                    OntologyQueryAuditEventKind::QueryFailed,
                    &input,
                    evidence_refs,
                );
                self.cache_receipt(&input.idempotency_key, fingerprint, &receipt);
                receipt
            }
        }
    }

    pub fn audit_events(&self) -> &[OntologyQueryAuditEvent] {
        &self.audit_events
    }

    pub fn receipt_count(&self) -> usize {
        self.receipts_by_idempotency_key.len()
    }

    fn policy_denial(
        &mut self,
        input: &OntologyQueryExecutionInput,
        evidence_refs: &[String],
    ) -> Option<OntologyQueryExecutionReceipt> {
        let denial_kind = if input.policy_decision.tenant_id != input.request.tenant_id {
            Some(OntologyQueryDenialKind::TenantMismatch)
        } else if input.policy_decision.principal_id != input.principal_id {
            Some(OntologyQueryDenialKind::PrincipalMismatch)
        } else if !input
            .policy_decision
            .allowed_query_surfaces
            .iter()
            .any(|surface| surface == &input.query_surface)
        {
            Some(OntologyQueryDenialKind::SurfaceDenied)
        } else if input.request.max_depth > input.policy_decision.max_depth_ceiling {
            Some(OntologyQueryDenialKind::DepthCeilingExceeded)
        } else {
            None
        }?;

        let receipt = OntologyQueryExecutionReceipt {
            idempotency_key: input.idempotency_key.clone(),
            tenant_id: input.request.tenant_id.clone(),
            principal_id: input.principal_id.clone(),
            status: OntologyQueryExecutionStatus::Denied,
            denial_kind: Some(denial_kind),
            failure: None,
            response: None,
            evidence_refs: evidence_refs.to_vec(),
        };
        self.record_event(
            OntologyQueryAuditEventKind::QueryDenied,
            input,
            evidence_refs.to_vec(),
        );
        Some(receipt)
    }

    fn conflict_receipt(
        &mut self,
        input: OntologyQueryExecutionInput,
    ) -> OntologyQueryExecutionReceipt {
        let evidence_refs = canonical_evidence_refs(&input);
        let receipt = OntologyQueryExecutionReceipt {
            idempotency_key: input.idempotency_key.clone(),
            tenant_id: input.request.tenant_id.clone(),
            principal_id: input.principal_id.clone(),
            status: OntologyQueryExecutionStatus::Denied,
            denial_kind: Some(OntologyQueryDenialKind::IdempotencyConflict),
            failure: None,
            response: None,
            evidence_refs: evidence_refs.clone(),
        };
        self.record_event(
            OntologyQueryAuditEventKind::IdempotencyConflict,
            &input,
            evidence_refs,
        );
        receipt
    }

    fn record_event(
        &mut self,
        kind: OntologyQueryAuditEventKind,
        input: &OntologyQueryExecutionInput,
        evidence_refs: Vec<String>,
    ) {
        self.audit_events.push(OntologyQueryAuditEvent {
            kind,
            tenant_id: input.request.tenant_id.clone(),
            principal_id: input.principal_id.clone(),
            idempotency_key: input.idempotency_key.clone(),
            query_id: input.request.query_id.clone(),
            evidence_refs,
        });
    }

    fn cache_receipt(
        &mut self,
        idempotency_key: &str,
        fingerprint: OntologyQueryIntentFingerprint,
        receipt: &OntologyQueryExecutionReceipt,
    ) {
        self.fingerprints_by_idempotency_key
            .insert(idempotency_key.to_string(), fingerprint);
        self.receipts_by_idempotency_key
            .insert(idempotency_key.to_string(), receipt.clone());
    }
}

impl OntologyQueryIntentFingerprint {
    fn from(input: &OntologyQueryExecutionInput) -> Self {
        let mut edge_type_ids = input.request.edge_type_ids.clone();
        edge_type_ids.sort();
        edge_type_ids.dedup();
        Self {
            tenant_id: input.request.tenant_id.clone(),
            principal_id: input.principal_id.clone(),
            query_surface: input.query_surface.clone(),
            root_entity_id: input.request.root_entity_id.clone(),
            edge_type_ids,
            max_depth: input.request.max_depth,
            freshness_floor_epoch_seconds: input.request.freshness_floor_epoch_seconds,
            policy_decision_id: input.policy_decision.decision_id.clone(),
        }
    }
}

fn invalid_input_receipt(
    input: OntologyQueryExecutionInput,
    evidence_refs: Vec<String>,
) -> OntologyQueryExecutionReceipt {
    OntologyQueryExecutionReceipt {
        idempotency_key: input.idempotency_key,
        tenant_id: input.request.tenant_id,
        principal_id: input.principal_id,
        status: OntologyQueryExecutionStatus::Denied,
        denial_kind: Some(OntologyQueryDenialKind::InvalidInput),
        failure: None,
        response: None,
        evidence_refs,
    }
}

fn canonical_evidence_refs(input: &OntologyQueryExecutionInput) -> Vec<String> {
    let mut refs = vec![
        input.policy_decision.evidence_ref.clone(),
        input.request_evidence_ref.clone(),
        input.trace_context_ref.clone(),
    ];
    refs.retain(|value| !value.trim().is_empty());
    refs.sort();
    refs.dedup();
    refs
}
