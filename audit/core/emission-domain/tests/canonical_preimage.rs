//! Coverage for `canonical_preimage`'s injection-proofness: the encoding
//! must be unambiguous, so no two distinct field tuples can collide onto the
//! same byte string. A naive separator-joined encoding (e.g. joining fields
//! with `,`) fails the boundary-shift case below and is not acceptable.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "assertion failure IS the test signal; ADR-0083 Tier 3 cfg(test) exemption"
)]

use audit_emission_domain::canonical_preimage;
use audit_emission_kernel::ChainCoordinate;

fn coordinate(pack: &str, tenant_partition: &str, period: &str) -> ChainCoordinate {
    ChainCoordinate {
        pack: pack.to_string(),
        tenant_partition: tenant_partition.to_string(),
        period: period.to_string(),
    }
}

#[test]
fn boundary_shift_between_pack_and_tenant_partition_changes_the_preimage() {
    let a = canonical_preimage(&coordinate("ab", "c", "2026-02-20"), "evt-1", "digest");
    let b = canonical_preimage(&coordinate("a", "bc", "2026-02-20"), "evt-1", "digest");
    assert_ne!(
        a, b,
        "distinct (pack, tenant_partition) tuples must not collide"
    );
}

#[test]
fn boundary_shift_between_tenant_partition_and_period_changes_the_preimage() {
    let a = canonical_preimage(
        &coordinate("pack-kr", "xy", "z-2026-02-20"),
        "evt-1",
        "digest",
    );
    let b = canonical_preimage(
        &coordinate("pack-kr", "xyz", "-2026-02-20"),
        "evt-1",
        "digest",
    );
    assert_ne!(a, b);
}

#[test]
fn boundary_shift_between_event_id_and_payload_digest_changes_the_preimage() {
    let a = canonical_preimage(&coordinate("pack-kr", "t", "2026-02-20"), "ab", "c");
    let b = canonical_preimage(&coordinate("pack-kr", "t", "2026-02-20"), "a", "bc");
    assert_ne!(a, b);
}

#[test]
fn same_inputs_produce_the_same_preimage() {
    let a = canonical_preimage(
        &coordinate("pack-kr", "tenant-alpha", "2026-02-20"),
        "evt-1",
        "digest",
    );
    let b = canonical_preimage(
        &coordinate("pack-kr", "tenant-alpha", "2026-02-20"),
        "evt-1",
        "digest",
    );
    assert_eq!(a, b);
}

#[test]
fn changing_any_single_field_changes_the_preimage() {
    let base = canonical_preimage(
        &coordinate("pack-kr", "tenant-alpha", "2026-02-20"),
        "evt-1",
        "digest",
    );

    let different_pack = canonical_preimage(
        &coordinate("pack-eu", "tenant-alpha", "2026-02-20"),
        "evt-1",
        "digest",
    );
    let different_tenant_partition = canonical_preimage(
        &coordinate("pack-kr", "tenant-beta", "2026-02-20"),
        "evt-1",
        "digest",
    );
    let different_period = canonical_preimage(
        &coordinate("pack-kr", "tenant-alpha", "2026-02-21"),
        "evt-1",
        "digest",
    );
    let different_event_id = canonical_preimage(
        &coordinate("pack-kr", "tenant-alpha", "2026-02-20"),
        "evt-2",
        "digest",
    );
    let different_payload_digest = canonical_preimage(
        &coordinate("pack-kr", "tenant-alpha", "2026-02-20"),
        "evt-1",
        "digest-2",
    );

    assert_ne!(base, different_pack);
    assert_ne!(base, different_tenant_partition);
    assert_ne!(base, different_period);
    assert_ne!(base, different_event_id);
    assert_ne!(base, different_payload_digest);
}

#[test]
fn empty_fields_do_not_collide_with_absent_fields() {
    // An empty tenant_partition sandwiched between two non-empty fields must
    // not encode identically to a shifted split that "swallows" it.
    let a = canonical_preimage(&coordinate("pack", "", "2026-02-20"), "evt", "digest");
    let b = canonical_preimage(&coordinate("pack", "2026-02-20", ""), "evt", "digest");
    assert_ne!(a, b);
}
