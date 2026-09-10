//! The deterministic flag-evaluation engine.

use crate::bucket::bucket_basis_points;
use crate::model::{
    AttrValue, Condition, EvaluationContext, Flag, Operand, Operator, Rollout, Rule, RuleOutcome,
    Variant, VariantKey,
};

/// Why a particular variant was served (OpenFeature-compatible reason taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The flag is disabled; the `off_variant` was served.
    Disabled,
    /// A targeting rule matched and served a fixed variant.
    TargetingMatch,
    /// The subject was assigned by a percentage rollout (rule-scoped or default).
    Split,
    /// No rule/rollout applied; the flag default variant was served.
    Default,
    /// The flag was malformed (referenced an unknown variant, had no variants, etc.). Fail-closed:
    /// the engine serves the safest resolvable fallback and surfaces this reason for alerting.
    Error,
}

/// The outcome of evaluating a flag for a context.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// The selected variant key.
    pub variant: VariantKey,
    /// The selected variant's value.
    pub value: crate::model::FlagValue,
    /// Why this variant was chosen.
    pub reason: Reason,
    /// If `reason == Error`, a machine-stable code describing the defect; else `None`.
    pub error_code: Option<EvalErrorCode>,
}

/// Machine-stable error codes for malformed flags. Fail-closed: each maps to a safe served value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalErrorCode {
    /// The flag declared no variants at all; nothing can be served.
    NoVariants,
    /// A rule/rollout/default referenced a variant key absent from `flag.variants`.
    UnknownVariant,
}

/// Evaluate `flag` for `context`. Deterministic and side-effect-free.
///
/// Precedence: disabled → first matching rule → default rollout → default variant. Any reference to
/// an unknown variant fails closed to the flag's `off_variant` (or the first variant if even that is
/// unknown) with `Reason::Error`, never a panic.
pub fn evaluate(flag: &Flag, context: &EvaluationContext) -> Evaluation {
    let Some(first_variant) = flag.variants.first() else {
        return no_variants_outcome();
    };
    if !flag.enabled {
        return resolve(flag, &flag.off_variant, Reason::Disabled, first_variant);
    }
    if let Some(rule) = first_matching_rule(flag, context) {
        return matched_rule_outcome(flag, rule, context, first_variant);
    }
    default_rollout_outcome(flag, context, first_variant)
        .unwrap_or_else(|| default_variant_outcome(flag, first_variant))
}

/// A flag with no variants can serve nothing at all: hard fail-closed.
fn no_variants_outcome() -> Evaluation {
    Evaluation {
        variant: String::new(),
        value: crate::model::FlagValue::Bool(false),
        reason: Reason::Error,
        error_code: Some(EvalErrorCode::NoVariants),
    }
}

fn first_matching_rule<'flag>(
    flag: &'flag Flag,
    context: &EvaluationContext,
) -> Option<&'flag Rule> {
    flag.rules.iter().find(|rule| {
        rule.conditions
            .iter()
            .all(|condition| condition_matches(condition, context))
    })
}

fn matched_rule_outcome(
    flag: &Flag,
    rule: &Rule,
    context: &EvaluationContext,
    first_variant: &Variant,
) -> Evaluation {
    match &rule.outcome {
        RuleOutcome::Fixed(variant) => {
            resolve(flag, variant, Reason::TargetingMatch, first_variant)
        }
        RuleOutcome::Rollout(rollout) => match assign_rollout(flag, rollout, context) {
            RolloutAssignment::Variant(variant) => {
                resolve(flag, &variant, Reason::Split, first_variant)
            }
            RolloutAssignment::Unallocated => default_variant_outcome(flag, first_variant),
        },
    }
}

/// Progressive delivery to the unruled population; `None` when the subject falls
/// in the rollout's unallocated remainder.
fn default_rollout_outcome(
    flag: &Flag,
    context: &EvaluationContext,
    first_variant: &Variant,
) -> Option<Evaluation> {
    let rollout = flag.default_rollout.as_ref()?;
    match assign_rollout(flag, rollout, context) {
        RolloutAssignment::Variant(variant) => {
            Some(resolve(flag, &variant, Reason::Split, first_variant))
        }
        RolloutAssignment::Unallocated => None,
    }
}

fn default_variant_outcome(flag: &Flag, first_variant: &Variant) -> Evaluation {
    resolve(flag, &flag.default_variant, Reason::Default, first_variant)
}

