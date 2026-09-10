pub fn validate_cloud_compute_k8s_cluster_create_request(
    request: &CloudComputeK8sClusterCreateApiRequest,
) -> Result<ResourceId, CloudComputeK8sApiError> {
    validate_cloud_compute_k8s_cluster_create_request_with_authorization_verifier(
        request,
        &CloudComputeK8sFailClosedAuthorizationVerifier,
    )
}

pub fn validate_cloud_compute_k8s_cluster_create_request_with_authorization_verifier(
    request: &CloudComputeK8sClusterCreateApiRequest,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<ResourceId, CloudComputeK8sApiError> {
    validate_boundary(&request.boundary)?;
    validate_path_cluster_id(&request.path_cluster_id, &request.body.resource_id)?;
    let resource_id = validate_cluster_resource_id(&request.path_cluster_id)?;
    validate_tenant_binding(
        &request.boundary,
        &request.principal,
        &resource_id,
        &request.body.tenant_id,
    )?;
    validate_authorization(
        &request.principal,
        &request.authorization.decision_id,
        CLOUD_COMPUTE_K8S_CLUSTER_CREATE_SURFACE,
        authorization_verifier,
    )?;
    Ok(resource_id)
}

pub async fn create_cloud_compute_k8s_cluster_from_api(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterCreateApiRequest,
) -> Result<CloudComputeK8sClusterCreateSuccessResponse, CloudComputeK8sApiError> {
    validate_cloud_compute_k8s_cluster_create_request(&request)?;
    create_validated_cloud_compute_k8s_cluster(repository, request).await
}

pub async fn create_cloud_compute_k8s_cluster_from_api_with_authorization_verifier(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterCreateApiRequest,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<CloudComputeK8sClusterCreateSuccessResponse, CloudComputeK8sApiError> {
    validate_cloud_compute_k8s_cluster_create_request_with_authorization_verifier(
        &request,
        authorization_verifier,
    )?;
    create_validated_cloud_compute_k8s_cluster(repository, request).await
}

async fn create_validated_cloud_compute_k8s_cluster(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterCreateApiRequest,
) -> Result<CloudComputeK8sClusterCreateSuccessResponse, CloudComputeK8sApiError> {
    let input = cluster_create_input(&request.body)?;
    let fingerprint = cluster_create_fingerprint_for(&request.path_cluster_id, &input).canonical;
    let cluster = KubernetesCluster::new(input).map_err(CloudComputeK8sApiError::Compute)?;
    let expected_cluster = cluster_record(cluster);
    let operation_key = idempotency_key_for(
        &request.boundary,
        &request.principal,
        CLOUD_COMPUTE_K8S_CLUSTER_CREATE_SURFACE,
    );
    let receipt = repository
        .commit_create(CloudComputeK8sCreateCommand {
            operation_key,
            fingerprint,
            desired_spec: request.body,
            cluster: expected_cluster.clone(),
            request_id: request.boundary.request_id,
        })
        .await
        .map_err(create_repository_error)?;
    if receipt.cluster != expected_cluster {
        return Err(CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation);
    }
    Ok(CloudComputeK8sClusterCreateSuccessResponse::created(
        receipt.cluster,
        receipt.request_id,
    ))
}

fn create_repository_error(
    error: CloudComputeK8sLifecycleRepositoryError,
) -> CloudComputeK8sApiError {
    match error {
        CloudComputeK8sLifecycleRepositoryError::ClusterAlreadyExists => {
            CloudComputeK8sApiError::Compute(CloudComputeError::DuplicateKubernetesCluster)
        }
        CloudComputeK8sLifecycleRepositoryError::IdempotencyKeyReused { idempotency_key } => {
            CloudComputeK8sApiError::IdempotencyKeyReused { idempotency_key }
        }
        CloudComputeK8sLifecycleRepositoryError::Unavailable => {
            CloudComputeK8sApiError::LifecycleRepositoryUnavailable
        }
        CloudComputeK8sLifecycleRepositoryError::ClusterNotFound
        | CloudComputeK8sLifecycleRepositoryError::IntegrityViolation => {
            CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation
        }
    }
}

pub use crate::create_cloud_compute_k8s_cluster_from_api as create_cluster;

pub async fn create_cluster_with_authorization_verifier(
    repository: &impl CloudComputeK8sLifecycleRepository,
    request: CloudComputeK8sClusterCreateApiRequest,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<CloudComputeK8sClusterCreateSuccessResponse, CloudComputeK8sApiError> {
    create_cloud_compute_k8s_cluster_from_api_with_authorization_verifier(
        repository,
        request,
        authorization_verifier,
    )
    .await
}

fn validate_boundary(
    boundary: &CloudComputeK8sApiBoundaryContext,
) -> Result<(), CloudComputeK8sApiError> {
    if boundary.request_id.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyRequestId);
    }
    if boundary.tenant_id.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyTenantHeader);
    }
    if boundary.idempotency_key.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyIdempotencyKey);
    }
    Ok(())
}

