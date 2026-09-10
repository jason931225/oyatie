use crate::error::CloudNetworkError;
const CERT_REF_PREFIX: &str = "cert/";
const DNSSEC_KEY_REF_PREFIX: &str = "dnssec/";

const RUNBOOK_REF_PREFIX: &str = "runbook/";
const ONCALL_GROUP_REF_PREFIX: &str = "oncall/";
const CEDAR_POLICY_REF_PREFIX: &str = "cedar/";
const AUDIT_STREAM_REF_PREFIX: &str = "audit/";
const HEALTH_ALARM_REF_PREFIX: &str = "alarm/";
const EVIDENCE_REF_PREFIX: &str = "evidence://";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CertificateRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DnssecKeyRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RunbookRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OnCallGroupRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct CedarPolicyRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AuditStreamRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct HealthAlarmRef {
    pub value: String, // data_class: INTERNAL_ONLY
}

impl CertificateRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            CERT_REF_PREFIX,
            CloudNetworkError::InvalidCertificateRef,
        )
        .map(|value| Self { value })
    }
}

impl DnssecKeyRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            DNSSEC_KEY_REF_PREFIX,
            CloudNetworkError::InvalidDnssecKeyRef,
        )
        .map(|value| Self { value })
    }
}

impl RunbookRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            RUNBOOK_REF_PREFIX,
            CloudNetworkError::InvalidRunbookRef,
        )
        .map(|value| Self { value })
    }
}

impl OnCallGroupRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            ONCALL_GROUP_REF_PREFIX,
            CloudNetworkError::InvalidOnCallGroupRef,
        )
        .map(|value| Self { value })
    }
}

impl CedarPolicyRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            CEDAR_POLICY_REF_PREFIX,
            CloudNetworkError::InvalidCedarPolicyRef,
        )
        .map(|value| Self { value })
    }
}

impl AuditStreamRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            AUDIT_STREAM_REF_PREFIX,
            CloudNetworkError::InvalidAuditStreamRef,
        )
        .map(|value| Self { value })
    }
}

impl HealthAlarmRef {
    pub fn new(value: impl Into<String>) -> Result<Self, CloudNetworkError> {
        prefixed_ref(
            value.into(),
            HEALTH_ALARM_REF_PREFIX,
            CloudNetworkError::InvalidHealthAlarmRef,
        )
        .map(|value| Self { value })
    }
}

pub(crate) fn validate_evidence_ref(value: String) -> Result<String, CloudNetworkError> {
    let value = value.trim().to_string();
    if !value.starts_with(EVIDENCE_REF_PREFIX)
        || value.len() <= EVIDENCE_REF_PREFIX.len()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        return Err(CloudNetworkError::EvidenceRefMissing);
    }
    if looks_secret_like(&value) {
        return Err(CloudNetworkError::EvidenceRefLooksSecretLike);
    }
    Ok(value)
}

fn looks_secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "token=",
        "password",
        "secret=",
        "kubeconfig",
        "private-key",
        "api_key",
        "apikey",
        "-----begin",
        "sk-live",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn prefixed_ref(
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
