use super::support::*;

/// Structurally enforced: `upsert_link` validates both endpoints exist in
/// the same-tenant `ObjectGraph`.
#[test]
fn inbound_tenant_isolation() {
    let mut g = ObjectGraph::default();
    for (tenant, entity_id, entity_type) in [
        ("ten_alpha", "ent_root", "ety_account"),
        ("ten_alpha", "ent_pred", "ety_account"),
        ("ten_beta", "ent_beta_pred", "ety_account"),
        ("ten_beta", "ent_beta_root", "ety_account"),
    ] {
        g.upsert_entity(
            ObjectEntity::new(
                tenant.to_string(),
                entity_id.to_string(),
                entity_type.to_string(),
                vec![property("name")],
            )
            .unwrap(),
        )
        .unwrap();
    }
    let mut engine = KnowledgeGraphQueryEngine::default();
    // ten_alpha: ent_pred -> ent_root
    engine
        .upsert_link(
            &registry(),
            &g,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_pred", "ent_root", "lty_owns", 1)
                .unwrap(),
        )
        .unwrap();
    // ten_beta: ent_beta_pred -> ent_beta_root (different tenant — must not leak)
    engine
        .upsert_link(
            &registry(),
            &g,
            KnowledgeGraphLinkInstance::new(
                "ten_beta",
                "ent_beta_pred",
                "ent_beta_root",
                "lty_owns",
                1,
            )
            .unwrap(),
        )
        .unwrap();

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_inbound_isolation",
        "ent_root",
        Vec::<&str>::new(),
        2,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Inbound,
    )
    .unwrap();
    let resp = engine.query_graph_slice(&g, req).unwrap();
    let node_ids: Vec<&str> = resp.nodes.iter().map(|n| n.entity_id.as_str()).collect();

    assert!(
        node_ids.contains(&"ent_pred"),
        "same-tenant predecessor ent_pred must be visible"
    );
    assert!(
        !node_ids.contains(&"ent_beta_pred"),
        "cross-tenant ent_beta_pred must not be visible"
    );
    assert!(
        !node_ids.contains(&"ent_beta_root"),
        "cross-tenant ent_beta_root must not be visible"
    );
}

/// Graph (forming a cycle): ent_a -> ent_b -> ent_c -> ent_a
#[test]
fn inbound_cycle_no_unbounded_revisit() {
    let mut g = ObjectGraph::default();
    for (entity_id, entity_type) in [
        ("ent_a", "ety_account"),
        ("ent_b", "ety_account"),
        ("ent_c", "ety_account"),
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
    for (from, to) in [("ent_a", "ent_b"), ("ent_b", "ent_c"), ("ent_c", "ent_a")] {
        engine
            .upsert_link(
                &registry(),
                &g,
                KnowledgeGraphLinkInstance::new("ten_alpha", from, to, "lty_owns", 1).unwrap(),
            )
            .unwrap();
    }

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_inbound_cycle",
        "ent_a",
        Vec::<&str>::new(),
        16,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Inbound,
    )
    .unwrap();
    // Must terminate and return the 3 cycle nodes.
    let resp = engine.query_graph_slice(&g, req).unwrap();
    assert_eq!(
        resp.nodes.len(),
        3,
        "inbound cycle must terminate and return 3 nodes"
    );
    assert_eq!(resp.edges.len(), 3, "inbound cycle must return 3 edges");
}
