#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
)]

use audit_emission_domain::{CanonicalEnvelope, EmissionDomainError, Fingerprinter};
use audit_emission_kernel::ChainCoordinate;

/// A `Fingerprinter` that always agrees with whatever the caller claims.
/// Used by tests that only care about validation ordering, not fingerprint
/// mismatch behavior.
struct AlwaysAgrees;

impl Fingerprinter for AlwaysAgrees {
    fn fingerprint(&self, _preimage: &[u8]) -> String {
        "unused".to_string()
    }
}

fn valid_coordinate() -> ChainCoordinate {
    ChainCoordinate {
        pack: "pack-kr".to_string(),
        tenant_partition: "tenant-alpha".to_string(),
        period: "2026-02-20".to_string(),
    }
}

#[test]
fn empty_event_id_is_rejected() {
    let err = CanonicalEnvelope::build(valid_coordinate(), "", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyEventId);
}

#[test]
fn whitespace_only_event_id_is_rejected() {
    let err = CanonicalEnvelope::build(
        valid_coordinate(),
        "   \t\n  ",
        "digest",
        "unused",
        &AlwaysAgrees,
    )
    .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyEventId);
}

#[test]
fn empty_pack_is_rejected() {
    let mut coordinate = valid_coordinate();
    coordinate.pack = String::new();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPack);
}

#[test]
fn whitespace_only_pack_is_rejected() {
    let mut coordinate = valid_coordinate();
    coordinate.pack = "   ".to_string();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPack);
}

#[test]
fn empty_tenant_partition_is_rejected() {
    let mut coordinate = valid_coordinate();
    coordinate.tenant_partition = String::new();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyTenantPartition);
}

#[test]
fn whitespace_only_tenant_partition_is_rejected() {
    let mut coordinate = valid_coordinate();
    coordinate.tenant_partition = "\t".to_string();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyTenantPartition);
}

#[test]
fn empty_period_is_rejected() {
    let mut coordinate = valid_coordinate();
    coordinate.period = String::new();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPeriod);
}

#[test]
fn malformed_period_is_rejected() {
    let mut coordinate = valid_coordinate();
    coordinate.period = "2026/02/20".to_string();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(
        err,
        EmissionDomainError::MalformedPeriod {
            period: "2026/02/20".to_string()
        }
    );
}

#[test]
fn empty_payload_digest_is_rejected() {
    let err = CanonicalEnvelope::build(valid_coordinate(), "evt-1", "", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPayloadDigest);
}

#[test]
fn whitespace_only_payload_digest_is_rejected() {
    let err = CanonicalEnvelope::build(valid_coordinate(), "evt-1", "  ", "unused", &AlwaysAgrees)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPayloadDigest);
}

#[test]
fn valid_inputs_with_agreeing_fingerprinter_build_successfully() {
    let envelope = CanonicalEnvelope::build(
        valid_coordinate(),
        "evt-1",
        "digest-bytes",
        "unused",
        &AlwaysAgrees,
    )
    .expect("all fields valid, fingerprinter agrees");
    assert_eq!(envelope.event_id(), "evt-1");
    assert_eq!(envelope.payload_digest(), "digest-bytes");
    assert_eq!(envelope.coordinate(), &valid_coordinate());
    assert_eq!(envelope.fingerprint(), "unused");
}

#[test]
fn validation_runs_before_fingerprint_verification() {
    // A `Fingerprinter` that panics if it is ever called: proves that a
    // malformed coordinate is rejected before any hashing work happens.
    struct PanicsIfCalled;
    impl Fingerprinter for PanicsIfCalled {
        fn fingerprint(&self, _preimage: &[u8]) -> String {
            panic!("fingerprinter must not be invoked when validation fails first");
        }
    }

    let mut coordinate = valid_coordinate();
    coordinate.pack = String::new();
    let err = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &PanicsIfCalled)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyPack);
}

