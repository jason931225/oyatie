//! The loaded shapes: a rule, its selecting fixtures, and a declared deferral.

use port_engine_api::RuleId;
use serde::Deserialize;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectingFixture {
    pub id: String,
    pub unit: String,
    /// Whether the rule is expected to select for `unit`.
    pub selects: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedRule {
    pub id: RuleId,
    pub version: String,
    pub precondition: String,
    pub construction: String,
    /// Empty means the rule is unit-level.
    pub captures: Vec<String>,
    /// Declared precedence. Load-bearing: the loader refuses a pack whose precedence disagrees
    /// with declaration order, so this can never be a second, silently-ignored ordering.
    pub precedence: i64,
    /// Declared conflict policy. Only [`crate::CONFLICT_REFUSE`] is implemented.
    pub conflict: String,
    /// Selection fixtures, including at least one validated positive fixture.
    pub selecting_fixtures: Vec<SelectingFixture>,
}

/// Wire shape of one ownership rule.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionRule {
    /// Stable identity, so a decision can be cited.
    pub id: String,
    /// Required `mutated` value; omitted means "do not care".
    #[serde(default)]
    pub mutated: Option<bool>,
    /// Required `escapes` value; omitted means "do not care".
    #[serde(default)]
    pub escapes: Option<bool>,
    /// Required `effect_unknown` value; omitted means "do not care".
    #[serde(default)]
    pub effect_unknown: Option<bool>,
    /// Target type template for a parameter, with `{0}` for the pointee.
    pub target: String,
    /// Target form for a receiver. Absent means this disposition declines the receiver position,
    /// which is a refusal rather than a fallback.
    #[serde(default)]
    pub receiver: Option<String>,
    /// Why these facts deserve this form, and what it costs.
    pub reason: String,
}

/// How a trait method binds its receiver, and why. See [`DeferredKind`] on the required reason.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraitReceiver {
    /// `shared`, `exclusive`, or `owned`.
    pub mode: String,
    /// Why this mode, and what it costs.
    pub reason: String,
}

/// A declaration kind the pack knowingly does not translate, and why.
///
/// The reason is REQUIRED and travels in the pack bytes, therefore in the pack digest, therefore in
/// the receipt. That is the whole difference between a deferral and an omission: both emit nothing,
/// but one of them is a decision somebody made and can be found again.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeferredKind {
    /// The declaration kind left untranslated.
    pub kind: String,
    /// Why it is deferred, and where the analysis lives.
    pub reason: String,
}
