//! Stratification: refusing a model whose negation is self-referential.
//!
//! Returning "not a member" when a relation re-enters itself is a
//! least-fixed-point convention, and it is sound only while every operator is
//! monotone — adding a tuple may add grants, never remove one. `Difference` is
//! not monotone. If a relation reaches itself through the SUBTRACTED side, the
//! re-entry reads as "not excluded" and the check grants:
//!
//! ```text
//! doc#editor = Difference(This, ComputedUserset(banned))
//! doc#banned = ComputedUserset(editor)
//! doc:spec#editor@user:alice   ->   editor(alice) = ALLOW
//! ```
//!
//! The author wrote a rule that excludes the banned and got one that grants
//! them. Deciding it at check time would be a wrong allow; deciding it here
//! makes the model unbuildable instead.

use std::collections::{BTreeMap, BTreeSet};

use policy_cedar_domain::rebac::{RebacRelation, UsersetRewrite};

use crate::error::ExpansionError;
use crate::namespace::RelationKey;

/// One edge in the relation graph, and whether it passes under a negation.
struct Edge {
    to: RelationKey,
    negated: bool,
}

/// Refuse a model in which a relation reaches itself through the subtracted
/// side of a `Difference`.
pub(crate) fn assert_stratified(
    relations: &BTreeMap<RelationKey, UsersetRewrite>,
) -> Result<(), ExpansionError> {
    let types: BTreeSet<&str> = relations
        .keys()
        .map(|key| key.object_type.as_str())
        .collect();
    let mut graph: BTreeMap<RelationKey, Vec<Edge>> = BTreeMap::new();
    for (node, rewrite) in relations {
        let mut edges = Vec::new();
        collect(&node.object_type, rewrite, false, &types, &mut edges);
        graph.insert(node.clone(), edges);
    }

    // A cycle through a negated edge exists iff some negated edge has both
    // endpoints in one strongly connected component. Enumerating simple paths
    // decides the same question and is factorial: `TupleToUserset` fans out to
    // every defined type, so k types form a complete digraph with ~(k-1)!
    // paths, re-walked from each of k starts. Measured on a model with no
    // negation at all — where the answer is always "fine" — ten object types
    // took 7.5 seconds and thirteen would take hours. Since this is the only
    // constructor for an `Expander`, that made ordinary models unbuildable.
    let component = strongly_connected_components(&graph);
    for (from, edges) in &graph {
        for edge in edges {
            if edge.negated && component.get(from) == component.get(&edge.to) {
                return Err(ExpansionError::NonStratified {
                    object_type: edge.to.object_type.clone(),
                    relation: edge.to.relation.clone(),
                });
            }
        }
    }
    Ok(())
}

/// Tarjan's algorithm, iterative. Recursion here would reintroduce the stack
/// overflow that `collect` was flattened to avoid.
fn strongly_connected_components(
    graph: &BTreeMap<RelationKey, Vec<Edge>>,
) -> BTreeMap<RelationKey, usize> {
    let mut index_of: BTreeMap<&RelationKey, usize> = BTreeMap::new();
    let mut low: BTreeMap<&RelationKey, usize> = BTreeMap::new();
    let mut on_stack: BTreeSet<&RelationKey> = BTreeSet::new();
    let mut stack: Vec<&RelationKey> = Vec::new();
    let mut component: BTreeMap<RelationKey, usize> = BTreeMap::new();
    let mut next_index = 0usize;
    let mut next_component = 0usize;

    for root in graph.keys() {
        if index_of.contains_key(root) {
            continue;
        }
        // (node, how many of its edges have been taken)
        let mut frames: Vec<(&RelationKey, usize)> = vec![(root, 0)];
        index_of.insert(root, next_index);
        low.insert(root, next_index);
        next_index += 1;
        stack.push(root);
        on_stack.insert(root);

        while let Some((node, edge_index)) = frames.pop() {
            let edges = graph.get(node).map_or(&[][..], Vec::as_slice);
            if edge_index < edges.len() {
                frames.push((node, edge_index + 1));
                let Some(target) = graph.get_key_value(&edges[edge_index].to).map(|(k, _)| k)
                else {
                    // An edge to a relation the model never defined. The walk
                    // will refuse it as UndefinedRelation at check time.
                    continue;
                };
                if !index_of.contains_key(target) {
                    index_of.insert(target, next_index);
                    low.insert(target, next_index);
                    next_index += 1;
                    stack.push(target);
                    on_stack.insert(target);
                    frames.push((target, 0));
                } else if on_stack.contains(target) {
                    let target_index = index_of[target];
                    let entry = low.get_mut(node).expect("visited node has a lowlink");
                    *entry = (*entry).min(target_index);
                }
                continue;
            }

            if low[node] == index_of[node] {
                while let Some(member) = stack.pop() {
                    on_stack.remove(member);
                    component.insert(member.clone(), next_component);
                    if member == node {
                        break;
                    }
                }
                next_component += 1;
            }
            if let Some((parent, _)) = frames.last() {
                let child_low = low[node];
                let entry = low.get_mut(*parent).expect("parent is visited");
                *entry = (*entry).min(child_low);
            }
        }
    }
    component
}

/// Walk a rewrite tree iteratively.
///
/// Explicitly stacked rather than recursive: a model is authored input, and a
/// deeply nested one would otherwise abort the process here on a stack
/// overflow — exactly the failure this module exists to convert into a typed
/// refusal. A validator that dies on the input it validates is no validator.
fn collect(
    object_type: &str,
    rewrite: &UsersetRewrite,
    negated: bool,
    types: &BTreeSet<&str>,
    out: &mut Vec<Edge>,
) {
    let mut pending = vec![(rewrite, negated)];
    while let Some((node, negated)) = pending.pop() {
        match node {
            // A direct tuple introduces no relation dependency.
            UsersetRewrite::This => {}
            UsersetRewrite::ComputedUserset { relation } => out.push(Edge {
                to: RelationKey::new(object_type, relation),
                negated,
            }),
            // The tupleset crosses object types and the model does not record
            // which, so every defined type is a possible target.
            // Over-approximating can only refuse a model that might have been
            // fine; under-approximating would admit the wrong allow this
            // module exists to prevent.
            UsersetRewrite::TupleToUserset {
                computed_userset_relation,
                ..
            } => {
                for candidate in types {
                    out.push(Edge {
                        to: RelationKey::new(*candidate, computed_userset_relation),
                        negated,
                    });
                }
            }
            UsersetRewrite::Union { children } | UsersetRewrite::Intersection { children } => {
                pending.extend(children.iter().map(|child| (child, negated)));
            }
            UsersetRewrite::Difference { base, subtract } => {
                pending.push((base.as_ref(), negated));
                // Everything below a subtraction is negated, however deep.
                pending.push((subtract.as_ref(), true));
            }
        }
    }
}
