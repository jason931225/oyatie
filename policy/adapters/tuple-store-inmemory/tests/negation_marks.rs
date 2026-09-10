//! The arithmetic of the negation marks, one property per mutation it admits.
//!
//! `crosses_negation` asks whether a re-entered node sits BELOW the innermost
//! enclosing subtraction. Five things decide that: where a mark is recorded,
//! whether the comparison is strict, which mark is consulted, where the mark
//! is entered relative to the base, and whether push and pop stay balanced.
//! Every test in `negation_in_data.rs` passes with all five perturbed — they
//! pin only that the guard exists, not that it is placed correctly. Each test
//! here fails under exactly one such perturbation, and each asserts a value
//! that stratified semantics fixes independently of this implementation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use policy_cedar_domain::rebac::{RebacReadSnapshot, UsersetRewrite};
use policy_rebac_domain::{ExpansionError, NamespaceConfig};
use policy_tuple_store_inmemory::InMemoryTupleStore;

#[test]
fn the_first_node_under_a_subtraction_is_inside_it_not_before_it() {
    // The node entered immediately after `enter_negation` sits AT the mark.
    // It is inside the subtracted set, so a cycle returning to it is monotone
    // and contributes nothing. Recording the mark one too high, or comparing
    // with `<=`, puts it below the mark and refuses a decidable check.
    //
    // `banned` is reached through a group that names `banned` itself as a
    // member. The least fixed point of that is empty: nobody is banned, so
    // alice keeps the grant her `writer` tuple gives her.
    let model = NamespaceConfig::new()
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
        .expect("the model is stratified");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@user:alice");
    write(&mut store, "doc:spec#banned@group:a#member");
    // Closes the cycle back onto `doc:spec#banned`, the node at the mark.
    write(&mut store, "group:a#member@doc:spec#banned");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("editor"),
            &object("doc:spec")
        ),
        Ok(true),
        "the exclusion set is empty at its fixed point; alice is a writer and is not banned"
    );
}

#[test]
fn a_completed_subtraction_leaves_no_mark_behind_it() {
    // Within ONE walk: the union's first child runs a subtraction to
    // completion, then the second child is evaluated with no subtraction in
    // scope. If `leave_negation` does not pop, the second child inherits a
    // mark it never entered, and the ordinary cycle in its data — which
    // returns all the way to the root, below that stale mark — is misread as
    // crossing a negation.
    let model = NamespaceConfig::new()
        .define("doc", &relation("writer"), UsersetRewrite::this())
        .define("doc", &relation("banned"), UsersetRewrite::this())
        .define("doc", &relation("viewer"), UsersetRewrite::this())
        .define("group", &relation("member"), UsersetRewrite::this())
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::union(vec![
                UsersetRewrite::difference(
                    UsersetRewrite::computed_userset(relation("writer")),
                    UsersetRewrite::computed_userset(relation("banned")),
                ),
                UsersetRewrite::computed_userset(relation("viewer")),
            ])
            .expect("a two-child union is valid"),
        )
        .validated()
        .expect("the model is stratified");

    let mut store = InMemoryTupleStore::new();
    // Base holds and the subtraction excludes her, so the first child is
    // false and the union goes on to the second — having entered and left a
    // negation on the way.
    write(&mut store, "doc:spec#writer@user:alice");
    write(&mut store, "doc:spec#banned@user:alice");
    // The second child's data cycles back to the walk's root relation.
    write(&mut store, "doc:spec#viewer@group:a#member");
    write(&mut store, "group:a#member@group:b#member");
    write(&mut store, "group:b#member@doc:spec#editor");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("editor"),
            &object("doc:spec")
        ),
        Ok(false),
        "alice is excluded, and the viewer cycle grants nobody; this is an answer, not a refusal"
    );
}

#[test]
fn a_cycle_crossing_only_the_inner_subtraction_is_still_refused() {
    // Two nested subtractions. The cycle re-enters `doc#blocked`, which sits
    // below the INNER mark but at the outer one. Consulting the outermost
    // mark instead of the innermost reads it as monotone and answers, when
    // the exclusion genuinely depends on its own negation and has no fixed
    // point to report.
    let model = NamespaceConfig::new()
        .define("doc", &relation("flagged"), UsersetRewrite::this())
        .define("doc", &relation("exempt"), UsersetRewrite::this())
        .define(
            "doc",
            &relation("blocked"),
            UsersetRewrite::difference(
                UsersetRewrite::computed_userset(relation("flagged")),
                UsersetRewrite::computed_userset(relation("exempt")),
            ),
        )
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::difference(
                UsersetRewrite::this(),
                UsersetRewrite::computed_userset(relation("blocked")),
            ),
        )
        .validated()
        .expect("the model itself is stratified; the cycle is in the data");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#editor@user:alice");
    write(&mut store, "doc:spec#flagged@user:alice");
    write(&mut store, "doc:spec#exempt@doc:spec#blocked");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("editor"),
            &object("doc:spec")
        ),
        Err(ExpansionError::NegatedCycleInData {
            object_type: "doc".to_owned(),
            relation: "blocked".to_owned(),
        }),
        "an exclusion that depends on its own negation must refuse, however deeply nested"
    );
}

#[test]
fn the_base_of_a_difference_is_not_evaluated_under_its_own_mark() {
    // The mark belongs to the subtracted side only. Recording it before the
    // base is evaluated puts the base under a negation it is not inside, and
    // an ordinary monotone cycle through the base — a group whose membership
    // is defined by the very relation being checked — refuses instead of
    // answering.
    let model = NamespaceConfig::new()
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
        .expect("the model is stratified");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@group:a#member");
    // The cycle runs through the BASE: writer -> group -> back to editor.
    write(&mut store, "group:a#member@doc:spec#editor");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("editor"),
            &object("doc:spec")
        ),
        Ok(false),
        "the cycle never crosses the subtraction; nobody is granted, and it is decidable"
    );
}

#[test]
fn leaving_an_inner_subtraction_keeps_the_outer_mark() {
    // Nesting again, but the inner subtraction runs to COMPLETION inside the
    // outer one and the walk carries on. Popping more than one mark on the
    // way out discards the outer subtraction, and the cycle that crosses it
    // is then invisible — the exclusion grants what it exists to withhold.
    let model = NamespaceConfig::new()
        .define("doc", &relation("flagged"), UsersetRewrite::this())
        .define("doc", &relation("exempt"), UsersetRewrite::this())
        .define("doc", &relation("cyclic"), UsersetRewrite::this())
        .define(
            "doc",
            &relation("banned"),
            UsersetRewrite::union(vec![
                UsersetRewrite::difference(
                    UsersetRewrite::computed_userset(relation("flagged")),
                    UsersetRewrite::computed_userset(relation("exempt")),
                ),
                UsersetRewrite::computed_userset(relation("cyclic")),
            ])
            .expect("a two-child union is valid"),
        )
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::difference(
                UsersetRewrite::this(),
                UsersetRewrite::computed_userset(relation("banned")),
            ),
        )
        .validated()
        .expect("the model itself is stratified; the cycle is in the data");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#editor@user:alice");
    // Inner difference: flagged holds, exempt holds, so it yields false and
    // the union goes on to `cyclic` — having entered and left the inner mark.
    write(&mut store, "doc:spec#flagged@user:alice");
    write(&mut store, "doc:spec#exempt@user:alice");
    // Which cycles back across the OUTER subtraction.
    write(&mut store, "doc:spec#cyclic@doc:spec#editor");

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
        "the outer exclusion still depends on its own negation and must not grant"
    );
}
