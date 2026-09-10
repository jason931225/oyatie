//! Desired properties for one joined authorization decision scope.
//!
//! Adapted from the exact-base red source, SHA-256
//! `370b3cd43a22e1fc9a4e9dd6cb5c15e260fde8fe4da5f5d942a74b3fcca61a08`. The
//! hash is the only link back to that source and is recorded nowhere else.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod decision_scope_adversaries;
mod decision_scope_support;
mod join_fixtures;

use std::collections::BTreeMap;

use decision_scope_support::MustNotDecide;
use join_fixtures::*;
use policy_cedar_domain::rebac::{RebacReadSnapshot, RebacTupleStoreError, SnapshotToken};
use policy_decision_domain::{
    DecisionError, IdentityMappingError, PrincipalMapping, decide, materialize_parents,
};
use policy_rebac_domain::{ExpansionBounds, ExpansionError};
use policy_tuple_store_inmemory::InMemoryTupleStore;
use shared_platform_contracts_kernel::pdp::Decision;

#[test]
fn invalid_identity_mapping_is_rejected_at_construction() {
    assert!(matches!(
        PrincipalMapping::new("User", "user:spoofed"),
        Err(IdentityMappingError::InvalidRebacIdentity(_))
    ));
}

#[test]
fn matching_request_and_graph_identity_can_allow() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "group:eng#member@user:alice");
    let namespace = model();
    let candidates = [eng_candidate()];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);

    let result = decide(
        &pdp(),
        &inputs,
        &request("alice"),
        BTreeMap::new(),
        context_entities(),
    )
    .expect("a coherent joined decision completes");

    assert_eq!(result.outcome.response.decision, Decision::Allow);
    assert_eq!(result.relationship_snapshot.as_str(), "1");
}

#[test]
fn absent_membership_is_a_decided_deny() {
    let store = InMemoryTupleStore::new();
    let namespace = model();
    let candidates = [eng_candidate()];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);

    let result = decide(
        &pdp(),
        &inputs,
        &request("bob"),
        BTreeMap::new(),
        context_entities(),
    )
    .expect("an absent membership is a completed decision");

    assert_eq!(result.outcome.response.decision, Decision::Deny);
}

#[test]
fn graph_membership_cannot_transfer_between_principals() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "group:eng#member@user:alice");
    let namespace = model();
    let candidates = [eng_candidate()];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);

    let result = decide(
        &pdp(),
        &inputs,
        &request("bob"),
        BTreeMap::new(),
        context_entities(),
    )
    .expect("Bob's graph is complete");

    assert_eq!(result.outcome.response.decision, Decision::Deny);
}

#[test]
fn graph_membership_cannot_cross_tenant_scope() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "group:eng#member@user:alice");
    let namespace = model();
    let candidates = [eng_candidate()];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);
    let mut other_tenant_request = request("alice");
    other_tenant_request.tenant_id = "ten_other".to_owned();

    let result = decide(
        &pdp(),
        &inputs,
        &other_tenant_request,
        BTreeMap::new(),
        context_entities(),
    )
    .expect("the other tenant's graph is complete");

    assert_eq!(result.outcome.response.decision, Decision::Deny);
}

#[test]
fn all_candidates_use_one_resolved_snapshot() {
    let store = AdvancingLatestStore::new();
    let namespace = model();
    let candidates = [candidate("a"), candidate("b")];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);

    let materialized =
        materialize_parents(&inputs, &request("alice")).expect("materialization completes");

    assert_eq!(materialized.parents(), &[entity("Group", "a")]);
    assert_eq!(materialized.resolved_snapshot().as_str(), "5");
    assert_eq!(store.resolution_count(), 1);
}

#[test]
fn all_candidates_share_one_total_tuple_budget() {
    let store = WideCandidateStore::new(2);
    let namespace = model();
    let candidates = [candidate("a"), candidate("b")];
    let bounds = ExpansionBounds {
        max_tuples_read: 3,
        ..ExpansionBounds::DEFAULT
    };
    let inputs = bounded_graph(
        &store,
        &namespace,
        RebacReadSnapshot::latest(),
        &candidates,
        bounds,
    );
    let engine = MustNotDecide::new();

    let result = decide(
        &engine,
        &inputs,
        &request("alice"),
        BTreeMap::new(),
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DecisionError::Expansion(
            ExpansionError::TupleBudgetExceeded { limit: 3 }
        ))
    ));
    assert!(!engine.was_consulted());
}

#[test]
fn candidate_enumeration_uses_the_decision_scope_budget() {
    let store = InMemoryTupleStore::new();
    let namespace = model();
    let candidates = [candidate("a"), candidate("b")];
    let bounds = ExpansionBounds {
        max_candidates: 1,
        ..ExpansionBounds::DEFAULT
    };
    let inputs = bounded_graph(
        &store,
        &namespace,
        RebacReadSnapshot::latest(),
        &candidates,
        bounds,
    );

    let result = materialize_parents(&inputs, &request("alice"));

    assert!(matches!(
        result,
        Err(policy_decision_domain::MaterializationError::Expansion(
            ExpansionError::CandidateBudgetExceeded { limit: 1 }
        ))
    ));
}

#[test]
fn coherent_multi_candidate_materialization_makes_progress() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "group:a#member@user:alice");
    write(&mut store, "group:b#member@user:alice");
    let namespace = model();
    let candidates = [candidate("a"), candidate("b")];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);

    let materialized =
        materialize_parents(&inputs, &request("alice")).expect("both coherent candidates complete");

    assert_eq!(
        materialized.parents(),
        &[entity("Group", "a"), entity("Group", "b")]
    );
    assert_eq!(materialized.resolved_snapshot().as_str(), "2");
}

#[test]
fn malformed_identity_is_refused_before_cedar() {
    let store = InMemoryTupleStore::new();
    let namespace = model();
    let candidates = [eng_candidate()];
    let inputs = graph(&store, &namespace, RebacReadSnapshot::latest(), &candidates);
    let mut malformed = request("alice");
    malformed.principal.entity_id = "alice#delegated".to_owned();
    let engine = MustNotDecide::new();

    let result = decide(&engine, &inputs, &malformed, BTreeMap::new(), Vec::new());

    assert!(matches!(
        result,
        Err(DecisionError::Identity(
            IdentityMappingError::InvalidRebacIdentity(_)
        ))
    ));
    assert!(!engine.was_consulted());
}

#[test]
fn stale_snapshot_is_refused_before_cedar() {
    let store = InMemoryTupleStore::new();
    let namespace = model();
    let candidates = [eng_candidate()];
    let requested = RebacReadSnapshot::at(SnapshotToken::new("9").expect("token is valid"));
    let inputs = graph(&store, &namespace, requested, &candidates);
    let engine = MustNotDecide::new();

    let result = decide(
        &engine,
        &inputs,
        &request("alice"),
        BTreeMap::new(),
        Vec::new(),
    );

    assert!(matches!(
        result,
        Err(DecisionError::Expansion(ExpansionError::Store(
            RebacTupleStoreError::StaleSnapshot { .. }
        )))
    ));
    assert!(!engine.was_consulted());
}
