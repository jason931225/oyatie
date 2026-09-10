//! Two sources, one law.
//!
//! The regression net for the merged law tests — consent, freshness,
//! direction, edge filters, caps and cursors all run against the
//! store-backed path here without being restated.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod source_equivalence_support;
use source_equivalence_support::*;

use foundry_ontology_query_domain::{
    EdgeConsent, TraversalDirection, query_graph_slice_from_store,
};

#[test]
fn both_sources_answer_identically_for_every_request_shape() {
    let (graph, engine) = in_memory();
    let store = in_store();

    for (name, request) in shapes() {
        let from_memory = engine.query_graph_slice(&graph, request.clone()).unwrap();
        let from_store = query_graph_slice_from_store(&store, request).unwrap();
        assert_eq!(
            from_memory, from_store,
            "sources disagree on '{name}' — the law is written once, so this can only be a source bug",
        );
    }
}

#[test]
fn the_stale_edge_is_actually_exercised() {
    // Guards the equivalence test from passing vacuously: the floor must
    // genuinely change the answer, or "identical" proves nothing.
    let store = in_store();
    let unfiltered = query_graph_slice_from_store(
        &store,
        request(
            vec![],
            3,
            0,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
            "ent_root",
        ),
    )
    .unwrap();
    let filtered = query_graph_slice_from_store(
        &store,
        request(
            vec![],
            3,
            5,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
            "ent_root",
        ),
    )
    .unwrap();
    assert!(
        filtered.edges.len() < unfiltered.edges.len(),
        "the freshness floor must drop the stale edge in the store-backed path too",
    );
}

#[test]
fn a_missing_root_refuses_in_the_store_backed_path() {
    let store = in_store();
    let refusal = query_graph_slice_from_store(
        &store,
        request(
            vec![],
            2,
            0,
            EdgeConsent::Unrestricted,
            TraversalDirection::Outbound,
            "ent_absent",
        ),
    );
    assert!(
        refusal.is_err(),
        "a root the store does not hold is a refusal, not an empty graph: {refusal:?}",
    );
}
