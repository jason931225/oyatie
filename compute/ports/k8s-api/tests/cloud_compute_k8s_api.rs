// ADR-0083 Tier 3: integration tests use `.unwrap()` / `.expect()` /
// `.expect_err()` / `.unwrap_err()` to assert invariants — Tier 3 exemption.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use compute_domain::CloudComputeError;
use compute_k8s_api::{
    CLOUD_COMPUTE_K8S_CLUSTER_CREATE_SURFACE, CLOUD_COMPUTE_K8S_CLUSTER_DELETE_SURFACE,
    CloudComputeK8sApiAuthorization, CloudComputeK8sApiAuthorizationProof,
    CloudComputeK8sApiBoundaryContext, CloudComputeK8sApiError, CloudComputeK8sApiPrincipal,
    CloudComputeK8sClusterCreateApiRequest, CloudComputeK8sClusterCreateApiStatus,
    CloudComputeK8sClusterCreateRequest, CloudComputeK8sClusterCreateSuccessResponse,
    CloudComputeK8sClusterDeleteApiRequest, CloudComputeK8sClusterDeleteApiStatus,
    CloudComputeK8sClusterDeleteSuccessResponse, CloudComputeK8sClusterRecord,
    CloudComputeK8sCreateCommand, CloudComputeK8sCreateReceipt, CloudComputeK8sDeleteCommand,
    CloudComputeK8sDeleteReceipt, CloudComputeK8sLifecycleRepository,
    CloudComputeK8sLifecycleRepositoryError, CloudComputeK8sNodePoolCreateRequest,
    CloudComputeK8sNodePoolFlavorSpec, CloudComputeK8sOperationKey, CloudComputeK8sQuotaEnvelope,
    CloudComputeK8sRepositoryFuture, CloudComputeK8sSecurityGroupRef,
    CloudComputeK8sTrustedAuthorizationVerifier,
    create_cloud_compute_k8s_cluster_from_api as create_cloud_compute_k8s_cluster_from_api_without_authorization_verifier,
    create_cloud_compute_k8s_cluster_from_api_with_authorization_verifier,
    create_cluster as create_cluster_without_authorization_verifier,
    create_cluster_with_authorization_verifier,
    delete_cloud_compute_k8s_cluster_from_api as delete_cloud_compute_k8s_cluster_from_api_without_authorization_verifier,
    delete_cloud_compute_k8s_cluster_from_api_with_authorization_verifier,
    delete_cluster as delete_cluster_without_authorization_verifier,
    delete_cluster_with_authorization_verifier,
    validate_cloud_compute_k8s_cluster_record_projection,
};

const CLUSTER_ID: &str = "oyatie:cloud:region-home:ten_alpha:k8s:prod";
const K8S_AUTHZ_EVALUATION_EPOCH_SECONDS: u64 = 1_700_099_500;

