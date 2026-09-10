use super::*;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

// ── ST1: Ipv4Cidr containment + overlap ──────────────────────────────────

#[test]
fn ipv4_cidr_contains_cidr_returns_true_when_child_is_fully_inside_parent() {
    let parent = Ipv4Cidr::new("10.0.0.0/8").unwrap();
    let child = Ipv4Cidr::new("10.1.2.0/24").unwrap();
    assert!(
        parent.contains_cidr(&child).unwrap(),
        "10.1.2.0/24 is inside 10.0.0.0/8"
    );
}

#[test]
fn ipv4_cidr_contains_cidr_returns_false_when_cidrs_are_disjoint() {
    let a = Ipv4Cidr::new("10.0.0.0/8").unwrap();
    let b = Ipv4Cidr::new("192.168.0.0/16").unwrap();
    assert!(
        !a.contains_cidr(&b).unwrap(),
        "10.0.0.0/8 and 192.168.0.0/16 are disjoint"
    );
}

#[test]
fn ipv4_cidr_contains_cidr_returns_true_for_equal_cidrs() {
    let a = Ipv4Cidr::new("10.0.0.0/16").unwrap();
    let b = Ipv4Cidr::new("10.0.0.0/16").unwrap();
    assert!(a.contains_cidr(&b).unwrap(), "a CIDR contains itself");
}

#[test]
fn ipv4_cidr_contains_cidr_returns_false_when_child_is_broader_than_parent() {
    let narrow = Ipv4Cidr::new("10.0.0.0/24").unwrap();
    let broad = Ipv4Cidr::new("10.0.0.0/16").unwrap();
    assert!(
        !narrow.contains_cidr(&broad).unwrap(),
        "a /24 does not contain a /16"
    );
}

#[test]
fn ipv4_cidr_overlaps_cidr_returns_true_for_partially_overlapping_ranges() {
    // 10.0.0.0/23 covers 10.0.0.0-10.0.1.255; 10.0.1.0/24 is inside it
    let a = Ipv4Cidr::new("10.0.0.0/23").unwrap();
    let b = Ipv4Cidr::new("10.0.1.0/24").unwrap();
    assert!(
        a.overlaps_cidr(&b).unwrap(),
        "10.0.0.0/23 and 10.0.1.0/24 overlap"
    );
}

#[test]
fn ipv4_cidr_overlaps_cidr_returns_false_for_disjoint_ranges() {
    let a = Ipv4Cidr::new("10.1.0.0/24").unwrap();
    let b = Ipv4Cidr::new("10.2.0.0/24").unwrap();
    assert!(
        !a.overlaps_cidr(&b).unwrap(),
        "10.1.0.0/24 and 10.2.0.0/24 do not overlap"
    );
}

#[test]
fn ipv4_cidr_overlaps_cidr_returns_true_for_equal_cidrs() {
    let a = Ipv4Cidr::new("172.16.0.0/12").unwrap();
    let b = Ipv4Cidr::new("172.16.0.0/12").unwrap();
    assert!(a.overlaps_cidr(&b).unwrap(), "equal CIDRs overlap");
}

#[test]
fn ipv4_cidr_contains_ip_returns_true_for_address_inside_prefix() {
    let cidr = Ipv4Cidr::new("10.42.0.0/16").unwrap();
    let addr = "10.42.1.100".parse::<Ipv4Addr>().unwrap();
    assert!(cidr.contains_ip(addr).unwrap());
}

#[test]
fn ipv4_cidr_contains_ip_returns_false_for_address_outside_prefix() {
    let cidr = Ipv4Cidr::new("10.42.0.0/16").unwrap();
    let addr = "10.99.1.1".parse::<Ipv4Addr>().unwrap();
    assert!(!cidr.contains_ip(addr).unwrap());
}

#[test]
fn ipv4_cidr_returns_invalid_cidr_prefix_for_malformed_input() {
    // Malformed CIDR stored directly (bypassing new() which would reject it)
    let bad = Ipv4Cidr {
        value: "not-a-cidr".to_string(),
    };
    let good = Ipv4Cidr::new("10.0.0.0/8").unwrap();
    assert_eq!(
        bad.contains_cidr(&good),
        Err(CloudNetworkError::InvalidCidrPrefix),
        "malformed self CIDR returns InvalidCidrPrefix"
    );
    assert_eq!(
        good.contains_cidr(&bad),
        Err(CloudNetworkError::InvalidCidrPrefix),
        "malformed other CIDR returns InvalidCidrPrefix"
    );
}

