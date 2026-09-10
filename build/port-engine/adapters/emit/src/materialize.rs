//! Writing emitted bytes to disk, under an allowlisted root.
//!
//! Widening WHAT may be written did not widen WHERE. The corpus-root refusal, the `..` refusal and
//! the basename requirement apply to every path here, and a region id — the only part of a
//! destination that comes from DATA — must be a bare identifier before it can name a file.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use port_engine_api::RegionId;

use crate::canary::{CanaryArtifact, assert_matches_golden};
use crate::error::EmitError;
use crate::{CANARY_FILENAME, CANARY_OUT_DIRNAME, EMIT_OUT_DIRNAME};

/// Validate `out_dir` is allowlisted for single-file canary materialize.
///
/// # Errors
/// [`EmitError::PathRefused`] when basename is wrong, path has `..`, or points at the corpus root.
pub fn validate_canary_out_dir(out_dir: &Path) -> Result<(), EmitError> {
    validate_out_dir(out_dir, CANARY_OUT_DIRNAME)
}

/// Validate `out_dir` is allowlisted for whole-tree materialize.
///
/// # Errors
/// [`EmitError::PathRefused`] when basename is wrong, path has `..`, or points at the corpus root.
pub fn validate_emit_out_dir(out_dir: &Path) -> Result<(), EmitError> {
    validate_out_dir(out_dir, EMIT_OUT_DIRNAME)
}

fn validate_out_dir(out_dir: &Path, required_basename: &str) -> Result<(), EmitError> {
    let raw = out_dir.to_string_lossy();
    if raw.contains("..") {
        return Err(EmitError::PathRefused {
            detail: "path must not contain `..`".into(),
        });
    }
    for component in out_dir.components() {
        if let std::path::Component::Normal(name) = component
            && name == "k8s"
        {
            return Err(EmitError::PathRefused {
                detail: "refusing materialize under `k8s/` (bulk corpus forbidden in W0-B)".into(),
            });
        }
    }
    let Some(base) = out_dir.file_name().and_then(|s| s.to_str()) else {
        return Err(EmitError::PathRefused {
            detail: "out path missing basename".into(),
        });
    };
    if base != required_basename {
        return Err(EmitError::PathRefused {
            detail: format!("basename must be `{required_basename}`, got `{base}`"),
        });
    }
    Ok(())
}

/// Materialize a whole emit tree under `out_dir`, one file per region.
///
/// Returns the written paths in region order.
///
/// # Errors
/// [`EmitError::PathRefused`] on a refused destination or an unusable region id, or
/// [`EmitError::Io`] on a filesystem failure.
pub fn materialize_tree(
    out_dir: &Path,
    emitted: &BTreeMap<RegionId, Vec<u8>>,
) -> Result<Vec<PathBuf>, EmitError> {
    validate_emit_out_dir(out_dir)?;
    fs::create_dir_all(out_dir).map_err(|err| EmitError::Io {
        detail: err.to_string(),
    })?;

    let mut written = Vec::with_capacity(emitted.len());
    for (region, bytes) in emitted {
        let dest = out_dir.join(region_filename(region)?);
        fs::write(&dest, bytes).map_err(|err| EmitError::Io {
            detail: err.to_string(),
        })?;
        written.push(dest);
    }
    Ok(written)
}

/// A region id is the only part of a destination path that comes from DATA, so it is checked
/// rather than trusted: `../escape` as a region name would place a file outside the validated root.
fn region_filename(region: &RegionId) -> Result<String, EmitError> {
    if region.0.is_empty()
        || !region
            .0
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(EmitError::PathRefused {
            detail: format!(
                "region id `{}` is not a bare identifier and cannot name a file",
                region.0
            ),
        });
    }
    Ok(format!("{}.rs", region.0))
}

/// Materialize the single canary file under `out_dir` (create dir if needed).
///
/// # Errors
/// Path refusal, IO failure.
pub fn materialize_canary(out_dir: &Path, artifact: &CanaryArtifact) -> Result<PathBuf, EmitError> {
    validate_canary_out_dir(out_dir)?;
    assert_matches_golden(artifact)?;
    fs::create_dir_all(out_dir).map_err(|err| EmitError::Io {
        detail: err.to_string(),
    })?;
    let dest = out_dir.join(CANARY_FILENAME);
    fs::write(&dest, &artifact.bytes).map_err(|err| EmitError::Io {
        detail: err.to_string(),
    })?;
    Ok(dest)
}

/// Materialize the canary then read it back; refuse on byte drift.
///
/// # Errors
/// Path refusal, IO failure, or round-trip mismatch.
pub fn materialize_canary_roundtrip(
    out_dir: &Path,
    artifact: &CanaryArtifact,
) -> Result<PathBuf, EmitError> {
    let dest = materialize_canary(out_dir, artifact)?;
    let written = fs::read(&dest).map_err(|err| EmitError::Io {
        detail: err.to_string(),
    })?;
    if written != artifact.bytes {
        return Err(EmitError::Io {
            detail: format!(
                "canary round-trip mismatch at {}: wrote {} bytes, read {}",
                dest.display(),
                artifact.bytes.len(),
                written.len()
            ),
        });
    }
    Ok(dest)
}
