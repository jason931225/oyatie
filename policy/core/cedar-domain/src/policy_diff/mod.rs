//! Policy version diff — deterministic impact classification between two `PolicyVersion`s.
//!
//! `diff_policy_versions(prev, next)` compares every rule in `prev` against every rule
//! in `next` and classifies each change as one of the [`RuleDelta`] variants.
//! [`ImpactReport::has_widening`] signals whether the change broadens the allow surface
//! or removes a deny guard, enabling pre-publish security gates.
//!
//! Pure value transform: no I/O, no clock, no new dependencies beyond `serde`.

mod compute;
mod rule_serde;

pub use compute::diff_policy_versions;

use serde::{Deserialize, Serialize};

use crate::policy::{PolicyEffect, PolicyRuleInput};

/// The identity tuple used to match rules across versions.
///
/// Two rules are considered to address the *same subject* when they share the
/// same `(principal_role, action, resource_prefix, required_attribute)`.  Differences
/// in `effect` or `annotations` are captured as a delta, not a new key.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct RuleKey {
    pub(super) principal_role: String,
    pub(super) action: String,
    pub(super) resource_prefix: String,
    pub(super) required_attribute: Option<(String, String)>,
}

impl From<&PolicyRuleInput> for RuleKey {
    fn from(r: &PolicyRuleInput) -> Self {
        Self {
            principal_role: r.principal_role.clone(),
            action: r.action.clone(),
            resource_prefix: r.resource_prefix.clone(),
            required_attribute: r.required_attribute.clone(),
        }
    }
}

/// Classification of a single rule-level change between two policy versions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleDelta {
    /// A rule present in `next` that had no matching key in `prev`.
    ///
    /// Includes both Allow and Deny additions; see [`ImpactReport::has_widening`] for
    /// the exact widening predicate (only Allow additions widen the surface).
    RuleAdded(PolicyRuleInput),

    /// A rule present in `prev` that has no matching key in `next`.
    RuleRemoved(PolicyRuleInput),

    /// An Allow→Allow change on the same key where `next` has a *looser* resource
    /// scope: the `next` resource_prefix is a strict prefix of `prev`'s (shorter =
    /// broader match) or a `required_attribute` guard was dropped.
    BroadenedAllow {
        prev_rule: PolicyRuleInput,
        next_rule: PolicyRuleInput,
    },

    /// An Allow→Allow change on the same key where `next` has a *tighter* resource
    /// scope: the `next` resource_prefix is longer than `prev`'s (more specific) or a
    /// `required_attribute` guard was added.
    NarrowedAllow {
        prev_rule: PolicyRuleInput,
        next_rule: PolicyRuleInput,
    },

    /// A Deny rule whose exact key was absent in `prev` and is present in `next`.
    ///
    /// This is a non-widening addition of a deny guard.
    AddedDeny(PolicyRuleInput),

    /// A Deny rule present in `prev` that is absent in `next`.
    ///
    /// Removing a deny guard is always widening.
    RemovedDeny(PolicyRuleInput),

    /// The `effect` of a rule changed between versions (Allow↔Deny).
    EffectFlipped {
        prev_rule: PolicyRuleInput,
        next_rule: PolicyRuleInput,
    },
}

/// Aggregated diff result for a pair of `PolicyVersion`s.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImpactReport {
    pub prev_version: String,
    pub next_version: String,
    /// Ordered list of per-rule deltas (sorted deterministically).
    pub deltas: Vec<RuleDelta>,
}

impl ImpactReport {
    /// Returns `true` if any delta broadens the effective allow surface or removes a
    /// deny guard.
    pub fn has_widening(&self) -> bool {
        self.deltas.iter().any(|delta| match delta {
            RuleDelta::RuleAdded(r) => r.effect == PolicyEffect::Allow,
            RuleDelta::RemovedDeny(_) => true,
            RuleDelta::BroadenedAllow { .. } => true,
            RuleDelta::EffectFlipped { next_rule, .. } => next_rule.effect == PolicyEffect::Allow,
            RuleDelta::RuleRemoved(_)
            | RuleDelta::NarrowedAllow { .. }
            | RuleDelta::AddedDeny(_) => false,
        })
    }
}
