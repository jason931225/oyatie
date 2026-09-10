//! Multi-root search-around: the walk seeds from an object set — the
//! primary root plus additional roots at depth zero — with the same
//! fail-closed existence law as the primary root.

use super::support::*;

#[test]
fn additional_roots_expand_disconnected_components() {
    let mut graph = ObjectGraph::default();
    let mut engine = KnowledgeGraphQueryEngine::default();
    let mut add = |entity_id: &str| {
        graph
            .upsert_entity(
                ObjectEntity::new(
                    "ten_alpha".to_string(),
                    entity_id.to_string(),
                    "ety_account".to_string(),
                    vec![property("name")],
                )
                .unwrap(),
            )
            .unwrap();
    };
    for id in ["ent_a", "ent_a1", "ent_b", "ent_b1"] {
        add(id);
    }
    for (from, to) in [("ent_a", "ent_a1"), ("ent_b", "ent_b1")] {
        engine
            .upsert_link(
                &registry(),
                &graph,
                KnowledgeGraphLinkInstance::new("ten_alpha", from, to, "lty_knows", 100).unwrap(),
            )
            .unwrap();
    }

    let single = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_multi",
        "ent_a",
        Vec::<String>::new(),
        2,
        0,
        100,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap();
    let response = engine.query_graph_slice(&graph, single.clone()).unwrap();
    assert_eq!(response.nodes.len(), 2, "single root sees one component");

    let multi = single.with_additional_roots(vec!["ent_b"]).unwrap();
    let response = engine.query_graph_slice(&graph, multi).unwrap();
    assert_eq!(response.nodes.len(), 4, "both components reached");
    assert_eq!(response.edges.len(), 2);
}

#[test]
fn additional_roots_fail_closed() {
    let mut graph = ObjectGraph::default();
    graph
        .upsert_entity(
            ObjectEntity::new(
                "ten_alpha".to_string(),
                "ent_a".to_string(),
                "ety_account".to_string(),
                vec![property("name")],
            )
            .unwrap(),
        )
        .unwrap();
    let engine = KnowledgeGraphQueryEngine::default();

    let base = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_multi",
        "ent_a",
        Vec::<String>::new(),
        2,
        0,
        100,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap();

    assert_eq!(
        base.clone()
            .with_additional_roots(vec!["not_an_entity"])
            .unwrap_err(),
        KnowledgeGraphQueryError::InvalidEntityId
    );

    let ghost = base.with_additional_roots(vec!["ent_ghost"]).unwrap();
    assert_eq!(
        engine.query_graph_slice(&graph, ghost).unwrap_err(),
        KnowledgeGraphQueryError::MissingRootEntity
    );
}

#[test]
fn duplicate_roots_deduplicated() {
    let mut graph = ObjectGraph::default();
    graph
        .upsert_entity(
            ObjectEntity::new(
                "ten_alpha".to_string(),
                "ent_a".to_string(),
                "ety_account".to_string(),
                vec![property("name")],
            )
            .unwrap(),
        )
        .unwrap();
    let engine = KnowledgeGraphQueryEngine::default();

    let request = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_multi",
        "ent_a",
        Vec::<String>::new(),
        2,
        0,
        100,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap()
    .with_additional_roots(vec!["ent_a", "ent_a"])
    .unwrap();

    let response = engine.query_graph_slice(&graph, request).unwrap();
    assert_eq!(
        response.nodes.len(),
        1,
        "one node, however many times rooted"
    );
}
