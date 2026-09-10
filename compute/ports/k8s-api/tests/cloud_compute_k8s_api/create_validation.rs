#[tokio::test]
async fn k8s_create_api_rejects_foreign_security_group_proof_before_repository() {
    let repository = LifecycleTestRepository::default();
    let mut request = request("req-compute-k8s-sg-proof", "idem-compute-k8s-sg-proof");
    request.body.node_pools[0].security_groups[0].tenant_id = "ten_other".to_string();

    let error = create_cloud_compute_k8s_cluster_from_api(&repository, request)
        .await
        .expect_err("security group proof must match tenant boundary");

    assert!(matches!(
        error,
        CloudComputeK8sApiError::NodePoolSecurityGroupBindingMismatch { .. }
    ));
    assert_eq!(error.cluster_create_status_code(), 403);
    assert_eq!(repository.create_operation_count(), 0);
    assert_eq!(repository.cluster_count(), 0);
}

#[tokio::test]
async fn k8s_create_api_rejects_reused_idempotency_key_with_new_fingerprint() {
    let repository = LifecycleTestRepository::default();
    let request = request("req-compute-k8s-idem", "idem-compute-k8s-idem");
    create_cloud_compute_k8s_cluster_from_api(&repository, request.clone())
        .await
        .expect("initial create succeeds");

    let mut drifted = request;
    drifted.body.control_plane_version = "v1.31.0-oyatie.1".to_string();
    assert_eq!(
        create_cloud_compute_k8s_cluster_from_api(&repository, drifted).await,
        Err(CloudComputeK8sApiError::IdempotencyKeyReused {
            idempotency_key: "idem-compute-k8s-idem".to_string(),
        })
    );
    assert_eq!(repository.create_operation_count(), 1);
    assert_eq!(repository.cluster_count(), 1);
}

#[tokio::test]
async fn k8s_create_api_maps_duplicate_cluster_to_conflict() {
    let repository = LifecycleTestRepository::default();
    create_cloud_compute_k8s_cluster_from_api(
        &repository,
        request(
            "req-compute-k8s-duplicate-a",
            "idem-compute-k8s-duplicate-a",
        ),
    )
    .await
    .expect("first cluster create succeeds");

    let error = create_cloud_compute_k8s_cluster_from_api(
        &repository,
        request(
            "req-compute-k8s-duplicate-b",
            "idem-compute-k8s-duplicate-b",
        ),
    )
    .await
    .expect_err("second cluster with same id conflicts");

    assert_eq!(
        error,
        CloudComputeK8sApiError::Compute(CloudComputeError::DuplicateKubernetesCluster)
    );
    assert_eq!(error.cluster_create_status_code(), 409);
    assert_eq!(repository.create_operation_count(), 1);
    assert_eq!(repository.cluster_count(), 1);
}

#[tokio::test]
async fn k8s_create_api_maps_invalid_cluster_shape_to_bad_request() {
    let repository = LifecycleTestRepository::default();
    let mut request = request("req-compute-k8s-shape", "idem-compute-k8s-shape");
    request.body.node_pools.truncate(1);

    let error = create_cloud_compute_k8s_cluster_from_api(&repository, request)
        .await
        .expect_err("HA cluster requires three AZs");

    assert_eq!(
        error,
        CloudComputeK8sApiError::Compute(CloudComputeError::KubernetesHaRequiresThreeAzs)
    );
    assert_eq!(error.cluster_create_status_code(), 400);
    assert_eq!(repository.create_operation_count(), 0);
    assert_eq!(repository.cluster_count(), 0);
}

#[tokio::test]
async fn k8s_create_api_rejects_unbound_repository_receipts_without_disclosure() {
    let fixture = LifecycleTestRepository::default();
    let valid = create_cloud_compute_k8s_cluster_from_api(
        &fixture,
        request("req-create-receipt-fixture", "idem-create-receipt-fixture"),
    )
    .await
    .expect("fixture create succeeds")
    .data;

    for case in ["resource", "tenant", "desired_state"] {
        let repository = LifecycleTestRepository::default();
        let before = repository.snapshot();
        let mut cluster = valid.clone();
        match case {
            "resource" => {
                cluster.resource_id =
                    "oyatie:cloud:region-home:ten_beta:k8s:foreign".to_string();
            }
            "tenant" => cluster.tenant_id = "ten_beta".to_string(),
            "desired_state" => cluster.desired_state = "deleted".to_string(),
            _ => unreachable!("test case is exhaustive"),
        }
        repository.return_next_create_receipt(CloudComputeK8sCreateReceipt {
            cluster,
            request_id: format!("repository-create-receipt-{case}"),
        });

        let error = create_cloud_compute_k8s_cluster_from_api(
            &repository,
            request(
                &format!("req-create-receipt-{case}"),
                &format!("idem-create-receipt-{case}"),
            ),
        )
        .await
        .expect_err("unbound create receipt fails closed");

        assert_eq!(
            error,
            CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation
        );
        assert_eq!(error.cluster_create_status_code(), 503);
        let response = error.error_response(format!("req-create-receipt-{case}"));
        assert_eq!(
            response.error.code,
            "CLOUD_COMPUTE_K8S_LIFECYCLE_REPOSITORY_UNAVAILABLE"
        );
        let rendered = format!("{response:?}");
        assert!(!rendered.contains("ten_beta"));
        assert!(!rendered.contains("foreign"));
        assert_eq!(repository.snapshot(), before);
    }
}

