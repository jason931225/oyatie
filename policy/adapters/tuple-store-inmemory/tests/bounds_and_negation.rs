//! The cases four surviving mutants proved were untested, and the two
//! wrong-answer defects independent review found.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use policy_cedar_domain::rebac::{RebacReadSnapshot, UsersetRewrite};
use policy_rebac_domain::{ExpansionBounds, ExpansionError, NamespaceConfig};
use policy_tuple_store_inmemory::InMemoryTupleStore;

#[test]
fn a_model_whose_negation_is_self_referential_cannot_be_built() {
    // `doc#editor` excludes `doc#banned`, and `doc#banned` is defined as
    // `doc#editor`. Least-fixed-point re-entry reads the revisit as "not
    // excluded" and GRANTS — the config's author wrote a rule that excludes
    // the banned and would have got one that admits them.
    let cyclic = NamespaceConfig::new()
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::difference(
                UsersetRewrite::this(),
                UsersetRewrite::computed_userset(relation("banned")),
            ),
        )
        .define(
            "doc",
            &relation("banned"),
            UsersetRewrite::computed_userset(relation("editor")),
        );
    assert_eq!(
        cyclic.validated().err(),
        Some(ExpansionError::NonStratified {
            object_type: "doc".to_owned(),
            relation: "banned".to_owned(),
        }),
        "a non-monotone cycle must be refused when the model is built"
    );
}

#[test]
fn a_cycle_that_never_passes_under_a_subtraction_is_still_buildable() {
    // The refusal is specific to negation. Mutual membership is a legitimate
    // shape and must keep working.
    NamespaceConfig::new()
        .define("group", &relation("member"), UsersetRewrite::this())
        .validated()
        .expect("a monotone model is stratified");
}

#[test]
fn difference_denies_a_subject_in_neither_set() {
    // The surviving mutant: dropping the base and answering from `subtract`
    // alone reproduced both previously-tested outcomes, because nobody asked
    // about a subject that is in neither set.
    let model = NamespaceConfig::new()
        .define("doc", &relation("writer"), UsersetRewrite::this())
        .define("doc", &relation("banned"), UsersetRewrite::this())
        .define(
            "doc",
            &relation("editor"),
            UsersetRewrite::difference(
                UsersetRewrite::computed_userset(relation("writer")),
                UsersetRewrite::computed_userset(relation("banned")),
            ),
        )
        .validated()
        .expect("stratified");
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@user:alice");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert!(
        !expander
            .check(
                &user("user:carol"),
                &relation("editor"),
                &object("doc:spec")
            )
            .expect("the walk completes"),
        "a subject in neither the base nor the subtracted set holds nothing"
    );
}

#[test]
fn the_tuple_budget_refuses_rather_than_answering() {
    let mut store = InMemoryTupleStore::new();
    for filler in 0..12 {
        write(
            &mut store,
            &format!("folder:budget#viewer@user:filler{filler}"),
        );
    }
    let model = document_model();
    let bounds = ExpansionBounds {
        max_tuples_read: 4,
        ..ExpansionBounds::DEFAULT
    };
    let expander = bounded_expander(&store, &model, RebacReadSnapshot::latest(), bounds);
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("viewer"),
            &object("folder:budget")
        ),
        Err(ExpansionError::TupleBudgetExceeded { limit: 4 })
    );
}

#[test]
fn standalone_checks_each_receive_a_fresh_candidate_budget() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "folder:budget#viewer@user:alice");
    let model = document_model();
    let bounds = ExpansionBounds {
        max_candidates: 1,
        ..ExpansionBounds::DEFAULT
    };
    let expander = bounded_expander(&store, &model, RebacReadSnapshot::latest(), bounds);

    for _ in 0..2 {
        assert!(
            expander
                .check(
                    &user("user:alice"),
                    &relation("viewer"),
                    &object("folder:budget"),
                )
                .expect("each standalone check owns an independent budget")
        );
    }
}

#[test]
fn a_deeply_nested_rewrite_refuses_instead_of_exhausting_the_stack() {
    // Built iteratively, so this is the evaluator's recursion under test and
    // not the constructor's validation. Charging only object-graph descent
    // left the rewrite tree unbounded, and a deep enough model aborted the
    // process rather than returning a typed refusal.
    let mut nested = UsersetRewrite::this();
    for _ in 0..5_000 {
        nested = UsersetRewrite::Union {
            children: vec![nested],
        };
    }
    // Validation must survive the same tree: an iterative walk there too, or
    // the model that cannot be evaluated unsafely still cannot be CHECKED
    // safely. This overflowed until the stratifier was made iterative.
    let model = NamespaceConfig::new()
        .define("doc", &relation("viewer"), nested)
        .validated()
        .expect("a deep but monotone model still validates");
    let store = InMemoryTupleStore::new();
    let bounds = ExpansionBounds {
        max_depth: 8,
        ..ExpansionBounds::DEFAULT
    };
    let expander = bounded_expander(&store, &model, RebacReadSnapshot::latest(), bounds);
    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("viewer"),
            &object("doc:spec")
        ),
        Err(ExpansionError::DepthExceeded { limit: 8 })
    );
}

#[test]
fn a_userset_on_the_tupleset_side_grants_nothing() {
    // Widening it to its object dropped the `#relation` half and granted off
    // the bare object — strictly more access than the tuple says.
    let model = document_model();
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "folder:budget#viewer@user:alice");
    write(
        &mut store,
        "document:q3#parent@folder:budget#some_other_relation",
    );

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert!(
        !expander
            .check(
                &user("user:alice"),
                &relation("viewer"),
                &object("document:q3")
            )
            .expect("the walk completes"),
        "a tupleset entry naming a userset must not grant off its bare object"
    );
}
