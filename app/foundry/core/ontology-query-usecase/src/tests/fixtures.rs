//! Shared fixtures for the execution-usecase tests: the graph, the
//! registry every link upsert consults, and the canonical input.

pub(super) use crate::*;
pub(super) use data_ontology_kernel::ObjectGraph;
pub(super) use foundry_ontology_query_domain::{
    EdgeConsent, KnowledgeGraphQueryEngine, KnowledgeGraphQueryError, KnowledgeGraphQueryRequest,
    TraversalDirection,
};

pub(super) use data_boundary_kernel::{DataClass, PrivacyDataClass};
pub(super) use data_ontology_kernel::{
    EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, LinkCardinality,
    LinkTypeDefinition, LinkTypeId, ObjectEntity, ObjectProperty, OntologyEngine, PropertyTier,
};

pub(super) fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    for ety in ["ety_account", "ety_contact"] {
        engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    "ten_alpha",
                    EntityTypeId::new(ety).unwrap(),
                    "Fixture",
                    vec![
                        EntityTypePropertyDefinition::new(
                            "name",
                            PropertyTier::Scalar,
                            PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap(),
                            true,
                        )
                        .unwrap(),
                    ],
                    1,
                )
                .unwrap(),
            )
            .unwrap();
    }
    engine
        .register_link_type(
            LinkTypeDefinition::new(
                "ten_alpha",
                LinkTypeId::new("lty_owns").unwrap(),
                EntityTypeId::new("ety_account").unwrap(),
                EntityTypeId::new("ety_contact").unwrap(),
                LinkCardinality::ManyToMany,
                false,
            )
            .unwrap(),
        )
        .unwrap();
    engine
}
pub(super) use foundry_projection_draft::{
    AppliedEntry, EntryOutcome, KeyDesignations, MemoryProjectionStore, ProjectedLink,
    ProjectedObject, ProjectionStore,
};

pub(super) fn property(name: &str) -> ObjectProperty {
    ObjectProperty::new(
        name.to_string(),
        "value".to_string(),
        PropertyTier::Scalar,
        PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap(),
    )
}

/// The same two-object, one-edge graph the in-memory fixture built —
/// now as the durable projection the usecase actually reads.
pub(super) fn store() -> MemoryProjectionStore {
    let objects = [("ent_root", "ety_account"), ("ent_child", "ety_contact")]
        .into_iter()
        .map(|(entity_id, entity_type)| ProjectedObject {
            entity: ObjectEntity::new(
                "ten_alpha".to_string(),
                entity_id.to_string(),
                entity_type.to_string(),
                vec![property("name")],
            )
            .unwrap(),
            schema_revision: 1,
            last_ordinal: 1,
            last_actor: "prn_projector".to_string(),
        })
        .collect();
    let links = vec![ProjectedLink {
        link_type: "lty_owns".to_string(),
        from_object_ref: "ent_root".to_string(),
        to_object_ref: "ent_child".to_string(),
        observed_at_epoch_ms: 10_000,
    }];
    let mut store = MemoryProjectionStore::default();
    store
        .apply(
            AppliedEntry {
                tenant_id: "ten_alpha".to_string(),
                ordinal: 1,
                outcome: EntryOutcome::Applied { objects, links },
            },
            &KeyDesignations::default(),
        )
        .unwrap();
    store
}

pub(super) fn input(idempotency_key: &str) -> OntologyQueryExecutionInput {
    OntologyQueryExecutionInput {
        idempotency_key: idempotency_key.to_string(),
        principal_id: "usr_alice".to_string(),
        query_surface: "ontology.query.graph_slice".to_string(),
        request_evidence_ref: "evidence://query/request/kgq_alpha".to_string(),
        trace_context_ref: "trace://kgq_alpha".to_string(),
        request: KnowledgeGraphQueryRequest::new(
            "ten_alpha",
            "kgq_alpha",
            "ent_root",
            vec!["lty_owns"],
            1,
            0,
            12,
            EdgeConsent::granted(vec!["lty_owns"]),
            TraversalDirection::Outbound,
        )
        .unwrap(),
        policy_decision: OntologyQueryPolicyDecision {
            decision_id: "dec_allow_query".to_string(),
            tenant_id: "ten_alpha".to_string(),
            principal_id: "usr_alice".to_string(),
            allowed_query_surfaces: vec!["ontology.query.graph_slice".to_string()],
            max_depth_ceiling: 4,
            evidence_ref: "evidence://policy/dec_allow_query".to_string(),
        },
    }
}