#[tokio::test]
async fn k8s_create_repository_failure_before_commit_is_atomic_and_retryable() {
    let repository = LifecycleTestRepository::default();
    let before = repository.snapshot();
    repository.fail_next_commit();

    let error = create_cloud_compute_k8s_cluster_from_api(
        &repository,
        request("req-create-failed-commit", "idem-create-failed-commit"),
    )
    .await
    .expect_err("failure between staging and commit fails closed");

    assert_eq!(error, CloudComputeK8sApiError::LifecycleRepositoryUnavailable);
    assert_eq!(error.cluster_create_status_code(), 503);
    assert_eq!(repository.snapshot(), before);

    let response = create_cloud_compute_k8s_cluster_from_api(
        &repository,
        request("req-create-retry-commit", "idem-create-failed-commit"),
    )
    .await
    .expect("retry commits the intact operation");
    assert_eq!(response.metadata.request_id, "req-create-retry-commit");
    assert_eq!(repository.create_operation_count(), 1);
    assert_eq!(repository.cluster_count(), 1);
}

#[tokio::test]
async fn k8s_record_projection_rejects_every_immutable_field_drift() {
    let desired = body(CLUSTER_ID);
    let valid = create_cloud_compute_k8s_cluster_from_api(
        &LifecycleTestRepository::default(),
        request("req-projection", "idem-projection"),
    )
    .await
    .expect("valid projection fixture creates")
    .data;
    assert_eq!(
        validate_cloud_compute_k8s_cluster_record_projection(&desired, &valid),
        Ok(())
    );

    for case in [
        "resource",
        "tenant",
        "region",
        "flavor",
        "control_plane_version",
        "control_plane_private",
        "node_pool_count",
        "residency",
        "data_class",
        "created_at",
        "schema_version",
        "state",
        "desired_state",
        "inconsistent_lifecycle",
    ] {
        let mut drifted = valid.clone();
        match case {
            "resource" => drifted.resource_id.push_str("-drift"),
            "tenant" => drifted.tenant_id = "ten_beta".to_string(),
            "region" => drifted.region = "region-away".to_string(),
            "flavor" => drifted.flavor = "standard".to_string(),
            "control_plane_version" => drifted.control_plane_version.push_str("-drift"),
            "control_plane_private" => {
                drifted.control_plane_private = !drifted.control_plane_private;
            }
            "node_pool_count" => drifted.node_pool_count += 1,
            "residency" => drifted.residency = "global".to_string(),
            "data_class" => drifted.data_class = "INTERNAL_ONLY".to_string(),
            "created_at" => drifted.created_at_epoch_seconds += 1,
            "schema_version" => drifted.schema_version += 1,
            "state" => drifted.state = "corrupt".to_string(),
            "desired_state" => drifted.desired_state = "corrupt".to_string(),
            "inconsistent_lifecycle" => drifted.state = "draining".to_string(),
            _ => unreachable!("test case is exhaustive"),
        }
        assert_eq!(
            validate_cloud_compute_k8s_cluster_record_projection(&desired, &drifted),
            Err(CloudComputeK8sApiError::LifecycleRepositoryInvariantViolation),
            "{case} drift must fail closed"
        );
    }

    let mut corrupted_desired = desired.clone();
    corrupted_desired.data_class = "INTERNAL_ONLY".to_string();
    let mut matching_corruption = valid;
    matching_corruption.data_class = "INTERNAL_ONLY".to_string();
    assert!(validate_cloud_compute_k8s_cluster_record_projection(
        &corrupted_desired,
        &matching_corruption
    )
    .is_err());
}
