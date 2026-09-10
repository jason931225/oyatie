//! Composition root for the SVID-delivery operator: env-driven config, the
//! injected issuance backend and clock, and the reconcile loop. The decision
//! itself belongs to the kernel and the Secret projection to the adapter.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use iam_identity_workload_svid_operator_k8s::ExponentialBackoff;
use iam_identity_workload_svid_operator_kernel::{Clock, DesiredState};

/// The Secret name the consumer (`MtlsContext::from_path`) mounts.
pub const PDP_SVID_SECRET_NAME: &str = "cloud-iam-pdp-svid";
pub const DEFAULT_NAMESPACE: &str = "cloud-iam";
pub const DEFAULT_TTL_SECS: u64 = 3_600;
pub const DEFAULT_ROTATION_WINDOW_SECS: u64 = 600;

/// Forms `spiffe://oyatie.cell-<id>/platform/cloud-iam-pdp`.
pub const ENV_CELL_ID: &str = "OYATIE_SVID_OPERATOR_CELL_ID";
pub const ENV_NAMESPACE: &str = "OYATIE_SVID_OPERATOR_NAMESPACE";
pub const ENV_TTL_SECS: &str = "OYATIE_SVID_OPERATOR_TTL_SECS";
pub const ENV_ROTATION_WINDOW_SECS: &str = "OYATIE_SVID_OPERATOR_ROTATION_WINDOW_SECS";

/// Why the operator refused to start. Each variant exits the process non-zero;
/// none of them lets it boot degraded. The `Display` arms carry the operator
/// message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperatorStartupConfigError {
    MissingCellId,
    MalformedCellId(String),
    InvalidNumber {
        var: String,
        value: String,
    },
    ZeroTtl,
    /// A leaf would otherwise be born already inside its rotation window.
    RotationWindowNotBelowTtl {
        ttl_secs: u64,
        rotation_window_secs: u64,
    },
}

impl fmt::Display for OperatorStartupConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCellId => write!(f, "{ENV_CELL_ID} must be set to a non-empty cell id"),
            Self::MalformedCellId(value) => write!(
                f,
                "{ENV_CELL_ID} value {value:?} is malformed (no whitespace/control/'/' allowed)"
            ),
            Self::InvalidNumber { var, value } => {
                write!(f, "{var} value {value:?} is not a valid positive integer")
            }
            Self::ZeroTtl => write!(f, "{ENV_TTL_SECS} must be greater than zero"),
            Self::RotationWindowNotBelowTtl {
                ttl_secs,
                rotation_window_secs,
            } => write!(
                f,
                "{ENV_ROTATION_WINDOW_SECS} ({rotation_window_secs}) must be strictly below {ENV_TTL_SECS} ({ttl_secs})"
            ),
        }
    }
}

impl std::error::Error for OperatorStartupConfigError {}

/// The validated operator config.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorConfig {
    pub desired: DesiredState,
    pub backoff: ExponentialBackoff,
}

impl OperatorConfig {
    /// # Errors
    /// [`OperatorStartupConfigError`] on any missing, malformed, or invalid value.
    pub fn from_env() -> Result<Self, OperatorStartupConfigError> {
        Self::from_env_pairs(std::env::vars())
    }

    /// The testable core of [`OperatorConfig::from_env`].
    ///
    /// # Errors
    /// [`OperatorStartupConfigError`] on any missing, malformed, or invalid value.
    pub fn from_env_pairs<K, V, I>(pairs: I) -> Result<Self, OperatorStartupConfigError>
    where
        K: AsRef<str>,
        V: AsRef<str>,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut cell_id: Option<String> = None;
        let mut namespace: Option<String> = None;
        let mut ttl_secs: Option<u64> = None;
        let mut rotation_window_secs: Option<u64> = None;

        for (key, value) in pairs {
            let key = key.as_ref();
            let value = value.as_ref().trim();
            match key {
                ENV_CELL_ID if !value.is_empty() => cell_id = Some(value.to_owned()),
                ENV_NAMESPACE if !value.is_empty() => namespace = Some(value.to_owned()),
                ENV_TTL_SECS if !value.is_empty() => {
                    ttl_secs = Some(parse_u64(ENV_TTL_SECS, value)?);
                }
                ENV_ROTATION_WINDOW_SECS if !value.is_empty() => {
                    rotation_window_secs = Some(parse_u64(ENV_ROTATION_WINDOW_SECS, value)?);
                }
                _ => {}
            }
        }

        let cell_id = cell_id.ok_or(OperatorStartupConfigError::MissingCellId)?;
        if cell_id
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '/')
        {
            return Err(OperatorStartupConfigError::MalformedCellId(cell_id));
        }
        let namespace = namespace.unwrap_or_else(|| DEFAULT_NAMESPACE.to_owned());
        let ttl_secs = ttl_secs.unwrap_or(DEFAULT_TTL_SECS);
        let rotation_window_secs = rotation_window_secs.unwrap_or(DEFAULT_ROTATION_WINDOW_SECS);

        if ttl_secs == 0 {
            return Err(OperatorStartupConfigError::ZeroTtl);
        }
        if rotation_window_secs >= ttl_secs {
            return Err(OperatorStartupConfigError::RotationWindowNotBelowTtl {
                ttl_secs,
                rotation_window_secs,
            });
        }

        Ok(Self {
            desired: DesiredState {
                spiffe_id: format!("spiffe://oyatie.cell-{cell_id}/platform/cloud-iam-pdp"),
                ttl_secs,
                rotation_window_secs,
                secret_name: PDP_SVID_SECRET_NAME.to_owned(),
                secret_namespace: namespace,
            },
            backoff: default_operator_backoff(),
        })
    }
}

fn parse_u64(var: &str, value: &str) -> Result<u64, OperatorStartupConfigError> {
    value
        .parse::<u64>()
        .map_err(|_| OperatorStartupConfigError::InvalidNumber {
            var: var.to_owned(),
            value: value.to_owned(),
        })
}

#[must_use]
pub fn default_operator_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        base_seconds: 30,
        max_seconds: 600,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_epoch_seconds(&self) -> u64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => 0,
        }
    }
}
