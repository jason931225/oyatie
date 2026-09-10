//! File-backed signed policy-bundle store.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use iam_pdp_kernel::{BundleStoreError, PolicyBundleStore};
use serde::{Deserialize, Serialize};
use shared_audit_digest_adapter_awslc::Ed25519ChainVerifier;
use shared_audit_event_kernel::{ChainVerifier, DigestChainError, decode_hex};
use shared_pdp_kernel::PolicyBundle;
use shared_platform_contracts_kernel::pdp::PolicyVersion;

mod publication;
pub use publication::BundlePublishError;

#[cfg(test)]
mod publication_tests;
#[cfg(test)]
mod reader_test_support;
#[cfg(test)]
mod reader_tests;

/// One detached signature over a signed policy-bundle envelope's inner bytes.
///
/// `public_key_hex` is the signer's raw 32-byte Ed25519 Edwards public key
/// (lowercase hex), carried for diagnostics and self-description; it is NOT
/// trusted on the verify path — only the trusted-key set loaded from the trust
/// anchor is consulted (an attacker who embeds their own public key still
/// cannot forge a trusted `key_id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleSignature {
    pub key_id: String,         // data_class: INTERNAL_ONLY
    pub public_key_hex: String, // data_class: PUBLIC
    pub signature_hex: String,  // data_class: INTERNAL_ONLY
}

/// The signed outer envelope around a serialized [`PolicyBundle`]. The
/// `bundle` field holds the EXACT inner bytes the signatures cover (verbatim,
/// not a re-serialization), so verify operates on the same bytes that are then
/// parsed — sign==verify by construction, no canonicalization trap. CLOSED
/// schema: unknown envelope fields are rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedPolicyBundleDoc {
    pub bundle: String,                   // data_class: INTERNAL_ONLY
    pub signatures: Vec<BundleSignature>, // data_class: INTERNAL_ONLY
}

#[derive(Debug, Clone)]
pub struct FilePolicyBundleStore {
    path: PathBuf,
    trust_dir: PathBuf,
}

impl FilePolicyBundleStore {
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, trust_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            trust_dir: trust_dir.into(),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn trust_dir(&self) -> &Path {
        &self.trust_dir
    }
}

/// File extension trusted-key files must carry (defensive: skip stray files in
/// a ConfigMap mount such as `..data` symlinks Kubernetes projects).
const TRUST_KEY_EXTENSION: &str = "pub";

/// Each `*.pub` file's stem is the `key_id` and its contents are the raw
/// 32-byte Edwards public key as lowercase hex.
///
/// # Errors
/// Fail-closed: an unreadable directory, or one carrying no usable key, is a
/// [`BundleStoreError`] — a PDP that cannot prove which keys to trust must
/// never serve.
fn load_trust_anchor(trust_dir: &Path) -> Result<Ed25519ChainVerifier, BundleStoreError> {
    let entries = std::fs::read_dir(trust_dir).map_err(|e| BundleStoreError::Unavailable {
        detail: format!("cannot read trust anchor dir {}: {e}", trust_dir.display()),
    })?;
    let mut verifier = Ed25519ChainVerifier::new();
    let mut trusted_keys = 0usize;
    for entry in entries {
        let entry = entry.map_err(|e| BundleStoreError::Unavailable {
            detail: format!("trust anchor dir {} entry: {e}", trust_dir.display()),
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(TRUST_KEY_EXTENSION) {
            continue;
        }
        let Some(key_id) = path.file_stem().and_then(|s| s.to_str()).map(str::to_owned) else {
            continue;
        };
        let hex = std::fs::read_to_string(&path).map_err(|e| BundleStoreError::Unavailable {
            detail: format!("cannot read trusted key {}: {e}", path.display()),
        })?;
        let public_key = decode_hex(hex.trim()).map_err(|e| BundleStoreError::Malformed {
            detail: format!("trusted key {} is not valid hex: {e}", path.display()),
        })?;
        verifier = verifier.with_key(key_id, public_key);
        trusted_keys += 1;
    }
    if trusted_keys == 0 {
        return Err(BundleStoreError::SignatureRejected {
            detail: format!(
                "trust anchor dir {} carried no trusted signing keys ({TRUST_KEY_EXTENSION} files)",
                trust_dir.display()
            ),
        });
    }
    Ok(verifier)
}

/// Verify a signed envelope against the trusted key set and return the verified
/// inner bytes. Fail-closed: empty signatures, or no signature whose `key_id`
/// is trusted AND validates the inner bytes, is [`BundleStoreError::SignatureRejected`].
fn verify_envelope<'a>(
    doc: &'a SignedPolicyBundleDoc,
    verifier: &Ed25519ChainVerifier,
) -> Result<&'a str, BundleStoreError> {
    if doc.signatures.is_empty() {
        return Err(BundleStoreError::SignatureRejected {
            detail: "signed bundle carries no signatures".to_owned(),
        });
    }
    if any_trusted_signature_validates(doc, verifier)? {
        return Ok(&doc.bundle);
    }
    Err(BundleStoreError::SignatureRejected {
        detail: "no trusted key produced a valid signature over the bundle".to_owned(),
    })
}

