use super::support::*;

#[test]
fn unrestricted_consent_traverses_every_edge() {
    let g = consent_graph();
    let engine = consent_engine(&g);

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_no_consent_filter",
        "ent_root",
        Vec::<&str>::new(),
        3,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap();

    let response = engine.query_graph_slice(&g, req).unwrap();

    let node_ids: Vec<&str> = response
        .nodes
        .iter()
        .map(|n| n.entity_id.as_str())
        .collect();
    assert!(
        node_ids.contains(&"ent_b"),
        "ent_b must be present with empty consent scope"
    );
    assert!(
        node_ids.contains(&"ent_c"),
        "ent_c must be present with empty consent scope"
    );
    assert!(
        node_ids.contains(&"ent_d"),
        "ent_d must be present with empty consent scope"
    );

    assert_eq!(
        response.edges.len(),
        3,
        "all 3 edges must be returned when consent scope is empty"
    );
}

#[test]
fn consent_gate_fires_before_cap_check_and_does_not_set_result_truncated() {
    let g = consent_graph();
    let engine = consent_engine(&g);

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_consent_no_trunc",
        "ent_root",
        Vec::<&str>::new(),
        3,
        0,
        1,
        EdgeConsent::granted(vec!["lty_partner"]),
        TraversalDirection::Outbound,
    )
    .unwrap();

    let response = engine.query_graph_slice(&g, req).unwrap();
    assert!(
        !response.result_truncated,
        "result_truncated must be false when pruning reduces result well below caps"
    );
}

#[test]
fn freshness_filter_still_applies_to_consented_edges() {
    let mut g = ObjectGraph::default();
    for (entity_id, entity_type) in [("ent_root", "ety_account"), ("ent_b", "ety_contact")] {
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
    let mut engine = KnowledgeGraphQueryEngine::default();
    // Insert a consented edge that is stale (observed_at = 5, freshness_floor = 10).
    engine
        .upsert_link(
            &registry(),
            &g,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_root", "ent_b", "lty_partner", 5)
                .unwrap(),
        )
        .unwrap();

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_stale_consented",
        "ent_root",
        Vec::<&str>::new(),
        2,
        10, // freshness floor is 10
        1,
        // lty_partner is consented, but observed_at=5 < floor=10
        EdgeConsent::granted(vec!["lty_partner"]),
        TraversalDirection::Outbound,
    )
    .unwrap();

    let response = engine.query_graph_slice(&g, req).unwrap();
    let node_ids: Vec<&str> = response
        .nodes
        .iter()
        .map(|n| n.entity_id.as_str())
        .collect();
    assert!(
        !node_ids.contains(&"ent_b"),
        "a consented but stale edge must be pruned by freshness; ent_b must be absent"
    );
}

#[test]
fn inbound_reaches_predecessors_outbound_cannot() {
    let g = dir_graph();
    let engine = dir_engine(&g);

    let inbound_req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_inbound",
        "ent_root",
        Vec::<&str>::new(),
        2,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Inbound,
    )
    .unwrap();
    let inbound_resp = engine.query_graph_slice(&g, inbound_req).unwrap();
    let inbound_nodes: Vec<&str> = inbound_resp
        .nodes
        .iter()
        .map(|n| n.entity_id.as_str())
        .collect();

    assert!(
        inbound_nodes.contains(&"ent_pred"),
        "Inbound must reach predecessor ent_pred"
    );
    assert!(
        !inbound_nodes.contains(&"ent_succ"),
        "Inbound must NOT reach successor ent_succ"
    );

    let outbound_req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_outbound_dir",
        "ent_root",
        Vec::<&str>::new(),
        2,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap();
    let outbound_resp = engine.query_graph_slice(&g, outbound_req).unwrap();
    let outbound_nodes: Vec<&str> = outbound_resp
        .nodes
        .iter()
        .map(|n| n.entity_id.as_str())
        .collect();

    assert!(
        !outbound_nodes.contains(&"ent_pred"),
        "Outbound must NOT reach predecessor ent_pred"
    );
    assert!(
        outbound_nodes.contains(&"ent_succ"),
        "Outbound must reach successor ent_succ"
    );
}

#[test]
fn both_yields_union_of_outbound_and_inbound() {
    let g = dir_graph();
    let engine = dir_engine(&g);

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_both",
        "ent_root",
        Vec::<&str>::new(),
        2,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Both,
    )
    .unwrap();
    let resp = engine.query_graph_slice(&g, req).unwrap();
    let node_ids: Vec<&str> = resp.nodes.iter().map(|n| n.entity_id.as_str()).collect();

    assert!(node_ids.contains(&"ent_pred"), "Both must include ent_pred");
    assert!(node_ids.contains(&"ent_root"), "Both must include ent_root");
    assert!(node_ids.contains(&"ent_succ"), "Both must include ent_succ");
    // No duplicate nodes
    assert_eq!(node_ids.len(), 3, "Both must not duplicate nodes");
    // Both edges present with canonical from->to orientation
    assert_eq!(resp.edges.len(), 2, "Both must return exactly 2 edges");
    assert!(
        resp.edges
            .iter()
            .any(|e| e.from_entity_id == "ent_pred" && e.to_entity_id == "ent_root"),
        "pred->root edge must be present in canonical orientation"
    );
    assert!(
        resp.edges
            .iter()
            .any(|e| e.from_entity_id == "ent_root" && e.to_entity_id == "ent_succ"),
        "root->succ edge must be present in canonical orientation"
    );
}

// Deny-by-default law (parity row: consent-scoped traversal): a consent
// posture that grants nothing traverses nothing — only the root comes
// back.
#[test]
fn a_consent_input_granting_nothing_traverses_no_edges() {
    let g = consent_graph();
    let engine = consent_engine(&g);
    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_consent_nothing",
        "ent_root",
        Vec::<&str>::new(),
        3,
        0,
        1,
        EdgeConsent::Granted(Vec::new()),
        TraversalDirection::Outbound,
    )
    .unwrap();
    let response = engine.query_graph_slice(&g, req).unwrap();
    let node_ids: Vec<&str> = response
        .nodes
        .iter()
        .map(|n| n.entity_id.as_str())
        .collect();
    assert_eq!(
        node_ids,
        vec!["ent_root"],
        "grant-nothing consent must return the root alone"
    );
    assert!(
        response.edges.is_empty(),
        "grant-nothing consent must traverse zero edges"
    );
}
