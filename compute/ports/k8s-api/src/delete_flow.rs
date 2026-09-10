pub fn validate_cloud_compute_k8s_cluster_delete_request(
    request: &CloudComputeK8sClusterDeleteApiRequest,
) -> Result<ResourceId, CloudComputeK8sApiError> {
    validate_cloud_compute_k8s_cluster_delete_request_with_authorization_verifier(
        request,
        &CloudComputeK8sFailClosedAuthorizationVerifier,
    )
}

pub fn validate_cloud_compute_k8s_cluster_delete_request_with_authorization_verifier(
    request: &CloudComputeK8sClusterDeleteApiRequest,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<ResourceId, CloudComputeK8sApiError> {
    validate_boundary(&request.boundary)?;
    validate_path_cluster_id_only(&request.path_cluster_id)?;
    let resource_id = validate_cluster_resource_id(&request.path_cluster_id)?;
    validate_delete_tenant_binding(&request.boundary, &request.principal, &resource_id)?;
    validate_authorization(
        &request.principal,
        &request.authorization.decision_id,
        CLOUD_COMPUTE_K8S_CLUSTER_DELETE_SURFACE,
        authorization_verifier,
    )?;
    Ok(resource_id)
}

pub async fn delete_cloud_compute_k8s_cluster_from_api(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterDeleteApiRequest,
) -> Result<CloudComputeK8sClusterDeleteSuccessResponse, CloudComputeK8sApiError> {
    let resource_id = validate_cloud_compute_k8s_cluster_delete_request(&request)?;
    delete_validated_cloud_compute_k8s_cluster(repository, request, resource_id).await
}

pub async fn delete_cloud_compute_k8s_cluster_from_api_with_authorization_verifier(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterDeleteApiRequest,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<CloudComputeK8sClusterDeleteSuccessResponse, CloudComputeK8sApiError> {
    let resource_id =
        validate_cloud_compute_k8s_cluster_delete_request_with_authorization_verifier(
            &request,
            authorization_verifier,
        )?;
    delete_validated_cloud_compute_k8s_cluster(repository, request, resource_id).await
}

async fn delete_validated_cloud_compute_k8s_cluster(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterDeleteApiRequest,
    resource_id: ResourceId,
) -> Result<CloudComputeK8sClusterDeleteSuccessResponse, CloudComputeK8sApiError> {
    let cluster_id = request.path_cluster_id.clone();
    let expected_resource_id = resource_id.clone();
    let expected_tenant_id = request.boundary.tenant_id.clone();
    let receipt = repository
        .commit_deletion(CloudComputeK8sDeleteCommand {
            operation_key: CloudComputeK8sOperationKey {
                tenant_id: request.boundary.tenant_id,
                principal_id: request.principal.principal_id,
                surface: CLOUD_COMPUTE_K8S_CLUSTER_DELETE_SURFACE.to_string(),
                idempotency_key: request.boundary.idempotency_key,
            },
            resource_id,
            request_id: request.boundary.request_id,
        })
        .await
        .map_err(|error| delete_repository_error(error, cluster_id))?;
    validate_delete_receipt(&receipt, &expected_resource_id, &expected_tenant_id)?;
    Ok(CloudComputeK8sClusterDeleteSuccessResponse::accepted(
        receipt.cluster,
        receipt.request_id,
    ))
}

fn validate_delete_receipt(
    receipt: &CloudComputeK8sDeleteReceipt,
    expected_resource_id: &ResourceId,
    expected_tenant_id: &str,
) -> Result<(), CloudComputeK8sApiError> {
    let resource_region = expected_resource_id
        .region()
        .map_err(|_| CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation)?;
    let residency = parse_residency_class_label(&receipt.cluster.residency)
        .ok_or(CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation)?;
    let valid_node_pool_count = match receipt.cluster.flavor.as_str() {
        "standard" => receipt.cluster.node_pool_count >= 1,
        "high_availability" => receipt.cluster.node_pool_count >= 3,
        _ => false,
    };
    if receipt.request_id.trim().is_empty()
        || receipt.cluster.resource_id != expected_resource_id.value
        || receipt.cluster.tenant_id != expected_tenant_id
        || receipt.cluster.region != resource_region.value
        || ControlPlaneVersion::new(receipt.cluster.control_plane_version.clone()).is_err()
        || !valid_node_pool_count
        || !residency_class_allows_home_region_label(&residency, &receipt.cluster.region)
        || !matches!(
            receipt.cluster.state.as_str(),
            "creating" | "ready" | "reconciling" | "draining" | "deleted"
        )
        || receipt.cluster.desired_state != "deleted"
        || receipt.cluster.data_class != "PUBLIC"
        || receipt.cluster.schema_version != CLOUD_COMPUTE_K8S_CLUSTER_RECORD_SCHEMA_VERSION
    {
        return Err(CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation);
    }
    Ok(())
}

fn delete_repository_error(
    error: CloudComputeK8sLifecycleRepositoryError,
    cluster_id: String,
) -> CloudComputeK8sApiError {
    match error {
        CloudComputeK8sLifecycleRepositoryError::ClusterNotFound => {
            CloudComputeK8sApiError::ClusterNotFound { cluster_id }
        }
        CloudComputeK8sLifecycleRepositoryError::IdempotencyKeyReused { idempotency_key } => {
            CloudComputeK8sApiError::IdempotencyKeyReused { idempotency_key }
        },
        CloudComputeK8sLifecycleRepositoryError::Unavailable => {
            CloudComputeK8sApiError::LifecycleRepositoryUnavailable
        }
        CloudComputeK8sLifecycleRepositoryError::ClusterAlreadyExists
        | CloudComputeK8sLifecycleRepositoryError::IntegrityViolation => {
            CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation
        }
    }
}

pub use crate::delete_cloud_compute_k8s_cluster_from_api as delete_cluster;

pub async fn delete_cluster_with_authorization_verifier(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterDeleteApiRequest,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<CloudComputeK8sClusterDeleteSuccessResponse, CloudComputeK8sApiError> {
    delete_cloud_compute_k8s_cluster_from_api_with_authorization_verifier(
        repository,
        request,
        authorization_verifier,
    )
    .await
}

fn validate_path_cluster_id_only(path_cluster_id: &str) -> Result<(), CloudComputeK8sApiError> {
    if path_cluster_id.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyPathClusterId);
    }
    Ok(())
}

fn validate_delete_tenant_binding(
    boundary: &CloudComputeK8sApiBoundaryContext,
    principal: &CloudComputeK8sApiPrincipal,
    resource_id: &ResourceId,
) -> Result<(), CloudComputeK8sApiError> {
    if principal.principal_id.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyPrincipalId);
    }
    let resource_tenant_id =
        resource_id
            .tenant_id()
            .map_err(|_| CloudComputeK8sApiError::InvalidClusterId {
                cluster_id: resource_id.value.clone(),
            })?;
    if boundary.tenant_id != principal.tenant_id || boundary.tenant_id != resource_tenant_id {
        return Err(CloudComputeK8sApiError::TenantMismatch {
            header_tenant_id: boundary.tenant_id.clone(),
            principal_tenant_id: principal.tenant_id.clone(),
            resource_tenant_id,
            body_tenant_id: String::new(),
        });
    }
    Ok(())
}
