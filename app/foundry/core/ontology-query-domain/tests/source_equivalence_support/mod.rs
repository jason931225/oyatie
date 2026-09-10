//! Fixtures for the source-equivalence proof: one graph, loaded
//! into the in-memory index and into the durable store.

use data_boundary_kernel::{DataClass, PrivacyDataClass};
use data_ontology_kernel::{
    EntityTypeDefinition, EntityTypeId, EntityTypePropertyDefinition, LinkCardinality,
    LinkTypeDefinition, LinkTypeId, ObjectEntity, ObjectGraph, ObjectProperty, OntologyEngine,
    PropertyTier,
};
use foundry_ontology_query_domain::{
    EdgeConsent, KnowledgeGraphLinkInstance, KnowledgeGraphQueryEngine, KnowledgeGraphQueryRequest,
    TraversalDirection, query_graph_slice_from_store,
};
use foundry_projection_draft::{
    AppliedEntry, EntryOutcome, KeyDesignations, MemoryProjectionStore, ProjectedLink,
    ProjectedObject, ProjectionStore,
};

pub(crate) struct EdgeFixture {
    pub(crate) from: &'static str,
    pub(crate) to: &'static str,
    pub(crate) edge_type: &'static str,
    pub(crate) observed_at_seconds: u64,
}

const fn edge(
    from: &'static str,
    to: &'static str,
    edge_type: &'static str,
    observed_at_seconds: u64,
) -> EdgeFixture {
    EdgeFixture {
        from,
        to,
        edge_type,
        observed_at_seconds,
    }
}

pub(crate) const EDGES: &[EdgeFixture] = &[
    edge("ent_root", "ent_b", "lty_partner", 10),
    edge("ent_b", "ent_c", "lty_partner", 10),
    edge("ent_root", "ent_d", "lty_member", 10),
    // Deliberately stale: the freshness floor must drop it in BOTH.
    edge("ent_root", "ent_e", "lty_partner", 1),
];

pub(crate) const NODES: &[(&str, &str)] = &[
    ("ent_root", "ety_account"),
    ("ent_b", "ety_contact"),
    ("ent_c", "ety_contact"),
    ("ent_d", "ety_contact"),
    ("ent_e", "ety_contact"),
];

pub(crate) fn internal() -> PrivacyDataClass {
    PrivacyDataClass::try_from(DataClass::InternalOnly).unwrap()
}

pub(crate) fn property() -> ObjectProperty {
    ObjectProperty::new(
        "name".to_string(),
        "value".to_string(),
        PropertyTier::Scalar,
        internal(),
    )
}

pub(crate) fn registry() -> OntologyEngine {
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
                            internal(),
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
    for lty in ["lty_partner", "lty_member"] {
        engine
            .register_link_type(
                LinkTypeDefinition::new(
                    "ten_alpha",
                    LinkTypeId::new(lty).unwrap(),
                    EntityTypeId::new("ety_account").unwrap(),
                    EntityTypeId::new("ety_contact").unwrap(),
                    LinkCardinality::ManyToMany,
                    false,
                )
                .unwrap(),
            )
            .unwrap();
    }
    engine
}

pub(crate) fn in_memory() -> (ObjectGraph, KnowledgeGraphQueryEngine) {
    let mut graph = ObjectGraph::default();
    for (id, ety) in NODES {
        graph
            .upsert_entity(
                ObjectEntity::new(
                    "ten_alpha".to_string(),
                    (*id).to_string(),
                    (*ety).to_string(),
                    vec![property()],
                )
                .unwrap(),
            )
            .unwrap();
    }
    let registry = registry();
    let mut engine = KnowledgeGraphQueryEngine::default();
    for fixture in EDGES {
        engine
            .upsert_link(
                &registry,
                &graph,
                KnowledgeGraphLinkInstance::new(
                    "ten_alpha",
                    fixture.from,
                    fixture.to,
                    fixture.edge_type,
                    fixture.observed_at_seconds,
                )
                .unwrap(),
            )
            .unwrap();
    }
    (graph, engine)
}

/// The SAME graph as the durable projection holds it.
pub(crate) fn in_store() -> MemoryProjectionStore {
    let mut store = MemoryProjectionStore::default();
    let objects: Vec<ProjectedObject> = NODES
        .iter()
        .map(|(id, ety)| ProjectedObject {
            entity: ObjectEntity::new(
                "ten_alpha".to_string(),
                (*id).to_string(),
                (*ety).to_string(),
                vec![property()],
            )
            .unwrap(),
            schema_revision: 1,
            last_ordinal: 1,
            last_actor: "prn_projector".to_string(),
        })
        .collect();
    let links: Vec<ProjectedLink> = EDGES
        .iter()
        .map(|fixture| ProjectedLink {
            link_type: fixture.edge_type.to_string(),
            from_object_ref: fixture.from.to_string(),
            to_object_ref: fixture.to.to_string(),
            observed_at_epoch_ms: fixture.observed_at_seconds * 1_000,
        })
        .collect();
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

pub(crate) fn request(
    edge_types: Vec<&str>,
    depth: u32,
    floor: u64,
    consent: EdgeConsent,
    direction: TraversalDirection,
    root: &str,
) -> KnowledgeGraphQueryRequest {
    KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_equivalence",
        root,
        edge_types,
        depth,
        floor,
        12,
        consent,
        direction,
    )
    .unwrap()
}

pub(crate) fn shapes() -> Vec<(&'static str, KnowledgeGraphQueryRequest)> {
    vec![
        (
            "unfiltered outbound",
            request(
                vec![],
                3,
                0,
                EdgeConsent::Unrestricted,
                TraversalDirection::Outbound,
                "ent_root",
            ),
        ),
        (
            "freshness floor drops the stale edge",
            request(
                vec![],
                3,
                5,
                EdgeConsent::Unrestricted,
                TraversalDirection::Outbound,
                "ent_root",
            ),
        ),
        (
            "edge-type filter",
            request(
                vec!["lty_partner"],
                3,
                0,
                EdgeConsent::Unrestricted,
                TraversalDirection::Outbound,
                "ent_root",
            ),
        ),
        (
            "consent grants one edge type",
            request(
                vec![],
                3,
                0,
                EdgeConsent::granted(vec!["lty_partner"]),
                TraversalDirection::Outbound,
                "ent_root",
            ),
        ),
        (
            "consent granting nothing traverses nothing",
            request(
                vec![],
                3,
                0,
                EdgeConsent::Granted(Vec::new()),
                TraversalDirection::Outbound,
                "ent_root",
            ),
        ),
        (
            "inbound direction",
            request(
                vec![],
                3,
                0,
                EdgeConsent::Unrestricted,
                TraversalDirection::Inbound,
                "ent_c",
            ),
        ),
        (
            "both directions",
            request(
                vec![],
                3,
                0,
                EdgeConsent::Unrestricted,
                TraversalDirection::Both,
                "ent_b",
            ),
        ),
        (
            "depth ceiling of one",
            request(
                vec![],
                1,
                0,
                EdgeConsent::Unrestricted,
                TraversalDirection::Outbound,
                "ent_root",
            ),
        ),
    ]
}
