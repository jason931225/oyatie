// ADR-0083 Tier 3: integration tests use `.unwrap()` / `.expect()` /
// `.expect_err()` / `.unwrap_err()` to assert invariants — Tier 3 exemption.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use compute_domain::{CloudComputeCatalog, CloudComputeError};
use compute_vm_api::{
    CLOUD_COMPUTE_VM_CREATE_SURFACE, CloudComputeVmApiAuthorization,
    CloudComputeVmApiAuthorizationProof, CloudComputeVmApiBoundaryContext, CloudComputeVmApiError,
    CloudComputeVmApiPrincipal, CloudComputeVmCreateApiRequest, CloudComputeVmCreateApiStatus,
    CloudComputeVmCreateIdempotencyLedger, CloudComputeVmCreateRequest,
    CloudComputeVmCreateSuccessResponse, CloudComputeVmFlavorSpec, CloudComputeVmIamRoleRef,
    CloudComputeVmQuotaEnvelope, CloudComputeVmSecurityGroupRef,
    CloudComputeVmTrustedAuthorizationVerifier, create_cloud_compute_vm_from_api,
    create_cloud_compute_vm_from_api_with_verifier,
};

const INSTANCE_ID: &str = "oyatie:cloud:region-home:ten_alpha:instance:app-1";
const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const VERIFIER_EVALUATION_EPOCH_SECONDS: u64 = 1_700_099_500;

fn boundary_for(request_id: &str, idempotency_key: &str) -> CloudComputeVmApiBoundaryContext {
    CloudComputeVmApiBoundaryContext {
        request_id: request_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn principal_for(principal_id: &str) -> CloudComputeVmApiPrincipal {
    CloudComputeVmApiPrincipal {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
    }
}

fn authorization_for(principal_id: &str, surfaces: &[&str]) -> CloudComputeVmApiAuthorization {
    let decision_id = format!("authz_decision_{principal_id}");
    CloudComputeVmApiAuthorization {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        decision_id: decision_id.clone(),
        allowed_surfaces: surfaces
            .iter()
            .map(|surface| (*surface).to_string())
            .collect(),
        proof: Some(authorization_proof_for(
            principal_id,
            CLOUD_COMPUTE_VM_CREATE_SURFACE,
            &decision_id,
        )),
    }
}
fn authorization_proof_for(
    principal_id: &str,
    surface: &str,
    decision_id: &str,
) -> CloudComputeVmApiAuthorizationProof {
    CloudComputeVmApiAuthorizationProof {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        surface: surface.to_string(),
        decision_id: decision_id.to_string(),
        verified: true,
        issued_at_epoch_seconds: 1_700_099_000,
        expires_at_epoch_seconds: 1_700_100_000,
    }
}

fn trusted_verifier_for(
    request: &CloudComputeVmCreateApiRequest,
) -> CloudComputeVmTrustedAuthorizationVerifier {
    CloudComputeVmTrustedAuthorizationVerifier::new(VERIFIER_EVALUATION_EPOCH_SECONDS)
        .with_trusted_proof(authorization_proof_for(
            &request.principal.principal_id,
            CLOUD_COMPUTE_VM_CREATE_SURFACE,
            &request.authorization.decision_id,
        ))
}

fn trusted_verifier_with(
    proof: CloudComputeVmApiAuthorizationProof,
) -> CloudComputeVmTrustedAuthorizationVerifier {
    CloudComputeVmTrustedAuthorizationVerifier::new(VERIFIER_EVALUATION_EPOCH_SECONDS)
        .with_trusted_proof(proof)
}

fn create_vm_with_trusted_verifier(
    catalog: &mut CloudComputeCatalog,
    ledger: &mut CloudComputeVmCreateIdempotencyLedger,
    request: CloudComputeVmCreateApiRequest,
) -> Result<CloudComputeVmCreateSuccessResponse, CloudComputeVmApiError> {
    let verifier = trusted_verifier_for(&request);
    create_cloud_compute_vm_from_api_with_verifier(catalog, ledger, request, &verifier)
}

fn authorization_denied() -> CloudComputeVmApiError {
    CloudComputeVmApiError::AuthorizationDenied {
        surface: CLOUD_COMPUTE_VM_CREATE_SURFACE.to_string(),
    }
}

fn quota() -> CloudComputeVmQuotaEnvelope {
    CloudComputeVmQuotaEnvelope {
        vcpu_limit: 128,
        memory_gb_limit: 512,
        gpu_limit: 8,
        local_ssd_gb_limit: 4_096,
        current_vcpu: 4,
        current_memory_gb: 16,
        current_gpu: 0,
        current_local_ssd_gb: 100,
    }
}

fn flavor() -> CloudComputeVmFlavorSpec {
    CloudComputeVmFlavorSpec {
        class: "general_purpose".to_string(),
        vcpu: 4,
        memory_gb: 16,
        gpu_count: 0,
        local_ssd_gb: 100,
    }
}

fn body(resource_id: &str) -> CloudComputeVmCreateRequest {
    CloudComputeVmCreateRequest {
        resource_id: resource_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-home".to_string(),
        az: "region-home-a".to_string(),
        cell_id: "cell-region-home-a-001".to_string(),
        flavor: flavor(),
        image: format!("oci://harbor.region-home.oyatie.io/ten_alpha/app@sha256:{DIGEST}"),
        key_pair: Some("key_prod".to_string()),
        vpc_id: "oyatie:cloud:region-home:ten_alpha:vpc:prod".to_string(),
        subnet_id: "oyatie:cloud:region-home:ten_alpha:subnet:prod-a".to_string(),
        security_groups: vec![
            CloudComputeVmSecurityGroupRef {
                value: "sg_web".to_string(),
                tenant_id: "ten_alpha".to_string(),
                region: "region-home".to_string(),
                vpc_id: "oyatie:cloud:region-home:ten_alpha:vpc:prod".to_string(),
            },
            CloudComputeVmSecurityGroupRef {
                value: "sg_app".to_string(),
                tenant_id: "ten_alpha".to_string(),
                region: "region-home".to_string(),
                vpc_id: "oyatie:cloud:region-home:ten_alpha:vpc:prod".to_string(),
            },
        ],
        iam_role: Some(CloudComputeVmIamRoleRef {
            value: "role_app".to_string(),
            tenant_id: "ten_alpha".to_string(),
            region: "region-home".to_string(),
            vpc_id: "oyatie:cloud:region-home:ten_alpha:vpc:prod".to_string(),
        }),
        user_data_uri: Some("userdata/ten_alpha/app-1/cloud-init.yaml".to_string()),
        quota: quota(),
        residency: "strict_home_region".to_string(),
        data_class: "PUBLIC".to_string(),
        created_at_epoch_seconds: 1_700_100_000,
    }
}

fn request(request_id: &str, idempotency_key: &str) -> CloudComputeVmCreateApiRequest {
    CloudComputeVmCreateApiRequest {
        path_instance_id: INSTANCE_ID.to_string(),
        boundary: boundary_for(request_id, idempotency_key),
        principal: principal_for("sp_compute"),
        authorization: authorization_for("sp_compute", &[CLOUD_COMPUTE_VM_CREATE_SURFACE]),
        body: body(INSTANCE_ID),
    }
}

include!("cloud_compute_vm_api/surface_and_replay.rs");
include!("cloud_compute_vm_api/boundary_validation.rs");
include!("cloud_compute_vm_api/authorization.rs");
include!("cloud_compute_vm_api/resource_validation.rs");
include!("cloud_compute_vm_api/data_class_and_retention.rs");
