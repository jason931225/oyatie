//! Binds a Cedar policy version to an IAM role, committing neither side unless
//! both kernels accept. Value validation still belongs to the domain crates.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub use iam_domain::{
    CloudIamBoundaryCellId, CloudIamBoundaryRegionId, CloudIamBoundaryTenantId,
    CloudIamPlacementBoundary as CloudIamUseCaseBoundary,
};
use iam_domain::{CloudIamError, IamDirectory, IamRole, IamRoleCreate};
use iam_policy_cedar_domain::{
    PolicyError, PolicyScope, PolicySet, PolicyVersion, PublishedPolicy,
};

pub const CLOUD_IAM_CEDAR_POLICY_BIND_SURFACE: &str = "cloud.iam.cedar_policy.bind";
pub const CLOUD_IAM_CEDAR_POLICY_BIND_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudIamCedarPolicyBindRequest {
    pub request_id: String,                // data_class: INTERNAL_ONLY
    pub tenant_id: String,                 // data_class: INTERNAL_ONLY
    pub boundary: CloudIamUseCaseBoundary, // data_class: INTERNAL_ONLY
    pub policy: PolicyVersion,             // data_class: INTERNAL_ONLY
    pub role: IamRoleCreate,               // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudIamCedarPolicyBindSuccessResponse {
    pub policy: PublishedPolicy, // data_class: INTERNAL_ONLY
    pub role: IamRole,           // data_class: INTERNAL_ONLY
    pub metadata: CloudIamCedarPolicyBindMetadata, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloudIamCedarPolicyBindMetadata {
    pub request_id: String,     // data_class: INTERNAL_ONLY
    pub tenant_id: String,      // data_class: INTERNAL_ONLY
    pub cell_id: String,        // data_class: INTERNAL_ONLY
    pub region_id: String,      // data_class: PUBLIC
    pub policy_id: String,      // data_class: INTERNAL_ONLY
    pub policy_version: String, // data_class: INTERNAL_ONLY
    pub role_id: String,        // data_class: INTERNAL_ONLY
    pub schema_version: u32,    // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloudIamCedarPolicyBindError {
    EmptyRequestId,
    EmptyTenantId,
    EmptyBoundaryCell,
    EmptyBoundaryRegion,
    BoundaryTenantMismatch {
        request_tenant_id: String,
        boundary_tenant_id: String,
    },
    RequestTenantRoleMismatch {
        request_tenant_id: String,
        role_tenant_id: String,
    },
    RolePolicyMismatch {
        role_policy_id: String,
        role_policy_version: String,
        policy_id: String,
        policy_version: String,
    },
    RoleTenantPolicyScopeMismatch {
        role_tenant_id: String,
        policy_tenant_id: String,
    },
    CedarPolicy(PolicyError),
    CloudIam(CloudIamError),
}

/// All-or-nothing: both kernels are mutated on clones, and the clones are
/// committed back only once both have succeeded, so a failure leaves neither
/// the policy set nor the directory changed.
pub fn bind_cedar_policy(
    policies: &mut PolicySet,
    directory: &mut IamDirectory,
    request: CloudIamCedarPolicyBindRequest,
) -> Result<CloudIamCedarPolicyBindSuccessResponse, CloudIamCedarPolicyBindError> {
    validate_bind_request(&request)?;

    let mut policies_tx = policies.clone();
    let mut directory_tx = directory.clone();

    let policy = policies_tx
        .publish(request.policy.clone())
        .map_err(CloudIamCedarPolicyBindError::CedarPolicy)?;
    let role = directory_tx
        .create_role(request.role.clone())
        .map_err(CloudIamCedarPolicyBindError::CloudIam)?;

    *policies = policies_tx;
    *directory = directory_tx;

    Ok(CloudIamCedarPolicyBindSuccessResponse {
        policy,
        role,
        metadata: CloudIamCedarPolicyBindMetadata {
            request_id: request.request_id,
            tenant_id: request.tenant_id,
            cell_id: request.boundary.cell_id.value,
            region_id: request.boundary.region_id.value,
            policy_id: request.policy.policy_id,
            policy_version: request.policy.version,
            role_id: request.role.id,
            schema_version: CLOUD_IAM_CEDAR_POLICY_BIND_SCHEMA_VERSION,
        },
    })
}

fn validate_bind_request(
    request: &CloudIamCedarPolicyBindRequest,
) -> Result<(), CloudIamCedarPolicyBindError> {
    if request.request_id.trim().is_empty() {
        return Err(CloudIamCedarPolicyBindError::EmptyRequestId);
    }
    if request.tenant_id.trim().is_empty() {
        return Err(CloudIamCedarPolicyBindError::EmptyTenantId);
    }
    validate_use_case_boundary(&request.tenant_id, &request.boundary)?;
    if request.tenant_id != request.role.tenant_id {
        return Err(CloudIamCedarPolicyBindError::RequestTenantRoleMismatch {
            request_tenant_id: request.tenant_id.clone(),
            role_tenant_id: request.role.tenant_id.clone(),
        });
    }
    if request.role.cedar_policy_id != request.policy.policy_id
        || request.role.cedar_policy_version != request.policy.version
    {
        return Err(CloudIamCedarPolicyBindError::RolePolicyMismatch {
            role_policy_id: request.role.cedar_policy_id.clone(),
            role_policy_version: request.role.cedar_policy_version.clone(),
            policy_id: request.policy.policy_id.clone(),
            policy_version: request.policy.version.clone(),
        });
    }

    if let PolicyScope::Tenant(policy_tenant_id) = &request.policy.scope
        && request.role.tenant_id != *policy_tenant_id
    {
        return Err(
            CloudIamCedarPolicyBindError::RoleTenantPolicyScopeMismatch {
                role_tenant_id: request.role.tenant_id.clone(),
                policy_tenant_id: policy_tenant_id.clone(),
            },
        );
    }

    Ok(())
}

fn validate_use_case_boundary(
    tenant_id: &str,
    boundary: &CloudIamUseCaseBoundary,
) -> Result<(), CloudIamCedarPolicyBindError> {
    if boundary.tenant_id.value.trim().is_empty() {
        return Err(CloudIamCedarPolicyBindError::EmptyTenantId);
    }
    if boundary.tenant_id.value != tenant_id {
        return Err(CloudIamCedarPolicyBindError::BoundaryTenantMismatch {
            request_tenant_id: tenant_id.to_string(),
            boundary_tenant_id: boundary.tenant_id.value.clone(),
        });
    }
    if boundary.cell_id.value.trim().is_empty() {
        return Err(CloudIamCedarPolicyBindError::EmptyBoundaryCell);
    }
    if boundary.region_id.value.trim().is_empty() {
        return Err(CloudIamCedarPolicyBindError::EmptyBoundaryRegion);
    }
    Ok(())
}
