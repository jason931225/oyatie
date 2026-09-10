//! Internal userset-rewrite evaluation at one already-resolved snapshot.

use policy_cedar_domain::rebac::{
    RebacObjectRef, RebacRelation, RebacSubjectRef, RebacTuple, RebacTupleQuery, RebacTupleStore,
    RebacTupleStoreError, ResolvedRebacSnapshot, UsersetRewrite,
};

use crate::bounds::ExpansionBounds;
use crate::error::ExpansionError;
use crate::namespace::ValidatedNamespace;
use crate::walk::Walk;

pub(crate) struct ResolvedExpansion<'a, S: RebacTupleStore> {
    store: &'a S,
    namespace: &'a ValidatedNamespace,
    snapshot: ResolvedRebacSnapshot,
    bounds: ExpansionBounds,
}

impl<'a, S: RebacTupleStore> ResolvedExpansion<'a, S> {
    #[must_use]
    pub(crate) fn new(
        store: &'a S,
        namespace: &'a ValidatedNamespace,
        snapshot: ResolvedRebacSnapshot,
        bounds: ExpansionBounds,
    ) -> Self {
        Self {
            store,
            namespace,
            snapshot,
            bounds,
        }
    }

    #[must_use]
    pub(crate) fn resolved_snapshot(&self) -> &ResolvedRebacSnapshot {
        &self.snapshot
    }

    pub(crate) fn bounds(&self) -> ExpansionBounds {
        self.bounds
    }

    /// Resolve `object#relation` for the walk's subject.
    pub(crate) fn resolve(
        &self,
        walk: &mut Walk<'_, '_>,
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        // Resolve before `enter`: an error must not leave a stale ordered path
        // frame, because negation-cycle detection reads that order by position.
        let rewrite = self.namespace.rewrite(object.object_type(), relation)?;
        // A relation already on this path contributes no new grant. Answering
        // false rather than refusing keeps a legitimately cyclic graph (groups
        // that contain each other) usable instead of unanswerable.
        if !walk.enter(object, relation) {
            // Under a subtraction the least-fixed-point convention inverts:
            // "not a member" becomes "not excluded" and grants. Refuse instead.
            if walk.crosses_negation(object, relation) {
                return Err(ExpansionError::NegatedCycleInData {
                    object_type: object.object_type().to_owned(),
                    relation: relation.as_str().to_owned(),
                });
            }
            return Ok(false);
        }
        let held = self.eval(walk, rewrite, relation, object);
        walk.leave(object, relation);
        held
    }

    /// Descend into `object#relation`, honouring the depth bound.
    fn descend_into(
        &self,
        walk: &mut Walk<'_, '_>,
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        walk.descend()?;
        let held = self.resolve(walk, relation, object);
        walk.ascend();
        held
    }

    fn eval(
        &self,
        walk: &mut Walk<'_, '_>,
        rewrite: &UsersetRewrite,
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        match rewrite {
            UsersetRewrite::This => self.direct(walk, relation, object),
            UsersetRewrite::ComputedUserset { relation: computed } => {
                self.descend_into(walk, computed, object)
            }
            UsersetRewrite::TupleToUserset {
                tupleset_relation,
                computed_userset_relation,
            } => self.tuple_to_userset(walk, tupleset_relation, computed_userset_relation, object),
            // Composite arms recurse over the rewrite TREE, which is
            // attacker-shaped just as the object graph is: a config can nest
            // `Union` to any depth. Charging only object-graph descent left
            // this path unbounded, and a deep enough tree aborted the process
            // on a stack overflow instead of refusing.
            UsersetRewrite::Union { children } => {
                walk.descend()?;
                let held = self.any_child(walk, children, relation, object);
                walk.ascend();
                held
            }
            UsersetRewrite::Intersection { children } => {
                walk.descend()?;
                let held = self.every_child(walk, children, relation, object);
                walk.ascend();
                held
            }
            UsersetRewrite::Difference { base, subtract } => {
                walk.descend()?;
                let held = self.difference(walk, base, subtract, relation, object);
                walk.ascend();
                held
            }
        }
    }

