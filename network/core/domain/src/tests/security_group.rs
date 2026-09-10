use super::*;

#[test]
fn evaluate_returns_allow_when_ingress_flow_matches_ingress_rule() {
    let sg = sg_with_rules(vec![ingress_tcp_rule("10.0.0.0/8", Some((443, 443)))]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port: Some(443),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.1.2.3/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Allow { .. }));
}

#[test]
fn evaluate_returns_allow_when_egress_flow_matches_egress_rule() {
    let sg = sg_with_rules(vec![egress_tcp_rule("0.0.0.0/0", Some((80, 80)))]);
    let flow = FlowMatch {
        direction: RuleDirection::Egress,
        protocol: IpProtocol::Tcp,
        port: Some(80),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("203.0.113.0/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Allow { .. }));
}

#[test]
fn evaluate_returns_deny_with_no_matched_rule_when_no_rule_matches_direction() {
    // Only an ingress rule exists; egress flow → deny
    let sg = sg_with_rules(vec![ingress_tcp_rule("0.0.0.0/0", Some((443, 443)))]);
    let flow = FlowMatch {
        direction: RuleDirection::Egress,
        protocol: IpProtocol::Tcp,
        port: Some(443),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.0.0.1/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Deny { matched_rule: None }));
}

#[test]
fn evaluate_returns_deny_when_port_does_not_match_rule_port_range() {
    let sg = sg_with_rules(vec![ingress_tcp_rule("10.0.0.0/8", Some((443, 443)))]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port: Some(8080),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.1.0.1/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Deny { matched_rule: None }));
}

#[test]
fn evaluate_returns_deny_when_protocol_does_not_match_rule_protocol() {
    let sg = sg_with_rules(vec![ingress_tcp_rule("10.0.0.0/8", Some((443, 443)))]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Udp,
        port: Some(443),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.1.0.1/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Deny { matched_rule: None }));
}

#[test]
fn evaluate_returns_deny_when_peer_cidr_is_outside_rule_cidr() {
    let sg = sg_with_rules(vec![ingress_tcp_rule("10.0.0.0/8", Some((443, 443)))]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port: Some(443),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("192.168.1.1/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Deny { matched_rule: None }));
}

#[test]
fn evaluate_returns_allow_for_first_matching_rule_ignoring_later_rules() {
    // Two ingress rules: first covers 10.0.0.0/8 port 443; second covers 10.0.0.0/8 port 8080.
    // Flow matches both directions and CIDRs but only port 443.
    // Correct: first rule wins.
    let rule_a = ingress_tcp_rule("10.0.0.0/8", Some((443, 443)));
    let rule_b = ingress_tcp_rule("10.0.0.0/8", Some((8080, 8080)));
    let sg = sg_with_rules(vec![rule_a.clone(), rule_b]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port: Some(443),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.5.0.1/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    match decision {
        Decision::Allow { matched_rule } => {
            assert_eq!(matched_rule.port_range, Some((443, 443)));
        }
        Decision::Deny { .. } => panic!("expected Allow"),
    }
}

#[test]
fn evaluate_allows_when_rule_protocol_is_any_regardless_of_flow_protocol() {
    let sg = sg_with_rules(vec![ingress_any_rule("10.0.0.0/8")]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Udp,
        port: Some(53),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.0.0.1/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(
        matches!(decision, Decision::Allow { .. }),
        "IpProtocol::Any rule matches any flow protocol"
    );
}

#[test]
fn evaluate_allows_when_rule_has_no_port_range_for_portless_flow() {
    // ICMP rule has no port_range; flow has no port
    let icmp_rule = SecurityRule {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Icmp,
        port_range: None,
        cidr: RouteDestination::Ipv4(Ipv4Cidr::new("0.0.0.0/0").unwrap()),
        description: "allow all icmp ingress".to_string(),
    };
    let sg = sg_with_rules(vec![icmp_rule]);
    let flow = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Icmp,
        port: None,
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("8.8.8.8/32").unwrap()),
    };
    let decision = sg.evaluate(&flow).unwrap();
    assert!(matches!(decision, Decision::Allow { .. }));
}

#[test]
fn evaluate_allows_for_port_within_range_and_denies_outside_range() {
    let sg = sg_with_rules(vec![ingress_tcp_rule("10.0.0.0/8", Some((1024, 65535)))]);
    let inside = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port: Some(8080),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.1.0.1/32").unwrap()),
    };
    assert!(matches!(
        sg.evaluate(&inside).unwrap(),
        Decision::Allow { .. }
    ));

    let outside = FlowMatch {
        direction: RuleDirection::Ingress,
        protocol: IpProtocol::Tcp,
        port: Some(80),
        peer_cidr: RouteDestination::Ipv4(Ipv4Cidr::new("10.1.0.1/32").unwrap()),
    };
    assert!(matches!(
        sg.evaluate(&outside).unwrap(),
        Decision::Deny { matched_rule: None }
    ));
}