/// Resolve a variant key to a full [`Evaluation`], failing closed if the key is unknown.
fn resolve(flag: &Flag, key: &str, reason: Reason, fallback: &Variant) -> Evaluation {
    match flag.variant(key) {
        Some(v) => Evaluation {
            variant: v.key.clone(),
            value: v.value.clone(),
            reason,
            error_code: None,
        },
        None => {
            // Fail closed: the requested variant does not exist. Prefer the off variant if it
            // resolves; otherwise the first declared variant. Surface Reason::Error regardless.
            let safe = flag.variant(&flag.off_variant).unwrap_or(fallback);
            Evaluation {
                variant: safe.key.clone(),
                value: safe.value.clone(),
                reason: Reason::Error,
                error_code: Some(EvalErrorCode::UnknownVariant),
            }
        }
    }
}

/// The result of mapping a subject onto a rollout.
enum RolloutAssignment {
    /// The subject landed inside an allocated bucket.
    Variant(VariantKey),
    /// The subject landed in the unallocated remainder (weights summed to < 100%).
    Unallocated,
}

/// Assign a subject to a variant within `rollout` using the deterministic bucket.
///
/// Buckets are laid out as contiguous half-open basis-point ranges in declared order. A subject's
/// bucket `b` selects the first range `[lo, lo+weight)` that contains it. If `b` exceeds the sum of
/// all weights, the subject is [`RolloutAssignment::Unallocated`].
fn assign_rollout(
    flag: &Flag,
    rollout: &Rollout,
    context: &EvaluationContext,
) -> RolloutAssignment {
    let bucket = bucket_basis_points(&flag.key, &rollout.salt, &context.targeting_key);
    let mut cursor: u32 = 0;
    for (variant, weight) in &rollout.buckets {
        // Saturating add guards a malformed rollout whose weights overflow u32.
        let upper = cursor.saturating_add(*weight);
        if bucket >= cursor && bucket < upper {
            return RolloutAssignment::Variant(variant.clone());
        }
        cursor = upper;
    }
    RolloutAssignment::Unallocated
}

/// True iff `condition` holds for `context`.
fn condition_matches(condition: &Condition, context: &EvaluationContext) -> bool {
    let actual = context.attributes.get(&condition.attribute);
    match (&condition.operator, &condition.operand) {
        (Operator::Eq, Operand::Value(expected)) => actual == Some(expected),
        (Operator::NotEq, Operand::Value(expected)) => actual != Some(expected),
        (Operator::In, Operand::Set(set)) => actual
            .map(|a| set.iter().any(|s| s == &attr_to_string(a)))
            .unwrap_or(false),
        (Operator::NotIn, Operand::Set(set)) => {
            // Absent attribute is NOT in the set → NotIn holds (fail-open on absence is intentional
            // for exclusion rules; presence is required for inclusion rules above).
            actual
                .map(|a| !set.iter().any(|s| s == &attr_to_string(a)))
                .unwrap_or(true)
        }
        // Operator/operand shape mismatch (e.g. `In` with a single `Value`) never matches.
        _ => false,
    }
}

/// Canonical string form of an attribute for set membership comparisons.
fn attr_to_string(value: &AttrValue) -> String {
    match value {
        AttrValue::Bool(b) => b.to_string(),
        AttrValue::Str(s) => s.clone(),
        AttrValue::Int(i) => i.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FlagValue, TOTAL_BASIS_POINTS};

    fn rule_rollout_flag(on_weight: u32) -> Flag {
        Flag {
            key: "checkout.rule-rollout".into(),
            enabled: true,
            variants: vec![
                Variant {
                    key: "on".into(),
                    value: FlagValue::Bool(true),
                },
                Variant {
                    key: "fallback".into(),
                    value: FlagValue::Bool(false),
                },
                Variant {
                    key: "off".into(),
                    value: FlagValue::Bool(false),
                },
            ],
            rules: vec![Rule {
                id: "everyone".into(),
                conditions: vec![],
                outcome: RuleOutcome::Rollout(Rollout {
                    buckets: vec![("on".into(), on_weight)],
                    salt: String::new(),
                }),
            }],
            default_rollout: None,
            default_variant: "fallback".into(),
            off_variant: "off".into(),
        }
    }

    #[test]
    fn rule_scoped_rollout_serves_its_allocated_variant() {
        let flag = rule_rollout_flag(TOTAL_BASIS_POINTS);
        let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
        assert_eq!(ev.variant, "on");
        assert_eq!(ev.reason, Reason::Split);
    }

    #[test]
    fn rule_scoped_rollout_remainder_serves_default_variant_not_off() {
        let flag = rule_rollout_flag(0);
        let ev = evaluate(&flag, &EvaluationContext::for_key("u1"));
        assert_eq!(ev.variant, "fallback");
        assert_eq!(ev.reason, Reason::Default);
    }
}
