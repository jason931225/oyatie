//! Typed refusals from canary emit and materialize.

use std::fmt;

use crate::{CANARY_OUT_DIRNAME, CANARY_RULE_SUFFIX};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmitError {
    MissingCanary,
    AmbiguousCanary {
        count: usize,
    },
    GoldenMismatch {
        actual: String,
        expected: String,
        /// UTF-8 lossy spelling of emitted bytes (for golden authoring).
        actual_utf8: String,
    },
    PathRefused {
        detail: String,
    },
    Io {
        detail: String,
    },
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCanary => {
                write!(f, "canary emit: no `{CANARY_RULE_SUFFIX}` region in tree")
            }
            Self::AmbiguousCanary { count } => write!(
                f,
                "canary emit: expected exactly one canary region, found {count}"
            ),
            Self::GoldenMismatch {
                actual,
                expected,
                actual_utf8,
            } => write!(
                f,
                "canary emit golden mismatch: actual `{actual}`, expected `{expected}`, bytes={actual_utf8:?}"
            ),
            Self::PathRefused { detail } => write!(f, "emit path refused: {detail}"),
            Self::Io { detail } => write!(f, "emit io failed: {detail}"),
        }
    }
}

impl std::error::Error for EmitError {}