fn boundary_for(request_id: &str, idempotency_key: &str) -> CloudComputeK8sApiBoundaryContext {
    CloudComputeK8sApiBoundaryContext {
        request_id: request_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn principal_for(principal_id: &str) -> CloudComputeK8sApiPrincipal {
    CloudComputeK8sApiPrincipal {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
    }
}

fn authorization_for(principal_id: &str, surfaces: &[&str]) -> CloudComputeK8sApiAuthorization {
    let decision_id = format!("authz_decision_{principal_id}");
    CloudComputeK8sApiAuthorization {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        decision_id: decision_id.clone(),
        allowed_surfaces: surfaces
            .iter()
            .map(|surface| (*surface).to_string())
            .collect(),
        proof: Some(authorization_proof_for(
            principal_id,
            CLOUD_COMPUTE_K8S_CLUSTER_CREATE_SURFACE,
            &decision_id,
        )),
    }
}
fn authorization_proof_for(
    principal_id: &str,
    surface: &str,
    decision_id: &str,
) -> CloudComputeK8sApiAuthorizationProof {
    CloudComputeK8sApiAuthorizationProof {
        tenant_id: "ten_alpha".to_string(),
        principal_id: principal_id.to_string(),
        surface: surface.to_string(),
        decision_id: decision_id.to_string(),
        verified: true,
        issued_at_epoch_seconds: 1_700_099_000,
        expires_at_epoch_seconds: 1_700_100_000,
    }
}

fn quota() -> CloudComputeK8sQuotaEnvelope {
    CloudComputeK8sQuotaEnvelope {
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

fn node_flavor() -> CloudComputeK8sNodePoolFlavorSpec {
    CloudComputeK8sNodePoolFlavorSpec {
        class: "general_purpose".to_string(),
        vcpu: 4,
        memory_gb: 16,
        gpu_count: 0,
        local_ssd_gb: 100,
    }
}

fn node_pool(id: &str, az: &str, subnet: &str) -> CloudComputeK8sNodePoolCreateRequest {
    CloudComputeK8sNodePoolCreateRequest {
        id: id.to_string(),
        az: az.to_string(),
        cell_id: format!("cell-{az}-001"),
        subnet_id: subnet.to_string(),
        security_groups: vec![
            CloudComputeK8sSecurityGroupRef {
                value: format!("sg_{id}_web"),
                tenant_id: "ten_alpha".to_string(),
                region: "region-home".to_string(),
                subnet_id: subnet.to_string(),
            },
            CloudComputeK8sSecurityGroupRef {
                value: format!("sg_{id}_app"),
                tenant_id: "ten_alpha".to_string(),
                region: "region-home".to_string(),
                subnet_id: subnet.to_string(),
            },
        ],
        flavor: node_flavor(),
        min_nodes: 1,
        max_nodes: 5,
        autoscaling_enabled: true,
    }
}

fn body(resource_id: &str) -> CloudComputeK8sClusterCreateRequest {
    CloudComputeK8sClusterCreateRequest {
        resource_id: resource_id.to_string(),
        tenant_id: "ten_alpha".to_string(),
        region: "region-home".to_string(),
        flavor: "high_availability".to_string(),
        control_plane_version: "v1.30.2-oyatie.1".to_string(),
        control_plane_private: true,
        node_pools: vec![
            node_pool(
                "np_a",
                "region-home-a",
                "oyatie:cloud:region-home:ten_alpha:subnet:prod-a",
            ),
            node_pool(
                "np_b",
                "region-home-b",
                "oyatie:cloud:region-home:ten_alpha:subnet:prod-b",
            ),
            node_pool(
                "np_c",
                "region-home-c",
                "oyatie:cloud:region-home:ten_alpha:subnet:prod-c",
            ),
        ],
        quota: quota(),
        residency: "strict_home_region".to_string(),
        data_class: "PUBLIC".to_string(),
        created_at_epoch_seconds: 1_700_100_010,
    }
}

fn request(request_id: &str, idempotency_key: &str) -> CloudComputeK8sClusterCreateApiRequest {
    CloudComputeK8sClusterCreateApiRequest {
        path_cluster_id: CLUSTER_ID.to_string(),
        boundary: boundary_for(request_id, idempotency_key),
        principal: principal_for("sp_compute"),
        authorization: authorization_for("sp_compute", &[CLOUD_COMPUTE_K8S_CLUSTER_CREATE_SURFACE]),
        body: body(CLUSTER_ID),
    }
}

fn trusted_create_verifier_for(
    request: &CloudComputeK8sClusterCreateApiRequest,
) -> CloudComputeK8sTrustedAuthorizationVerifier {
    CloudComputeK8sTrustedAuthorizationVerifier::new(K8S_AUTHZ_EVALUATION_EPOCH_SECONDS)
        .with_authorization_proof(authorization_proof_for(
            &request.principal.principal_id,
            CLOUD_COMPUTE_K8S_CLUSTER_CREATE_SURFACE,
            &request.authorization.decision_id,
        ))
}

async fn create_cloud_compute_k8s_cluster_from_api(
    repository: &LifecycleTestRepository,
    request: CloudComputeK8sClusterCreateApiRequest,
) -> Result<CloudComputeK8sClusterCreateSuccessResponse, CloudComputeK8sApiError> {
    let verifier = trusted_create_verifier_for(&request);
    create_cloud_compute_k8s_cluster_from_api_with_authorization_verifier(
        repository, request, &verifier,
    )
    .await
}

async fn create_cluster(
    repository: &LifecycleTestRepository,
    request: CloudComputeK8sClusterCreateApiRequest,
) -> Result<CloudComputeK8sClusterCreateSuccessResponse, CloudComputeK8sApiError> {
    let verifier = trusted_create_verifier_for(&request);
    create_cluster_with_authorization_verifier(repository, request, &verifier).await
}

#[path = "cloud_compute_k8s_api/lifecycle_test_repository.rs"]
mod lifecycle_test_repository;
use lifecycle_test_repository::LifecycleTestRepository;

include!("cloud_compute_k8s_api/create_replay.rs");
include!("cloud_compute_k8s_api/create_authorization.rs");
include!("cloud_compute_k8s_api/create_validation.rs");
include!("cloud_compute_k8s_api/delete_support.rs");
include!("cloud_compute_k8s_api/delete_success.rs");
include!("cloud_compute_k8s_api/delete_authorization.rs");
include!("cloud_compute_k8s_api/delete_idempotency.rs");

#[path = "cloud_compute_k8s_api/acceptance.rs"]
mod acceptance;
#[path = "cloud_compute_k8s_api/acceptance_authorization.rs"]
mod acceptance_authorization;
#[path = "cloud_compute_k8s_api/acceptance_identity.rs"]
mod acceptance_identity;
#[path = "cloud_compute_k8s_api/acceptance_integrity.rs"]
mod acceptance_integrity;
#[path = "cloud_compute_k8s_api/acceptance_receipt.rs"]
mod acceptance_receipt;
#[path = "cloud_compute_k8s_api/acceptance_test_repository.rs"]
mod acceptance_test_repository;
