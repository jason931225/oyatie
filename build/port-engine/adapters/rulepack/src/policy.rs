//! Validating the pack's POLICY: what it defers, and how a trait binds its receiver.

use std::collections::BTreeSet;

use crate::error::RulepackError;
use port_engine_api::PointerDisposition;

use crate::rule::{DeferredKind, DispositionRule, LoadedRule, TraitReceiver};

/// Convert and check the ownership rules.
///
/// An id must be unique, because a decision cites it and two rules answering to one name make a
/// citation ambiguous.
///
/// # Errors
/// [`RulepackError::Schema`] on a missing id, target or reason, and
/// [`RulepackError::Schema`] on a duplicate id.
pub(crate) fn validate_dispositions(
    rules: &[DispositionRule],
) -> Result<Vec<PointerDisposition>, RulepackError> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::with_capacity(rules.len());
    for rule in rules {
        if rule.id.is_empty() {
            return Err(RulepackError::Schema {
                field: "pointer_dispositions[].id",
            });
        }
        if rule.target.trim().is_empty() {
            return Err(RulepackError::Schema {
                field: "pointer_dispositions[].target",
            });
        }
        if rule.reason.trim().is_empty() {
            return Err(RulepackError::Schema {
                field: "pointer_dispositions[].reason",
            });
        }
        if !seen.insert(rule.id.clone()) {
            return Err(RulepackError::Schema {
                field: "pointer_dispositions[].id(duplicate)",
            });
        }
        out.push(PointerDisposition {
            id: rule.id.clone(),
            when_mutated: rule.mutated,
            when_escapes: rule.escapes,
            when_effect_unknown: rule.effect_unknown,
            target: rule.target.clone(),
            receiver: rule.receiver.clone(),
            reason: rule.reason.clone(),
        });
    }
    Ok(out)
}

/// Check the declared policy and return the deferred-kind set.
///
/// # Errors
/// [`RulepackError`] on a reasonless deferral, a contradictory one, or an unimplemented receiver
/// mode.
pub(crate) fn validate_policy(
    deferred_kinds: &[DeferredKind],
    trait_receiver: Option<&TraitReceiver>,
    loaded_rules: &[LoadedRule],
) -> Result<BTreeSet<String>, RulepackError> {
    let mut deferred_kind_set = BTreeSet::new();
    for deferred in deferred_kinds {
        if deferred.kind.is_empty() {
            return Err(RulepackError::Schema {
                field: "deferred_kinds[].kind",
            });
        }
        if deferred.reason.trim().is_empty() {
            return Err(RulepackError::Schema {
                field: "deferred_kinds[].reason",
            });
        }
        if let Some(rule) = loaded_rules
            .iter()
            .find(|rule| rule.captures.contains(&deferred.kind))
        {
            return Err(RulepackError::DeferredKindAlsoCaptured {
                kind: deferred.kind.clone(),
                rule: rule.id.0.clone(),
            });
        }
        deferred_kind_set.insert(deferred.kind.clone());
    }

    if let Some(receiver) = trait_receiver {
        if !matches!(receiver.mode.as_str(), "shared" | "exclusive" | "owned") {
            return Err(RulepackError::Schema {
                field: "trait_receiver.mode",
            });
        }
        if receiver.reason.trim().is_empty() {
            return Err(RulepackError::Schema {
                field: "trait_receiver.reason",
            });
        }
    }

    Ok(deferred_kind_set)
}
