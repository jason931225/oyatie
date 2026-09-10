use crate::NETWORK_SCHEMA_VERSION;
use crate::error::CloudNetworkError;
use crate::provider::{
    NetworkProviderKind, NetworkProviderVpcOperation, validate_network_provider_ref,
};
use crate::vpc::{Vpc, VpcCreate};
use compute_resource::PrincipalId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderVpcCreateRequest {
    pub request_id: String,              // data_class: INTERNAL_ONLY
    pub provider_vcn_ref: String,        // data_class: INTERNAL_ONLY
    pub vpc: VpcCreate,                  // data_class: INTERNAL_ONLY
    pub actor: String,                   // data_class: INTERNAL_ONLY
    pub idempotency_key: String,         // data_class: INTERNAL_ONLY
    pub requested_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderVpcReceipt {
    pub provider: NetworkProviderKind,          // data_class: PUBLIC
    pub operation: NetworkProviderVpcOperation, // data_class: PUBLIC
    pub request_id: String,                     // data_class: INTERNAL_ONLY
    pub provider_request_id: String,            // data_class: INTERNAL_ONLY
    pub provider_vcn_ref: String,               // data_class: INTERNAL_ONLY
    pub resource_id: String,                    // data_class: INTERNAL_ONLY
    pub tenant_id: String,                      // data_class: INTERNAL_ONLY
    pub region: String,                         // data_class: PUBLIC
    pub cidr_v4: String,                        // data_class: PUBLIC
    pub cidr_v6: String,                        // data_class: PUBLIC
    pub flow_logs_enabled: bool,                // data_class: PUBLIC
    pub actor: String,                          // data_class: INTERNAL_ONLY
    pub idempotency_key: String,                // data_class: INTERNAL_ONLY
    pub provider_evidence_ref: String,          // data_class: INTERNAL_ONLY
    pub occurred_at_epoch_seconds: u64,         // data_class: INTERNAL_ONLY
    pub schema_version: u32,                    // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkProviderVpcError {
    InvalidProviderVcnRef,
    InvalidProviderRequestId,
    InvalidProviderEvidenceRef,
    InvalidIdempotencyKey,
    InvalidActorRef,
    InvalidRequestShape(CloudNetworkError),
    ProviderRejected {
        provider: NetworkProviderKind, // data_class: PUBLIC
        reason: String,                // data_class: INTERNAL_ONLY
    },
    ProviderUnavailable {
        provider: NetworkProviderKind, // data_class: PUBLIC
        reason: String,                // data_class: INTERNAL_ONLY
    },
}

impl NetworkProviderVpcCreateRequest {
    pub fn validate(&self) -> Result<(), NetworkProviderVpcError> {
        validate_network_provider_ref(
            &self.request_id,
            NetworkProviderVpcError::InvalidProviderRequestId,
        )?;
        validate_network_provider_ref(
            &self.provider_vcn_ref,
            NetworkProviderVpcError::InvalidProviderVcnRef,
        )?;
        validate_network_provider_ref(
            &self.idempotency_key,
            NetworkProviderVpcError::InvalidIdempotencyKey,
        )?;
        Vpc::new(self.vpc.clone()).map_err(NetworkProviderVpcError::InvalidRequestShape)?;
        PrincipalId::new(self.actor.clone())
            .map_err(|_| NetworkProviderVpcError::InvalidActorRef)?;
        Ok(())
    }
}

impl NetworkProviderVpcReceipt {
    pub fn create_vpc(
        provider: NetworkProviderKind,
        input: NetworkProviderVpcCreateRequest,
        provider_request_id: impl Into<String>,
        provider_evidence_ref: impl Into<String>,
    ) -> Result<Self, NetworkProviderVpcError> {
        input.validate()?;
        let provider_request_id = provider_request_id.into();
        let provider_evidence_ref = provider_evidence_ref.into();
        validate_network_provider_ref(
            &provider_request_id,
            NetworkProviderVpcError::InvalidProviderRequestId,
        )?;
        validate_network_provider_ref(
            &provider_evidence_ref,
            NetworkProviderVpcError::InvalidProviderEvidenceRef,
        )?;
        Ok(Self {
            provider,
            operation: NetworkProviderVpcOperation::CreateVpc,
            request_id: input.request_id,
            provider_request_id,
            provider_vcn_ref: input.provider_vcn_ref,
            resource_id: input.vpc.resource_id,
            tenant_id: input.vpc.tenant_id,
            region: input.vpc.region,
            cidr_v4: input.vpc.cidr_v4,
            cidr_v6: input.vpc.cidr_v6,
            flow_logs_enabled: input.vpc.flow_logs_enabled,
            actor: input.actor,
            idempotency_key: input.idempotency_key,
            provider_evidence_ref,
            occurred_at_epoch_seconds: input.requested_at_epoch_seconds,
            schema_version: NETWORK_SCHEMA_VERSION,
        })
    }
}