fn any_trusted_signature_validates(
    doc: &SignedPolicyBundleDoc,
    verifier: &Ed25519ChainVerifier,
) -> Result<bool, BundleStoreError> {
    let inner = doc.bundle.as_bytes();
    for sig in &doc.signatures {
        match verifier.verify(&sig.key_id, inner, &sig.signature_hex) {
            Ok(()) => return Ok(true),
            Err(DigestChainError::UnknownKeyId(_) | DigestChainError::SignatureInvalid { .. }) => {}
            Err(other) => {
                // Malformed signature hex (or any other structural fault) is a
                // hard reject — a well-formed signed bundle never carries one.
                return Err(BundleStoreError::SignatureRejected {
                    detail: format!("signature for key_id {:?} unusable: {other}", sig.key_id),
                });
            }
        }
    }
    Ok(false)
}

fn parse_bundle(raw: &str) -> Result<PolicyBundle, BundleStoreError> {
    let bundle: PolicyBundle =
        serde_json::from_str(raw).map_err(|e| BundleStoreError::Malformed {
            detail: e.to_string(),
        })?;
    // serde(transparent) deserialization bypasses PolicyVersion::new's
    // opaque-token invariants; re-run them so a malformed version token can
    // never become a serving bundle (it would corrupt zookie comparisons and
    // decision-cache keys downstream).
    PolicyVersion::new(bundle.version.as_str()).map_err(|violations| {
        BundleStoreError::Malformed {
            detail: format!("bundle version token rejected: {violations:?}"),
        }
    })?;
    Ok(bundle)
}

fn parse_signed_bundle(
    raw: &str,
    verifier: &Ed25519ChainVerifier,
) -> Result<PolicyBundle, BundleStoreError> {
    let doc: SignedPolicyBundleDoc =
        serde_json::from_str(raw).map_err(|e| BundleStoreError::Malformed {
            detail: format!("signed bundle envelope malformed: {e}"),
        })?;
    let verified_inner = verify_envelope(&doc, verifier)?;
    parse_bundle(verified_inner)
}

impl PolicyBundleStore for FilePolicyBundleStore {
    fn load(&self) -> Result<PolicyBundle, BundleStoreError> {
        let verifier = load_trust_anchor(&self.trust_dir)?;
        let raw =
            std::fs::read_to_string(&self.path).map_err(|e| BundleStoreError::Unavailable {
                detail: format!("cannot read {}: {e}", self.path.display()),
            })?;
        parse_signed_bundle(&raw, &verifier)
    }

    fn describe(&self) -> String {
        format!(
            "file:{} (trust:{})",
            self.path.display(),
            self.trust_dir.display()
        )
    }
}
