//! Domain model for cloud-agnostic flag evaluation.

use std::collections::BTreeMap;

/// Identifier of a flag (the key clients resolve, e.g. `checkout.new-cart`).
pub type FlagKey = String;

/// Identifier of a variant within a flag (e.g. `on`, `off`, `treatment-b`).
pub type VariantKey = String;

/// The typed value a variant resolves to. Cloud-agnostic: a flag value is one of the
/// OpenFeature-compatible scalar/structured shapes, never a backend handle.
#[derive(Debug, Clone, PartialEq)]
pub enum FlagValue {
    Bool(bool),
    Str(String),
    /// i64 domain; the serialization layer narrows as needed.
    Int(i64),
    Float(f64),
    /// Ordered key→value pairs, so iteration is deterministic.
    Object(BTreeMap<String, String>),
}

/// A named, valued outcome of a flag.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    /// Stable variant key (used for bucketing, telemetry, and rule targeting).
    pub key: VariantKey,
    /// The value delivered when this variant is selected.
    pub value: FlagValue,
}

/// An attribute value carried on an [`EvaluationContext`]. Kept deliberately small and
/// comparison-friendly; richer types are normalized into these at the edge.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// Boolean attribute.
    Bool(bool),
    /// String attribute.
    Str(String),
    /// Integer attribute.
    Int(i64),
}

/// The evaluation context: who/what we are evaluating the flag for.
///
/// `targeting_key` is the stable bucketing identity (user id, tenant id, device id, ...). It MUST
/// be stable for a subject so percentage rollouts are sticky. `attributes` drive rule targeting.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EvaluationContext {
    /// Stable identity used for deterministic percentage bucketing. Empty string is allowed
    /// (anonymous); such subjects still bucket deterministically (on the empty key).
    pub targeting_key: String,
    /// Arbitrary subject attributes consulted by targeting rules.
    pub attributes: BTreeMap<String, AttrValue>,
}

impl EvaluationContext {
    pub fn for_key(targeting_key: impl Into<String>) -> Self {
        Self {
            targeting_key: targeting_key.into(),
            attributes: BTreeMap::new(),
        }
    }

    pub fn with_attr(mut self, name: impl Into<String>, value: AttrValue) -> Self {
        self.attributes.insert(name.into(), value);
        self
    }
}

/// Comparison operators a targeting condition can apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operator {
    /// Attribute equals the operand.
    Eq,
    /// Attribute does not equal the operand.
    NotEq,
    /// Attribute (as string) is a member of the operand set.
    In,
    /// Attribute (as string) is NOT a member of the operand set.
    NotIn,
}

/// The operand a [`Condition`] compares against.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Single value operand (for `Eq`/`NotEq`).
    Value(AttrValue),
    /// Set operand (for `In`/`NotIn`), compared on the string form of the attribute.
    Set(Vec<String>),
}

/// A single attribute predicate within a targeting rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// Attribute name to read from the [`EvaluationContext`].
    pub attribute: String,
    /// Operator applied between the attribute and the operand.
    pub operator: Operator,
    /// The operand the operator compares against.
    pub operand: Operand,
}

/// How a matched rule resolves to an outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleOutcome {
    /// Serve a fixed variant to every subject the rule matches.
    Fixed(VariantKey),
    /// Split matched subjects across variants by deterministic percentage (basis points).
    Rollout(Rollout),
}

/// A targeting rule: a conjunction of conditions plus the outcome served when ALL match.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// Stable rule identifier (telemetry / debugging); does not affect evaluation order.
    pub id: String,
    /// All conditions must hold (logical AND) for the rule to match.
    pub conditions: Vec<Condition>,
    /// Outcome served to subjects this rule matches.
    pub outcome: RuleOutcome,
}

/// A percentage split across variants, in BASIS POINTS (0..=10000, i.e. hundredths of a percent).
///
/// Weights are matched against a deterministic bucket derived from
/// `(flag_key, salt, targeting_key)`. The sum of weights SHOULD equal `TOTAL_BASIS_POINTS`; if it
/// is short, the unallocated remainder falls through to the flag default (see engine semantics).
#[derive(Debug, Clone, PartialEq)]
pub struct Rollout {
    /// Ordered (variant, weight-in-basis-points) buckets. Order is significant and stable: the
    /// engine assigns contiguous half-open ranges in this order, so reordering changes assignment.
    pub buckets: Vec<(VariantKey, u32)>,
    /// Optional salt mixed into the bucketing hash so independent rollouts of the same subject on
    /// the same flag can be made independent (e.g. distinct experiments). Empty = no extra salt.
    pub salt: String,
}

/// Total basis points in a full rollout (100% == 10000 bp).
pub const TOTAL_BASIS_POINTS: u32 = 10_000;

/// A feature flag definition: the cloud-agnostic unit the engine evaluates.
/// Evaluation precedence is documented on [`crate::engine::evaluate`].
#[derive(Debug, Clone, PartialEq)]
pub struct Flag {
    /// The flag key (also mixed into bucketing so the same subject buckets independently per flag).
    pub key: FlagKey,
    /// Master switch. When false, the engine short-circuits to `off_variant`.
    pub enabled: bool,
    /// The complete set of variants this flag may resolve to. MUST be non-empty and MUST contain
    /// every variant referenced by rules/rollouts/defaults (validated at evaluation time).
    pub variants: Vec<Variant>,
    /// Ordered targeting rules; first match wins.
    pub rules: Vec<Rule>,
    /// Optional default rollout applied when no rule matches (progressive delivery to the whole
    /// remaining population).
    pub default_rollout: Option<Rollout>,
    /// Variant served when no rule matches and no default rollout assigns the subject.
    pub default_variant: VariantKey,
    /// Variant served when the flag is disabled.
    pub off_variant: VariantKey,
}

impl Flag {
    pub fn variant(&self, key: &str) -> Option<&Variant> {
        self.variants.iter().find(|v| v.key == key)
    }
}