#[test]
fn empty_claimed_fingerprint_is_rejected_without_invoking_the_fingerprinter() {
    // Closes the fail-open gap where a degraded `Fingerprinter` adapter
    // returns `""` on error and a caller separately claims `""`: this must
    // be rejected before the port is ever invoked, not accepted because the
    // two empty strings happen to agree.
    struct PanicsIfCalled;
    impl Fingerprinter for PanicsIfCalled {
        fn fingerprint(&self, _preimage: &[u8]) -> String {
            panic!("fingerprinter must not be invoked for an empty claimed fingerprint");
        }
    }

    let err = CanonicalEnvelope::build(valid_coordinate(), "evt-1", "digest", "", &PanicsIfCalled)
        .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyFingerprint);
}

#[test]
fn whitespace_only_claimed_fingerprint_is_rejected() {
    let err = CanonicalEnvelope::build(
        valid_coordinate(),
        "evt-1",
        "digest",
        "   \t  ",
        &AlwaysAgrees,
    )
    .unwrap_err();
    assert_eq!(err, EmissionDomainError::EmptyFingerprint);
}

#[test]
fn leading_and_trailing_whitespace_on_tenant_partition_is_trimmed_not_distinguished() {
    let spellings = [
        "tenant-alpha",
        " tenant-alpha",
        "tenant-alpha\n",
        "\ttenant-alpha ",
    ];
    for spelling in spellings {
        let mut coordinate = valid_coordinate();
        coordinate.tenant_partition = spelling.to_string();
        let envelope =
            CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
                .expect("whitespace-padded tenant partition trims to a valid value");
        assert_eq!(envelope.coordinate().tenant_partition, "tenant-alpha");
    }
}

#[test]
fn leading_and_trailing_whitespace_on_pack_is_trimmed_not_distinguished() {
    let mut coordinate = valid_coordinate();
    coordinate.pack = "  pack-kr\t".to_string();
    let envelope = CanonicalEnvelope::build(coordinate, "evt-1", "digest", "unused", &AlwaysAgrees)
        .expect("whitespace-padded pack trims to a valid value");
    assert_eq!(envelope.coordinate().pack, "pack-kr");
}

#[test]
fn leading_and_trailing_whitespace_on_event_id_is_trimmed_not_distinguished() {
    let envelope = CanonicalEnvelope::build(
        valid_coordinate(),
        " evt-1 ",
        "digest",
        "unused",
        &AlwaysAgrees,
    )
    .expect("whitespace-padded event id trims to a valid value");
    assert_eq!(envelope.event_id(), "evt-1");
}

#[test]
fn leading_and_trailing_whitespace_on_payload_digest_is_trimmed_not_distinguished() {
    let envelope = CanonicalEnvelope::build(
        valid_coordinate(),
        "evt-1",
        "  digest-bytes\n",
        "unused",
        &AlwaysAgrees,
    )
    .expect("whitespace-padded payload digest trims to a valid value");
    assert_eq!(envelope.payload_digest(), "digest-bytes");
}

#[test]
fn trimmed_and_untrimmed_spellings_of_the_same_tenant_produce_the_same_fingerprint() {
    // The load-bearing property behind trimming: two envelopes that differ
    // only in incidental whitespace must fingerprint identically, so no
    // whitespace spelling can create a shadow partition that a
    // canonical-name-keyed query, seal, or retention sweep would miss.
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
    let fingerprinter = Fnv1aFingerprinter;

    let canonical_fingerprint = fingerprinter.fingerprint(
        &audit_emission_domain::canonical_preimage(&valid_coordinate(), "evt-1", "digest"),
    );

    let clean = CanonicalEnvelope::build(
        valid_coordinate(),
        "evt-1",
        "digest",
        canonical_fingerprint.clone(),
        &fingerprinter,
    )
    .expect("clean spelling builds");

    let mut padded_coordinate = valid_coordinate();
    padded_coordinate.tenant_partition = "  tenant-alpha  ".to_string();
    let padded = CanonicalEnvelope::build(
        padded_coordinate,
        "evt-1",
        "digest",
        canonical_fingerprint,
        &fingerprinter,
    )
    .expect("padded spelling trims to the same coordinate and also builds");

    assert_eq!(clean.fingerprint(), padded.fingerprint());
    assert_eq!(clean.coordinate(), padded.coordinate());
}
