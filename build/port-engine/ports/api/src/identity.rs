//! Stable identities, and the language pair they are addressed under.
//!
//! The engine COMPARES these and never interprets them: a unit id is not a path, a digest is not an
//! algorithm, and a language slug is not a language.

use crate::error::PortError;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LanguagePair {
    /// Slug of the language being read (matches [`SourceModel::language`]).
    pub source: String, // data_class: INTERNAL_ONLY
    /// Slug of the language being emitted (matches [`TargetIr::target_language`]).
    pub target: String, // data_class: INTERNAL_ONLY
}

const fn is_slug_byte(byte: u8) -> bool {
    byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'+'
}

impl LanguagePair {
    /// The `<pair>` path segment of the rule namespace, e.g. `source-target`.
    ///
    /// FAIL-CLOSED. The segment is the two slugs joined by [`PAIR_SEPARATOR`], so the join is
    /// injective only while neither slug can carry that byte, and the joined value must be ONE
    /// portable path component: anything else addresses a namespace other than the one it names.
    ///
    /// # Errors
    /// [`PortError::AmbiguousLanguagePair`] when either slug is empty or carries a byte the
    /// component grammar does not admit (the separator among them).
    pub fn slug(&self) -> Result<String, PortError> {
        for slug in [&self.source, &self.target] {
            if slug.is_empty() || !slug.bytes().all(is_slug_byte) {
                return Err(PortError::AmbiguousLanguagePair {
                    source: self.source.clone(),
                    target: self.target.clone(),
                });
            }
        }
        Ok(format!("{}{PAIR_SEPARATOR}{}", self.source, self.target))
    }
}

/// The byte joining the two slugs of a [`LanguagePair::slug`], and therefore the byte neither slug
/// may contain.
pub const PAIR_SEPARATOR: char = '-';

const _: () = assert!(
    !is_slug_byte(PAIR_SEPARATOR as u8),
    "the separator must sit outside the slug grammar or the join stops being injective"
);

/// A stable identity for one translatable unit of the source model.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UnitId(pub String); // data_class: INTERNAL_ONLY

/// A stable identity for one rule in a [`RulePack`].
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RuleId(pub String); // data_class: INTERNAL_ONLY

/// A stable identity for one emitted region — the registered regenerable region of ADR-0597
/// (archived; live via apex ADR-0704).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RegionId(pub String); // data_class: INTERNAL_ONLY

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Digest(pub String); // data_class: INTERNAL_ONLY
