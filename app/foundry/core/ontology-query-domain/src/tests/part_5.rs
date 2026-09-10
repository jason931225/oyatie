use super::support::*;

#[test]
fn default_direction_reproduces_outbound_result() {
    let g = dir_graph();
    let engine = dir_engine(&g);

    let explicit = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_explicit_out",
        "ent_root",
        Vec::<&str>::new(),
        2,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    )
    .unwrap();
    let default_dir = KnowledgeGraphQueryRequest {
        query_id: "kgq_default_dir".to_string(),
        ..explicit.clone()
    };

    let r_explicit = engine.query_graph_slice(&g, explicit).unwrap();
    let r_default = engine.query_graph_slice(&g, default_dir).unwrap();

    assert_eq!(
        r_explicit.nodes, r_default.nodes,
        "default direction must produce same nodes as explicit Outbound"
    );
    assert_eq!(
        r_explicit.edges, r_default.edges,
        "default direction must produce same edges as explicit Outbound"
    );
    assert_eq!(
        r_explicit.result_truncated, r_default.result_truncated,
        "default direction must produce same result_truncated as explicit Outbound"
    );
}

/// Graph: ent_pred --lty_owns--> ent_root <--lty_partner-- ent_other
#[test]
fn inbound_consent_prunes_non_consented_edges() {
    let mut g = ObjectGraph::default();
    for (entity_id, entity_type) in [
        ("ent_pred", "ety_account"),
        ("ent_root", "ety_account"),
        ("ent_other", "ety_account"),
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
    let mut engine = KnowledgeGraphQueryEngine::default();
    engine
        .upsert_link(
            &registry(),
            &g,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_pred", "ent_root", "lty_owns", 1)
                .unwrap(),
        )
        .unwrap();
    engine
        .upsert_link(
            &registry(),
            &g,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_other", "ent_root", "lty_partner", 1)
                .unwrap(),
        )
        .unwrap();

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_inbound_consent",
        "ent_root",
        Vec::<&str>::new(),
        2,
        0,
        1,
        EdgeConsent::granted(vec!["lty_partner"]),
        TraversalDirection::Inbound,
    )
    .unwrap();
    let resp = engine.query_graph_slice(&g, req).unwrap();
    let node_ids: Vec<&str> = resp.nodes.iter().map(|n| n.entity_id.as_str()).collect();

    assert!(
        node_ids.contains(&"ent_other"),
        "ent_other (via consented lty_partner inbound) must be present"
    );
    assert!(
        !node_ids.contains(&"ent_pred"),
        "ent_pred (via non-consented lty_owns inbound) must be absent"
    );
}

#[test]
fn inbound_freshness_floor_prunes_stale_edges() {
    let mut g = ObjectGraph::default();
    for (entity_id, entity_type) in [("ent_pred", "ety_account"), ("ent_root", "ety_account")] {
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
    // stale inbound edge: observed_at=5, freshness_floor=10
    engine
        .upsert_link(
            &registry(),
            &g,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_pred", "ent_root", "lty_owns", 5)
                .unwrap(),
        )
        .unwrap();

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_inbound_stale",
        "ent_root",
        Vec::<&str>::new(),
        2,
        10,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Inbound,
    )
    .unwrap();
    let resp = engine.query_graph_slice(&g, req).unwrap();
    let node_ids: Vec<&str> = resp.nodes.iter().map(|n| n.entity_id.as_str()).collect();

    assert!(
        !node_ids.contains(&"ent_pred"),
        "stale inbound edge must be pruned by freshness floor"
    );
}

#[test]
fn inbound_node_cap_triggers_result_truncated() {
    let cap = MAX_QUERY_RESULT_NODES;
    let pred_count = cap + 1;

    let mut g = ObjectGraph::default();
    g.upsert_entity(
        ObjectEntity::new(
            "ten_alpha".to_string(),
            "ent_root".to_string(),
            "ety_account".to_string(),
            vec![property("name")],
        )
        .unwrap(),
    )
    .unwrap();
    for i in 0..pred_count {
        g.upsert_entity(
            ObjectEntity::new(
                "ten_alpha".to_string(),
                format!("ent_pred_{i:04}"),
                "ety_account".to_string(),
                vec![property("name")],
            )
            .unwrap(),
        )
        .unwrap();
    }

    let mut engine = KnowledgeGraphQueryEngine::default();
    for i in 0..pred_count {
        engine
            .upsert_link(
                &registry(),
                &g,
                KnowledgeGraphLinkInstance::new(
                    "ten_alpha",
                    format!("ent_pred_{i:04}"),
                    "ent_root",
                    "lty_owns",
                    1_u64,
                )
                .unwrap(),
            )
            .unwrap();
    }

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_inbound_cap",
        "ent_root",
        Vec::<&str>::new(),
        1,
        0,
        0,
        EdgeConsent::Unrestricted,
        TraversalDirection::Inbound,
    )
    .unwrap();
    let resp = engine.query_graph_slice(&g, req).unwrap();

    assert!(
        resp.result_truncated,
        "Inbound node cap must set result_truncated"
    );
    assert!(
        resp.nodes.len() <= cap + 1,
        "Inbound nodes must not exceed cap + root"
    );
    assert_every_edge_endpoint_is_returned(&resp);
}
