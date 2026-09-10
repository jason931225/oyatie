use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlobRefError {
    /// Only the `sha256:` scheme is defined.
    UnknownScheme,
    /// The digest is not exactly 64 lowercase hex characters.
    MalformedDigest,
}

/// The address of a blob: the SHA-256 of its bytes.
///
/// Constructed only from content ([`BlobRef::for_bytes`]) or by parsing a
/// previously rendered reference — never assembled from an arbitrary string,
/// so an address always names bytes something once actually hashed.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlobRef {
    digest_hex: String,
}

impl BlobRef {
    pub fn for_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut digest_hex = String::with_capacity(64);
        for byte in digest {
            use std::fmt::Write;
            let _ = write!(digest_hex, "{byte:02x}");
        }
        Self { digest_hex }
    }

    /// Parse the `sha256:<64 hex>` form produced by [`std::fmt::Display`].
    pub fn parse(rendered: &str) -> Result<Self, BlobRefError> {
        let Some(digest_hex) = rendered.strip_prefix("sha256:") else {
            return Err(BlobRefError::UnknownScheme);
        };
        if digest_hex.len() != 64
            || !digest_hex
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err(BlobRefError::MalformedDigest);
        }
        Ok(Self {
            digest_hex: digest_hex.to_owned(),
        })
    }

    /// The lowercase hex digest, without the scheme.
    pub fn digest_hex(&self) -> &str {
        &self.digest_hex
    }
}

impl std::fmt::Display for BlobRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "sha256:{}", self.digest_hex)
    }
}
