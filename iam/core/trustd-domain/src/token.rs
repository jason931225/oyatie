//! Cluster join-token verification.

use crate::error::{Result, TrustError};

/// The cluster-wide bootstrap join token configured on every node. Mirrors the
/// `.cluster.token` field of the Talos machine config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinToken {
    value: String,
}

impl JoinToken {
    /// Minimum length Talos-style tokens use (`<id>.<secret>`, base62-ish).
    pub const MIN_LEN: usize = 16;

    /// Construct and validate a join token.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        Self::validate_format(&value)?;
        Ok(JoinToken { value })
    }

    /// Validate the textual format: `<id>.<secret>`, both non-empty, total
    /// length at least [`JoinToken::MIN_LEN`], no whitespace.
    pub fn validate_format(value: &str) -> Result<()> {
        if value.len() < Self::MIN_LEN {
            return Err(TrustError::invalid("join token too short"));
        }
        if value.chars().any(char::is_whitespace) {
            return Err(TrustError::invalid("join token contains whitespace"));
        }
        let mut parts = value.split('.');
        let id = parts.next().unwrap_or("");
        let secret = parts.next().unwrap_or("");
        if id.is_empty() || secret.is_empty() || parts.next().is_some() {
            return Err(TrustError::invalid("join token must be '<id>.<secret>'"));
        }
        Ok(())
    }

    /// Deterministically derive a join token from a cluster seed.
    ///
    /// HAZARD: fully predictable from `cluster_seed`, so it sits behind the
    /// non-default `modeled-crypto` feature no production target enables. Real
    /// cluster tokens come from the machine config via [`JoinToken::new`].
    #[cfg(any(test, feature = "modeled-crypto"))]
    pub fn derive(cluster_seed: &[u8]) -> Self {
        let id = base36(fnv64(cluster_seed, 0x01));
        let secret = format!(
            "{}{}",
            base36(fnv64(cluster_seed, 0x02)),
            base36(fnv64(cluster_seed, 0x03))
        );
        // `id`/`secret` are always non-empty base36, so the format is valid.
        JoinToken {
            value: format!("{id}.{secret}"),
        }
    }

    /// The token-id portion (before the dot), which is non-secret and can be
    /// logged.
    pub fn id(&self) -> &str {
        self.value.split('.').next().unwrap_or("")
    }

    /// The secret portion (after the dot). Secret — do not log.
    pub fn secret(&self) -> &str {
        self.value.split('.').nth(1).unwrap_or("")
    }

    /// A non-secret fingerprint of the whole token, safe to log, that still lets
    /// operators correlate which token a node presented.
    pub fn fingerprint(&self) -> String {
        crate::x509::hex_encode(&fnv64(self.value.as_bytes(), 0x10).to_be_bytes())
    }

    /// Constant-time equality against a presented token string. Returns
    /// `Ok(())` only on an exact match.
    pub fn verify_presented(&self, presented: &str) -> Result<()> {
        if presented.is_empty() {
            return Err(TrustError::token_mismatch("no join token presented"));
        }
        if constant_time_eq(self.value.as_bytes(), presented.as_bytes()) {
            Ok(())
        } else {
            Err(TrustError::token_mismatch(
                "join token does not match cluster token",
            ))
        }
    }

    /// Borrow the raw token value (secret — do not log).
    pub fn expose(&self) -> &str {
        &self.value
    }
}

/// FNV-1a 64-bit over `data` mixed with a domain-separation `salt`.
fn fnv64(data: &[u8], salt: u64) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ salt;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

/// Lowercase base36 encoding of a u64, always non-empty.
fn base36(mut n: u64) -> String {
    const ALPHABET: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut buf = Vec::new();
    while n > 0 {
        buf.push(ALPHABET[(n % 36) as usize]);
        n /= 36;
    }
    buf.reverse();
    String::from_utf8(buf).expect("base36 alphabet is ASCII")
}

