use crate::error::CloudNetworkError;
use crate::identifier::BgpSessionId;
use crate::route::RouteDestination;
use std::collections::BTreeSet;
use std::net::IpAddr;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BgpSessionCreate {
    pub id: String,            // data_class: INTERNAL_ONLY
    pub local_asn: u32,        // data_class: INTERNAL_ONLY
    pub peer_asn: u32,         // data_class: INTERNAL_ONLY
    pub local_address: String, // data_class: INTERNAL_ONLY
    pub peer_address: String,  // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BgpSession {
    pub id: BgpSessionId,      // data_class: INTERNAL_ONLY
    pub local_asn: u32,        // data_class: INTERNAL_ONLY
    pub peer_asn: u32,         // data_class: INTERNAL_ONLY
    pub local_address: IpAddr, // data_class: INTERNAL_ONLY
    pub peer_address: IpAddr,  // data_class: INTERNAL_ONLY
}

pub(crate) fn bgp_sessions(
    input: Vec<BgpSessionCreate>,
) -> Result<Vec<BgpSession>, CloudNetworkError> {
    if input.len() < 2 {
        return Err(CloudNetworkError::InterconnectRedundancyRequired);
    }
    let mut seen = BTreeSet::new();
    let mut sessions = Vec::with_capacity(input.len());
    for session in input {
        let id = BgpSessionId::new(session.id)?;
        if !seen.insert(id.clone()) {
            return Err(CloudNetworkError::DuplicateBgpSession);
        }
        validate_asn(session.local_asn)?;
        validate_asn(session.peer_asn)?;
        let local_address = IpAddr::from_str(&session.local_address)
            .map_err(|_| CloudNetworkError::InvalidBgpSession)?;
        let peer_address = IpAddr::from_str(&session.peer_address)
            .map_err(|_| CloudNetworkError::InvalidBgpSession)?;
        if local_address == peer_address || local_address.is_ipv4() != peer_address.is_ipv4() {
            return Err(CloudNetworkError::InvalidBgpSession);
        }
        sessions.push(BgpSession {
            id,
            local_asn: session.local_asn,
            peer_asn: session.peer_asn,
            local_address,
            peer_address,
        });
    }
    Ok(sessions)
}

fn validate_asn(value: u32) -> Result<(), CloudNetworkError> {
    if value == 0 || value == 23_456 {
        Err(CloudNetworkError::InvalidAsn)
    } else {
        Ok(())
    }
}

pub(crate) fn advertised_prefixes(
    input: Vec<String>,
) -> Result<Vec<RouteDestination>, CloudNetworkError> {
    if input.is_empty() {
        return Err(CloudNetworkError::InvalidRoute);
    }
    let mut seen = BTreeSet::new();
    let mut prefixes = Vec::with_capacity(input.len());
    for prefix in input {
        let prefix = RouteDestination::new(prefix)?;
        if !seen.insert(prefix.clone()) {
            return Err(CloudNetworkError::DuplicateRoute);
        }
        prefixes.push(prefix);
    }
    Ok(prefixes)
}
