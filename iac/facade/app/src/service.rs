use super::*;

pub struct CloudIacAppService {
    pub(super) router: Router<SyncHandler>,
    pub(super) middleware: MiddlewareChain<HttpRequest, HttpResponse>,
    pub(super) server_config: ServerConfig,
}

impl CloudIacAppService {
    pub fn route_count(&self) -> usize {
        self.router.count()
    }

    pub fn middleware_count(&self) -> usize {
        self.middleware.count()
    }

    pub fn server_config(&self) -> &ServerConfig {
        &self.server_config
    }

    pub fn into_serve_parts(
        self,
    ) -> (
        Router<SyncHandler>,
        MiddlewareChain<HttpRequest, HttpResponse>,
        ServerConfig,
    ) {
        (self.router, self.middleware, self.server_config)
    }
}

#[derive(Debug)]
pub enum CloudIacAppError {
    Config(CloudIacAppConfigError),
    Bind(String),
    ReleaseIndex(CloudIacReleaseIndexError),
    RegistryService(CloudIacModuleRegistryServiceAssemblyError),
    Router(RouterError),
    Hyper(HyperRuntimeError),
}

impl std::fmt::Display for CloudIacAppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => write!(f, "iac-app app config failed: {error:?}"),
            Self::Bind(reason) => write!(f, "iac-app app bind failed: {reason}"),
            Self::ReleaseIndex(error) => {
                write!(f, "iac-app release-index load failed: {error:?}")
            }
            Self::RegistryService(error) => {
                write!(f, "iac-app module registry assembly failed: {error:?}")
            }
            Self::Router(error) => write!(f, "iac-app app route registration failed: {error:?}"),
            Self::Hyper(error) => write!(f, "iac-app hyper runtime failed: {error}"),
        }
    }
}

impl std::error::Error for CloudIacAppError {}

impl From<CloudIacAppConfigError> for CloudIacAppError {
    fn from(value: CloudIacAppConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<CloudIacReleaseIndexError> for CloudIacAppError {
    fn from(value: CloudIacReleaseIndexError) -> Self {
        Self::ReleaseIndex(value)
    }
}

impl From<CloudIacModuleRegistryServiceAssemblyError> for CloudIacAppError {
    fn from(value: CloudIacModuleRegistryServiceAssemblyError) -> Self {
        Self::RegistryService(value)
    }
}

impl From<RouterError> for CloudIacAppError {
    fn from(value: RouterError) -> Self {
        Self::Router(value)
    }
}

impl From<HyperRuntimeError> for CloudIacAppError {
    fn from(value: HyperRuntimeError) -> Self {
        Self::Hyper(value)
    }
}

pub fn build_iac_app_service_from_release_index_path(
    path: impl AsRef<Path>,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
) -> Result<CloudIacAppService, CloudIacAppError> {
    let seed = load_release_index_seed_from_path(path)?;
    build_iac_app_service_from_release_index_seed(&seed, authz_provider)
}

pub fn build_iac_app_service_from_release_index_str(
    input: &str,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
) -> Result<CloudIacAppService, CloudIacAppError> {
    let seed = load_release_index_seed_from_str(input)?;
    build_iac_app_service_from_release_index_seed(&seed, authz_provider)
}

pub fn build_iac_app_service_from_release_index_seed(
    seed: &CloudIacReleaseIndexSeed,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
) -> Result<CloudIacAppService, CloudIacAppError> {
    let registry = build_module_registry_from_release_index_seed(seed)?;
    let artifacts = archive_artifacts_from_seed(seed)?;
    build_iac_app_service_with_artifacts(
        registry,
        iac_app_bootstrap_boundary(),
        authz_provider,
        artifacts,
    )
}

pub(super) fn iac_app_bootstrap_boundary() -> CloudIacModuleRegistryApiBoundaryContext {
    CloudIacModuleRegistryApiBoundaryContext {
        request_id: "req_iac_app_bootstrap_local_001".to_string(),
    }
}

pub fn build_iac_app_service(
    registry: ModuleRegistry,
    boundary: CloudIacModuleRegistryApiBoundaryContext,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
) -> Result<CloudIacAppService, CloudIacAppError> {
    build_iac_app_service_with_artifacts(registry, boundary, authz_provider, BTreeMap::new())
}

pub(super) fn build_iac_app_service_with_artifacts(
    registry: ModuleRegistry,
    boundary: CloudIacModuleRegistryApiBoundaryContext,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
    artifacts: BTreeMap<String, CloudIacAppArchiveArtifact>,
) -> Result<CloudIacAppService, CloudIacAppError> {
    // The infra handler holds the provider and does the module-registry PEP
    // (discovery/versions/download). The artifact route is the SECOND PEP and
    // shares the SAME provider — both gate on a verified principal + PDP decision.
    let registry_service = assemble_module_registry_http_service(
        CloudIacModuleRegistryHttpHandler::new(registry, boundary, authz_provider.clone()),
    )?;
    let (mut router, middleware, server_config) = registry_service.into_serve_parts();
    register_artifact_routes(&mut router, artifacts, authz_provider)?;
    register_health_routes(&mut router)?;
    Ok(CloudIacAppService {
        router,
        middleware,
        server_config,
    })
}

pub(super) fn register_artifact_routes(
    router: &mut Router<SyncHandler>,
    artifacts: BTreeMap<String, CloudIacAppArchiveArtifact>,
    authz_provider: Arc<CloudIacModuleRegistryAuthzProvider>,
) -> Result<(), RouterError> {
    if artifacts.is_empty() {
        return Ok(());
    }
    router.route(
        HttpMethod::Get,
        CLOUD_IAC_APP_ARTIFACT_ROUTE_TEMPLATE,
        archive_artifact_handler(Arc::new(artifacts), authz_provider),
    )?;
    Ok(())
}