    fn any_child(
        &self,
        walk: &mut Walk<'_, '_>,
        children: &[UsersetRewrite],
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        for child in children {
            if self.eval(walk, child, relation, object)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn every_child(
        &self,
        walk: &mut Walk<'_, '_>,
        children: &[UsersetRewrite],
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        for child in children {
            if !self.eval(walk, child, relation, object)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn difference(
        &self,
        walk: &mut Walk<'_, '_>,
        base: &UsersetRewrite,
        subtract: &UsersetRewrite,
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        if !self.eval(walk, base, relation, object)? {
            return Ok(false);
        }
        walk.enter_negation();
        let excluded = self.eval(walk, subtract, relation, object);
        walk.leave_negation();
        Ok(!excluded?)
    }

    /// `This`: tuples written directly against `object#relation`. A tuple
    /// whose subject is a userset expands in turn.
    fn direct(
        &self,
        walk: &mut Walk<'_, '_>,
        relation: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        for tuple in self.read_tupleset(object, relation, walk)? {
            match &tuple.subject {
                candidate if candidate == walk.subject => return Ok(true),
                RebacSubjectRef::Userset {
                    object: via,
                    relation: via_relation,
                } => {
                    if self.descend_into(walk, via_relation, via)? {
                        return Ok(true);
                    }
                }
                RebacSubjectRef::Object { .. } => {}
            }
        }
        Ok(false)
    }

    /// `TupleToUserset`: walk `object#tupleset` to its objects, then ask each
    /// for `computed`. This is what makes `document#viewer` follow
    /// `document#parent` into `folder#viewer`.
    fn tuple_to_userset(
        &self,
        walk: &mut Walk<'_, '_>,
        tupleset_relation: &RebacRelation,
        computed: &RebacRelation,
        object: &RebacObjectRef,
    ) -> Result<bool, ExpansionError> {
        for tuple in self.read_tupleset(object, tupleset_relation, walk)? {
            let via = match &tuple.subject {
                RebacSubjectRef::Object { object } => object,
                // A userset here names no single object to ask. Treating it as
                // its object silently drops the `#relation` half and grants off
                // the bare object, which is strictly more access than written.
                // Skip it: an unusable tupleset entry contributes nothing.
                RebacSubjectRef::Userset { .. } => continue,
            };
            if self.descend_into(walk, computed, via)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Reads every page of `object#relation`.
    fn read_tupleset(
        &self,
        object: &RebacObjectRef,
        relation: &RebacRelation,
        walk: &mut Walk<'_, '_>,
    ) -> Result<Vec<RebacTuple>, ExpansionError> {
        let mut collected = Vec::new();
        let mut page_token = None;
        for _ in 0..self.bounds.max_pages_per_tupleset {
            let query = RebacTupleQuery::object_relation(
                self.snapshot.tenant().clone(),
                object.clone(),
                relation.clone(),
            )
            .at_page(page_token);
            let page = self.store.read_tuples(&query, &self.snapshot)?;
            if page.snapshot != self.snapshot {
                return Err(RebacTupleStoreError::InconsistentSnapshot {
                    requested: self.snapshot.clone(),
                    served: page.snapshot,
                }
                .into());
            }
            walk.budget.charge_tuples(page.tuples.len())?;
            for tuple in &page.tuples {
                if !query.matches(tuple) {
                    return Err(RebacTupleStoreError::TupleOutsideQuery {
                        query: Box::new(query),
                        tuple: Box::new(tuple.clone()),
                    }
                    .into());
                }
            }
            collected.extend(page.tuples);
            match page.next_page_token {
                Some(token) => page_token = Some(token),
                None => return Ok(collected),
            }
        }
        Err(ExpansionError::PageBudgetExceeded {
            limit: self.bounds.max_pages_per_tupleset,
        })
    }
}
