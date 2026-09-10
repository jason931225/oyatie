//! Typed refusals from rulepack decode and validation.

use std::fmt;

use crate::CONFLICT_REFUSE;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RulepackError {
    Parse {
        detail: String,
    },
    Schema {
        field: &'static str,
    },
    MissingSelectingFixture {
        rule: String,
    },
    NoPositiveFixture {
        rule: String,
        fixture_count: usize,
    },
    FixtureExpectationMismatch {
        rule: String,
        fixture: String,
        unit: String,
        expected: bool,
        actual: bool,
    },
    UndeclaredApply {
        unit: String,
        rule: String,
    },
    UnimplementedSemantics {
        rule: String,
        field: &'static str,
    },
    PrecedenceDisagreesWithOrder {
        rule: String,
        precedence: i64,
        previous: i64,
    },
    UnknownConflictPolicy {
        rule: String,
        policy: String,
    },
    /// The pack says both things at once: it defers the kind and also captures it.
    DeferredKindAlsoCaptured {
        kind: String,
        rule: String,
    },
    Pair(port_engine_api::PortError),
}

impl fmt::Display for RulepackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { detail } => write!(f, "rulepack JSON parse failed: {detail}"),
            Self::Schema { field } => write!(f, "rulepack schema missing or invalid: {field}"),
            Self::MissingSelectingFixture { rule } => write!(
                f,
                "rulepack rule `{rule}` cannot load without ≥1 selecting fixture"
            ),
            Self::NoPositiveFixture {
                rule,
                fixture_count,
            } => write!(
                f,
                "rulepack rule `{rule}` declares {fixture_count} fixture(s) but none selects it"
            ),
            Self::FixtureExpectationMismatch {
                rule,
                fixture,
                unit,
                expected,
                actual,
            } => write!(
                f,
                "rulepack fixture `{fixture}` for rule `{rule}` and unit `{unit}` expected \
                 selects={expected}, derived selects={actual}"
            ),
            Self::UndeclaredApply { unit, rule } => write!(
                f,
                "rulepack applies rule `{rule}` to unit `{unit}` but rules[] does not declare it"
            ),
            Self::UnimplementedSemantics { rule, field } => write!(
                f,
                "rulepack rule `{rule}` declares `{field}`, which the engine does not implement — \
                 a pack may not declare semantics that are silently dropped"
            ),
            Self::PrecedenceDisagreesWithOrder {
                rule,
                precedence,
                previous,
            } => write!(
                f,
                "rulepack rule `{rule}` declares precedence {precedence} after {previous}: \
                 declaration order is the transform order, so a precedence that disagrees with \
                 it is a second ordering nothing obeys"
            ),
            Self::UnknownConflictPolicy { rule, policy } => write!(
                f,
                "rulepack rule `{rule}` declares conflict policy `{policy}`; only \
                 `{CONFLICT_REFUSE}` is implemented"
            ),
            Self::DeferredKindAlsoCaptured { kind, rule } => write!(
                f,
                "rulepack defers kind `{kind}` and also captures it in rule `{rule}`: the pack \
                 cannot both translate it and record it as untranslated"
            ),
            Self::Pair(err) => write!(f, "rulepack language pair refused: {err}"),
        }
    }
}

impl std::error::Error for RulepackError {}
