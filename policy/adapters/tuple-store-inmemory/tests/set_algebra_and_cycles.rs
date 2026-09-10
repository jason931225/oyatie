#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::*;
use policy_cedar_domain::rebac::{RebacReadSnapshot, UsersetRewrite};
use policy_rebac_domain::{ExpansionError, NamespaceConfig, ValidatedNamespace};
use policy_tuple_store_inmemory::InMemoryTupleStore;

fn difference_model() -> ValidatedNamespace {
    NamespaceConfig::new()
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
        .expect("stratified")
}

#[test]
fn difference_subtracts_the_excluded_userset() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@user:alice");
    write(&mut store, "doc:spec#writer@user:mallory");
    write(&mut store, "doc:spec#banned@user:mallory");

    let model = difference_model();
    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());

    assert!(
        expander
            .check(
                &user("user:alice"),
                &relation("editor"),
                &object("doc:spec")
            )
            .expect("the walk completes"),
        "a writer who is not banned is an editor"
    );
    assert!(
        !expander
            .check(
                &user("user:mallory"),
                &relation("editor"),
                &object("doc:spec")
            )
            .expect("the walk completes"),
        "the subtracted userset must remove a grant the base allowed"
    );
}

#[test]
fn intersection_requires_every_child() {
    let model = NamespaceConfig::new()
        .define("doc", &relation("writer"), UsersetRewrite::this())
        .define("doc", &relation("onboarded"), UsersetRewrite::this())
        .define(
            "doc",
            &relation("publisher"),
            UsersetRewrite::intersection(vec![
                UsersetRewrite::computed_userset(relation("writer")),
                UsersetRewrite::computed_userset(relation("onboarded")),
            ])
            .expect("a two-child intersection is valid"),
        )
        .validated()
        .expect("stratified");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#writer@user:alice");
    write(&mut store, "doc:spec#onboarded@user:alice");
    write(&mut store, "doc:spec#writer@user:bob");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert!(
        expander
            .check(
                &user("user:alice"),
                &relation("publisher"),
                &object("doc:spec")
            )
            .expect("the walk completes")
    );
    assert!(
        !expander
            .check(
                &user("user:bob"),
                &relation("publisher"),
                &object("doc:spec")
            )
            .expect("the walk completes"),
        "holding one child of an intersection must not be enough"
    );
}

#[test]
fn a_userset_subject_expands_to_its_members() {
    let model = NamespaceConfig::new()
        .define("group", &relation("member"), UsersetRewrite::this())
        .define("doc", &relation("viewer"), UsersetRewrite::this())
        .validated()
        .expect("stratified");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "group:platform#member@user:alice");
    write(&mut store, "doc:spec#viewer@group:platform#member");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert!(
        expander
            .check(
                &user("user:alice"),
                &relation("viewer"),
                &object("doc:spec")
            )
            .expect("the walk completes"),
        "a tuple whose subject is a userset must expand to that userset's members"
    );
}

#[test]
fn a_cycle_answers_instead_of_hanging() {
    let model = NamespaceConfig::new()
        .define("group", &relation("member"), UsersetRewrite::this())
        .validated()
        .expect("stratified");

    let mut store = InMemoryTupleStore::new();
    write(&mut store, "group:a#member@group:b#member");
    write(&mut store, "group:b#member@group:a#member");

    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());
    assert!(
        !expander
            .check(&user("user:alice"), &relation("member"), &object("group:a"))
            .expect("a cyclic graph still answers"),
        "a cycle grants nothing on its own"
    );
}

#[test]
fn an_undefined_relation_denies_rather_than_falling_back() {
    let mut store = InMemoryTupleStore::new();
    write(&mut store, "doc:spec#viewer@user:alice");

    // The tuple exists, but nothing defines what `doc#viewer` means. Falling
    // back to direct tuples would make a typo in the config grant exactly the
    // access the config meant to constrain.
    let model = NamespaceConfig::new().validated().expect("stratified");
    let expander = new_expander(&store, &model, RebacReadSnapshot::latest());

    assert_eq!(
        expander.check(
            &user("user:alice"),
            &relation("viewer"),
            &object("doc:spec")
        ),
        Err(ExpansionError::UndefinedRelation {
            object_type: "doc".to_owned(),
            relation: "viewer".to_owned(),
        })
    );
}
