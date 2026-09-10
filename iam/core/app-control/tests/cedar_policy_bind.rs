#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use data_boundary_kernel::DataClass;
use iam_control_app::{
    CLOUD_IAM_CEDAR_POLICY_BIND_SCHEMA_VERSION, CLOUD_IAM_CEDAR_POLICY_BIND_SURFACE,
    CloudIamBoundaryCellId, CloudIamBoundaryRegionId, CloudIamBoundaryTenantId,
    CloudIamCedarPolicyBindError, CloudIamCedarPolicyBindRequest, CloudIamUseCaseBoundary,
    bind_cedar_policy,
};
use iam_domain::{
    IamDirectory, IamPrincipalCreate, IamPrincipalKind, IamRoleCreate, IamRoleId, MfaState,
};
use iam_policy_cedar_domain::{
    AuthorizationQuery, AuthorizationSubject, PolicyEffect, PolicyRuleInput, PolicyScope,
    PolicySet, PolicyVersion,
};
use std::collections::BTreeMap;

const TENANT_ID: &str = "ten_alpha";
const PRINCIPAL_ID: &str = "sp_cloud_provisioner";
const POLICY_ID: &str = "pol_cloud_compute_admin";
const POLICY_VERSION: &str = "1.0.0";
const ROLE_ID: &str = "role_compute_admin";

fn directory_with_principal() -> IamDirectory {
    let mut directory = IamDirectory::default();
    directory
        .create_principal(IamPrincipalCreate {
            id: PRINCIPAL_ID.to_string(),
            tenant_id: TENANT_ID.to_string(),
            kind: IamPrincipalKind::ServiceAccount,
            display_name: "cloud provisioner".to_string(),
            external_subject: None,
            identity_provider_id: None,
            region_pack: "pack-alpha".to_string(),
            mfa_state: MfaState::NotRequired,
            last_authenticated_at_epoch_seconds: None,
            created_at_epoch_seconds: 1_700_000_001,
        })
        .expect("service principal registers");
    directory
}

fn policy_version() -> PolicyVersion {
    PolicyVersion {
        policy_id: POLICY_ID.to_string(),
        version: POLICY_VERSION.to_string(),
        scope: PolicyScope::Tenant(TENANT_ID.to_string()),
        supersedes: None,
        rules: vec![PolicyRuleInput {
            effect: PolicyEffect::Allow,
            principal_role: "cloud-compute-admin".to_string(),
            action: "cloud.compute.write".to_string(),
            resource_prefix: "cloud:ten_alpha:compute".to_string(),
            required_attribute: None,
            annotations: vec![],
        }],
    }
}

fn role_create() -> IamRoleCreate {
    IamRoleCreate {
        id: ROLE_ID.to_string(),
        tenant_id: TENANT_ID.to_string(),
        region: "home-region".to_string(),
        name: "compute-admin".to_string(),
        cedar_policy_id: POLICY_ID.to_string(),
        cedar_policy_version: POLICY_VERSION.to_string(),
        assumable_by: vec![PRINCIPAL_ID.to_string()],
        max_session_duration_sec: 900,
        data_class: DataClass::Public,
        created_at_epoch_seconds: 1_700_000_003,
    }
}

fn bind_request() -> CloudIamCedarPolicyBindRequest {
    CloudIamCedarPolicyBindRequest {
        request_id: "req_bind_001".to_string(),
        tenant_id: TENANT_ID.to_string(),
        boundary: use_case_boundary(TENANT_ID, "cell-alpha-region-a-001", "home-region"),
        policy: policy_version(),
        role: role_create(),
    }
}

fn use_case_boundary(tenant_id: &str, cell_id: &str, region_id: &str) -> CloudIamUseCaseBoundary {
    CloudIamUseCaseBoundary {
        tenant_id: CloudIamBoundaryTenantId {
            value: tenant_id.to_string(),
        },
        cell_id: CloudIamBoundaryCellId {
            value: cell_id.to_string(),
        },
        region_id: CloudIamBoundaryRegionId {
            value: region_id.to_string(),
        },
    }
}

