//! Typed refusals from snapshot decode and producer validation.

use std::fmt;

use crate::vocabulary::{
    PRODUCER_BOOTSTRAP_GO, PRODUCER_OWNED_RUST, SCHEMA_VERSION_DECLARATIONS,
    SCHEMA_VERSION_IDENTITY_ONLY,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotError {
    Parse {
        /// Parser detail (no path — adapter receives bytes only).
        detail: String,
    },
    Schema {
        field: &'static str,
    },
    UnknownProducer {
        actual: String,
    },
    /// Refused rather than deduplicated: a repeat makes the model shape non-deterministic.
    DuplicateUnit {
        unit_id: String,
    },
    UnknownSchemaVersion {
        actual: u32,
    },
    UnknownDeclarationKind {
        unit_id: String,
        actual: String,
    },
    UnknownTypeKind {
        unit_id: String,
        actual: String,
    },
    UnknownAttr {
        unit_id: String,
        actual: String,
    },
    UnknownFlag {
        unit_id: String,
        actual: String,
    },
    DuplicateDeclaration {
        unit_id: String,
        name: String,
    },
    VersionPayloadMismatch {
        detail: &'static str,
    },
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { detail } => {
                write!(f, "source-model snapshot JSON parse failed: {detail}")
            }
            Self::Schema { field } => {
                write!(
                    f,
                    "source-model snapshot schema missing or invalid: {field}"
                )
            }
            Self::UnknownProducer { actual } => write!(
                f,
                "source-model snapshot package producer must be `{PRODUCER_BOOTSTRAP_GO}` or `{PRODUCER_OWNED_RUST}`, got `{actual}`"
            ),
            Self::DuplicateUnit { unit_id } => {
                write!(f, "source-model snapshot has duplicate unit_id `{unit_id}`")
            }
            Self::UnknownSchemaVersion { actual } => write!(
                f,
                "source-model snapshot schema_version must be {SCHEMA_VERSION_IDENTITY_ONLY} or \
                 {SCHEMA_VERSION_DECLARATIONS}, got {actual}"
            ),
            Self::UnknownDeclarationKind { unit_id, actual } => write!(
                f,
                "source-model snapshot unit `{unit_id}` declares unknown kind `{actual}`"
            ),
            Self::UnknownFlag { unit_id, actual } => write!(
                f,
                "source-model snapshot unit `{unit_id}` carries unknown flag `{actual}`"
            ),
            Self::UnknownTypeKind { unit_id, actual } => write!(
                f,
                "source-model snapshot unit `{unit_id}` carries unknown type kind `{actual}`"
            ),
            Self::UnknownAttr { unit_id, actual } => write!(
                f,
                "source-model snapshot unit `{unit_id}` carries unknown attribute `{actual}`"
            ),
            Self::DuplicateDeclaration { unit_id, name } => write!(
                f,
                "source-model snapshot unit `{unit_id}` declares `{name}` more than once in one \
                 namespace"
            ),
            Self::VersionPayloadMismatch { detail } => {
                write!(
                    f,
                    "source-model snapshot version/payload mismatch: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for SnapshotError {}
