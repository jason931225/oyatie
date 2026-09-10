//! Admission: two byte-identical artifacts in, a pin-bound verified model out.

use port_engine_api::{Declaration, Digest, SourceModel};
use port_engine_frontend_go::{
    GoSourceModel, PRODUCER_BOOTSTRAP_GO, SCHEMA_VERSION_DECLARATIONS, SnapshotError,
};
use port_engine_hash::digest_bytes;
use port_engine_source_pin::{load_embedded, receipt_pin};

use crate::admitted::AdmittedSnapshot;
use crate::error::AdmitError;
use crate::preimage::{snapshot_preimage, snapshot_preimage_v1};
use crate::{
    FIXTURE_SNAPSHOT_DRIFT_AFTER_V1_JSON, FIXTURE_SNAPSHOT_DRIFT_BEFORE_V1_JSON,
    FIXTURE_SNAPSHOT_FAILURE_V1_JSON, FIXTURE_SNAPSHOT_INTERFACE_V1_JSON, FIXTURE_SNAPSHOT_JSON,
    FIXTURE_SNAPSHOT_OWNERSHIP_V1_JSON, FIXTURE_SNAPSHOT_REFUSED_V1_JSON, FIXTURE_SNAPSHOT_V1_JSON,
};

/// Admit two byte-identical snapshot artifacts against the fleet pin.
///
/// # Errors
/// [`AdmitError::SnapshotMismatch`] when the two extractor passes differ, or another
/// [`AdmitError`] on decode, pin, language, or digest mismatch.
pub fn admit_reproducible_pair(
    first: &[u8],
    second: &[u8],
) -> Result<AdmittedSnapshot, AdmitError> {
    if first != second {
        return Err(AdmitError::SnapshotMismatch {
            first: digest_bytes(first),
            second: digest_bytes(second),
        });
    }
    admit_one(first, digest_bytes(first))
}

fn admit_one(bytes: &[u8], artifact_digest: Digest) -> Result<AdmittedSnapshot, AdmitError> {
    let model = GoSourceModel::decode(bytes).map_err(AdmitError::Snapshot)?;
    if model.language() != "go" {
        return Err(AdmitError::Language {
            actual: model.language().to_owned(),
        });
    }

    let units = model.units();
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(units.len());
    for unit in &units {
        let producer =
            model
                .producer_for(unit)
                .ok_or(AdmitError::Snapshot(SnapshotError::Schema {
                    field: "packages.producer",
                }))?;
        if producer != PRODUCER_BOOTSTRAP_GO {
            return Err(AdmitError::ProducerNotAuthorized {
                unit: unit.0.clone(),
                actual: producer.to_owned(),
            });
        }
        pairs.push((unit.0.clone(), producer.to_owned()));
    }

    // The preimage is chosen by the artifact's declared version, not by whether declarations
    // happen to be present. Choosing on presence would mean an artifact whose declarations were
    // dropped in transit re-digests cleanly under the v0 rule and admits as a valid empty corpus.
    let computed = if model.schema_version() == SCHEMA_VERSION_DECLARATIONS {
        let mut packages: Vec<(&str, &str, Vec<Declaration>)> = Vec::with_capacity(units.len());
        for ((unit, producer), id) in pairs.iter().zip(units.iter()) {
            let declarations =
                model
                    .declarations_for(id)
                    .ok_or(AdmitError::Snapshot(SnapshotError::Schema {
                        field: "packages.declarations",
                    }))?;
            packages.push((unit.as_str(), producer.as_str(), declarations));
        }
        digest_bytes(&snapshot_preimage_v1(model.language(), &packages))
    } else {
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(u, p)| (u.as_str(), p.as_str()))
            .collect();
        digest_bytes(&snapshot_preimage(model.language(), &refs))
    };
    let claimed = model.snapshot_digest();
    if claimed != computed {
        return Err(AdmitError::DigestMismatch {
            claimed: claimed.0,
            computed: computed.0,
        });
    }

    let pin = load_embedded().map_err(AdmitError::Pin)?;
    Ok(AdmittedSnapshot {
        pin: receipt_pin(&pin),
        artifact_digest,
        model_digest: computed,
        model,
    })
}

/// Admit the package-local OOB bootstrap fixture.
///
/// The embedded fixture has one hermetic byte source, so pairing it with itself exercises normal
/// admission without pretending that a second extractor execution occurred. External extractor
/// output must enter through [`admit_reproducible_pair`] with two independently produced artifacts.
///
/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// Same single-byte-source caveat as [`admit_embedded_fixture`].
///
/// # Errors
/// [`AdmitError`] on fixture defect — including a digest that the Rust preimage disagrees with,
/// which is how a drift between the Go and Rust encoders is meant to surface.
pub fn admit_embedded_fixture_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// This ADMITS — the snapshot is a faithful model of source the translator cannot yet handle, and
/// a model of hard code is not itself invalid. The refusal belongs downstream, at the transform,
/// where the construct is named.
///
/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture_refused_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_REFUSED_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture_ownership_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_OWNERSHIP_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture_interface_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_INTERFACE_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture_failure_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_FAILURE_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture_drift_before_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_DRIFT_BEFORE_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}

/// # Errors
/// [`AdmitError`] on fixture defect.
pub fn admit_embedded_fixture_drift_after_v1() -> Result<AdmittedSnapshot, AdmitError> {
    let bytes = FIXTURE_SNAPSHOT_DRIFT_AFTER_V1_JSON.as_bytes();
    admit_reproducible_pair(bytes, bytes)
}