fn validate_path_cluster_id(
    path_cluster_id: &str,
    body_resource_id: &str,
) -> Result<(), CloudComputeK8sApiError> {
    if path_cluster_id.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyPathClusterId);
    }
    if path_cluster_id != body_resource_id {
        return Err(CloudComputeK8sApiError::ClusterIdMismatch {
            path_cluster_id: path_cluster_id.to_string(),
            body_resource_id: body_resource_id.to_string(),
        });
    }
    Ok(())
}

fn validate_cluster_resource_id(value: &str) -> Result<ResourceId, CloudComputeK8sApiError> {
    let id = ResourceId::new(value.to_string()).map_err(|_| {
        CloudComputeK8sApiError::InvalidClusterId {
            cluster_id: value.to_string(),
        }
    })?;
    let kind_label = id
        .kind_label()
        .map_err(|_| CloudComputeK8sApiError::InvalidClusterId {
            cluster_id: value.to_string(),
        })?;
    if kind_label != "k8s" {
        return Err(CloudComputeK8sApiError::ClusterKindMismatch {
            cluster_id: value.to_string(),
            kind_label,
        });
    }
    Ok(id)
}

fn validate_tenant_binding(
    boundary: &CloudComputeK8sApiBoundaryContext,
    principal: &CloudComputeK8sApiPrincipal,
    resource_id: &ResourceId,
    body_tenant_id: &str,
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
    if boundary.tenant_id != principal.tenant_id
        || boundary.tenant_id != resource_tenant_id
        || boundary.tenant_id != body_tenant_id
    {
        return Err(CloudComputeK8sApiError::TenantMismatch {
            header_tenant_id: boundary.tenant_id.clone(),
            principal_tenant_id: principal.tenant_id.clone(),
            resource_tenant_id,
            body_tenant_id: body_tenant_id.to_string(),
        });
    }
    Ok(())
}

fn validate_authorization(
    principal: &CloudComputeK8sApiPrincipal,
    decision_id: &str,
    surface: &str,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<(), CloudComputeK8sApiError> {
    if decision_id.trim().is_empty() {
        return Err(CloudComputeK8sApiError::EmptyAuthorizationDecisionId);
    }
    validate_authorization_proof(principal, decision_id, surface, authorization_verifier)
}

fn validate_authorization_proof(
    principal: &CloudComputeK8sApiPrincipal,
    decision_id: &str,
    surface: &str,
    authorization_verifier: &impl CloudComputeK8sAuthorizationVerifier,
) -> Result<(), CloudComputeK8sApiError> {
    let Some(proof) = authorization_verifier.verified_authorization_proof(decision_id) else {
        return Err(CloudComputeK8sApiError::AuthorizationDenied {
            surface: surface.to_string(),
        });
    };
    let evaluation_epoch_seconds = authorization_verifier.evaluation_epoch_seconds();
    if !proof.verified
        || proof.tenant_id != principal.tenant_id
        || proof.principal_id != principal.principal_id
        || proof.surface != surface
        || proof.decision_id != decision_id
        || proof.issued_at_epoch_seconds >= proof.expires_at_epoch_seconds
        || evaluation_epoch_seconds < proof.issued_at_epoch_seconds
        || evaluation_epoch_seconds >= proof.expires_at_epoch_seconds
    {
        return Err(CloudComputeK8sApiError::AuthorizationDenied {
            surface: surface.to_string(),
        });
    }
    Ok(())
}
