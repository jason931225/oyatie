use super::*;

#[derive(Clone, Eq, PartialEq)]
pub struct CloudIacAppConfig {
    pub bind_addr: SocketAddr,
    pub release_index_path: PathBuf,
    pub module_registry_bearer: Option<String>, // data_class: SECRET
    pub module_registry_principal_id: Option<String>,
}

impl std::fmt::Debug for CloudIacAppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudIacAppConfig")
            .field("bind_addr", &self.bind_addr)
            .field("release_index_path", &self.release_index_path)
            .field(
                "module_registry_bearer",
                &self.module_registry_bearer.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "module_registry_principal_id",
                &self.module_registry_principal_id,
            )
            .finish()
    }
}

impl Default for CloudIacAppConfig {
    fn default() -> Self {
        Self {
            bind_addr: CLOUD_IAC_APP_DEFAULT_BIND_ADDR
                .parse()
                .expect("static default bind address parses"),
            release_index_path: PathBuf::from(CLOUD_IAC_APP_DEFAULT_RELEASE_INDEX_PATH),
            module_registry_bearer: None,
            module_registry_principal_id: None,
        }
    }
}

impl CloudIacAppConfig {
    pub fn from_env_pairs<I, K, V>(pairs: I) -> Result<Self, CloudIacAppConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut bind_addr = CLOUD_IAC_APP_DEFAULT_BIND_ADDR.to_string();
        let mut release_index_path = CLOUD_IAC_APP_DEFAULT_RELEASE_INDEX_PATH.to_string();
        let mut module_registry_bearer = None;
        let mut module_registry_principal_id = None;

        for (key, value) in pairs {
            match key.as_ref() {
                CLOUD_IAC_APP_BIND_ADDR_ENV => bind_addr = value.as_ref().to_string(),
                CLOUD_IAC_APP_RELEASE_INDEX_PATH_ENV => {
                    release_index_path = value.as_ref().to_string();
                }
                CLOUD_IAC_APP_MODULE_REGISTRY_BEARER_ENV => {
                    module_registry_bearer = Some(value.as_ref().to_string());
                }
                CLOUD_IAC_APP_MODULE_REGISTRY_PRINCIPAL_ENV => {
                    module_registry_principal_id = Some(value.as_ref().to_string());
                }
                _ => {}
            }
        }

        Ok(Self {
            bind_addr: parse_bind_addr(&bind_addr)?,
            release_index_path: parse_release_index_path(&release_index_path)?,
            module_registry_bearer,
            module_registry_principal_id,
        })
    }

    /// # Errors
    /// [`CloudIacAppConfigError`] when the bearer or principal is unset, or the
    /// bearer is malformed.
    pub fn module_registry_authz_provider(
        &self,
    ) -> Result<Arc<CloudIacModuleRegistryAuthzProvider>, CloudIacAppConfigError> {
        let Some(bearer) = &self.module_registry_bearer else {
            return Err(CloudIacAppConfigError::MissingModuleRegistryBearer);
        };
        let Some(principal_id) = &self.module_registry_principal_id else {
            return Err(CloudIacAppConfigError::MissingModuleRegistryPrincipal);
        };
        build_module_registry_authz_provider(bearer, principal_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudIacAppConfigError {
    EmptyBindAddr,
    InvalidBindAddr { value: String, reason: String },
    EmptyReleaseIndexPath,
    MissingModuleRegistryBearer,
    MissingModuleRegistryPrincipal,
    InvalidModuleRegistryBearer { reason: String },
}

pub(super) fn parse_bind_addr(value: &str) -> Result<SocketAddr, CloudIacAppConfigError> {
    if value.trim().is_empty() {
        return Err(CloudIacAppConfigError::EmptyBindAddr);
    }
    value.parse().map_err(|error: std::net::AddrParseError| {
        CloudIacAppConfigError::InvalidBindAddr {
            value: value.to_string(),
            reason: error.to_string(),
        }
    })
}

pub(super) fn parse_release_index_path(value: &str) -> Result<PathBuf, CloudIacAppConfigError> {
    if value.trim().is_empty() {
        return Err(CloudIacAppConfigError::EmptyReleaseIndexPath);
    }
    Ok(PathBuf::from(value))
}

const CLOUD_IAC_MODULE_REGISTRY_READER_SURFACES: [&str; 3] = [
    CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE,
    CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE,
    CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE,
];

/// The bearer must be free of whitespace/control characters so the `Bearer
/// <token>` header round-trips byte-for-byte.
pub(super) fn build_module_registry_authz_provider(
    bearer: &str,
    principal_id: &str,
) -> Result<Arc<CloudIacModuleRegistryAuthzProvider>, CloudIacAppConfigError> {
    if bearer.trim().is_empty()
        || bearer.chars().any(|ch| ch.is_ascii_whitespace())
        || bearer.chars().any(|ch| ch.is_control())
    {
        return Err(CloudIacAppConfigError::InvalidModuleRegistryBearer {
            reason: "bearer must not contain whitespace or control characters".to_string(),
        });
    }
    let verifier =
        ConfiguredBearerPrincipalVerifier::new(bearer, principal_id).map_err(|error| {
            CloudIacAppConfigError::InvalidModuleRegistryBearer {
                reason: error.to_string(),
            }
        })?;
    let authorizer = ConfiguredSurfaceAuthorizer::new(
        CLOUD_IAC_MODULE_REGISTRY_READER_SURFACES
            .iter()
            .map(|surface| (*surface).to_string()),
    );
    Ok(Arc::new(CloudIacModuleRegistryAuthzProvider::new(
        Arc::new(verifier),
        Arc::new(authorizer),
    )))
}
