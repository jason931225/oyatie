//! Shared fixtures for the query-engine test corpus.

pub(super) use std::collections::BTreeSet;

pub(super) use data_ontology_kernel::ObjectGraph;

pub(super) use crate::*;
pub(super) use data_boundary_kernel::{DataClass, PrivacyDataClass};
pub(super) use data_ontology_kernel::{
    EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, LinkCardinality,
    LinkTypeDefinition, LinkTypeId, ObjectEntity, ObjectProperty, OntologyEngine, PropertyTier,
};

/// The registry every link upsert consults: entity types per tenant,
/// then every lty_ id the corpus uses. Endpoint pairs are DECLARED
/// loosely for the heterogeneous fixtures (lty_partner spans
/// account->contact and contact->contact) — endpoint-type law is not
/// enforced in this store and arrives with the re-root, so the pairs
/// here are registration vocabulary, not law.
pub(super) fn registry() -> OntologyEngine {
    let mut engine = OntologyEngine::default();
    for (tenant, ety) in [
        ("ten_alpha", "ety_account"),
        ("ten_alpha", "ety_contact"),
        ("ten_alpha", "ety_case"),
        ("ten_beta", "ety_account"),
    ] {
        engine
            .register_entity_type(
                EntityTypeDefinition::new(
                    tenant,
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
    for (tenant, lty, from, to) in [
        ("ten_alpha", "lty_owns", "ety_account", "ety_contact"),
        ("ten_alpha", "lty_partner", "ety_account", "ety_contact"),
        ("ten_alpha", "lty_member", "ety_account", "ety_contact"),
        ("ten_alpha", "lty_related", "ety_account", "ety_contact"),
        ("ten_alpha", "lty_knows", "ety_account", "ety_contact"),
        ("ten_beta", "lty_owns", "ety_account", "ety_account"),
    ] {
        engine
            .register_link_type(
                LinkTypeDefinition::new(
                    tenant,
                    LinkTypeId::new(lty).unwrap(),
                    EntityTypeId::new(from).unwrap(),
                    EntityTypeId::new(to).unwrap(),
                    LinkCardinality::ManyToMany,
                    false,
                )
                .unwrap(),
            )
            .unwrap();
    }
    engine
}

pub(super) fn property(name: &str) -> ObjectProperty {
    ObjectProperty::new(
        name.to_string(),
        "value".to_string(),
        PropertyTier::Scalar,
        PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap(),
    )
}

pub(super) fn graph() -> ObjectGraph {
    let mut graph = ObjectGraph::default();
    for (tenant_id, entity_id, entity_type) in [
        ("ten_alpha", "ent_root", "ety_account"),
        ("ten_alpha", "ent_contact", "ety_contact"),
        ("ten_alpha", "ent_case", "ety_case"),
        ("ten_alpha", "ent_cycle", "ety_case"),
        ("ten_beta", "ent_beta_root", "ety_account"),
    ] {
        graph
            .upsert_entity(
                ObjectEntity::new(
                    tenant_id.to_string(),
                    entity_id.to_string(),
                    entity_type.to_string(),
                    vec![property("name")],
                )
                .unwrap(),
            )
            .unwrap();
    }
    graph
}

pub(super) fn request(
    root_entity_id: &str,

    edge_type_ids: Vec<&str>,

    max_depth: u32,

    freshness_floor_epoch_seconds: u64,
) -> KnowledgeGraphQueryRequest {
    KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_two_hop",
        root_entity_id,
        edge_type_ids,
        max_depth,
        freshness_floor_epoch_seconds,
        12,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap()
}

pub(super) fn assert_every_edge_endpoint_is_returned(response: &KnowledgeGraphQueryResponse) {
    let node_ids: BTreeSet<&str> = response
        .nodes
        .iter()
        .map(|node| node.entity_id.as_str())
        .collect();
    for edge in &response.edges {
        assert!(
            node_ids.contains(edge.from_entity_id.as_str()),
            "edge source {} must be present in response nodes",
            edge.from_entity_id
        );
        assert!(
            node_ids.contains(edge.to_entity_id.as_str()),
            "edge target {} must be present in response nodes",
            edge.to_entity_id
        );
    }
}

/// Builds a consent-scoped graph used by the consent gate tests:
///   ent_root --lty_partner--> ent_b --lty_partner--> ent_c
///   ent_root --lty_member-->  ent_d
pub(super) fn consent_graph() -> ObjectGraph {
    let mut g = ObjectGraph::default();
    for (entity_id, entity_type) in [
        ("ent_root", "ety_account"),
        ("ent_b", "ety_contact"),
        ("ent_c", "ety_contact"),
        ("ent_d", "ety_contact"),
    ] {
        g.upsert_entity(
            ObjectEntity::new(
                "ten_alpha".to_string(),
                entity_id.to_string(),
                entity_type.to_string(),
                vec![property("name")],
            )
            .unwrap(),
        )
        .unwrap();
    }
    g
}

/// Builds the consent-scoped engine for the consent gate tests.
pub(super) fn consent_engine(g: &ObjectGraph) -> KnowledgeGraphQueryEngine {
    let mut engine = KnowledgeGraphQueryEngine::default();
    for (from, to, edge_type) in [
        ("ent_root", "ent_b", "lty_partner"),
        ("ent_b", "ent_c", "lty_partner"),
        ("ent_root", "ent_d", "lty_member"),
    ] {
        engine
            .upsert_link(
                &registry(),
                g,
                KnowledgeGraphLinkInstance::new("ten_alpha", from, to, edge_type, 1).unwrap(),
            )
            .unwrap();
    }
    engine
}

/// Builds a directed chain graph for direction traversal tests:
///   ent_pred --lty_owns--> ent_root --lty_owns--> ent_succ
///
/// Outbound from ent_root reaches ent_succ only.
/// Inbound from ent_root reaches ent_pred only.
/// Both from ent_root reaches ent_pred and ent_succ.
pub(super) fn dir_graph() -> ObjectGraph {
    let mut g = ObjectGraph::default();
    for (entity_id, entity_type) in [
        ("ent_pred", "ety_account"),
        ("ent_root", "ety_account"),
        ("ent_succ", "ety_account"),
    ] {
        g.upsert_entity(
            ObjectEntity::new(
                "ten_alpha".to_string(),
                entity_id.to_string(),
                entity_type.to_string(),
                vec![property("name")],
            )
            .unwrap(),
        )
        .unwrap();
    }
    g
}

pub(super) fn dir_engine(g: &ObjectGraph) -> KnowledgeGraphQueryEngine {
    let mut engine = KnowledgeGraphQueryEngine::default();
    for (from, to) in [("ent_pred", "ent_root"), ("ent_root", "ent_succ")] {
        engine
            .upsert_link(
                &registry(),
                g,
                KnowledgeGraphLinkInstance::new("ten_alpha", from, to, "lty_owns", 1).unwrap(),
            )
            .unwrap();
    }
    engine
}
