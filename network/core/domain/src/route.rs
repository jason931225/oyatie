use crate::cidr::{Ipv4Cidr, Ipv6Cidr, parse_ipv4_cidr, parse_ipv6_cidr};
use crate::error::CloudNetworkError;
use crate::identifier::RouteTableId;
use std::collections::BTreeSet;
use std::net::IpAddr;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RouteNextHopKind {
    Local,
    InternetGateway,
    NatGateway,
    VpcPeering,
    TransitGateway,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RouteDestination {
    Ipv4(Ipv4Cidr),
    Ipv6(Ipv6Cidr),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Route {
    pub destination: RouteDestination, // data_class: PUBLIC
    pub next_hop: RouteNextHopKind,    // data_class: PUBLIC
    pub target_ref: Option<String>,    // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteCreate {
    pub destination: String,        // data_class: PUBLIC
    pub next_hop: RouteNextHopKind, // data_class: PUBLIC
    pub target_ref: Option<String>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteTableCreate {
    pub id: String,               // data_class: INTERNAL_ONLY
    pub routes: Vec<RouteCreate>, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteTable {
    pub id: RouteTableId,   // data_class: INTERNAL_ONLY
    pub routes: Vec<Route>, // data_class: PUBLIC
}

impl RouteDestination {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        let value = value.into();
        if value.contains(':') {
            Ok(Self::Ipv6(Ipv6Cidr::new(value)?))
        } else {
            Ok(Self::Ipv4(Ipv4Cidr::new(value)?))
        }
    }
}

impl RouteTable {
    pub fn new(input: RouteTableCreate) -> Result<Self, CloudNetworkError> {
        let id = RouteTableId::new(input.id)?;
        let mut seen = BTreeSet::new();
        let mut routes = Vec::with_capacity(input.routes.len());
        for route in input.routes {
            let destination = RouteDestination::new(route.destination)?;
            if !seen.insert(destination.clone()) {
                return Err(CloudNetworkError::DuplicateRoute);
            }
            if matches!(route.next_hop, RouteNextHopKind::Local) && route.target_ref.is_some() {
                return Err(CloudNetworkError::InvalidRoute);
            }
            if !matches!(route.next_hop, RouteNextHopKind::Local) && route.target_ref.is_none() {
                return Err(CloudNetworkError::InvalidRoute);
            }
            routes.push(Route {
                destination,
                next_hop: route.next_hop,
                target_ref: route.target_ref,
            });
        }
        Ok(Self { id, routes })
    }

    /// Resolve the most-specific (longest-prefix-match) [`Route`] for `addr`.
    ///
    /// - IPv4 addresses match only `RouteDestination::Ipv4` routes.
    /// - IPv6 addresses match only `RouteDestination::Ipv6` routes.
    /// - When multiple routes contain `addr`, the one with the longest prefix
    ///   length wins. Ties are broken by Vec insertion order (first entry wins),
    ///   making the result fully deterministic for a given `RouteTable`.
    /// - Returns `Ok(None)` when no route covers `addr`.
    /// - Propagates `Err(CloudNetworkError::InvalidCidrPrefix)` if a stored
    ///   CIDR value is malformed (bypassed the constructor).
    pub fn resolve_next_hop(&self, addr: IpAddr) -> Result<Option<&Route>, CloudNetworkError> {
        let mut best: Option<(&Route, u8)> = None;
        for route in &self.routes {
            let matched_prefix: Option<u8> = match (&route.destination, addr) {
                (RouteDestination::Ipv4(cidr), IpAddr::V4(v4)) => {
                    if cidr.contains_ip(v4)? {
                        let (_, prefix) = parse_ipv4_cidr(&cidr.value)?;
                        Some(prefix)
                    } else {
                        None
                    }
                }
                (RouteDestination::Ipv6(cidr), IpAddr::V6(v6)) => {
                    if cidr.contains_ip(v6)? {
                        let (_, prefix) = parse_ipv6_cidr(&cidr.value)?;
                        Some(prefix)
                    } else {
                        None
                    }
                }
                // Cross-family: skip without error.
                _ => None,
            };
            if let Some(prefix) = matched_prefix {
                match best {
                    None => best = Some((route, prefix)),
                    Some((_, best_prefix)) if prefix > best_prefix => {
                        best = Some((route, prefix));
                    }
                    _ => {}
                }
            }
        }
        Ok(best.map(|(route, _)| route))
    }
}
