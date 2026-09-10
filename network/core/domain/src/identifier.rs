use crate::error::CloudNetworkError;
const ROUTE_TABLE_ID_PREFIX: &str = "rtb_";
const SECURITY_GROUP_ID_PREFIX: &str = "sg_";
const TARGET_GROUP_ID_PREFIX: &str = "tg_";
const WAF_POLICY_ID_PREFIX: &str = "waf_";

const FLOW_ANOMALY_ID_PREFIX: &str = "flowanom_";
const INTERCONNECT_PARTNER_ID_PREFIX: &str = "ixp_";
const INTERCONNECT_PORT_ID_PREFIX: &str = "icp_";
const BGP_SESSION_ID_PREFIX: &str = "bgp_";
const MESH_ID_PREFIX: &str = "mesh_";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RouteTableId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SecurityGroupId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TargetGroupId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct WafPolicyId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FlowAnomalyId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct InterconnectPartnerId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct InterconnectPortId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BgpSessionId {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MeshId {
    pub value: String, // data_class: INTERNAL_ONLY
}

impl RouteTableId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            ROUTE_TABLE_ID_PREFIX,
            CloudNetworkError::InvalidRouteTableId,
        )
        .map(|value| Self { value })
    }
}

impl SecurityGroupId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            SECURITY_GROUP_ID_PREFIX,
            CloudNetworkError::InvalidSecurityGroupId,
        )
        .map(|value| Self { value })
    }
}

impl TargetGroupId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            TARGET_GROUP_ID_PREFIX,
            CloudNetworkError::InvalidTargetGroupId,
        )
        .map(|value| Self { value })
    }
}

impl WafPolicyId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            WAF_POLICY_ID_PREFIX,
            CloudNetworkError::InvalidWafPolicyId,
        )
        .map(|value| Self { value })
    }
}

impl FlowAnomalyId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        let value = value.into();
        if value.starts_with(FLOW_ANOMALY_ID_PREFIX) && value.len() > FLOW_ANOMALY_ID_PREFIX.len() {
            Ok(Self { value })
        } else {
            Err(CloudNetworkError::InvalidFlowAnomalyId)
        }
    }
}

impl InterconnectPartnerId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            INTERCONNECT_PARTNER_ID_PREFIX,
            CloudNetworkError::InvalidInterconnectPartnerId,
        )
        .map(|value| Self { value })
    }
}

impl InterconnectPortId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            INTERCONNECT_PORT_ID_PREFIX,
            CloudNetworkError::InvalidInterconnectPortId,
        )
        .map(|value| Self { value })
    }
}

impl BgpSessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            BGP_SESSION_ID_PREFIX,
            CloudNetworkError::InvalidBgpSessionId,
        )
        .map(|value| Self { value })
    }
}

impl MeshId {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_id(
            value.into(),
            MESH_ID_PREFIX,
            CloudNetworkError::InvalidMeshId,
        )
        .map(|value| Self { value })
    }
}

fn prefixed_id(
    value: String,
    prefix: &str,
    error: CloudNetworkError,
) -> Result<String, CloudNetworkError> {
    if value.starts_with(prefix)
        && value.len() > prefix.len()
        && !value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        Ok(value)
    } else {
        Err(error)
    }
}
