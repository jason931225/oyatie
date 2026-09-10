#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;

use shared_olap_client_kernel::TenantId;

#[derive(Debug)]
pub enum BootError {
    InvalidListenAddr(String),
    MissingConfig(&'static str),
    InvalidTenantId(String),
}

impl fmt::Display for BootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidListenAddr(addr) => write!(f, "invalid listen address: {addr}"),
            Self::MissingConfig(key) => write!(f, "missing required config: {key}"),
            Self::InvalidTenantId(detail) => write!(f, "invalid tenant id: {detail}"),
        }
    }
}

impl std::error::Error for BootError {}

#[derive(Clone, Debug)]
pub struct AnalyticsConfig {
    pub listen_addr: String,
    pub clickhouse_url: String,
    pub clickhouse_user: String,
    /// data_class: INTERNAL_ONLY (secret)
    pub clickhouse_password: String,
    pub primary_tenant_id: String,
}

impl AnalyticsConfig {
    fn non_empty_required_fields(&self) -> [(&'static str, &str); 3] {
        [
            ("listen_addr", &self.listen_addr),
            ("clickhouse_url", &self.clickhouse_url),
            ("clickhouse_user", &self.clickhouse_user),
        ]
    }

    /// # Errors
    /// Returns [`BootError::MissingConfig`] if any required field is empty, or
    /// [`BootError::InvalidTenantId`] if the tenant ID is syntactically invalid.
    pub fn validate(&self) -> Result<TenantId, BootError> {
        for (name, value) in self.non_empty_required_fields() {
            if value.is_empty() {
                return Err(BootError::MissingConfig(name));
            }
        }
        let tenant_id = TenantId::try_new(&self.primary_tenant_id)
            .map_err(|e| BootError::InvalidTenantId(e.to_string()))?;
        Ok(tenant_id)
    }
}

pub struct AnalyticsApp {
    config: AnalyticsConfig,
    primary_tenant_id: TenantId,
}

impl AnalyticsApp {
    pub fn new(config: AnalyticsConfig) -> Result<Self, BootError> {
        let primary_tenant_id = config.validate()?;
        Ok(Self {
            config,
            primary_tenant_id,
        })
    }

    #[must_use]
    pub fn listen_addr(&self) -> &str {
        &self.config.listen_addr
    }

    #[must_use]
    pub fn primary_tenant_id(&self) -> &TenantId {
        &self.primary_tenant_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config() -> AnalyticsConfig {
        AnalyticsConfig {
            listen_addr: "127.0.0.1:8080".to_string(),
            clickhouse_url: "http://ch:8123".to_string(),
            clickhouse_user: "default".to_string(),
            clickhouse_password: "pass".to_string(),
            primary_tenant_id: "t1".to_string(),
        }
    }

    #[test]
    fn config_validate_fails_on_empty_listen_addr() {
        let mut cfg = valid_config();
        cfg.listen_addr = "".to_string();
        match cfg.validate().unwrap_err() {
            BootError::MissingConfig(key) => assert_eq!(key, "listen_addr"),
            other => panic!("wrong error: {other}"),
        }
    }

    #[test]
    fn empty_password_is_not_a_missing_required_field() {
        let mut cfg = valid_config();
        cfg.clickhouse_password = String::new();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn config_validate_fails_on_invalid_tenant_id() {
        let mut cfg = valid_config();
        cfg.primary_tenant_id = "".to_string();
        assert!(matches!(
            cfg.validate().unwrap_err(),
            BootError::InvalidTenantId(_)
        ));
    }

    #[test]
    fn app_builds_successfully() {
        let app = AnalyticsApp::new(valid_config()).unwrap();
        assert_eq!(app.listen_addr(), "127.0.0.1:8080");
        assert_eq!(app.primary_tenant_id().as_str(), "t1");
    }
}
