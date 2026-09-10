#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use http_router_kernel::HttpMethod;
use iac_rest::{
    CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE, CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE,
    CLOUD_IAC_MODULE_REGISTRY_REST_ROUTER_NON_CLAIM, CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE,
    CloudIacModuleRegistryRestError, CloudIacModuleRegistryRestRoute,
    MODULE_REGISTRY_DISCOVERY_REST_ROUTE, MODULE_REGISTRY_DOWNLOAD_REST_ROUTE,
    MODULE_REGISTRY_REST_METHOD, MODULE_REGISTRY_VERSIONS_REST_ROUTE,
    match_module_registry_rest_route, module_registry_rest_router, module_registry_route_surfaces,
};

#[test]
fn rest_router_registers_official_opentofu_module_registry_templates() {
    let router = module_registry_rest_router().expect("router registers");
    let routes = router.routes().collect::<Vec<_>>();

    assert_eq!(router.count(), 3);
    assert_eq!(
        routes,
        vec![
            (
                MODULE_REGISTRY_REST_METHOD,
                MODULE_REGISTRY_DISCOVERY_REST_ROUTE
            ),
            (
                MODULE_REGISTRY_REST_METHOD,
                MODULE_REGISTRY_VERSIONS_REST_ROUTE
            ),
            (
                MODULE_REGISTRY_REST_METHOD,
                MODULE_REGISTRY_DOWNLOAD_REST_ROUTE
            ),
        ]
    );
    assert_eq!(
        module_registry_route_surfaces(),
        vec![
            CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE,
            CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE,
            CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE,
        ]
    );
    assert_eq!(
        CLOUD_IAC_MODULE_REGISTRY_REST_ROUTER_NON_CLAIM,
        "framework-free-rest-router-boundary-no-live-http-runtime"
    );
}

#[test]
fn rest_router_matches_low_cardinality_templates_and_captures() {
    let versions = match_module_registry_rest_route(
        HttpMethod::Get,
        "/v1/modules/oyatie/vpc/opentofu/versions",
    )
    .expect("versions route matches");
    assert_eq!(versions.route, CloudIacModuleRegistryRestRoute::Versions);
    assert_eq!(
        versions.matched_template,
        MODULE_REGISTRY_VERSIONS_REST_ROUTE
    );
    assert_eq!(
        versions.required_surface,
        CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE
    );
    assert_eq!(versions.captures.get("namespace").unwrap(), "oyatie");
    assert_eq!(versions.captures.get("name").unwrap(), "vpc");
    assert_eq!(versions.captures.get("system").unwrap(), "opentofu");
    assert!(!versions.matched_template.contains("oyatie"));

    let download = match_module_registry_rest_route(
        HttpMethod::Get,
        "/v1/modules/oyatie/vpc/opentofu/1.2.0/download",
    )
    .expect("download route matches");
    assert_eq!(download.route, CloudIacModuleRegistryRestRoute::Download);
    assert_eq!(
        download.matched_template,
        MODULE_REGISTRY_DOWNLOAD_REST_ROUTE
    );
    assert_eq!(
        download.required_surface,
        CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE
    );
    assert_eq!(download.captures.get("version").unwrap(), "1.2.0");
}

#[test]
fn rest_router_rejects_wrong_methods_unknown_paths_dot_segments_and_whitespace_mutations() {
    let wrong_method = match_module_registry_rest_route(
        HttpMethod::Post,
        "/v1/modules/oyatie/vpc/opentofu/versions",
    )
    .expect_err("POST is not a registry route");
    assert!(matches!(
        wrong_method,
        CloudIacModuleRegistryRestError::RouteNotFound { .. }
    ));

    let unknown = match_module_registry_rest_route(HttpMethod::Get, "/v1/modules/oyatie/vpc")
        .expect_err("incomplete path is not a route");
    assert!(matches!(
        unknown,
        CloudIacModuleRegistryRestError::RouteNotFound { .. }
    ));

    let dot_segment = match_module_registry_rest_route(
        HttpMethod::Get,
        "/v1/modules/oyatie/../opentofu/versions",
    )
    .expect_err("router kernel rejects dot-segment captures");
    assert!(matches!(
        dot_segment,
        CloudIacModuleRegistryRestError::RouteNotFound { .. }
    ));

    let padded = match_module_registry_rest_route(HttpMethod::Get, " /.well-known/terraform.json ")
        .expect_err("path matching is exact");
    assert!(matches!(
        padded,
        CloudIacModuleRegistryRestError::RouteNotFound { .. }
    ));
}

#[test]
fn rest_route_surface_metadata_is_route_specific() {
    assert_eq!(
        CloudIacModuleRegistryRestRoute::Discovery.required_surface(),
        CLOUD_IAC_MODULE_REGISTRY_DISCOVERY_SURFACE
    );
    assert_eq!(
        CloudIacModuleRegistryRestRoute::Versions.required_surface(),
        CLOUD_IAC_MODULE_REGISTRY_VERSIONS_SURFACE
    );
    assert_eq!(
        CloudIacModuleRegistryRestRoute::Download.required_surface(),
        CLOUD_IAC_MODULE_REGISTRY_DOWNLOAD_SURFACE
    );
}
