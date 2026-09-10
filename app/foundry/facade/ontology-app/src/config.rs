use std::path::PathBuf;

use crate::auth::OperatorCredential;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    /// The address the listener binds. Loopback by default — exposure is a
    /// deployment decision, never a default.
    pub listen_addr: String, // data_class: INTERNAL_ONLY
    pub action_log: PathBuf, // data_class: INTERNAL_ONLY
    pub denial_log: PathBuf, // data_class: INTERNAL_ONLY
    /// The tenants this process serves. `RecordsLog` cannot enumerate
    /// tenants, so the roster IS the served set.
    pub tenants: Vec<String>, // data_class: INTERNAL_ONLY
    pub operators: Vec<OperatorCredential>, // data_class: SECRET
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    Missing { variable: &'static str },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { variable } => {
                write!(formatter, "required configuration {variable} is absent")
            }
        }
    }
}

const LISTEN_ADDR: &str = "OYATIE_FOUNDRY_ONTOLOGY_LISTEN_ADDR";
const ACTION_LOG: &str = "OYATIE_FOUNDRY_ONTOLOGY_ACTION_LOG";
const DENIAL_LOG: &str = "OYATIE_FOUNDRY_ONTOLOGY_DENIAL_LOG";
const TENANTS: &str = "OYATIE_FOUNDRY_ONTOLOGY_TENANTS";
const OPERATORS: &str = "OYATIE_FOUNDRY_ONTOLOGY_OPERATORS";

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            listen_addr: non_empty(LISTEN_ADDR).unwrap_or_else(|| "127.0.0.1:8090".into()),
            action_log: PathBuf::from(non_empty(ACTION_LOG).ok_or(ConfigError::Missing {
                variable: ACTION_LOG,
            })?),
            denial_log: PathBuf::from(non_empty(DENIAL_LOG).ok_or(ConfigError::Missing {
                variable: DENIAL_LOG,
            })?),
            tenants: non_empty(TENANTS)
                .ok_or(ConfigError::Missing { variable: TENANTS })?
                .split(',')
                .map(str::trim)
                .filter(|tenant| !tenant.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            operators: operators_from_env(),
        })
    }
}

fn non_empty(variable: &str) -> Option<String> {
    std::env::var(variable)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn operators_from_env() -> Vec<OperatorCredential> {
    non_empty(OPERATORS)
        .unwrap_or_default()
        .split(',')
        .filter_map(parse_operator)
        .collect()
}

fn parse_operator(entry: &str) -> Option<OperatorCredential> {
    let mut fields = entry.trim().split(':');
    let token = fields.next()?.trim();
    let tenant_id = fields.next()?.trim();
    let principal_id = fields.next()?.trim();
    if token.is_empty() || tenant_id.is_empty() || principal_id.is_empty() {
        return None;
    }
    Some(OperatorCredential {
        token: token.to_owned(),
        tenant_id: tenant_id.to_owned(),
        principal_id: principal_id.to_owned(),
        roles: fields
            .next()
            .unwrap_or_default()
            .split('|')
            .map(str::trim)
            .filter(|role| !role.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    })
}
