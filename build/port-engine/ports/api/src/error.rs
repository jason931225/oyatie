//! The closed refusal set every seam reports through.

use std::collections::BTreeSet;
use std::fmt;

use crate::identity::{PAIR_SEPARATOR, RegionId, RuleId, UnitId};

/// A typed, fail-closed refusal. Every variant carries enough to act on without re-deriving.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortError {
    /// A language slug did not match the one it was paired against.
    LanguageMismatch {
        expected: String, // data_class: INTERNAL_ONLY
        actual: String,   // data_class: INTERNAL_ONLY
    },
    /// The source model emitted the same unit id twice, so step order is ambiguous.
    DuplicateUnit {
        unit: UnitId, // data_class: INTERNAL_ONLY
    },
    /// `rules_for` returned a rule the pack does not declare.
    UndeclaredRule {
        unit: UnitId, // data_class: INTERNAL_ONLY
        rule: RuleId, // data_class: INTERNAL_ONLY
    },
    /// `rules_for` returned pack-declared rules in an order that is not the pack's own.
    RuleOrderViolation {
        unit: UnitId, // data_class: INTERNAL_ONLY
        rule: RuleId, // data_class: INTERNAL_ONLY
    },
    /// A renderer's emitted region set was not exactly the IR's region set.
    RegionSetMismatch {
        missing: BTreeSet<RegionId>,    // data_class: INTERNAL_ONLY
        unexpected: BTreeSet<RegionId>, // data_class: INTERNAL_ONLY
    },
    DuplicateRegion {
        region: RegionId, // data_class: INTERNAL_ONLY
    },
    DuplicateRule {
        rule: RuleId, // data_class: INTERNAL_ONLY
    },
    Render {
        detail: String, // data_class: INTERNAL_ONLY
    },
    /// A [`LanguagePair`] cannot address a rule namespace unambiguously.
    AmbiguousLanguagePair {
        source: String, // data_class: INTERNAL_ONLY
        target: String, // data_class: INTERNAL_ONLY
    },
}

impl fmt::Display for PortError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LanguageMismatch { expected, actual } => {
                write!(
                    f,
                    "language mismatch: expected `{expected}`, got `{actual}`"
                )
            }
            Self::DuplicateUnit { unit } => {
                write!(
                    f,
                    "duplicate source unit `{}`: plan order is ambiguous",
                    unit.0
                )
            }
            Self::UndeclaredRule { unit, rule } => write!(
                f,
                "rule `{}` applied to unit `{}` is not declared by the pack",
                rule.0, unit.0
            ),
            Self::RuleOrderViolation { unit, rule } => write!(
                f,
                "rule `{}` arrived out of pack order for unit `{}`: rules_for must answer in the \
                 order rules() declares",
                rule.0, unit.0
            ),
            Self::RegionSetMismatch {
                missing,
                unexpected,
            } => write!(
                f,
                "renderer region set mismatch: {} missing, {} unexpected",
                missing.len(),
                unexpected.len()
            ),
            Self::DuplicateRegion { region } => write!(
                f,
                "duplicate declared region `{}`: region identity is ambiguous",
                region.0
            ),
            Self::DuplicateRule { rule } => write!(
                f,
                "duplicate declared rule `{}`: rule order is ambiguous",
                rule.0
            ),
            Self::Render { detail } => write!(f, "renderer refused: {detail}"),
            Self::AmbiguousLanguagePair { source, target } => write!(
                f,
                "language pair (`{source}`, `{target}`) cannot address a rule namespace \
                 unambiguously: neither slug may be empty or carry a byte outside the path \
                 component grammar (`{PAIR_SEPARATOR}` among them), because the joined value is \
                 ONE path component"
            ),
        }
    }
}

impl std::error::Error for PortError {}
