//! Framework-free REST router boundary for the Cloud IaC OpenTofu module registry.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use http_router_kernel::{HttpMethod, Router, RouterError};

pub const MODULE_REGISTRY_DISCOVERY_REST_ROUTE: &str = "/.well-known/terraform.json";
pub const MODULE_REGISTRY_VERSIONS_REST_ROUTE: &str =
    "/v1/modules/{namespace}/{name}/{system}/versions";
pub const MODULE_REGISTRY_DOWNLOAD_REST_ROUTE: &str =
    "/v1/modules/{namespace}/{name}/{system}/{version}/download";
pub const MODULE_REGISTRY_REST_METHOD: HttpMethod = HttpMethod::Get;
pub const CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE: &str = "cloud.iac.module_registry.discovery";
pub const CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE: &str = "cloud.iac.module_registry.versions";
pub const CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE: &str = "cloud.iac.module_registry.download";
pub const CLOUD_IAC_MODULE_REGISTRY_REST_ROUTER_NON_CLAIM: &str =
    "framework-free-rest-router-boundary-no-live-http-runtime";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CloudIacModuleRegistryRestRoute {
    Discovery,
    Versions,
    Download,
}

impl CloudIacModuleRegistryRestRoute {
    pub const fn required_surface(self) -> &'static str {
        match self {
            Self::Discovery => CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE,
            Self::Versions => CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE,
            Self::Download => CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudIacModuleRegistryRestMatch {
    pub route: CloudIacModuleRegistryRestRoute,
    pub captures: BTreeMap<String, String>,
    pub matched_template: String,
    pub required_surface: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudIacModuleRegistryRestError {
    Router(RouterError),
    RouteNotFound { method: String, path: String },
}

pub fn module_registry_rest_router()
-> Result<Router<CloudIacModuleRegistryRestRoute>, CloudIacModuleRegistryRestError> {
    let mut router = Router::new();
    router
        .route(
            MODULE_REGISTRY_REST_METHOD,
            MODULE_REGISTRY_DISCOVERY_REST_ROUTE,
            CloudIacModuleRegistryRestRoute::Discovery,
        )
        .map_err(CloudIacModuleRegistryRestError::Router)?;
    router
        .route(
            MODULE_REGISTRY_REST_METHOD,
            MODULE_REGISTRY_VERSIONS_REST_ROUTE,
            CloudIacModuleRegistryRestRoute::Versions,
        )
        .map_err(CloudIacModuleRegistryRestError::Router)?;
    router
        .route(
            MODULE_REGISTRY_REST_METHOD,
            MODULE_REGISTRY_DOWNLOAD_REST_ROUTE,
            CloudIacModuleRegistryRestRoute::Download,
        )
        .map_err(CloudIacModuleRegistryRestError::Router)?;
    Ok(router)
}

pub fn module_registry_route_surfaces() -> Vec<&'static str> {
    vec![
        CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE,
        CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE,
        CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE,
    ]
}

pub fn match_module_registry_rest_route(
    method: HttpMethod,
    path: &str,
) -> Result<CloudIacModuleRegistryRestMatch, CloudIacModuleRegistryRestError> {
    let router = module_registry_rest_router()?;
    let Some((route, captures, matched_template)) = router.match_route(method, path) else {
        return Err(CloudIacModuleRegistryRestError::RouteNotFound {
            method: method.name().to_string(),
            path: path.to_string(),
        });
    };

    Ok(CloudIacModuleRegistryRestMatch {
        route: *route,
        captures,
        matched_template: matched_template.to_string(),
        required_surface: route.required_surface().to_string(),
    })
}
