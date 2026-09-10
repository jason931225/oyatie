use super::*;
use std::net::IpAddr;

// ── ST4: RouteTable::resolve_next_hop ─────────────────────────────────────

/// Build a minimal valid RouteTable directly (bypass RouteTable::new
/// constructor to freely compose test fixtures with any next_hop kind).
fn route_table_from_routes(routes: Vec<Route>) -> RouteTable {
    RouteTable {
        id: RouteTableId {
            value: "rtb_test".to_string(),
        },
        routes,
    }
}

fn ipv4_local_route(cidr: &str) -> Route {
    Route {
        destination: RouteDestination::Ipv4(Ipv4Cidr::new(cidr).unwrap()),
        next_hop: RouteNextHopKind::Local,
        target_ref: None,
    }
}

fn ipv4_gateway_route(cidr: &str, target: &str) -> Route {
    Route {
        destination: RouteDestination::Ipv4(Ipv4Cidr::new(cidr).unwrap()),
        next_hop: RouteNextHopKind::InternetGateway,
        target_ref: Some(target.to_string()),
    }
}

fn ipv6_local_route(cidr: &str) -> Route {
    Route {
        destination: RouteDestination::Ipv6(Ipv6Cidr::new(cidr).unwrap()),
        next_hop: RouteNextHopKind::Local,
        target_ref: None,
    }
}

/// (a) /32 beats /24 beats 0.0.0.0/0 for a covered IPv4 addr
#[test]
fn lpm_v4_most_specific_wins() {
    let default_route = ipv4_gateway_route("0.0.0.0/0", "igw_default");
    let covering_24 = ipv4_gateway_route("10.0.0.0/24", "igw_24");
    let host_32 = ipv4_local_route("10.0.0.42/32");

    let rt = route_table_from_routes(vec![default_route, covering_24.clone(), host_32.clone()]);

    let addr: IpAddr = "10.0.0.42".parse().unwrap();
    let result = rt.resolve_next_hop(addr).unwrap();
    let route = result.expect("should match something");
    // /32 is the most specific
    assert_eq!(
        route.destination,
        RouteDestination::Ipv4(Ipv4Cidr::new("10.0.0.42/32").unwrap()),
        "/32 host route must win over /24 and default"
    );
    assert_eq!(route.next_hop, RouteNextHopKind::Local);

    // Now test /24 beats default when /32 is absent
    let rt24 = route_table_from_routes(vec![
        ipv4_gateway_route("0.0.0.0/0", "igw_default"),
        covering_24.clone(),
    ]);
    let result24 = rt24.resolve_next_hop(addr).unwrap();
    let route24 = result24.expect("should match /24");
    assert_eq!(
        route24.destination,
        RouteDestination::Ipv4(Ipv4Cidr::new("10.0.0.0/24").unwrap()),
        "/24 must beat default route"
    );
}

/// (b) no-match returns Ok(None)
#[test]
fn lpm_v4_no_match_returns_ok_none() {
    let rt = route_table_from_routes(vec![ipv4_local_route("10.0.0.0/8")]);
    let addr: IpAddr = "192.168.1.1".parse().unwrap();
    let result = rt.resolve_next_hop(addr).unwrap();
    assert!(result.is_none(), "192.168.1.1 not in 10.0.0.0/8 → None");
}

/// (b) empty table returns Ok(None)
#[test]
fn lpm_empty_table_returns_ok_none() {
    let rt = route_table_from_routes(vec![]);
    let addr: IpAddr = "10.0.0.1".parse().unwrap();
    assert!(rt.resolve_next_hop(addr).unwrap().is_none());
}

/// (c) IPv6 address only matches IPv6 routes; IPv4-only table → Ok(None)
#[test]
fn lpm_v6_family_isolation() {
    // IPv4-only table: IPv6 query → None
    let rt_v4_only = route_table_from_routes(vec![ipv4_gateway_route("0.0.0.0/0", "igw_default")]);
    let v6_addr: IpAddr = "2001:db8::1".parse().unwrap();
    assert!(
        rt_v4_only.resolve_next_hop(v6_addr).unwrap().is_none(),
        "IPv6 addr must not match IPv4 routes"
    );

    // IPv6-only table: IPv4 query → None
    let rt_v6_only = route_table_from_routes(vec![ipv6_local_route("2001:db8::/32")]);
    let v4_addr: IpAddr = "10.0.0.1".parse().unwrap();
    assert!(
        rt_v6_only.resolve_next_hop(v4_addr).unwrap().is_none(),
        "IPv4 addr must not match IPv6 routes"
    );

    // IPv6 table: IPv6 query matches
    let rt_v6 = route_table_from_routes(vec![
        ipv6_local_route("::/0"),
        ipv6_local_route("2001:db8::/32"),
    ]);
    let result = rt_v6.resolve_next_hop(v6_addr).unwrap();
    let route = result.expect("should match 2001:db8::/32");
    assert_eq!(
        route.destination,
        RouteDestination::Ipv6(Ipv6Cidr::new("2001:db8::/32").unwrap()),
        "/32 IPv6 prefix must win over ::/0"
    );
}

/// (d) Local vs gateway next_hop returned intact
#[test]
fn lpm_next_hop_returned_intact() {
    let local = ipv4_local_route("10.0.0.0/8");
    let gateway = ipv4_gateway_route("0.0.0.0/0", "igw_main");
    let rt = route_table_from_routes(vec![local, gateway]);

    // Addr in 10.0.0.0/8 → Local
    let addr_local: IpAddr = "10.1.2.3".parse().unwrap();
    let r = rt.resolve_next_hop(addr_local).unwrap().unwrap();
    assert_eq!(r.next_hop, RouteNextHopKind::Local);
    assert!(r.target_ref.is_none());

    // Addr outside 10.0.0.0/8 → InternetGateway
    let addr_gw: IpAddr = "203.0.113.5".parse().unwrap();
    let r2 = rt.resolve_next_hop(addr_gw).unwrap().unwrap();
    assert_eq!(r2.next_hop, RouteNextHopKind::InternetGateway);
    assert_eq!(r2.target_ref.as_deref(), Some("igw_main"));
}

/// (e) determinism: same table + addr always returns same Route
#[test]
fn lpm_determinism() {
    let rt = route_table_from_routes(vec![
        ipv4_gateway_route("0.0.0.0/0", "igw_default"),
        ipv4_local_route("10.0.0.0/24"),
        ipv4_gateway_route("10.0.0.0/16", "igw_16"),
    ]);
    let addr: IpAddr = "10.0.0.7".parse().unwrap();

    let r1 = rt.resolve_next_hop(addr).unwrap().unwrap();
    let r2 = rt.resolve_next_hop(addr).unwrap().unwrap();
    assert_eq!(r1, r2, "repeated calls must return identical Route");
    // /24 is more specific than /16 which is more specific than /0
    assert_eq!(
        r1.destination,
        RouteDestination::Ipv4(Ipv4Cidr::new("10.0.0.0/24").unwrap())
    );
}

/// default route 0.0.0.0/0 matches any IPv4 when no more-specific route exists
#[test]
fn lpm_default_route_matches_all_ipv4() {
    let rt = route_table_from_routes(vec![ipv4_gateway_route("0.0.0.0/0", "igw_default")]);
    let addr: IpAddr = "8.8.8.8".parse().unwrap();
    let r = rt.resolve_next_hop(addr).unwrap().unwrap();
    assert_eq!(r.next_hop, RouteNextHopKind::InternetGateway);
}
