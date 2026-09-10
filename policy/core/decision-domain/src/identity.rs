//! Validated binding from the authenticated Cedar principal to ReBAC identity.

use std::fmt;

use policy_cedar_domain::rebac::{
    RebacObjectRef, RebacSubjectRef, RebacTenantScope, RebacTupleValidationError,
};
use shared_platform_contracts_kernel::ContractViolation;
use shared_platform_contracts_kernel::pdp::AuthorizationRequest;

/// One policy-owned mapping from a Cedar principal type to a ReBAC object type.
///
/// The mapping controls names only. Tenant and principal id always come from
/// the same validated authorization request and cannot be supplied separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PrincipalMapping {
    cedar_entity_type: String,
    rebac_object_type: String,
}

impl PrincipalMapping {
    pub fn new(
        cedar_entity_type: impl Into<String>,
        rebac_object_type: impl Into<String>,
    ) -> Result<Self, IdentityMappingError> {
        let cedar_entity_type = cedar_entity_type.into();
        if cedar_entity_type.is_empty()
            || !cedar_entity_type.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == '_' || character == ':'
            })
        {
            return Err(IdentityMappingError::InvalidCedarEntityType {
                value: cedar_entity_type,
            });
        }
        let rebac_object_type = rebac_object_type.into();
        RebacObjectRef::new(rebac_object_type.clone(), "mapping-validation")
            .map_err(IdentityMappingError::InvalidRebacIdentity)?;
        Ok(Self {
            cedar_entity_type,
            rebac_object_type,
        })
    }

    pub(crate) fn derive(
        &self,
        request: &AuthorizationRequest,
    ) -> Result<MappedIdentity, IdentityMappingError> {
        request
            .validate()
            .map_err(IdentityMappingError::InvalidRequest)?;
        if request.principal.entity_type != self.cedar_entity_type {
            return Err(IdentityMappingError::UnmappedPrincipalType {
                expected: self.cedar_entity_type.clone(),
                actual: request.principal.entity_type.clone(),
            });
        }
        let tenant = RebacTenantScope::new(request.tenant_id.clone())
            .map_err(IdentityMappingError::InvalidRebacIdentity)?;
        let principal = RebacObjectRef::new(
            self.rebac_object_type.clone(),
            request.principal.entity_id.clone(),
        )
        .map_err(IdentityMappingError::InvalidRebacIdentity)?;
        Ok(MappedIdentity {
            tenant,
            subject: RebacSubjectRef::object(principal),
        })
    }
}

pub(crate) struct MappedIdentity {
    pub(crate) tenant: RebacTenantScope,
    pub(crate) subject: RebacSubjectRef,
}

/// Why an authorization request could not become one graph identity.
#[derive(Debug, PartialEq)]
pub enum IdentityMappingError {
    InvalidCedarEntityType { value: String },
    InvalidRebacIdentity(RebacTupleValidationError),
    InvalidRequest(Vec<ContractViolation>),
    UnmappedPrincipalType { expected: String, actual: String },
}

impl fmt::Display for IdentityMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCedarEntityType { value } => {
                write!(formatter, "invalid Cedar principal entity type {value:?}")
            }
            Self::InvalidRebacIdentity(error) => {
                write!(formatter, "invalid ReBAC identity: {error}")
            }
            Self::InvalidRequest(violations) => {
                write!(formatter, "invalid authorization request: {violations:?}")
            }
            Self::UnmappedPrincipalType { expected, actual } => write!(
                formatter,
                "unmapped Cedar principal type {actual:?}; expected {expected:?}"
            ),
        }
    }
}

impl std::error::Error for IdentityMappingError {}
