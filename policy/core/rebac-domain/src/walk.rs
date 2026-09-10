//! State carried through one expansion.
//!
//! Depth, the cycle-detection path and the tuple budget all belong to a single
//! walk rather than to the expander, which is shared and immutable. Bundling
//! them keeps the recursive signatures readable and makes it impossible to
//! recurse while forgetting to charge the budget or record the path.

use std::collections::BTreeSet;

use policy_cedar_domain::rebac::{RebacObjectRef, RebacRelation, RebacSubjectRef};

use crate::bounds::Budget;
use crate::error::ExpansionError;

/// One in-progress expansion, for one subject.
pub(crate) struct Walk<'subject, 'budget> {
    /// The subject the whole walk is asking about; never changes.
    pub(crate) subject: &'subject RebacSubjectRef,
    depth: u32,
    max_depth: u32,
    /// `(object, relation)` pairs on the current path, for cycle detection.
    path: BTreeSet<(String, String)>,
    /// The same pairs in visit order, so a re-entry can be placed relative to
    /// where each enclosing subtraction began.
    order: Vec<(String, String)>,
    pub(crate) budget: &'budget mut Budget,
    /// `order.len()` when each enclosing subtraction was entered.
    ///
    /// Re-entry is only unsound when the cycle CROSSES the subtraction. A
    /// monotone cycle sitting wholly inside the subtracted set - two groups
    /// that contain each other, named by a blocklist - is a legitimate shape,
    /// and refusing it turns two tenant-writable tuples into a permanently
    /// failing check.
    marks: Vec<usize>,
}

impl<'subject, 'budget> Walk<'subject, 'budget> {
    pub(crate) fn new(
        subject: &'subject RebacSubjectRef,
        max_depth: u32,
        budget: &'budget mut Budget,
    ) -> Self {
        Self {
            subject,
            depth: 0,
            max_depth,
            path: BTreeSet::new(),
            order: Vec::new(),
            budget,
            marks: Vec::new(),
        }
    }

    /// Enter the subtracted side of a `Difference`.
    pub(crate) fn enter_negation(&mut self) {
        self.marks.push(self.order.len());
    }

    pub(crate) fn leave_negation(&mut self) {
        self.marks.pop();
    }

    /// Does re-entering `object#relation` close a cycle that CROSSES the
    /// innermost enclosing subtraction?
    ///
    /// True only when the node was already on the path before that
    /// subtraction was entered: the cycle then runs through the negated edge,
    /// where "not a member" inverts to "not excluded" and grants. A node first
    /// visited inside the subtracted set is an ordinary monotone cycle and
    /// contributes nothing, as it would anywhere else.
    pub(crate) fn crosses_negation(
        &self,
        object: &RebacObjectRef,
        relation: &RebacRelation,
    ) -> bool {
        let Some(&mark) = self.marks.last() else {
            return false;
        };
        let key = (object.to_canonical_string(), relation.as_str().to_owned());
        self.order
            .iter()
            .position(|entry| *entry == key)
            .is_some_and(|position| position < mark)
    }

    /// Descend one level, refusing past the depth bound.
    pub(crate) fn descend(&mut self) -> Result<(), ExpansionError> {
        self.depth = self.depth.saturating_add(1);
        if self.depth > self.max_depth {
            return Err(ExpansionError::DepthExceeded {
                limit: self.max_depth,
            });
        }
        Ok(())
    }

    pub(crate) fn ascend(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Record `object#relation` on the current path. `false` means it is
    /// already there — a cycle, which contributes no new grant.
    pub(crate) fn enter(&mut self, object: &RebacObjectRef, relation: &RebacRelation) -> bool {
        let key = (object.to_canonical_string(), relation.as_str().to_owned());
        if self.path.insert(key.clone()) {
            self.order.push(key);
            return true;
        }
        false
    }

    pub(crate) fn leave(&mut self, object: &RebacObjectRef, relation: &RebacRelation) {
        let key = (object.to_canonical_string(), relation.as_str().to_owned());
        self.path.remove(&key);
        // `resolve` calls `leave` only on the frame whose `enter` returned
        // true, and takes no `?` between the two, so this frame is the last
        // one pushed. Popping unconditionally keeps `order` and `path` the
        // same set; the assertion states the invariant that makes reading
        // `order` by position meaningful.
        debug_assert_eq!(
            self.order.last(),
            Some(&key),
            "leave must unwind the frame enter pushed"
        );
        self.order.pop();
    }
}