// ── ST1: Ipv6Cidr containment + overlap ──────────────────────────────────

#[test]
fn ipv6_cidr_contains_cidr_returns_true_when_child_is_fully_inside_parent() {
    let parent = Ipv6Cidr::new("2001:db8::/32").unwrap();
    let child = Ipv6Cidr::new("2001:db8:42::/56").unwrap();
    assert!(
        parent.contains_cidr(&child).unwrap(),
        "2001:db8:42::/56 is inside 2001:db8::/32"
    );
}

#[test]
fn ipv6_cidr_contains_cidr_returns_false_when_cidrs_are_disjoint() {
    let a = Ipv6Cidr::new("2001:db8::/32").unwrap();
    let b = Ipv6Cidr::new("fd00::/8").unwrap();
    assert!(
        !a.contains_cidr(&b).unwrap(),
        "2001:db8::/32 and fd00::/8 are disjoint"
    );
}

#[test]
fn ipv6_cidr_contains_cidr_returns_true_for_equal_cidrs() {
    let a = Ipv6Cidr::new("2001:db8:42::/48").unwrap();
    let b = Ipv6Cidr::new("2001:db8:42::/48").unwrap();
    assert!(a.contains_cidr(&b).unwrap(), "a CIDR contains itself");
}

#[test]
fn ipv6_cidr_contains_cidr_returns_false_when_child_is_broader_than_parent() {
    let narrow = Ipv6Cidr::new("2001:db8:42::/64").unwrap();
    let broad = Ipv6Cidr::new("2001:db8:42::/48").unwrap();
    assert!(
        !narrow.contains_cidr(&broad).unwrap(),
        "/64 does not contain /48"
    );
}

#[test]
fn ipv6_cidr_overlaps_cidr_returns_true_for_overlapping_ranges() {
    let a = Ipv6Cidr::new("2001:db8::/32").unwrap();
    let b = Ipv6Cidr::new("2001:db8:1::/48").unwrap();
    assert!(
        a.overlaps_cidr(&b).unwrap(),
        "2001:db8::/32 and 2001:db8:1::/48 overlap"
    );
}

#[test]
fn ipv6_cidr_overlaps_cidr_returns_false_for_disjoint_ranges() {
    let a = Ipv6Cidr::new("2001:db8:1::/48").unwrap();
    let b = Ipv6Cidr::new("2001:db8:2::/48").unwrap();
    assert!(
        !a.overlaps_cidr(&b).unwrap(),
        "2001:db8:1::/48 and 2001:db8:2::/48 do not overlap"
    );
}

#[test]
fn ipv6_cidr_overlaps_cidr_returns_true_for_equal_cidrs() {
    let a = Ipv6Cidr::new("fd00::/8").unwrap();
    let b = Ipv6Cidr::new("fd00::/8").unwrap();
    assert!(a.overlaps_cidr(&b).unwrap(), "equal CIDRs overlap");
}

#[test]
fn ipv6_cidr_contains_ip_returns_true_for_address_inside_prefix() {
    let cidr = Ipv6Cidr::new("2001:db8:42::/56").unwrap();
    let addr = "2001:db8:42:1::1".parse::<Ipv6Addr>().unwrap();
    assert!(cidr.contains_ip(addr).unwrap());
}

#[test]
fn ipv6_cidr_contains_ip_returns_false_for_address_outside_prefix() {
    let cidr = Ipv6Cidr::new("2001:db8:42::/56").unwrap();
    let addr = "2001:db8:99::1".parse::<Ipv6Addr>().unwrap();
    assert!(!cidr.contains_ip(addr).unwrap());
}

#[test]
fn ipv6_cidr_returns_invalid_cidr_prefix_for_malformed_input() {
    let bad = Ipv6Cidr {
        value: "not-valid-ipv6-cidr".to_string(),
    };
    let good = Ipv6Cidr::new("fd00::/8").unwrap();
    assert_eq!(
        bad.contains_cidr(&good),
        Err(CloudNetworkError::InvalidCidrPrefix)
    );
    assert_eq!(
        good.contains_cidr(&bad),
        Err(CloudNetworkError::InvalidCidrPrefix)
    );
}
