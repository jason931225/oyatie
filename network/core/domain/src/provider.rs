pub(crate) mod direct_interconnect;
pub(crate) mod dns_zone;
pub(crate) mod load_balancer;
pub(crate) mod vpc;

pub use direct_interconnect::*;
pub use dns_zone::*;
pub use load_balancer::*;
pub use vpc::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NetworkProviderKind {
    OciVcn,
    OciLoadBalancer,
    OciDnsZone,
    OciFastConnect,
    SelfHostedColoVpc,
    SelfHostedColoDnsZone,
}

impl NetworkProviderKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OciVcn => "oci_vcn",
            Self::OciLoadBalancer => "oci_load_balancer",
            Self::OciDnsZone => "oci_dns_zone",
            Self::OciFastConnect => "oci_fast_connect",
            Self::SelfHostedColoVpc => "selfhosted_colo_vpc",
            Self::SelfHostedColoDnsZone => "selfhosted_colo_dns_zone",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NetworkProviderVpcOperation {
    CreateVpc,
}

impl NetworkProviderVpcOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CreateVpc => "create_vpc",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NetworkProviderLoadBalancerOperation {
    CreateLoadBalancer,
}

impl NetworkProviderLoadBalancerOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CreateLoadBalancer => "create_load_balancer",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NetworkProviderDnsZoneOperation {
    CreateDnsZone,
}

impl NetworkProviderDnsZoneOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CreateDnsZone => "create_dns_zone",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NetworkProviderDirectInterconnectOperation {
    CreateDirectInterconnect,
}

impl NetworkProviderDirectInterconnectOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CreateDirectInterconnect => "create_direct_interconnect",
        }
    }
}

pub(crate) fn validate_network_provider_ref(
    value: &str,
    error: NetworkProviderVpcError,
) -> Result<(), NetworkProviderVpcError> {
    if value.trim().is_empty()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        Err(error)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_network_provider_load_balancer_ref(
    value: &str,
    error: NetworkProviderLoadBalancerError,
) -> Result<(), NetworkProviderLoadBalancerError> {
    if value.trim().is_empty()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        Err(error)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_network_provider_dns_zone_ref(
    value: &str,
    error: NetworkProviderDnsZoneError,
) -> Result<(), NetworkProviderDnsZoneError> {
    if value.trim().is_empty()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        Err(error)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_network_provider_direct_interconnect_ref(
    value: &str,
    error: NetworkProviderDirectInterconnectError,
) -> Result<(), NetworkProviderDirectInterconnectError> {
    if value.trim().is_empty()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        Err(error)
    } else {
        Ok(())
    }
}
