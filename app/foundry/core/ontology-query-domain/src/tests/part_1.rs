use super::support::*;

#[test]
fn bounded_query_returns_deterministic_two_hop_subgraph() {
    let graph = graph();
    let mut engine = KnowledgeGraphQueryEngine::default();
    engine
        .upsert_link(
            &registry(),
            &graph,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_root", "ent_contact", "lty_owns", 10)
                .unwrap(),
        )
        .unwrap();
    engine
        .upsert_link(
            &registry(),
            &graph,
            KnowledgeGraphLinkInstance::new(
                "ten_alpha",
                "ent_contact",
                "ent_case",
                "lty_related",
                11,
            )
            .unwrap(),
        )
        .unwrap();

    let response = engine
        .query_graph_slice(&graph, request("ent_root", vec![], 2, 0))
        .unwrap();

    assert_eq!(
        response
            .nodes
            .iter()
            .map(|node| node.entity_id.as_str())
            .collect::<Vec<_>>(),
        vec!["ent_case", "ent_contact", "ent_root"]
    );
    assert_eq!(
        response
            .edges
            .iter()
            .map(|edge| {
                (
                    edge.from_entity_id.as_str(),
                    edge.edge_type_id.as_str(),
                    edge.to_entity_id.as_str(),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("ent_contact", "lty_related", "ent_case"),
            ("ent_root", "lty_owns", "ent_contact")
        ]
    );
    assert_eq!(response.observed_at_epoch_seconds, 12);
}

#[test]
fn edge_type_filter_and_freshness_floor_prune_traversal() {
    let graph = graph();
    let mut engine = KnowledgeGraphQueryEngine::default();
    for (from, to, edge, observed_at) in [
        ("ent_root", "ent_contact", "lty_owns", 100),
        ("ent_root", "ent_case", "lty_related", 100),
        ("ent_contact", "ent_case", "lty_owns", 10),
    ] {
        engine
            .upsert_link(
                &registry(),
                &graph,
                KnowledgeGraphLinkInstance::new("ten_alpha", from, to, edge, observed_at).unwrap(),
            )
            .unwrap();
    }

    let response = engine
        .query_graph_slice(&graph, request("ent_root", vec!["lty_owns"], 2, 50))
        .unwrap();

    assert_eq!(
        response
            .nodes
            .iter()
            .map(|node| node.entity_id.as_str())
            .collect::<Vec<_>>(),
        vec!["ent_contact", "ent_root"]
    );
    assert_eq!(response.edges.len(), 1);
    assert_eq!(response.edges[0].edge_type_id, "lty_owns");
}

#[test]
fn tenant_isolation_blocks_cross_tenant_and_query_leakage() {
    let graph = graph();
    let mut engine = KnowledgeGraphQueryEngine::default();
    let cross_tenant_link =
        KnowledgeGraphLinkInstance::new("ten_alpha", "ent_root", "ent_beta_root", "lty_owns", 1)
            .unwrap();

    assert_eq!(
        engine.upsert_link(&registry(), &graph, cross_tenant_link),
        Err(KnowledgeGraphQueryError::DanglingLinkEndpoint {
            entity_id: "ent_beta_root".to_string()
        })
    );

    engine
        .upsert_link(
            &registry(),
            &graph,
            KnowledgeGraphLinkInstance::new("ten_alpha", "ent_root", "ent_contact", "lty_owns", 1)
                .unwrap(),
        )
        .unwrap();
    let beta_response = engine
        .query_graph_slice(
            &graph,
            KnowledgeGraphQueryRequest::new(
                "ten_beta",
                "kgq_beta",
                "ent_beta_root",
                Vec::<&str>::new(),
                2,
                0,
                2,
                EdgeConsent::Unrestricted,
                TraversalDirection::Outbound,
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(beta_response.nodes.len(), 1);
    assert_eq!(beta_response.nodes[0].entity_id, "ent_beta_root");
    assert!(beta_response.edges.is_empty());
}

#[test]
fn validation_rejects_bad_ids_missing_root_and_unbounded_depth() {
    assert_eq!(
        KnowledgeGraphQueryRequest::new(
            "tenant_alpha",
            "kgq_bad_tenant",
            "ent_root",
            Vec::<&str>::new(),
            1,
            0,
            1,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
        ),
        Err(KnowledgeGraphQueryError::InvalidTenantId)
    );
    assert_eq!(
        KnowledgeGraphQueryRequest::new(
            "ten_alpha",
            "query_bad",
            "ent_root",
            Vec::<&str>::new(),
            1,
            0,
            1,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
        ),
        Err(KnowledgeGraphQueryError::InvalidQueryId)
    );
    assert_eq!(
        KnowledgeGraphQueryRequest::new(
            "ten_alpha",
            "kgq_bad_depth",
            "ent_root",
            Vec::<&str>::new(),
            MAX_QUERY_DEPTH + 1,
            0,
            1,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
        ),
        Err(KnowledgeGraphQueryError::DepthCeilingExceeded)
    );

    let engine = KnowledgeGraphQueryEngine::default();
    assert_eq!(
        engine.query_graph_slice(&graph(), request("ent_missing", Vec::<&str>::new(), 1, 0)),
        Err(KnowledgeGraphQueryError::MissingRootEntity)
    );
}

#[test]
fn cycle_edges_are_reported_without_unbounded_revisit() {
    let graph = graph();
    let mut engine = KnowledgeGraphQueryEngine::default();
    for (from, to) in [
        ("ent_root", "ent_contact"),
        ("ent_contact", "ent_cycle"),
        ("ent_cycle", "ent_root"),
    ] {
        engine
            .upsert_link(
                &registry(),
                &graph,
                KnowledgeGraphLinkInstance::new("ten_alpha", from, to, "lty_owns", 1).unwrap(),
            )
            .unwrap();
    }

    let response = engine
        .query_graph_slice(&graph, request("ent_root", Vec::<&str>::new(), 16, 0))
        .unwrap();

    assert_eq!(response.nodes.len(), 3);
    assert_eq!(response.edges.len(), 3);
    assert!(
        response
            .edges
            .iter()
            .any(|edge| edge.from_entity_id == "ent_cycle" && edge.to_entity_id == "ent_root")
    );
}

// Deny-by-default law (parity row: link types with cardinality
// enforcement): an edge whose lty_ type was never registered for the
// tenant must be refused at upsert.
#[test]
fn an_unregistered_link_type_is_refused_at_upsert() {
    let g = graph();
    let mut engine = KnowledgeGraphQueryEngine::default();
    let outcome = engine.upsert_link(
        &registry(),
        &g,
        KnowledgeGraphLinkInstance::new(
            "ten_alpha",
            "ent_root",
            "ent_contact",
            "lty_never_registered",
            10,
        )
        .unwrap(),
    );
    assert!(
        matches!(outcome, Err(KnowledgeGraphQueryError::UnregisteredLinkType)),
        "an unregistered link type must not enter the store: {outcome:?}"
    );
}
