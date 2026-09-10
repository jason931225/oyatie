use super::support::*;

#[test]
fn max_depth_above_ceiling_returns_depth_ceiling_exceeded_not_invalid_max_depth() {
    let result = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_over_depth",
        "ent_root",
        Vec::<&str>::new(),
        MAX_QUERY_DEPTH + 1,
        0,
        1,
        EdgeConsent::Unrestricted,
        TraversalDirection::Outbound,
    );
    assert_eq!(
        result,
        Err(KnowledgeGraphQueryError::DepthCeilingExceeded),
        "max_depth > MAX_QUERY_DEPTH must return DepthCeilingExceeded"
    );
}

#[test]
fn max_depth_at_ceiling_is_accepted() {
    assert!(
        KnowledgeGraphQueryRequest::new(
            "ten_alpha",
            "kgq_at_ceiling",
            "ent_root",
            Vec::<&str>::new(),
            MAX_QUERY_DEPTH,
            0,
            1,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
        )
        .is_ok(),
        "max_depth == MAX_QUERY_DEPTH must be accepted"
    );
}

#[test]
fn max_depth_zero_returns_invalid_max_depth_not_depth_ceiling_exceeded() {
    assert_eq!(
        KnowledgeGraphQueryRequest::new(
            "ten_alpha",
            "kgq_zero_depth",
            "ent_root",
            Vec::<&str>::new(),
            0,
            0,
            1,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
        ),
        Err(KnowledgeGraphQueryError::InvalidMaxDepth),
        "max_depth == 0 must return InvalidMaxDepth"
    );
}

#[test]
fn malformed_consent_grant_id_rejected() {
    let result = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_bad_grant",
        "ent_root",
        Vec::<&str>::new(),
        1,
        0,
        1,
        EdgeConsent::granted(vec!["bad_id"]),
        TraversalDirection::Outbound,
    );
    assert_eq!(
        result,
        Err(KnowledgeGraphQueryError::MalformedConsentGrantId {
            id: "bad_id".to_string()
        }),
        "a consent grant id without the lty_ prefix must return MalformedConsentGrantId"
    );
}

#[test]
fn well_formed_consent_grant_id_accepted() {
    let result = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_good_grant",
        "ent_root",
        Vec::<&str>::new(),
        1,
        0,
        1,
        EdgeConsent::granted(vec!["lty_partner"]),
        TraversalDirection::Outbound,
    );
    assert!(
        result.is_ok(),
        "a consent grant id with a valid lty_ prefix must be accepted"
    );
}

#[test]
fn edge_consent_permits_exactly_the_named_grants() {
    let granted = EdgeConsent::granted(vec!["lty_partner", "lty_member"]);
    assert!(granted.permits("lty_partner"));
    assert!(granted.permits("lty_member"));
    assert!(
        !granted.permits("lty_owns"),
        "an unnamed edge type is not consented"
    );
    assert!(EdgeConsent::Unrestricted.permits("lty_owns"));
}

#[test]
fn consent_filter_prunes_non_consented_edges() {
    let g = consent_graph();
    let engine = consent_engine(&g);

    let req = KnowledgeGraphQueryRequest::new(
        "ten_alpha",
        "kgq_consent_prune",
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

    let node_ids: Vec<&str> = response
        .nodes
        .iter()
        .map(|n| n.entity_id.as_str())
        .collect();
    assert!(
        node_ids.contains(&"ent_b"),
        "ent_b (reached via consented lty_partner) must be in response nodes"
    );
    assert!(
        node_ids.contains(&"ent_c"),
        "ent_c (reached via consented lty_partner hop) must be in response nodes"
    );
    assert!(
        !node_ids.contains(&"ent_d"),
        "ent_d (reachable only via non-consented lty_member) must be absent from response nodes"
    );

    let member_edges: Vec<_> = response
        .edges
        .iter()
        .filter(|e| e.edge_type_id == "lty_member")
        .collect();
    assert!(
        member_edges.is_empty(),
        "no lty_member edge must appear in response edges when lty_member is not in consent scope"
    );

    let partner_edges: Vec<_> = response
        .edges
        .iter()
        .filter(|e| e.edge_type_id == "lty_partner")
        .collect();
    assert_eq!(
        partner_edges.len(),
        2,
        "both lty_partner edges (root->b, b->c) must appear in response edges"
    );
}
