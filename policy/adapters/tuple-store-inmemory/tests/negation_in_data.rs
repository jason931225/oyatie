//! Negation that a model-time check cannot see, and the line between the
//! cycle that inverts it and the one that does not.
//!
//! `NamespaceConfig::validated()` refuses a relation reaching itself under a
//! subtraction in the MODEL. A tuple whose subject is a userset can close the
//! same cycle in DATA, which no static check reaches. The walk-time guard
//! covers that — but only for a cycle that CROSSES the subtraction. One
//! sitting wholly inside the subtracted set is monotone and ordinary, and
//! refusing it would let two tenant-writable tuples brick a check.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use policy_cedar_domain::rebac::{RebacReadSnapshot, UsersetRewrite};
use policy_rebac_domain::{ExpansionError, NamespaceConfig, ValidatedNamespace};
use policy_tuple_store_inmemory::InMemoryTupleStore;

fn editor_excludes_banned() -> ValidatedNamespace {
    NamespaceConfig::new()
        .define("doc", &relation("writer"), UsersetRewrite::this())
        .define("doc", &relation("banned"), UsersetRewrite::this())
        .define("group", &relation("member"), UsersetRewrite::this())
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::difference(
                UsersetRewrite::computed_userset(relation("writer")),
                UsersetRewrite::computed_userset(relation("banned")),
            ),
        )
        .validated()
        .expect("no negated cycle in the model")
}

#[test]
fn a_tuple_that_closes_a_negated_cycle_refuses() {
    // The model is stratified; the TUPLE closes the cycle. Reading the
    // re-entry as "not a member" would make it "not excluded" and GRANT
    // alice exactly what the rule excludes.
    let model = NamespaceConfig::new()
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::difference(
                UsersetRewrite::this(),
                UsersetRewrite::computed_userset(relation("banned")),
            ),
        )
        .define("doc", &relation("banned"), UsersetRewrite::this())
        .validated()
        .expect("the model itself is stratified");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#editor@user:alice");
    write(&mut store, "doc:spec#banned@doc:spec#editor");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("editor"),
            &object("doc:spec")
        ),
        Err(ExpansionError::NegatedCycleInData {
            object_type: "doc".to_owned(),
            relation: "editor".to_owned(),
        }),
        "an exclusion that depends on itself cannot be decided, and must not grant"
    );
}

#[test]
fn a_monotone_cycle_inside_the_subtracted_set_still_answers() {
    // Two groups containing each other is a shape this crate declares
    // legitimate. Naming one on a blocklist must not disable the check.
    let model = editor_excludes_banned();
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@user:alice");
    write(&mut store, "group:a#member@group:b#member");
    write(&mut store, "group:b#member@group:a#member");
    write(&mut store, "doc:spec#banned@group:a#member");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("editor"),
            &object("doc:spec")
        ),
        Ok(true),
        "alice is a writer and is not banned; a cycle among the banned is not her problem"
    );
}

#[test]
fn a_walk_does_not_inherit_a_previous_walks_negation_state() {
    // Each `check` builds a fresh `Walk`, so marks cannot outlive one
    // decision. This says nothing about a mark leaking WITHIN a walk - it
    // passes with `leave_negation` gutted, because a fresh walk has an empty
    // stack either way. That property is
    // `a_completed_subtraction_leaves_no_mark_behind_it` in
    // `negation_marks.rs`; do not read this test as covering it.
    let model = editor_excludes_banned();
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@user:alice");
    write(&mut store, "group:a#member@group:b#member");
    write(&mut store, "group:b#member@group:a#member");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    let first = expander.check(
        &user("user:alice"),
        &relation("editor"),
        &object("doc:spec"),
    );
    let second = expander.check(
        &user("user:alice"),
        &relation("editor"),
        &object("doc:spec"),
    );
    assert_eq!(first, Ok(true));
    assert_eq!(
        second, first,
        "a walk must not inherit a prior walk's negation state"
    );

    assert_eq!(
        expander.check(&user("user:bob"), &relation("editor"), &object("doc:spec")),
        Ok(false),
        "bob is not a writer, so the base fails and the subtraction never runs"
    );
}
