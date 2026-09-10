//! The single-fixture canary seam: select exactly one region, compare it to the golden.

use std::collections::BTreeMap;

use port_engine_api::{Digest, RegionId};
use port_engine_hash::digest_bytes;

use crate::error::EmitError;
use crate::{CANARY_RULE_SUFFIX, GOLDEN_CANARY_RS};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanaryArtifact {
    pub region: RegionId,
    pub bytes: Vec<u8>,
    pub digest: Digest,
}

/// Select the single canary region from a rendered emit tree.
///
/// # Errors
/// [`EmitError::MissingCanary`] or [`EmitError::AmbiguousCanary`].
pub fn select_canary(emitted: &BTreeMap<RegionId, Vec<u8>>) -> Result<CanaryArtifact, EmitError> {
    let mut matches: Vec<(&RegionId, &Vec<u8>)> = emitted
        .iter()
        .filter(|(id, _)| id.0.ends_with(CANARY_RULE_SUFFIX))
        .collect();
    match matches.len() {
        0 => Err(EmitError::MissingCanary),
        1 => {
            let (region, bytes) = matches.pop().expect("len checked");
            Ok(CanaryArtifact {
                region: region.clone(),
                digest: digest_bytes(bytes),
                bytes: bytes.clone(),
            })
        }
        n => Err(EmitError::AmbiguousCanary { count: n }),
    }
}

#[must_use]
pub fn golden_canary_bytes() -> &'static [u8] {
    GOLDEN_CANARY_RS.as_bytes()
}

#[must_use]
pub fn golden_canary_digest() -> Digest {
    digest_bytes(golden_canary_bytes())
}

/// Fail closed unless `artifact` matches the embedded golden byte-for-byte.
///
/// # Errors
/// [`EmitError::GoldenMismatch`].
pub fn assert_matches_golden(artifact: &CanaryArtifact) -> Result<(), EmitError> {
    let expected = golden_canary_digest();
    if artifact.bytes != golden_canary_bytes() {
        return Err(EmitError::GoldenMismatch {
            actual: artifact.digest.0.clone(),
            expected: expected.0,
            actual_utf8: String::from_utf8_lossy(&artifact.bytes).into_owned(),
        });
    }
    Ok(())
}

/// Select canary, check golden, return artifact (no filesystem write).
///
/// # Errors
/// Selection or golden mismatch.
pub fn emit_canary_checked(
    emitted: &BTreeMap<RegionId, Vec<u8>>,
) -> Result<CanaryArtifact, EmitError> {
    let artifact = select_canary(emitted)?;
    assert_matches_golden(&artifact)?;
    Ok(artifact)
}
