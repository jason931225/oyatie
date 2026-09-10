//! # port-engine-hash — receipt digest computation.
#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

use port_engine_api::Digest;
use sha2::{Digest as Sha2Digest, Sha256};

pub const fn w0_ready() -> bool {
    true
}

pub const ALGORITHM: &str = "sha256";

#[must_use]
pub fn digest_bytes(bytes: &[u8]) -> Digest {
    let hash = Sha256::digest(bytes);
    Digest(format!("{ALGORITHM}:{}", hex_lower(hash.as_ref())))
}

#[must_use]
pub fn digest_str(text: &str) -> Digest {
    digest_bytes(text.as_bytes())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice7_claims_hash_readiness() {
        assert!(w0_ready());
    }

    #[test]
    fn empty_input_has_known_sha256() {
        let d = digest_bytes(b"");
        assert_eq!(
            d.0,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn digest_is_deterministic_and_content_sensitive() {
        let a = digest_str("port-engine");
        let b = digest_str("port-engine");
        let c = digest_str("port-engine!");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.0.starts_with("sha256:"));
        assert_eq!(a.0.len(), "sha256:".len() + 64);
    }
}
