//! Envelope-encryption key identifiers.
//!
//! `KekId` and `DekId` are the foundational identifiers for the P08-kms envelope
//! encryption substrate (AES-256-GCM, per-tenant DEK isolation). These types
//! live here — adjacent to `KmsKeyId` — as the merge-variant delta-1 backport
//! from the P08 impl-plan into the existing live `cloud-kms-domain` crate.
//!
//! Format invariants (enforced at construction):
//! - `KekId`: `"kek/"` prefix followed by a non-empty, `/`-free slug; e.g. `"kek/ten_abc123"`
//! - `DekId`: `"dek/"` prefix followed by a non-empty, `/`-free slug; e.g. `"dek/ten_abc123"`
//!
//! Neither type stores key material; they are opaque reference handles that the
//! adapter layer resolves against OCI Vault or the in-memory test adapter.

use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeKeyError {
    InvalidPrefix,
    EmptySlug,
    SlugContainsSlash,
}

impl fmt::Display for EnvelopeKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix => {
                f.write_str("envelope key id: invalid prefix (expected 'kek/' or 'dek/')")
            }
            Self::EmptySlug => f.write_str("envelope key id: slug portion must not be empty"),
            Self::SlugContainsSlash => f.write_str("envelope key id: slug must not contain '/'"),
        }
    }
}

const KEK_PREFIX: &str = "kek/";
const DEK_PREFIX: &str = "dek/";

/// Key-Encryption Key identifier.
///
/// Opaque handle for the tenant's master wrapping key stored in OCI Vault or
/// OpenBao. Never contains key bytes.
///
/// The `value` field is private to preserve the format invariant
/// (`"kek/<slug>"`). Use [`KekId::new`] to construct and [`KekId::value`] to
/// read the underlying string.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct KekId {
    value: String,
}

impl KekId {
    /// Construct a validated `KekId`.
    ///
    /// Accepts strings of the form `"kek/<slug>"` where `<slug>` is non-empty
    /// and contains no `/` characters.
    pub fn new(value: impl Into<String>) -> Result<Self, EnvelopeKeyError> {
        let value = value.into();
        validate_envelope_key_id(&value, KEK_PREFIX)?;
        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Return the slug portion (the part after `"kek/"`).
    ///
    /// Panics cannot occur here because `value` is private and every
    /// instance is validated through [`KekId::new`].
    pub fn slug(&self) -> &str {
        &self.value[KEK_PREFIX.len()..]
    }
}

impl fmt::Display for KekId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

/// Data-Encryption Key identifier.
///
/// Opaque handle for a per-tenant DEK (AES-256-GCM). The DEK bytes themselves
/// are wrapped by the tenant's `KekId` and stored in OCI Vault; this type is
/// the reference that the envelope header carries.
///
/// The `value` field is private to preserve the format invariant
/// (`"dek/<slug>"`). Use [`DekId::new`] to construct and [`DekId::value`] to
/// read the underlying string.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DekId {
    value: String,
}

impl DekId {
    /// Construct a validated `DekId`.
    ///
    /// Accepts strings of the form `"dek/<slug>"` where `<slug>` is non-empty
    /// and contains no `/` characters.
    pub fn new(value: impl Into<String>) -> Result<Self, EnvelopeKeyError> {
        let value = value.into();
        validate_envelope_key_id(&value, DEK_PREFIX)?;
        Ok(Self { value })
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Return the slug portion (the part after `"dek/"`).
    ///
    /// Panics cannot occur here because `value` is private and every
    /// instance is validated through [`DekId::new`].
    pub fn slug(&self) -> &str {
        &self.value[DEK_PREFIX.len()..]
    }
}

impl fmt::Display for DekId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

fn validate_envelope_key_id(value: &str, prefix: &str) -> Result<(), EnvelopeKeyError> {
    if !value.starts_with(prefix) {
        return Err(EnvelopeKeyError::InvalidPrefix);
    }
    let slug = &value[prefix.len()..];
    if slug.is_empty() {
        return Err(EnvelopeKeyError::EmptySlug);
    }
    if slug.contains('/') {
        return Err(EnvelopeKeyError::SlugContainsSlash);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kek_id_valid_round_trip() {
        let kek = KekId::new("kek/ten_abc123").unwrap();
        assert_eq!(kek.value(), "kek/ten_abc123");
        assert_eq!(kek.slug(), "ten_abc123");
        assert_eq!(kek.to_string(), "kek/ten_abc123");
    }

    #[test]
    fn kek_id_wrong_prefix_rejected() {
        let err = KekId::new("dek/ten_abc123").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::InvalidPrefix);
    }

    #[test]
    fn kek_id_empty_slug_rejected() {
        let err = KekId::new("kek/").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::EmptySlug);
    }

    #[test]
    fn kek_id_slug_with_slash_rejected() {
        let err = KekId::new("kek/ten_abc/nested").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::SlugContainsSlash);
    }

    #[test]
    fn kek_id_no_prefix_rejected() {
        let err = KekId::new("ten_abc123").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::InvalidPrefix);
    }

    #[test]
    fn kek_id_ordering() {
        let a = KekId::new("kek/aaa").unwrap();
        let b = KekId::new("kek/bbb").unwrap();
        assert!(a < b);
    }

    #[test]
    fn dek_id_valid_round_trip() {
        let dek = DekId::new("dek/ten_xyz789").unwrap();
        assert_eq!(dek.value(), "dek/ten_xyz789");
        assert_eq!(dek.slug(), "ten_xyz789");
        assert_eq!(dek.to_string(), "dek/ten_xyz789");
    }

    #[test]
    fn dek_id_wrong_prefix_rejected() {
        let err = DekId::new("kek/ten_xyz789").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::InvalidPrefix);
    }

    #[test]
    fn dek_id_empty_slug_rejected() {
        let err = DekId::new("dek/").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::EmptySlug);
    }

    #[test]
    fn dek_id_slug_with_slash_rejected() {
        let err = DekId::new("dek/ten_xyz/nested").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::SlugContainsSlash);
    }

    #[test]
    fn dek_id_no_prefix_rejected() {
        let err = DekId::new("ten_xyz789").unwrap_err();
        assert_eq!(err, EnvelopeKeyError::InvalidPrefix);
    }

    #[test]
    fn dek_id_ordering() {
        let a = DekId::new("dek/aaa").unwrap();
        let b = DekId::new("dek/bbb").unwrap();
        assert!(a < b);
    }

    #[test]
    fn error_display_invalid_prefix() {
        let msg = EnvelopeKeyError::InvalidPrefix.to_string();
        assert!(msg.contains("kek/") || msg.contains("dek/"));
    }

    #[test]
    fn error_display_empty_slug() {
        let msg = EnvelopeKeyError::EmptySlug.to_string();
        assert!(msg.contains("empty"));
    }

    #[test]
    fn error_display_slug_contains_slash() {
        let msg = EnvelopeKeyError::SlugContainsSlash.to_string();
        assert!(msg.contains("/"));
    }

    #[test]
    fn kek_and_dek_same_slug_are_distinct_types() {
        let kek = KekId::new("kek/ten_shared").unwrap();
        let dek = DekId::new("dek/ten_shared").unwrap();
        // Different prefix means different values — confirmed by value strings
        assert_ne!(kek.value(), dek.value());
        assert_eq!(kek.slug(), dek.slug());
    }
}
