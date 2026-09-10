//! Signing backend abstraction: [`SigningBackend`] is the crypto boundary the CA
//! signs through (ADR-0561 D4). [`EcdsaP256Signer`] is the real ECDSA P-256
//! backend on `aws-lc-rs` (ADR-0506, NO ring); `InMemorySigner` is a modeled MAC
//! behind the non-default `modeled-crypto` feature, never linked in production.

use std::sync::Arc;

use aws_lc_rs::signature::{ECDSA_P256_SHA256_ASN1, UnparsedPublicKey};
use rcgen::{KeyPair, PKCS_ECDSA_P256_SHA256, PublicKeyData, SigningKey};

use crate::error::{Result, TrustError};
use crate::x509::hex_encode;

/// Length of an uncompressed SEC1 P-256 public point: `0x04 || X(32) || Y(32)`.
const P256_UNCOMPRESSED_POINT_LEN: usize = 65;

/// A pluggable signing backend over an ECDSA/Ed25519 (or modeled) private key.
pub trait SigningBackend {
    /// Produce a signature over `tbs` (the to-be-signed bytes).
    fn sign(&self, tbs: &[u8]) -> Vec<u8>;

    /// Verify `signature` over `tbs` against this backend's public identity.
    fn verify(&self, tbs: &[u8], signature: &[u8]) -> bool;

    /// A stable identifier of the public half, binding a certificate to its CA.
    fn key_id(&self) -> String;
}

/// Deterministic in-memory signer: an FNV-1a keyed hash over the private key
/// concatenated with the message. A *modeled* backend, not a weak real one:
/// [`InMemorySigner::from_seed`] makes the private key equal the seed bytes, so
/// anyone knowing the seed forges any signature it accepts — yet it satisfies
/// the [`SigningBackend`] bound [`crate::ca::CertificateAuthority::bootstrap`]
/// takes. Hence the non-default `modeled-crypto` feature: production cannot link it.
// Gate below the `derive` so it is the line immediately above the item, which is
// what `token::tests` asserts.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg(any(test, feature = "modeled-crypto"))]
pub struct InMemorySigner {
    private_key: Vec<u8>,
}

#[cfg(any(test, feature = "modeled-crypto"))]
impl InMemorySigner {
    /// Construct from raw private key bytes.
    pub fn new(private_key: impl Into<Vec<u8>>) -> Self {
        InMemorySigner {
            private_key: private_key.into(),
        }
    }

    /// Derive a signer deterministically from a textual seed.
    pub fn from_seed(seed: &str) -> Self {
        InMemorySigner {
            private_key: seed.as_bytes().to_vec(),
        }
    }

    fn mac(&self, tbs: &[u8]) -> [u8; 8] {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in self
            .private_key
            .iter()
            .chain(b"\x00sig\x00")
            .chain(tbs.iter())
        {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash.to_be_bytes()
    }
}

#[cfg(any(test, feature = "modeled-crypto"))]
impl SigningBackend for InMemorySigner {
    fn sign(&self, tbs: &[u8]) -> Vec<u8> {
        self.mac(tbs).to_vec()
    }

    fn verify(&self, tbs: &[u8], signature: &[u8]) -> bool {
        let expected = self.mac(tbs);
        signature.len() == expected.len()
            && signature.iter().zip(expected.iter()).all(|(a, b)| a == b)
    }

    fn key_id(&self) -> String {
        let mut hash: u64 = 0x8422_2325_cbf2_9ce4;
        for &b in &self.private_key {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hex_encode(&hash.to_be_bytes())
    }
}

/// The REAL ECDSA P-256 signing backend: an `rcgen` [`KeyPair`] on `aws-lc-rs`
/// (ADR-0506, NO ring) producing ASN.1/DER ECDSA-with-SHA-256 signatures. The
/// key sits behind an [`Arc`] so the CA, the [`crate::bundle::TrustBundle`] and a
/// verification anchor share it without re-serialising. [`crate::der`] issues the
/// real X.509 leaf from the SAME key, so leaf and `sign`/`verify` agree.
#[derive(Clone)]
pub struct EcdsaP256Signer {
    key: Arc<KeyPair>,
    spki_der: Arc<Vec<u8>>,
}

impl core::fmt::Debug for EcdsaP256Signer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Never render private key material; surface only the stable public id.
        f.debug_struct("EcdsaP256Signer")
            .field("key_id", &self.key_id())
            .finish()
    }
}

impl EcdsaP256Signer {
    /// Generate a fresh random ECDSA P-256 signer (AWS-LC entropy).
    ///
    /// # Errors
    /// [`TrustError`] if key generation fails (entropy/backend error).
    pub fn generate() -> Result<Self> {
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
            .map_err(|e| TrustError::Other(format!("ECDSA P-256 keygen failed: {e}")))?;
        Ok(Self::from_key_pair(key))
    }

    /// Reconstruct a signer from a PKCS#8 DER produced by [`EcdsaP256Signer::private_key_der`].
    ///
    /// # Errors
    /// [`TrustError`] if the DER is not a valid ECDSA P-256 PKCS#8 key.
    pub fn from_pkcs8_der(pkcs8_der: &[u8]) -> Result<Self> {
        let key = KeyPair::try_from(pkcs8_der)
            .map_err(|e| TrustError::Other(format!("invalid ECDSA P-256 PKCS#8 key: {e}")))?;
        Ok(Self::from_key_pair(key))
    }

    fn from_key_pair(key: KeyPair) -> Self {
        let spki_der = key.subject_public_key_info();
        Self {
            key: Arc::new(key),
            spki_der: Arc::new(spki_der),
        }
    }

