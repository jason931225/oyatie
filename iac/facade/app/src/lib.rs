//! Cloud IaC application entrypoint composition.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use http_middleware_kernel::{HttpRequest, HttpResponse, MiddlewareChain};
use http_router_kernel::{HttpMethod, Router, RouterError};
use http_runtime_hyper_adapter::{
    HyperRuntimeError, ServerConfig, ServingControl, ServingLimits, SyncHandler,
    dispatch as dispatch_hyper_adapter_request, serve_n_connections_on_std_listener,
    serve_on_std_listener, serve_with_signals_on_std_listener,
};
use iac_api::{
    CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE, CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE,
    CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE, CallerCredential,
    CloudIacModuleRegistryApiBoundaryContext, CloudIacModuleRegistryAuthzProvider,
    ConfiguredBearerPrincipalVerifier, ConfiguredSurfaceAuthorizer,
};
use iac_domain::{CloudIacError, ModuleRegistry, OpenTofuModuleRelease};
use iac_infrastructure::{
    CloudIacModuleRegistryHttpHandler, CloudIacModuleRegistryServiceAssemblyError,
    assemble_module_registry_http_service,
};
use sha2::{Digest, Sha256};

pub const CLOUD_IAC_APP_ENTRYPOINT_NON_CLAIM: &str =
    "local-app-entrypoint-health-and-module-registry-no-deploy-no-production-readiness";
pub const CLOUD_IAC_APP_RELEASE_INDEX_NON_CLAIM: &str =
    "local-release-index-loader-no-registry-publish-no-object-store-no-production-readiness";
pub const CLOUD_IAC_APP_ARTIFACT_DOWNLOAD_NON_CLAIM: &str =
    "local-filesystem-artifact-serving-no-object-store-no-production-readiness";
pub const CLOUD_IAC_APP_ARCHIVE_DIGEST_NON_CLAIM: &str =
    "local-request-time-sha256-check-no-signing-no-slsa-no-object-store";
pub const CLOUD_IAC_APP_REQUEST_AUTH_NON_CLAIM: &str =
    "local-request-bearer-gate-no-production-auth-no-cedar";
pub const CLOUD_IAC_APP_OBJECT_SOURCE_NON_CLAIM: &str =
    "opentofu-s3-gcs-source-location-no-live-object-store-no-upload";
pub const CLOUD_IAC_APP_OBJECT_PINNING_NON_CLAIM: &str =
    "object-source-metadata-pin-no-live-object-store-preconditions";
pub const CLOUD_IAC_APP_BINARY_NAME: &str = "iac-app";
pub const CLOUD_IAC_APP_PACKAGE_NAME: &str = "iac-app";
pub const CLOUD_IAC_APP_BIND_ADDR_ENV: &str = "OYATIE_CLOUD_IAC_BIND_ADDR";
pub const CLOUD_IAC_APP_DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";
pub const CLOUD_IAC_APP_RELEASE_INDEX_PATH_ENV: &str = "OYATIE_CLOUD_IAC_RELEASE_INDEX_PATH";
pub const CLOUD_IAC_APP_MODULE_REGISTRY_BEARER_ENV: &str =
    "OYATIE_CLOUD_IAC_MODULE_REGISTRY_BEARER";
pub const CLOUD_IAC_APP_MODULE_REGISTRY_PRINCIPAL_ENV: &str =
    "OYATIE_CLOUD_IAC_MODULE_REGISTRY_PRINCIPAL";
pub const CLOUD_IAC_APP_DEFAULT_RELEASE_INDEX_PATH: &str = "iac/tofu/modules/release-index.json";
pub const CLOUD_IAC_APP_ARTIFACTS_BASE_PATH: &str = "/artifacts/modules/";
pub const CLOUD_IAC_APP_ARTIFACT_ROUTE_TEMPLATE: &str = "/artifacts/modules/{archive_file}";
pub const CLOUD_IAC_HEALTH_PATH: &str = "/healthz";
pub const CLOUD_IAC_LIVENESS_PATH: &str = "/livez";

const CLOUD_IAC_APP_RELEASE_SOURCE_ROOT: &str = "iac/tofu/modules/";
const CLOUD_IAC_APP_ARCHIVE_FILE_ROOT: &str = "target/iac-app/module-archives/";
const CLOUD_IAC_APP_ARCHIVE_MEDIA_TYPE: &str = "archive/zip";

mod config;
pub use config::*;
mod release_index;
pub use release_index::*;
mod source_validation;
use source_validation::*;
mod artifact_paths;
use artifact_paths::*;
mod json;
use json::*;
mod service;
pub use service::*;
mod artifact_http;
use artifact_http::*;
mod serving;
pub use serving::*;
