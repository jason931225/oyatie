//! The stratifier against an independently formulated oracle.
//!
//! `assert_stratified` decides "is there a cycle through a negated edge" with
//! Tarjan's algorithm, hand-rolled and iterative. The shipped models are far
//! too small to exercise it: dropping the parent lowlink propagation entirely
//! leaves every other test in this crate green. A graph algorithm needs a
//! second opinion, and the oracle here shares no structure with it — a negated
//! edge `u -> v` closes a cycle iff `v` can reach `u`, decided by plain BFS.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use common::*;
use policy_cedar_domain::rebac::{RebacRelation, UsersetRewrite};
use policy_rebac_domain::NamespaceConfig;

type Node = (String, String);

/// A tiny deterministic generator; a seeded LCG keeps failures reproducible.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.0 >> 33
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() as usize) % bound.max(1)
    }
}

fn rewrite(rng: &mut Rng, names: &[RebacRelation], depth: u32) -> UsersetRewrite {
    let pick = |rng: &mut Rng| names[rng.below(names.len())].clone();
    match rng.below(if depth == 0 { 2 } else { 6 }) {
        0 => UsersetRewrite::this(),
        1 => UsersetRewrite::computed_userset(pick(rng)),
        2 => UsersetRewrite::tuple_to_userset(pick(rng), pick(rng)),
        3 => UsersetRewrite::union(vec![
            rewrite(rng, names, depth - 1),
            rewrite(rng, names, depth - 1),
        ])
        .expect("two children"),
        4 => UsersetRewrite::intersection(vec![
            rewrite(rng, names, depth - 1),
            rewrite(rng, names, depth - 1),
        ])
        .expect("two children"),
        _ => UsersetRewrite::difference(
            rewrite(rng, names, depth - 1),
            rewrite(rng, names, depth - 1),
        ),
    }
}

/// Edges of the same graph `assert_stratified` builds, formulated separately.
fn edges(
    object_type: &str,
    rw: &UsersetRewrite,
    negated: bool,
    types: &BTreeSet<String>,
    out: &mut Vec<(Node, bool)>,
) {
    match rw {
        UsersetRewrite::This => {}
        UsersetRewrite::ComputedUserset { relation } => {
            out.push((
                (object_type.to_owned(), relation.as_str().to_owned()),
                negated,
            ));
        }
        UsersetRewrite::TupleToUserset {
            computed_userset_relation,
            ..
        } => {
            for candidate in types {
                out.push((
                    (
                        candidate.clone(),
                        computed_userset_relation.as_str().to_owned(),
                    ),
                    negated,
                ));
            }
        }
        UsersetRewrite::Union { children } | UsersetRewrite::Intersection { children } => {
            for child in children {
                edges(object_type, child, negated, types, out);
            }
        }
        UsersetRewrite::Difference { base, subtract } => {
            edges(object_type, base, negated, types, out);
            edges(object_type, subtract, true, types, out);
        }
    }
}

fn oracle_non_stratified(graph: &BTreeMap<Node, Vec<(Node, bool)>>) -> bool {
    for (from, outgoing) in graph {
        for (to, negated) in outgoing {
            if !negated {
                continue;
            }
            let mut seen = BTreeSet::new();
            let mut queue = VecDeque::from([to.clone()]);
            while let Some(at) = queue.pop_front() {
                if &at == from {
                    return true;
                }
                if !seen.insert(at.clone()) {
                    continue;
                }
                if let Some(next) = graph.get(&at) {
                    queue.extend(next.iter().map(|(node, _)| node.clone()));
                }
            }
        }
    }
    false
}

#[test]
fn stratification_agrees_with_an_independent_reachability_oracle() {
    let mut rng = Rng(0x5eed);
    let mut mismatches = Vec::new();
    let mut refused = 0usize;

    for case in 0..4_000 {
        let type_count = 1 + rng.below(4);
        let relation_count = 1 + rng.below(4);
        let types: BTreeSet<String> = (0..type_count).map(|i| format!("t{i}")).collect();
        let names: Vec<RebacRelation> = (0..relation_count)
            .map(|i| RebacRelation::new(format!("r{i}")).expect("valid relation"))
            .collect();

        let mut config = NamespaceConfig::new();
        let mut graph: BTreeMap<Node, Vec<(Node, bool)>> = BTreeMap::new();
        for object_type in &types {
            for name in &names {
                let rw = rewrite(&mut rng, &names, 2);
                let mut out = Vec::new();
                edges(object_type, &rw, false, &types, &mut out);
                graph.insert((object_type.clone(), name.as_str().to_owned()), out);
                config = config.define(object_type.clone(), name, rw);
            }
        }

        let got = config.validated().is_err();
        let want = oracle_non_stratified(&graph);
        if got {
            refused += 1;
        }
        if got != want {
            mismatches.push(case);
        }
    }

    assert!(
        mismatches.is_empty(),
        "stratifier disagreed with the oracle on {} of 4000 models (first: {:?}); \
         refused {refused}",
        mismatches.len(),
        mismatches.first()
    );
    assert!(
        refused > 100,
        "the corpus must contain real refusals, got {refused}"
    );
}
