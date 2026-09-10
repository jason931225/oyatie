//! Typed refusals from snapshot admission.

use std::fmt;

use port_engine_api::Digest;
use port_engine_frontend_go::{PRODUCER_BOOTSTRAP_GO, SnapshotError};
use port_engine_source_pin::PinError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmitError {
    Snapshot(SnapshotError),
    Pin(PinError),
    SnapshotMismatch {
        first: Digest,
        second: Digest,
    },
    DigestMismatch {
        claimed: String,
        computed: String,
    },
    Language {
        actual: String,
    },
    ProducerNotAuthorized {
        /// Unit whose producer is premature.
        unit: String,
        actual: String,
    },
}

impl fmt::Display for AdmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Snapshot(err) => write!(f, "snapshot admit decode failed: {err}"),
            Self::Pin(err) => write!(f, "snapshot admit pin failed: {err}"),
            Self::SnapshotMismatch { first, second } => write!(
                f,
                "snapshot extractor passes differ: first `{}`, second `{}`",
                first.0, second.0
            ),
            Self::DigestMismatch { claimed, computed } => write!(
                f,
                "snapshot admit digest mismatch: claimed `{claimed}`, computed `{computed}`"
            ),
            Self::Language { actual } => write!(
                f,
                "snapshot admit language must be `go` for bootstrap admission, got `{actual}`"
            ),
            Self::ProducerNotAuthorized { unit, actual } => write!(
                f,
                "snapshot admit producer for unit `{unit}` must be `{PRODUCER_BOOTSTRAP_GO}` before \
                 front-end equivalence, got `{actual}`"
            ),
        }
    }
}

impl std::error::Error for AdmitError {}
