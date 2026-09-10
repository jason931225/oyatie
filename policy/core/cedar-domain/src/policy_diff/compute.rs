//! The diff algorithm: identity-key matching, scope-change merging, and
//! impact classification between two policy versions.

use std::collections::{BTreeMap, BTreeSet};

use super::{ImpactReport, RuleDelta, RuleKey};
use crate::policy::{PolicyEffect, PolicyRuleInput, PolicyVersion};

type RuleIndex<'a> = BTreeMap<RuleKey, &'a PolicyRuleInput>;

/// Compare two `PolicyVersion`s of the same `policy_id` and return an [`ImpactReport`].
///
/// The comparison is purely structural over the rule lists; it does not validate the
/// policy or interact with any runtime state.  The resulting deltas are deterministic
/// across repeated calls with identical inputs.
pub fn diff_policy_versions(prev: &PolicyVersion, next: &PolicyVersion) -> ImpactReport {
    let prev_map: RuleIndex<'_> = prev.rules.iter().map(|r| (RuleKey::from(r), r)).collect();
    let next_map: RuleIndex<'_> = next.rules.iter().map(|r| (RuleKey::from(r), r)).collect();

    let mut deltas: Vec<RuleDelta> = Vec::new();
    let mut prev_merged: BTreeSet<RuleKey> = BTreeSet::new();
    let mut next_merged: BTreeSet<RuleKey> = BTreeSet::new();

    merge_allow_scope_changes(
        &prev_map,
        &next_map,
        &mut prev_merged,
        &mut next_merged,
        &mut deltas,
    );
    classify_exact_key_changes(&prev_map, &next_map, &prev_merged, &mut deltas);
    collect_added_rules(&prev_map, &next_map, &next_merged, &mut deltas);

    ImpactReport {
        prev_version: prev.version.clone(),
        next_version: next.version.clone(),
        deltas,
    }
}

/// Emit [`RuleDelta::BroadenedAllow`] / [`RuleDelta::NarrowedAllow`] for Allow
/// rules that kept their `(principal_role, action)` but changed scope, and
/// record both sides as merged so the add/remove passes skip them.
fn merge_allow_scope_changes(
    prev_map: &RuleIndex<'_>,
    next_map: &RuleIndex<'_>,
    prev_merged: &mut BTreeSet<RuleKey>,
    next_merged: &mut BTreeSet<RuleKey>,
    deltas: &mut Vec<RuleDelta>,
) {
    let prev_allows = owned_allows(prev_map);
    let next_allows = owned_allows(next_map);

    for (prev_key, prev_rule) in &prev_allows {
        if next_map.contains_key(prev_key) || prev_merged.contains(prev_key) {
            continue;
        }

        for (next_key, next_rule) in &next_allows {
            if next_merged.contains(next_key) {
                continue;
            }
            if prev_rule.principal_role != next_rule.principal_role
                || prev_rule.action != next_rule.action
            {
                continue;
            }
            if prev_key == next_key {
                continue;
            }

            let prefix_broadened = next_rule.resource_prefix.len()
                < prev_rule.resource_prefix.len()
                && prev_rule
                    .resource_prefix
                    .starts_with(&next_rule.resource_prefix);

            let prefix_narrowed = next_rule.resource_prefix.len() > prev_rule.resource_prefix.len()
                && next_rule
                    .resource_prefix
                    .starts_with(&prev_rule.resource_prefix);

            let attr_dropped =
                prev_rule.required_attribute.is_some() && next_rule.required_attribute.is_none();
            let attr_added =
                prev_rule.required_attribute.is_none() && next_rule.required_attribute.is_some();

            if prefix_broadened || attr_dropped {
                deltas.push(RuleDelta::BroadenedAllow {
                    prev_rule: prev_rule.clone(),
                    next_rule: next_rule.clone(),
                });
            } else if prefix_narrowed || attr_added {
                deltas.push(RuleDelta::NarrowedAllow {
                    prev_rule: prev_rule.clone(),
                    next_rule: next_rule.clone(),
                });
            } else {
                continue;
            }
            prev_merged.insert(prev_key.clone());
            next_merged.insert(next_key.clone());
            break;
        }
    }
}

fn owned_allows(map: &RuleIndex<'_>) -> Vec<(RuleKey, PolicyRuleInput)> {
    map.iter()
        .filter(|(_, r)| r.effect == PolicyEffect::Allow)
        .map(|(k, r)| (k.clone(), (*r).clone()))
        .collect()
}

fn classify_exact_key_changes(
    prev_map: &RuleIndex<'_>,
    next_map: &RuleIndex<'_>,
    prev_merged: &BTreeSet<RuleKey>,
    deltas: &mut Vec<RuleDelta>,
) {
    for (key, prev_rule) in prev_map {
        if prev_merged.contains(key) {
            continue;
        }
        match next_map.get(key) {
            None => {
                if prev_rule.effect == PolicyEffect::Deny {
                    deltas.push(RuleDelta::RemovedDeny((*prev_rule).clone()));
                } else {
                    deltas.push(RuleDelta::RuleRemoved((*prev_rule).clone()));
                }
            }
            Some(next_rule) => {
                if prev_rule.effect != next_rule.effect {
                    deltas.push(RuleDelta::EffectFlipped {
                        prev_rule: (*prev_rule).clone(),
                        next_rule: (*next_rule).clone(),
                    });
                }
            }
        }
    }
}

fn collect_added_rules(
    prev_map: &RuleIndex<'_>,
    next_map: &RuleIndex<'_>,
    next_merged: &BTreeSet<RuleKey>,
    deltas: &mut Vec<RuleDelta>,
) {
    for (key, next_rule) in next_map {
        if next_merged.contains(key) || prev_map.contains_key(key) {
            continue;
        }
        if next_rule.effect == PolicyEffect::Deny {
            deltas.push(RuleDelta::AddedDeny((*next_rule).clone()));
        } else {
            deltas.push(RuleDelta::RuleAdded((*next_rule).clone()));
        }
    }
}
