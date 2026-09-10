//! Decision-scoped expansion budget shared across otherwise independent walks.

use policy_cedar_domain::rebac::{
    RebacObjectRef, RebacRelation, RebacSubjectRef, RebacTupleStore, ResolvedRebacSnapshot,
};

use crate::bounds::Budget;
use crate::error::ExpansionError;
use crate::expander::ResolvedExpansion;
use crate::namespace::ValidatedNamespace;
use crate::walk::Walk;

/// A bounded group of checks for one subject over one expander snapshot.
///
/// Subject identity plus candidate and tuple charges are shared across the
/// group. Every check still creates a fresh walk, so traversal path, depth,
/// and negation state cannot leak between candidate roots.
pub struct ExpansionSession<'store, S: RebacTupleStore> {
    evaluator: ResolvedExpansion<'store, S>,
    subject: RebacSubjectRef,
    budget: Budget,
}

impl<'store, S: RebacTupleStore> ExpansionSession<'store, S> {
    /// Start one decision-wide group at a previously resolved store snapshot.
    #[must_use]
    pub fn new(
        store: &'store S,
        namespace: &'store ValidatedNamespace,
        subject: RebacSubjectRef,
        snapshot: ResolvedRebacSnapshot,
        bounds: crate::ExpansionBounds,
    ) -> Self {
        let evaluator = ResolvedExpansion::new(store, namespace, snapshot, bounds);
        Self {
            subject,
            budget: Budget::new(evaluator.bounds()),
            evaluator,
        }
    }

    #[must_use]
    pub fn resolved_snapshot(&self) -> &ResolvedRebacSnapshot {
        self.evaluator.resolved_snapshot()
    }

    pub fn check(
        &mut self,
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        self.budget.charge_candidate()?;
        let mut walk = Walk::new(
            &self.subject,
            self.evaluator.bounds().max_depth,
            &mut self.budget,
        );
        self.evaluator.resolve(&mut walk, relation, object)
    }
}
