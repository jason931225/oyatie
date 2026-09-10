//! Cursor pagination: a capped walk hands back a resumable cursor; pages
//! partition the full result deterministically against an unchanged store.

use super::support::*;

/// A star: ent_root -> ent_spoke_N for N spokes, all fresh and consented.
fn star(spokes: usize) -> (ObjectGraph, KnowledgeGraphQueryEngine) {
    let mut graph = ObjectGraph::default();
    let mut engine = KnowledgeGraphQueryEngine::default();
    let mut add = |entity_id: &str, entity_type: &str| {
        graph
            .upsert_entity(
                ObjectEntity::new(
                    "ten_alpha".to_string(),
                    entity_id.to_string(),
                    entity_type.to_string(),
                    vec![property("name")],
                )
                .unwrap(),
            )
            .unwrap();
    };
    add("ent_root", "ety_account");
    for n in 0..spokes {
        add(&format!("ent_spoke_{n:04}"), "ety_contact");
    }
    for n in 0..spokes {
        engine
            .upsert_link(
                &registry(),
                &graph,
                KnowledgeGraphLinkInstance::new(
                    "ten_alpha",
                    "ent_root",
                    format!("ent_spoke_{n:04}"),
                    "lty_knows",
                    100,
                )
                .unwrap(),
            )
            .unwrap();
    }
    (graph, engine)
}

fn request() -> KnowledgeGraphQueryRequest {
    KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_page",
        "ent_root",
        Vec::<String>::new(),
        3,
        0,
        100,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap()
}

#[test]
fn untruncated_result_has_no_cursor() {
    let (graph, engine) = star(5);
    let response = engine.query_graph_slice(&graph, request()).unwrap();
    assert!(!response.result_truncated);
    assert_eq!(response.next_cursor, None);
    assert_eq!(response.nodes.len(), 6);
    assert_eq!(response.edges.len(), 5);
}

#[test]
fn capped_walk_pages_to_completion() {
    let spokes = MAX_QUERY_RESULT_NODES + 500;
    let (graph, engine) = star(spokes);

    let mut cursor = None;
    let mut all_nodes = BTreeSet::new();
    let mut all_edges = BTreeSet::new();
    let mut pages = 0;
    loop {
        let mut req = request();
        if let Some(c) = cursor {
            req = req.with_resume_cursor(c);
        }
        let response = engine.query_graph_slice(&graph, req).unwrap();
        pages += 1;
        for node in &response.nodes {
            assert!(
                all_nodes.insert(node.entity_id.clone()),
                "page {pages} re-emitted node {}",
                node.entity_id
            );
        }
        for edge in &response.edges {
            assert!(
                all_edges.insert(edge.to_entity_id.clone()),
                "page {pages} re-emitted edge to {}",
                edge.to_entity_id
            );
        }
        match response.next_cursor {
            Some(next) => {
                assert!(response.result_truncated);
                cursor = Some(next);
            }
            None => {
                assert!(!response.result_truncated);
                break;
            }
        }
        assert!(pages < 10, "cursor chain must terminate");
    }

    assert_eq!(pages, 2, "1500 spokes at a 1000-node cap is two pages");
    assert_eq!(all_nodes.len(), spokes + 1, "union of pages = whole graph");
    assert_eq!(all_edges.len(), spokes);
}

#[test]
fn cursor_past_the_end_is_an_empty_final_page() {
    let (graph, engine) = star(5);
    let response = engine
        .query_graph_slice(
            &graph,
            request().with_resume_cursor(QueryCursor {
                nodes_emitted: 1_000_000,
                edges_emitted: 1_000_000,
            }),
        )
        .unwrap();
    assert!(!response.result_truncated);
    assert_eq!(response.next_cursor, None);
    assert!(response.nodes.is_empty());
    assert!(response.edges.is_empty());
}
