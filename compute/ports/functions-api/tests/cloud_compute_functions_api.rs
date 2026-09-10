// ADR-0083 Tier 3: integration tests use `.unwrap()` / `.expect()` /
// `.expect_err()` / `.unwrap_err()` to assert invariants — Tier 3 exemption.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compute_domain::{
    CloudComputeCatalog, CloudComputeError, ComputeRepo, FunctionDeploymentCreate,
    FunctionDeploymentState,
};
use compute_functions_api::{
    CLOUD_COMPUTE_FUNCTIONS_INVOKE_SURFACE, CloudComputeFunctionsApiAuthorization,
    CloudComputeFunctionsApiBoundaryContext, CloudComputeFunctionsApiError,
    CloudComputeFunctionsApiPrincipal, CloudComputeFunctionsAuthorizationDecision,
    CloudComputeFunctionsAuthorizationVerifier, CloudComputeFunctionsInvokeApiRequest,
    CloudComputeFunctionsInvokeApiStatus, CloudComputeFunctionsInvokeIdempotencyLedger,
    CloudComputeFunctionsInvokeRequest, CloudComputeFunctionsTrustedAuthorizationDecision, invoke,
    invoke_cloud_compute_function_from_api,
    invoke_cloud_compute_function_from_api_with_authorization_verifier,
    invoke_with_authorization_verifier,
};
use compute_resource::FunctionRuntime;
use data_boundary_kernel::DataClass;
use network_residency::ResidencyClass;

const FUNCTION_ID: &str = "oyatie:cloud:region-home:ten_alpha:function:image-resize";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const FUNCTIONS_AUTHZ_EVALUATION_EPOCH_SECONDS: u64 = 1_700_100_040;

fn boundary_for(
    request_id: &str,
    idempotency_key: &str,
) -> CloudComputeFunctionsApiBoundaryContext {
    CloudComputeFunctionsApiBoundaryContext {
        request_id: request_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn principal_for(principal_id: &str) -> CloudComputeFunctionsApiPrincipal {
    CloudComputeFunctionsApiPrincipal {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
    }
}

fn authorization_for(
    principal_id: &str,
    surfaces: &[&str],
) -> CloudComputeFunctionsApiAuthorization {
    CloudComputeFunctionsApiAuthorization {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        decision_id: format!("authz_decision_{principal_id}"),
        requested_surface: CLOUD_COMPUTE_FUNCTIONS_INVOKE_SURFACE.to_string(),
        valid_until_epoch_seconds: 1_700_100_060,
        allowed_surfaces: surfaces
            .iter()
            .map(|surface| (*surface).to_string())
            .collect(),
    }
}

fn trusted_allow_for(
    decision_id: &str,
    principal_id: &str,
) -> CloudComputeFunctionsTrustedAuthorizationDecision {
    CloudComputeFunctionsTrustedAuthorizationDecision {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        decision_id: decision_id.to_string(),
        surface: CLOUD_COMPUTE_FUNCTIONS_INVOKE_SURFACE.to_string(),
        decision: CloudComputeFunctionsAuthorizationDecision::Allow,
        valid_until_epoch_seconds: 1_700_100_060,
    }
}

fn authorization_verifier_for(
    request: &CloudComputeFunctionsInvokeApiRequest,
) -> CloudComputeFunctionsAuthorizationVerifier {
    CloudComputeFunctionsAuthorizationVerifier::new(FUNCTIONS_AUTHZ_EVALUATION_EPOCH_SECONDS)
        .with_trusted_decision(trusted_allow_for(
            &request.authorization.decision_id,
            &request.principal.principal_id,
        ))
}

fn invoke_with_trusted_verifier(
    catalog: &mut CloudComputeCatalog,
    idempotency_ledger: &mut CloudComputeFunctionsInvokeIdempotencyLedger,
    request: CloudComputeFunctionsInvokeApiRequest,
) -> Result<
    compute_functions_api::CloudComputeFunctionsInvokeSuccessResponse,
    CloudComputeFunctionsApiError,
> {
    let authorization_verifier = authorization_verifier_for(&request);
    invoke_cloud_compute_function_from_api_with_authorization_verifier(
        catalog,
        idempotency_ledger,
        &authorization_verifier,
        request,
    )
}

fn function_create() -> FunctionDeploymentCreate {
    FunctionDeploymentCreate {
        resource_id: FUNCTION_ID.to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-home".to_string(),
        az: "region-home-a".to_string(),
        cell_id: "cell-region-home-a-001".to_string(),
        runtime: FunctionRuntime::Wasm,
        name: "image-resize".to_string(),
        bundle: format!(
            "function://harbor.region-home.oyatie.io/ten_alpha/image-resize@sha256:{DIGEST}"
        ),
        cold_start_budget_ms: 750,
        timeout_ms: 30_000,
        memory_mb: 512,
        max_concurrency: 250,
        allowed_data_classes: vec![DataClass::Public, DataClass::PiiIdentifying],
        residency: ResidencyClass::StrictHomeRegion,
        state: FunctionDeploymentState::Deploying,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_100_020,
    }
}

fn seed_active_function(catalog: &mut CloudComputeCatalog) {
    let function = catalog
        .register_function(function_create())
        .expect("function registers");
    catalog
        .activate_function(&function.resource_id.value)
        .expect("function activates");
}

fn seed_deploying_function(catalog: &mut CloudComputeCatalog) {
    catalog
        .register_function(function_create())
        .expect("function registers");
}

fn body(invocation_id: &str, payload_data_class: &str) -> CloudComputeFunctionsInvokeRequest {
    CloudComputeFunctionsInvokeRequest {
        invocation_id: invocation_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        function_id: FUNCTION_ID.to_string(),
        region: "region-home".to_string(),
        payload_data_class: payload_data_class.to_string(),
        current_concurrent_invocations: 0,
        requested_at_epoch_seconds: 1_700_100_030,
    }
}

fn request(
    request_id: &str,
    idempotency_key: &str,
    invocation_id: &str,
) -> CloudComputeFunctionsInvokeApiRequest {
    CloudComputeFunctionsInvokeApiRequest {
        path_function_id: FUNCTION_ID.to_string(),
        boundary: boundary_for(request_id, idempotency_key),
        principal: principal_for("sp_compute"),
        authorization: authorization_for("sp_compute", &[CLOUD_COMPUTE_FUNCTIONS_INVOKE_SURFACE]),
        body: body(invocation_id, "PII_IDENTIFYING"),
    }
}

include!("cloud_compute_functions_api/surface_and_replay.rs");
include!("cloud_compute_functions_api/boundary_failures.rs");
include!("cloud_compute_functions_api/idempotency.rs");
include!("cloud_compute_functions_api/policy_and_retention.rs");
