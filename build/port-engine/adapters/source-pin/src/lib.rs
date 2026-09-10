//! # port-engine-source-pin — upstream pin and license verification.
#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

use std::fmt;

use serde::Deserialize;

/// Embedded fleet pin (package-local mirror of `specs/k8s-port/upstream-pin.json` for buck2 hermeticity).
const UPSTREAM_PIN_JSON: &str = include_str!("upstream-pin.json");

pub const fn w0_ready() -> bool {
    true
}

/// Canonical upstream pin fields from `upstream-pin.json#pin`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpstreamPin {
    pub annotated_tag_object: String,
    pub peeled_commit: String,
    pub repository: String,
    /// SPDX license id; [`load_from_str`] refuses anything but `Apache-2.0`.
    pub source_license: String,
    /// Human-readable tag (e.g. `v1.36.1`).
    pub tag: String,
}

/// Typed refusal from pin loading or license verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PinError {
    Parse {
        /// Parser detail (no path — adapter receives bytes only).
        detail: String,
    },
    Schema {
        field: &'static str,
    },
    LicenseMismatch {
        actual: String,
    },
}

impl fmt::Display for PinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { detail } => write!(f, "upstream pin JSON parse failed: {detail}"),
            Self::Schema { field } => write!(f, "upstream pin schema missing or invalid: {field}"),
            Self::LicenseMismatch { actual } => {
                write!(f, "upstream pin license must be Apache-2.0, got `{actual}`")
            }
        }
    }
}

impl std::error::Error for PinError {}

#[derive(Deserialize)]
struct PinDocument {
    pin: PinFields,
}

#[derive(Deserialize)]
struct PinFields {
    annotated_tag_object: String,
    peeled_commit: String,
    repository: String,
    source_license: String,
    tag: String,
}

/// Load and verify the embedded fleet `upstream-pin.json`.
///
/// # Errors
/// [`PinError`] on parse failure, schema violation, or non-Apache license.
pub fn load_embedded() -> Result<UpstreamPin, PinError> {
    load_from_str(UPSTREAM_PIN_JSON)
}

/// Load and verify pin JSON from an in-memory string (test hook and future adapter input).
///
/// # Errors
/// Same refusals as [`load_embedded`].
pub fn load_from_str(json: &str) -> Result<UpstreamPin, PinError> {
    let doc: PinDocument = serde_json::from_str(json).map_err(|err| PinError::Parse {
        detail: err.to_string(),
    })?;
    let pin = doc.pin;
    if pin.annotated_tag_object.is_empty() {
        return Err(PinError::Schema {
            field: "pin.annotated_tag_object",
        });
    }
    if pin.peeled_commit.is_empty() {
        return Err(PinError::Schema {
            field: "pin.peeled_commit",
        });
    }
    if pin.repository.is_empty() {
        return Err(PinError::Schema {
            field: "pin.repository",
        });
    }
    if pin.tag.is_empty() {
        return Err(PinError::Schema { field: "pin.tag" });
    }
    if pin.source_license != "Apache-2.0" {
        return Err(PinError::LicenseMismatch {
            actual: pin.source_license,
        });
    }
    Ok(UpstreamPin {
        annotated_tag_object: pin.annotated_tag_object,
        peeled_commit: pin.peeled_commit,
        repository: pin.repository,
        source_license: pin.source_license,
        tag: pin.tag,
    })
}

/// Receipt-axis pin string for the loaded document (peeled commit is the immutable identity).
#[must_use]
pub fn receipt_pin(pin: &UpstreamPin) -> String {
    pin.peeled_commit.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_pin_loads_and_is_apache_licensed() {
        let pin = load_embedded().expect("embedded upstream-pin.json must parse");
        assert_eq!(pin.source_license, "Apache-2.0");
        assert!(!pin.peeled_commit.is_empty());
        assert_eq!(receipt_pin(&pin), pin.peeled_commit);
    }

    #[test]
    fn rejects_non_apache_license() {
        let json = r#"{"pin":{"annotated_tag_object":"a","peeled_commit":"b","repository":"https://example.com","source_license":"MIT","tag":"v0"}}"#;
        let err = load_from_str(json).expect_err("MIT must be refused");
        assert!(matches!(err, PinError::LicenseMismatch { .. }));
    }
}