    /// The PKCS#8 DER of the private key, so the CA can persist its signing key.
    #[must_use]
    pub fn private_key_der(&self) -> Vec<u8> {
        self.key.serialize_der()
    }

    /// The SubjectPublicKeyInfo DER of the public half (a certificate's SPKI).
    #[must_use]
    pub fn public_key_spki_der(&self) -> Vec<u8> {
        self.spki_der.as_ref().clone()
    }

    /// Borrow the underlying `rcgen` key pair (used by [`crate::der`] issuance).
    #[must_use]
    pub fn key_pair(&self) -> &KeyPair {
        &self.key
    }

    /// The uncompressed SEC1 point (`0x04 || X || Y`) in the SPKI tail; `None`
    /// if the SPKI is too short to contain one.
    fn public_point(&self) -> Option<&[u8]> {
        let spki = self.spki_der.as_ref();
        spki.len()
            .checked_sub(P256_UNCOMPRESSED_POINT_LEN)
            .map(|start| &spki[start..])
            .filter(|point| point.first() == Some(&0x04))
    }
}

impl SigningBackend for EcdsaP256Signer {
    fn sign(&self, tbs: &[u8]) -> Vec<u8> {
        // The trait is infallible by shape; a signing error (never expected for an
        // in-memory AWS-LC key) yields an empty signature, which
        // `Certificate::validate` rejects as unsigned — fail-closed, not a panic.
        self.key.sign(tbs).unwrap_or_default()
    }

    fn verify(&self, tbs: &[u8], signature: &[u8]) -> bool {
        let Some(point) = self.public_point() else {
            return false;
        };
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_ASN1, point)
            .verify(tbs, signature)
            .is_ok()
    }

    fn key_id(&self) -> String {
        // One-way FNV-1a over the real SPKI: stable across clones, leaks no key.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in self.spki_der.as_ref() {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hex_encode(&hash.to_be_bytes())
    }
}

/// Verify a signature, returning a [`TrustError`] rather than a bool.
pub fn verify_or_err(backend: &dyn SigningBackend, tbs: &[u8], signature: &[u8]) -> Result<()> {
    if backend.verify(tbs, signature) {
        Ok(())
    } else {
        Err(TrustError::verification_failed(
            "signature does not verify against issuer key",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_then_verify_round_trips() {
        let s = InMemorySigner::from_seed("ca-key");
        let sig = s.sign(b"hello");
        assert!(s.verify(b"hello", &sig));
    }

    #[test]
    fn wrong_key_does_not_verify() {
        let signer = InMemorySigner::from_seed("ca-key");
        let attacker = InMemorySigner::from_seed("other-key");
        let sig = signer.sign(b"payload");
        assert!(!attacker.verify(b"payload", &sig));
    }

    #[test]
    fn tampered_message_fails() {
        let s = InMemorySigner::from_seed("ca-key");
        let sig = s.sign(b"payload");
        assert!(!s.verify(b"PAYLOAD", &sig));
        assert!(verify_or_err(&s, b"PAYLOAD", &sig).is_err());
    }

    #[test]
    fn key_id_is_stable_and_hides_key() {
        let s = InMemorySigner::from_seed("ca-key");
        assert_eq!(s.key_id(), InMemorySigner::from_seed("ca-key").key_id());
        assert_ne!(s.key_id(), InMemorySigner::from_seed("ca-key2").key_id());
    }

    #[test]
    fn ecdsa_sign_then_verify_round_trips() {
        let s = EcdsaP256Signer::generate().unwrap();
        let sig = s.sign(b"hello-real-crypto");
        assert!(sig.len() > 8);
        assert!(s.verify(b"hello-real-crypto", &sig));
    }

    #[test]
    fn ecdsa_wrong_key_does_not_verify() {
        let signer = EcdsaP256Signer::generate().unwrap();
        let attacker = EcdsaP256Signer::generate().unwrap();
        let sig = signer.sign(b"payload");
        assert!(!attacker.verify(b"payload", &sig));
    }

    #[test]
    fn ecdsa_tampered_message_fails() {
        let s = EcdsaP256Signer::generate().unwrap();
        let sig = s.sign(b"payload");
        assert!(!s.verify(b"PAYLOAD", &sig));
        assert!(verify_or_err(&s, b"PAYLOAD", &sig).is_err());
    }

    #[test]
    fn ecdsa_key_id_stable_across_clone_and_reload() {
        let s = EcdsaP256Signer::generate().unwrap();
        assert_eq!(s.key_id(), s.clone().key_id());
        let reloaded = EcdsaP256Signer::from_pkcs8_der(&s.private_key_der()).unwrap();
        assert_eq!(s.key_id(), reloaded.key_id());
        let sig = s.sign(b"x");
        assert!(reloaded.verify(b"x", &sig));
    }

    #[test]
    fn ecdsa_two_keys_have_distinct_ids() {
        let a = EcdsaP256Signer::generate().unwrap();
        let b = EcdsaP256Signer::generate().unwrap();
        assert_ne!(a.key_id(), b.key_id());
    }

    #[test]
    fn ecdsa_public_spki_is_real_and_nonempty() {
        let s = EcdsaP256Signer::generate().unwrap();
        let spki = s.public_key_spki_der();
        assert!(spki.len() >= P256_UNCOMPRESSED_POINT_LEN);
        assert_eq!(spki[spki.len() - P256_UNCOMPRESSED_POINT_LEN], 0x04);
    }
}