/// Length-independent, value-constant-time byte comparison.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_token_parses() {
        let t = JoinToken::new("abcd1234.supersecret").unwrap();
        assert_eq!(t.id(), "abcd1234");
    }

    #[test]
    fn rejects_malformed_tokens() {
        assert!(JoinToken::new("short").is_err());
        assert!(JoinToken::new("nodotherexxxxxxxxxx").is_err());
        assert!(JoinToken::new("has .whitespacexxxx").is_err());
        assert!(JoinToken::new("id.sec.extraaaaaaaa").is_err());
    }

    #[test]
    fn verify_matches_only_exact() {
        let t = JoinToken::new("abcd1234.supersecret").unwrap();
        assert!(t.verify_presented("abcd1234.supersecret").is_ok());
        assert_eq!(
            t.verify_presented("abcd1234.wrong").unwrap_err().kind(),
            "token_mismatch"
        );
        assert_eq!(t.verify_presented("").unwrap_err().kind(), "token_mismatch");
    }

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn derive_is_deterministic_and_valid() {
        let a = JoinToken::derive(b"cluster-uuid-1234");
        let b = JoinToken::derive(b"cluster-uuid-1234");
        assert_eq!(a, b);
        JoinToken::validate_format(a.expose()).unwrap();
        assert!(a.verify_presented(a.expose()).is_ok());
    }

    #[test]
    fn derive_differs_per_seed() {
        let a = JoinToken::derive(b"cluster-a");
        let b = JoinToken::derive(b"cluster-b");
        assert_ne!(a, b);
        assert!(a.verify_presented(b.expose()).is_err());
    }

    #[test]
    fn id_secret_split() {
        let t = JoinToken::new("clusterid.thesecretpart").unwrap();
        assert_eq!(t.id(), "clusterid");
        assert_eq!(t.secret(), "thesecretpart");
    }

    #[test]
    fn fingerprint_is_stable_and_distinct() {
        let a = JoinToken::new("clusterid.secretone1").unwrap();
        let b = JoinToken::new("clusterid.secrettwo2").unwrap();
        assert_eq!(
            a.fingerprint(),
            JoinToken::new("clusterid.secretone1")
                .unwrap()
                .fingerprint()
        );
        assert_ne!(a.fingerprint(), b.fingerprint());
        assert!(!a.fingerprint().contains("secretone"));
    }

    /// Every constructor that mints *modeled* key material must stay behind the
    /// non-default `modeled-crypto` gate. Source-text assertion: a `cfg` cannot
    /// be observed from inside a build where it is enabled, and the gate must be
    /// caught when COMMENTED OUT as well as when deleted — see [`gated`].
    #[test]
    fn modeled_crypto_constructors_stay_behind_the_gate() {
        let required: [(&str, &str); 5] = [
            (
                include_str!("token.rs"),
                "pub fn derive(cluster_seed: &[u8]) -> Self {",
            ),
            (
                include_str!("x509.rs"),
                "pub fn from_seed(seed: &[u8]) -> Self {",
            ),
            (include_str!("signer.rs"), "pub struct InMemorySigner {"),
            (include_str!("signer.rs"), "impl InMemorySigner {"),
            (
                include_str!("signer.rs"),
                "impl SigningBackend for InMemorySigner {",
            ),
        ];

        for (src, signature) in required {
            assert!(
                gated(src, signature),
                "`{signature}` must be immediately preceded by {GATE}"
            );
        }
    }

    /// The crate-root re-export must carry the gate too: a `pub use` of a
    /// gated item is a compile error off-feature, so an un-gated re-export
    /// would force whoever hit it to delete the gate rather than the usage.
    #[test]
    fn modeled_signer_reexport_is_gated() {
        assert!(
            gated(include_str!("lib.rs"), "pub use signer::InMemorySigner;"),
            "the InMemorySigner re-export must be immediately preceded by {GATE}"
        );
    }

    const GATE: &str = "#[cfg(any(test, feature = \"modeled-crypto\"))]";

    /// True if some occurrence of `signature` in `src` is immediately preceded
    /// by a LINE that starts with [`GATE`].
    ///
    /// Prefix-matching the preceding line, not suffix-matching the text before
    /// the signature: `ends_with(GATE)` is also satisfied by a COMMENTED-OUT
    /// gate, which is the quiet way the barrier gets removed.
    fn gated(src: &str, signature: &str) -> bool {
        src.match_indices(signature).any(|(i, _)| {
            src[..i]
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .is_some_and(|l| l.trim_start().starts_with(GATE))
        })
    }

    #[test]
    fn base36_basic() {
        assert_eq!(base36(0), "0");
        assert_eq!(base36(35), "z");
        assert_eq!(base36(36), "10");
    }
}
