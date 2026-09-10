use iam_tenant_rbac_tenant_workload_manifest::{
    Fd001TenantWorkloadKind, Fd001TenantWorkloadManifestError, fd001_tenant_workload_manifest,
    fd001_workload_count, tenant_workload_official_doc_urls,
    validate_fd001_tenant_workload_manifest,
};

#[test]
fn fd001_tenant_workload_manifest_validates_controls_and_nonclaims() {
    let manifest = fd001_tenant_workload_manifest();
    validate_fd001_tenant_workload_manifest(&manifest).expect("manifest validates");

    assert_eq!(
        manifest.manifest_name,
        "fd001-tenant-rbac-tenant-workload-manifest"
    );
    assert_eq!(manifest.program_name, "fd-001-tenant-rbac-generic");
    assert_eq!(manifest.substrate_name, "oyatie-cloud");
    assert_eq!(manifest.tenant_namespace, "oyatie-fd001-tenant-rbac-dev");
    assert_eq!(manifest.tenant_cell_id, "cell-us-east-001");
    assert_eq!(manifest.residency_region, "us-east-1");
    assert_eq!(fd001_workload_count(&manifest), 4);
    assert!(manifest.fd001_product_goal_preserved);
    assert!(manifest.oyatie_cloud_substrate_only);
    assert!(manifest.review_only_contract);
    assert!(manifest.namespace_isolation_required);
    assert!(manifest.resource_quota_required);
    assert!(manifest.network_policy_required);
    assert!(manifest.service_account_boundary_required);
    assert!(manifest.gateway_route_required);
    assert!(manifest.route_auth_scope_required);
    assert!(manifest.tenant_claim_required);
    assert!(manifest.legal_entity_claim_required);
    assert!(manifest.otel_resource_identity_required);
    assert!(manifest.per_workload_evidence_required);
    assert!(!manifest.production_tenant_attached);
    assert!(!manifest.kubernetes_namespace_created);
    assert!(!manifest.resource_quota_applied);
    assert!(!manifest.network_policy_applied);
    assert!(!manifest.gateway_route_attached);
    assert!(!manifest.workload_runtime_deployed);
    assert!(!manifest.cloud_substrate_runtime_attached);
    assert!(!manifest.runtime_audit_chain_emission_attached);
}

#[test]
fn fd001_tenant_workload_manifest_covers_fd001_workloads_and_official_sources() {
    let manifest = fd001_tenant_workload_manifest();
    let kinds = manifest
        .workloads
        .iter()
        .map(|workload| workload.workload_kind)
        .collect::<std::collections::BTreeSet<_>>();

    for kind in [
        Fd001TenantWorkloadKind::TenantRbac,
        Fd001TenantWorkloadKind::HrEmployment,
        Fd001TenantWorkloadKind::PayrollRun,
        Fd001TenantWorkloadKind::AccountingJournal,
    ] {
        assert!(kinds.contains(&kind), "missing {kind:?}");
    }

    let docs = tenant_workload_official_doc_urls(&manifest);
    assert!(docs.contains(
        &"https://kubernetes.io/docs/concepts/overview/working-with-objects/namespaces/"
    ));
    assert!(docs.contains(&"https://kubernetes.io/docs/concepts/policy/resource-quotas/"));
    assert!(
        docs.contains(&"https://kubernetes.io/docs/concepts/services-networking/network-policies/")
    );
    assert!(docs.contains(&"https://gateway-api.sigs.k8s.io/docs/introduction/"));
    assert!(docs.contains(&"https://opentelemetry.io/docs/specs/semconv/resource/service/"));
}

#[test]
fn fd001_tenant_workload_manifest_preserves_per_workload_isolation_refs() {
    let manifest = fd001_tenant_workload_manifest();

    assert!(manifest.workloads.iter().all(|workload| {
        workload.tenant_namespace == manifest.tenant_namespace
            && workload.tenant_cell_id == manifest.tenant_cell_id
            && workload.residency_region == manifest.residency_region
            && workload.tenant_claim == "tenant_id"
            && workload.otel_service_namespace == "fd001-tenant-rbac"
            // Two prefixes because the reorg is mid-flight: `crates/` is the
            // old layout and `app/` the absorbed one. Both are legitimate.
            && (workload.runtime_package_ref.starts_with("crates/")
                || workload.runtime_package_ref.starts_with("app/"))
            && workload
                .resource_quota_ref
                .starts_with("deploy/oyatie-cloud/")
            && workload
                .network_policy_ref
                .starts_with("deploy/oyatie-cloud/")
            && workload.gateway_route_ref.starts_with("gateway/httproute/")
            && workload.evidence_ref.starts_with("evidence/multispectrum/")
            && workload.namespace_isolation_required
            && workload.resource_quota_required
            && workload.network_policy_required
            && workload.service_account_boundary_required
            && workload.gateway_route_required
            && workload.route_auth_scope_required
            && workload.rls_tenant_claim_required
            && workload.otel_resource_identity_required
            && !workload.production_runtime_attached
            && !workload.cloud_deployment_attached
            && !workload.runtime_audit_chain_emission_attached
    }));
}

#[test]
fn fd001_tenant_workload_manifest_rejects_missing_duplicate_and_doc_drift() {
    let mut manifest = fd001_tenant_workload_manifest();
    manifest.workloads.truncate(2);
    assert_eq!(
        validate_fd001_tenant_workload_manifest(&manifest),
        Err(Fd001TenantWorkloadManifestError::MissingWorkloads)
    );

    let mut manifest = fd001_tenant_workload_manifest();
    manifest.workloads[1].workload_id = manifest.workloads[0].workload_id;
    assert_eq!(
        validate_fd001_tenant_workload_manifest(&manifest),
        Err(Fd001TenantWorkloadManifestError::DuplicateWorkload(
            "tenant-rbac-runtime".to_owned()
        ))
    );

    let mut manifest = fd001_tenant_workload_manifest();
    manifest.official_doc_urls[0] = "https://example.com/kubernetes";
    assert_eq!(
        validate_fd001_tenant_workload_manifest(&manifest),
        Err(Fd001TenantWorkloadManifestError::InvalidOfficialDocUrl)
    );
}

#[test]
fn fd001_tenant_workload_manifest_rejects_unsafe_refs_missing_controls_and_overclaims() {
    let mut manifest = fd001_tenant_workload_manifest();
    manifest.workloads[0].network_policy_ref = "deploy/oyatie-cloud/secret-token";
    assert_eq!(
        validate_fd001_tenant_workload_manifest(&manifest),
        Err(Fd001TenantWorkloadManifestError::InvalidNetworkPolicyRef)
    );

    let mut manifest = fd001_tenant_workload_manifest();
    manifest.network_policy_required = false;
    assert_eq!(
        validate_fd001_tenant_workload_manifest(&manifest),
        Err(Fd001TenantWorkloadManifestError::MissingRequiredControl(
            "network_policy_required"
        ))
    );

    let mut manifest = fd001_tenant_workload_manifest();
    manifest.cloud_substrate_runtime_attached = true;
    assert_eq!(
        validate_fd001_tenant_workload_manifest(&manifest),
        Err(Fd001TenantWorkloadManifestError::RuntimeAttachmentOverclaim)
    );
}
