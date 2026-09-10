//! Cloud network aggregate kernel.
//!
//! This crate owns the VPC, subnet, load-balancer, DNS-zone, CDN, interconnect,
//! DDoS, and mesh invariants for `cloud.network.*` surfaces. It keeps the
//! AWS/GCP/Azure-shaped primitives explicit while staying adapter-free:
//! OVN/OVS/BGP/CoreDNS/Envoy implementations consume these typed contracts
//! later.
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::collections::{BTreeMap, BTreeSet};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use cell_region::{AzCode, CellId, RegionCode};
use compute_resource::{CloudResourceError, LbProtocol, PrincipalId, ResourceId, ResourceKind};
use data_boundary_kernel::{Classified, DataClass, PrivacyDataClass};
use network_residency::{ResidencyClass, residency_class_allows_home_region_label};

mod bgp;
mod catalog;
mod cdn;
mod cidr;
mod ddos;
mod dns;
mod dns_guardrail;
mod error;
mod flow_anomaly;
mod identifier;
mod interconnect;
mod load_balancer;
mod mesh;
mod port;
mod provider;
mod reference;
mod route;
mod security_group;
mod tenancy;
mod vpc;

#[cfg(test)]
mod tests;

pub use bgp::*;
pub use catalog::*;
pub use cdn::*;
pub use cidr::*;
pub use ddos::*;
pub use dns::*;
pub use dns_guardrail::*;
pub use error::*;
pub use flow_anomaly::*;
pub use identifier::*;
pub use interconnect::*;
pub use load_balancer::*;
pub use mesh::*;
pub use port::*;
pub use provider::*;
pub use reference::*;
pub use route::*;
pub use security_group::*;
pub use tenancy::*;
pub use vpc::*;

const NETWORK_SCHEMA_VERSION: u32 = 1;

fn public_metadata_class(data_class: DataClass) -> Result<PrivacyDataClass, CloudNetworkError> {
    if data_class != DataClass::Public {
        return Err(CloudNetworkError::InvalidDataClass);
    }
    PrivacyDataClass::new(data_class).map_err(|_| CloudNetworkError::InvalidDataClass)
}

fn public<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::Public)
}

fn internal<T>(value: T) -> Classified<T> {
    Classified::new(value, DataClass::InternalOnly)
}