#[test]
fn cedar_policy_bind_publishes_policy_and_creates_role_transactionally() {
    let mut policies = PolicySet::default();
    let mut directory = directory_with_principal();

    let response = bind_cedar_policy(&mut policies, &mut directory, bind_request())
        .expect("Cedar policy and IAM role bind succeeds");

    assert_eq!(response.policy.policy_id, POLICY_ID);
    assert_eq!(response.role.id.value.value, ROLE_ID);
    assert_eq!(
        response.metadata.schema_version,
        CLOUD_IAM_CEDAR_POLICY_BIND_SCHEMA_VERSION
    );
    assert_eq!(response.metadata.cell_id, "cell-alpha-region-a-001");
    assert_eq!(response.metadata.region_id, "home-region");
    assert_eq!(
        CLOUD_IAM_CEDAR_POLICY_BIND_SURFACE,
        "cloud.iam.cedar_policy.bind"
    );
    let role_id = IamRoleId::new(response.role.id.value.value.clone()).unwrap();
    assert!(directory.role(&role_id).is_some());

    let decision = policies.authorize(&AuthorizationQuery {
        subject: AuthorizationSubject {
            tenant_id: TENANT_ID.to_string(),
            roles: vec!["cloud-compute-admin".to_string()],
        },
        action: "cloud.compute.write".to_string(),
        resource: "cloud:ten_alpha:compute/vm/vm_001".to_string(),
        attributes: BTreeMap::new(),
    });
    assert!(decision.allowed);
}

#[test]
fn cedar_policy_bind_rejects_missing_or_drifted_use_case_boundary_before_mutation() {
    let mut policies = PolicySet::default();
    let mut directory = directory_with_principal();
    let mut missing_cell = bind_request();
    missing_cell.boundary.cell_id.value = String::new();

    assert_eq!(
        bind_cedar_policy(&mut policies, &mut directory, missing_cell),
        Err(CloudIamCedarPolicyBindError::EmptyBoundaryCell)
    );
    assert!(policies.get(POLICY_ID, POLICY_VERSION).is_none());

    let mut tenant_drift = bind_request();
    tenant_drift.boundary.tenant_id.value = "ten_beta".to_string();
    assert_eq!(
        bind_cedar_policy(&mut policies, &mut directory, tenant_drift),
        Err(CloudIamCedarPolicyBindError::BoundaryTenantMismatch {
            request_tenant_id: TENANT_ID.to_string(),
            boundary_tenant_id: "ten_beta".to_string(),
        })
    );
    assert!(policies.get(POLICY_ID, POLICY_VERSION).is_none());

    let mut missing_region = bind_request();
    missing_region.boundary.region_id.value = " ".to_string();
    assert_eq!(
        bind_cedar_policy(&mut policies, &mut directory, missing_region),
        Err(CloudIamCedarPolicyBindError::EmptyBoundaryRegion)
    );
    assert!(policies.get(POLICY_ID, POLICY_VERSION).is_none());
}

#[test]
fn cedar_policy_bind_rejects_role_policy_mismatch_before_any_kernel_mutation() {
    let mut policies = PolicySet::default();
    let mut directory = directory_with_principal();
    let mut request = bind_request();
    request.role.cedar_policy_version = "2.0.0".to_string();

    let error = bind_cedar_policy(&mut policies, &mut directory, request)
        .expect_err("role/policy version drift is rejected");

    assert!(matches!(
        error,
        CloudIamCedarPolicyBindError::RolePolicyMismatch { .. }
    ));
    assert!(policies.get(POLICY_ID, POLICY_VERSION).is_none());
}

#[test]
fn cedar_policy_bind_rolls_back_published_policy_when_iam_directory_rejects_role() {
    let mut policies = PolicySet::default();
    let mut directory = IamDirectory::default();

    let error = bind_cedar_policy(&mut policies, &mut directory, bind_request())
        .expect_err("IAM rejects missing assumable principal");

    assert!(matches!(error, CloudIamCedarPolicyBindError::CloudIam(_)));
    assert!(policies.get(POLICY_ID, POLICY_VERSION).is_none());
}
