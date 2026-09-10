use super::*;

// ── ST3: SecurityGroup::detect_shadowed_rules ─────────────────────────────

#[test]
fn detect_shadowed_rules_returns_empty_when_no_rules_are_present() {
    let sg = sg_with_rules(vec![]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert!(pairs.is_empty(), "no rules → no shadow pairs");
}

#[test]
fn detect_shadowed_rules_returns_empty_for_non_conflicting_disjoint_cidr_rules() {
    let rule_a = ingress_tcp_rule("10.0.0.0/8", Some((443, 443)));
    let rule_b = ingress_tcp_rule("192.168.0.0/16", Some((443, 443)));
    let sg = sg_with_rules(vec![rule_a, rule_b]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert!(
        pairs.is_empty(),
        "disjoint CIDRs with same port/protocol do not shadow each other"
    );
}

#[test]
fn detect_shadowed_rules_finds_shadowed_rule_when_earlier_rule_subsumes_later_rule() {
    // rule_a: ingress TCP 0.0.0.0/0 port 443-443  → subsumes rule_b
    // rule_b: ingress TCP 10.0.0.0/8 port 443-443
    let rule_a = ingress_tcp_rule("0.0.0.0/0", Some((443, 443)));
    let rule_b = ingress_tcp_rule("10.0.0.0/8", Some((443, 443)));
    let sg = sg_with_rules(vec![rule_a.clone(), rule_b.clone()]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert_eq!(pairs.len(), 1, "one shadow pair expected");
    let (shadowing, shadowed) = &pairs[0];
    assert_eq!(shadowing.cidr, rule_a.cidr);
    assert_eq!(shadowed.cidr, rule_b.cidr);
}

#[test]
fn detect_shadowed_rules_finds_redundant_identical_rule_pair() {
    let rule_a = ingress_tcp_rule("10.0.0.0/8", Some((443, 443)));
    let rule_b = ingress_tcp_rule("10.0.0.0/8", Some((443, 443)));
    let sg = sg_with_rules(vec![rule_a.clone(), rule_b.clone()]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert_eq!(pairs.len(), 1, "identical rules: first shadows second");
    let (shadowing, shadowed) = &pairs[0];
    assert_eq!(shadowing.port_range, rule_a.port_range);
    assert_eq!(shadowed.port_range, rule_b.port_range);
}

#[test]
fn detect_shadowed_rules_finds_shadow_when_earlier_rule_has_wider_port_range() {
    // rule_a covers 1024-65535; rule_b covers 8080-8080 — rule_a subsumes rule_b
    let rule_a = ingress_tcp_rule("10.0.0.0/8", Some((1024, 65535)));
    let rule_b = ingress_tcp_rule("10.0.0.0/8", Some((8080, 8080)));
    let sg = sg_with_rules(vec![rule_a.clone(), rule_b.clone()]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].0.port_range, Some((1024, 65535)));
}

#[test]
fn detect_shadowed_rules_does_not_shadow_when_later_rule_has_wider_port_range() {
    // rule_a covers 8080-8080; rule_b covers 1024-65535 — rule_a cannot shadow rule_b
    let rule_a = ingress_tcp_rule("10.0.0.0/8", Some((8080, 8080)));
    let rule_b = ingress_tcp_rule("10.0.0.0/8", Some((1024, 65535)));
    let sg = sg_with_rules(vec![rule_a, rule_b]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert!(
        pairs.is_empty(),
        "narrower earlier rule cannot shadow broader later rule"
    );
}

#[test]
fn detect_shadowed_rules_any_protocol_rule_shadows_specific_protocol_rule() {
    // rule_a: ingress ANY 0.0.0.0/0 (no port) → shadows rule_b: ingress TCP 0.0.0.0/0 port 443
    let rule_any = ingress_any_rule("0.0.0.0/0");
    let rule_tcp = ingress_tcp_rule("0.0.0.0/0", Some((443, 443)));
    let sg = sg_with_rules(vec![rule_any.clone(), rule_tcp.clone()]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert_eq!(
        pairs.len(),
        1,
        "Any protocol rule with no port restriction shadows TCP port rule"
    );
    assert_eq!(pairs[0].0.protocol, IpProtocol::Any);
    assert_eq!(pairs[0].1.protocol, IpProtocol::Tcp);
}

#[test]
fn detect_shadowed_rules_does_not_shadow_across_directions() {
    // Ingress rule cannot shadow egress rule even if all other fields match
    let ingress = ingress_tcp_rule("0.0.0.0/0", Some((443, 443)));
    let egress = egress_tcp_rule("0.0.0.0/0", Some((443, 443)));
    let sg = sg_with_rules(vec![ingress, egress]);
    let pairs = sg.detect_shadowed_rules().unwrap();
    assert!(pairs.is_empty(), "direction mismatch prevents shadowing");
}
