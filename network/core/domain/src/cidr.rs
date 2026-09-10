use crate::error::CloudNetworkError;
use crate::route::RouteDestination;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ipv4Cidr {
    pub value: String, // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Ipv6Cidr {
    pub value: String, // data_class: PUBLIC
}

impl Ipv4Cidr {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        let value = value.into();
        parse_ipv4_cidr(&value)?;
        Ok(Self { value })
    }

    /// Returns `true` if `other` is fully contained within `self`.
    pub fn contains_cidr(&self, other: &Ipv4Cidr) -> Result<bool, CloudNetworkError> {
        let (self_addr, self_prefix) =
            parse_ipv4_cidr(&self.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let (other_addr, other_prefix) =
            parse_ipv4_cidr(&other.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        if other_prefix < self_prefix {
            return Ok(false);
        }
        let mask = if self_prefix == 0 {
            0u32
        } else {
            u32::MAX << (32 - self_prefix)
        };
        Ok((self_addr & mask) == (other_addr & mask))
    }

    /// Returns `true` if `self` and `other` share at least one address.
    pub fn overlaps_cidr(&self, other: &Ipv4Cidr) -> Result<bool, CloudNetworkError> {
        let (self_addr, self_prefix) =
            parse_ipv4_cidr(&self.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let (other_addr, other_prefix) =
            parse_ipv4_cidr(&other.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let prefix = self_prefix.min(other_prefix);
        let mask = if prefix == 0 {
            0u32
        } else {
            u32::MAX << (32 - prefix)
        };
        Ok((self_addr & mask) == (other_addr & mask))
    }

    /// Returns `true` if `addr` falls within the prefix of `self`.
    pub fn contains_ip(&self, addr: Ipv4Addr) -> Result<bool, CloudNetworkError> {
        let (self_addr, self_prefix) =
            parse_ipv4_cidr(&self.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let mask = if self_prefix == 0 {
            0u32
        } else {
            u32::MAX << (32 - self_prefix)
        };
        Ok((self_addr & mask) == (u32::from(addr) & mask))
    }
}

impl Ipv6Cidr {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        let value = value.into();
        parse_ipv6_cidr(&value)?;
        Ok(Self { value })
    }

    /// Returns `true` if `other` is fully contained within `self`.
    pub fn contains_cidr(&self, other: &Ipv6Cidr) -> Result<bool, CloudNetworkError> {
        let (self_addr, self_prefix) =
            parse_ipv6_cidr(&self.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let (other_addr, other_prefix) =
            parse_ipv6_cidr(&other.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        if other_prefix < self_prefix {
            return Ok(false);
        }
        let mask = if self_prefix == 0 {
            0u128
        } else {
            u128::MAX << (128 - self_prefix)
        };
        Ok((self_addr & mask) == (other_addr & mask))
    }

    /// Returns `true` if `self` and `other` share at least one address.
    pub fn overlaps_cidr(&self, other: &Ipv6Cidr) -> Result<bool, CloudNetworkError> {
        let (self_addr, self_prefix) =
            parse_ipv6_cidr(&self.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let (other_addr, other_prefix) =
            parse_ipv6_cidr(&other.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let prefix = self_prefix.min(other_prefix);
        let mask = if prefix == 0 {
            0u128
        } else {
            u128::MAX << (128 - prefix)
        };
        Ok((self_addr & mask) == (other_addr & mask))
    }

    /// Returns `true` if `addr` falls within the prefix of `self`.
    pub fn contains_ip(&self, addr: Ipv6Addr) -> Result<bool, CloudNetworkError> {
        let (self_addr, self_prefix) =
            parse_ipv6_cidr(&self.value).map_err(|_| CloudNetworkError::InvalidCidrPrefix)?;
        let mask = if self_prefix == 0 {
            0u128
        } else {
            u128::MAX << (128 - self_prefix)
        };
        Ok((self_addr & mask) == (u128::from(addr) & mask))
    }
}

pub(crate) fn parse_ipv4_cidr(value: &str) -> Result<(u32, u8), CloudNetworkError> {
    let (addr, prefix) = value
        .split_once('/')
        .ok_or(CloudNetworkError::InvalidIpv4Cidr)?;
    let addr = Ipv4Addr::from_str(addr).map_err(|_| CloudNetworkError::InvalidIpv4Cidr)?;
    let prefix: u8 = prefix
        .parse()
        .map_err(|_| CloudNetworkError::InvalidIpv4Cidr)?;
    if prefix > 32 {
        return Err(CloudNetworkError::InvalidIpv4Cidr);
    }
    Ok((u32::from(addr), prefix))
}

pub(crate) fn parse_ipv6_cidr(value: &str) -> Result<(u128, u8), CloudNetworkError> {
    let (addr, prefix) = value
        .split_once('/')
        .ok_or(CloudNetworkError::InvalidIpv6Cidr)?;
    let addr = Ipv6Addr::from_str(addr).map_err(|_| CloudNetworkError::InvalidIpv6Cidr)?;
    let prefix: u8 = prefix
        .parse()
        .map_err(|_| CloudNetworkError::InvalidIpv6Cidr)?;
    if prefix > 128 {
        return Err(CloudNetworkError::InvalidIpv6Cidr);
    }
    Ok((u128::from(addr), prefix))
}

pub(crate) fn ipv4_contains(
    parent: &Ipv4Cidr,
    child: &Ipv4Cidr,
) -> Result<bool, CloudNetworkError> {
    let (parent_addr, parent_prefix) = parse_ipv4_cidr(&parent.value)?;
    let (child_addr, child_prefix) = parse_ipv4_cidr(&child.value)?;
    if child_prefix < parent_prefix {
        return Ok(false);
    }
    let mask = if parent_prefix == 0 {
        0
    } else {
        u32::MAX << (32 - parent_prefix)
    };
    Ok((parent_addr & mask) == (child_addr & mask))
}

pub(crate) fn ipv6_contains(
    parent: &Ipv6Cidr,
    child: &Ipv6Cidr,
) -> Result<bool, CloudNetworkError> {
    let (parent_addr, parent_prefix) = parse_ipv6_cidr(&parent.value)?;
    let (child_addr, child_prefix) = parse_ipv6_cidr(&child.value)?;
    if child_prefix < parent_prefix {
        return Ok(false);
    }
    let mask = if parent_prefix == 0 {
        0
    } else {
        u128::MAX << (128 - parent_prefix)
    };
    Ok((parent_addr & mask) == (child_addr & mask))
}

pub(crate) fn ipv4_overlaps(left: &Ipv4Cidr, right: &Ipv4Cidr) -> Result<bool, CloudNetworkError> {
    let (left_addr, left_prefix) = parse_ipv4_cidr(&left.value)?;
    let (right_addr, right_prefix) = parse_ipv4_cidr(&right.value)?;
    let prefix = left_prefix.min(right_prefix);
    let mask = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    Ok((left_addr & mask) == (right_addr & mask))
}

pub(crate) fn ipv6_overlaps(left: &Ipv6Cidr, right: &Ipv6Cidr) -> Result<bool, CloudNetworkError> {
    let (left_addr, left_prefix) = parse_ipv6_cidr(&left.value)?;
    let (right_addr, right_prefix) = parse_ipv6_cidr(&right.value)?;
    let prefix = left_prefix.min(right_prefix);
    let mask = if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - prefix)
    };
    Ok((left_addr & mask) == (right_addr & mask))
}

/// Returns `true` if `container` CIDR contains `inner` CIDR.
pub(crate) fn cidr_contains_cidr(
    container: &RouteDestination,
    inner: &RouteDestination,
) -> Result<bool, CloudNetworkError> {
    match (container, inner) {
        (RouteDestination::Ipv4(c), RouteDestination::Ipv4(i)) => c.contains_cidr(i),
        (RouteDestination::Ipv6(c), RouteDestination::Ipv6(i)) => c.contains_cidr(i),
        _ => Ok(false),
    }
}
