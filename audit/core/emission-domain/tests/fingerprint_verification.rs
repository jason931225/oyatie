//! Coverage for the `Fingerprinter` port: the domain never computes a hash
//! itself, but it must genuinely verify a caller-supplied fingerprint
//! against the canonical preimage, and `FingerprintMismatch` must be
//! reachable whenever the two disagree.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
)]

use audit_emission_domain::{CanonicalEnvelope, EmissionDomainError, Fingerprinter};
use audit_emission_kernel::ChainCoordinate;

/// A deterministic, non-cryptographic FNV-1a-style fingerprinter built only
/// from `std`, standing in for a real hash adapter (e.g. a SHA-256 wrapper)
/// that would live outside this domain crate.
struct Fnv1aFingerprinter;

impl Fingerprinter for Fnv1aFingerprinter {
    fn fingerprint(&self, preimage: &[u8]) -> String {
        const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
        const PRIME: u64 = 0x100000001b3;
        let mut hash = OFFSET_BASIS;
        for byte in preimage {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
        format!("fnv1a:{hash:016x}")
    }
}

fn coordinate() -> ChainCoordinate {
    ChainCoordinate {
        pack: "pack-kr".to_string(),
        tenant_partition: "tenant-alpha".to_string(),
        period: "2026-02-20".to_string(),
    }
}

#[test]
fn matching_fingerprint_builds_successfully() {
    let fingerprinter = Fnv1aFingerprinter;
    let preimage =
        audit_emission_domain::canonical_preimage(&coordinate(), "evt-1", "digest-bytes");
    let correct_fingerprint = fingerprinter.fingerprint(&preimage);

    let envelope = CanonicalEnvelope::build(
        coordinate(),
        "evt-1",
        "digest-bytes",
        correct_fingerprint.clone(),
        &fingerprinter,
    )
    .expect("correct fingerprint must verify");
    assert_eq!(envelope.fingerprint(), correct_fingerprint);
}

#[test]
fn wrong_fingerprint_is_rejected_with_fingerprint_mismatch() {
    let fingerprinter = Fnv1aFingerprinter;
    let err = CanonicalEnvelope::build(
        coordinate(),
        "evt-1",
        "digest-bytes",
        "not-the-real-fingerprint",
        &fingerprinter,
    )
    .unwrap_err();
    assert_eq!(err, EmissionDomainError::FingerprintMismatch);
}

#[test]
fn fingerprint_is_sensitive_to_every_field_of_the_preimage() {
    let fingerprinter = Fnv1aFingerprinter;
    let preimage =
        audit_emission_domain::canonical_preimage(&coordinate(), "evt-1", "digest-bytes");
    let fingerprint_for_original = fingerprinter.fingerprint(&preimage);

    // Building with a *different* payload_digest but the *original*
    // fingerprint must fail: the claimed fingerprint no longer matches this
    // envelope's own preimage.
    let err = CanonicalEnvelope::build(
        coordinate(),
        "evt-1",
        "different-digest-bytes",
        fingerprint_for_original,
        &fingerprinter,
    )
    .unwrap_err();
    assert_eq!(err, EmissionDomainError::FingerprintMismatch);
}

#[test]
fn verify_accepts_an_untampered_envelope() {
    let fingerprinter = Fnv1aFingerprinter;
    let preimage =
        audit_emission_domain::canonical_preimage(&coordinate(), "evt-1", "digest-bytes");
    let fingerprint = fingerprinter.fingerprint(&preimage);

    let envelope = CanonicalEnvelope::build(
        coordinate(),
        "evt-1",
        "digest-bytes",
        fingerprint,
        &fingerprinter,
    )
    .expect("valid build");

    envelope
        .verify(&fingerprinter)
        .expect("freshly built envelope must re-verify");
}

#[test]
fn verify_rejects_when_recomputed_with_a_disagreeing_fingerprinter() {
    struct AlwaysReturns(&'static str);
    impl Fingerprinter for AlwaysReturns {
        fn fingerprint(&self, _preimage: &[u8]) -> String {
            self.0.to_string()
        }
    }

    let built_with = AlwaysReturns("fingerprint-v1");
    let envelope = CanonicalEnvelope::build(
        coordinate(),
        "evt-1",
        "digest-bytes",
        "fingerprint-v1",
        &built_with,
    )
    .expect("valid build");

    let rotated = AlwaysReturns("fingerprint-v2");
    let err = envelope.verify(&rotated).unwrap_err();
    assert_eq!(err, EmissionDomainError::FingerprintMismatch);

    // The original fingerprinter still agrees with itself.
    envelope
        .verify(&built_with)
        .expect("re-verifying with the original fingerprinter still agrees");
}
