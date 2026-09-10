use crate::NETWORK_SCHEMA_VERSION;
use crate::error::CloudNetworkError;
use crate::load_balancer::{LbKind, LoadBalancer, LoadBalancerCreate};
use crate::provider::{
    NetworkProviderKind, NetworkProviderLoadBalancerOperation,
    validate_network_provider_load_balancer_ref,
};
use crate::vpc::{Subnet, SubnetCreate, Vpc, VpcCreate};
use compute_resource::PrincipalId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderLoadBalancerCreateRequest {
    pub request_id: String,                 // data_class: INTERNAL_ONLY
    pub provider_load_balancer_ref: String, // data_class: INTERNAL_ONLY
    pub vpc: VpcCreate,                     // data_class: INTERNAL_ONLY
    pub subnets: Vec<SubnetCreate>,         // data_class: INTERNAL_ONLY
    pub load_balancer: LoadBalancerCreate,  // data_class: INTERNAL_ONLY
    pub actor: String,                      // data_class: INTERNAL_ONLY
    pub idempotency_key: String,            // data_class: INTERNAL_ONLY
    pub requested_at_epoch_seconds: u64,    // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderLoadBalancerReceipt {
    pub provider: NetworkProviderKind, // data_class: PUBLIC
    pub operation: NetworkProviderLoadBalancerOperation, // data_class: PUBLIC
    pub request_id: String,            // data_class: INTERNAL_ONLY
    pub provider_request_id: String,   // data_class: INTERNAL_ONLY
    pub provider_load_balancer_ref: String, // data_class: INTERNAL_ONLY
    pub resource_id: String,           // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub vpc_id: String,                // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub kind: LbKind,                  // data_class: PUBLIC
    pub listener_count: usize,         // data_class: PUBLIC
    pub target_group_count: usize,     // data_class: PUBLIC
    pub mtls_enabled: bool,            // data_class: PUBLIC
    pub actor: String,                 // data_class: INTERNAL_ONLY
    pub idempotency_key: String,       // data_class: INTERNAL_ONLY
    pub provider_evidence_ref: String, // data_class: INTERNAL_ONLY
    pub occurred_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    pub schema_version: u32,           // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkProviderLoadBalancerError {
    InvalidProviderLoadBalancerRef,
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

impl NetworkProviderLoadBalancerCreateRequest {
    pub fn validate(&self) -> Result<(), NetworkProviderLoadBalancerError> {
        validate_network_provider_load_balancer_ref(
            &self.request_id,
            NetworkProviderLoadBalancerError::InvalidProviderRequestId,
        )?;
        validate_network_provider_load_balancer_ref(
            &self.provider_load_balancer_ref,
            NetworkProviderLoadBalancerError::InvalidProviderLoadBalancerRef,
        )?;
        validate_network_provider_load_balancer_ref(
            &self.idempotency_key,
            NetworkProviderLoadBalancerError::InvalidIdempotencyKey,
        )?;
        let vpc = Vpc::new(self.vpc.clone())
            .map_err(NetworkProviderLoadBalancerError::InvalidRequestShape)?;
        let mut known_subnets = BTreeMap::new();
        for subnet_input in self.subnets.clone() {
            let subnet = Subnet::new(&vpc, subnet_input)
                .map_err(NetworkProviderLoadBalancerError::InvalidRequestShape)?;
            if known_subnets
                .insert(subnet.resource_id.value.clone(), subnet)
                .is_some()
            {
                return Err(NetworkProviderLoadBalancerError::InvalidRequestShape(
                    CloudNetworkError::DuplicateSubnet,
                ));
            }
        }
        LoadBalancer::new(&vpc, &known_subnets, self.load_balancer.clone())
            .map_err(NetworkProviderLoadBalancerError::InvalidRequestShape)?;
        PrincipalId::new(self.actor.clone())
            .map_err(|_| NetworkProviderLoadBalancerError::InvalidActorRef)?;
        Ok(())
    }
}

impl NetworkProviderLoadBalancerReceipt {
    pub fn create_load_balancer(
        provider: NetworkProviderKind,
        input: NetworkProviderLoadBalancerCreateRequest,
        provider_request_id: impl Into<String>,
        provider_evidence_ref: impl Into<String>,
    ) -> Result<Self, NetworkProviderLoadBalancerError> {
        input.validate()?;
        let provider_request_id = provider_request_id.into();
        let provider_evidence_ref = provider_evidence_ref.into();
        validate_network_provider_load_balancer_ref(
            &provider_request_id,
            NetworkProviderLoadBalancerError::InvalidProviderRequestId,
        )?;
        validate_network_provider_load_balancer_ref(
            &provider_evidence_ref,
            NetworkProviderLoadBalancerError::InvalidProviderEvidenceRef,
        )?;
        Ok(Self {
            provider,
            operation: NetworkProviderLoadBalancerOperation::CreateLoadBalancer,
            request_id: input.request_id,
            provider_request_id,
            provider_load_balancer_ref: input.provider_load_balancer_ref,
            resource_id: input.load_balancer.resource_id,
            tenant_id: input.load_balancer.tenant_id,
            vpc_id: input.load_balancer.vpc_id,
            region: input.load_balancer.region,
            kind: input.load_balancer.kind,
            listener_count: input.load_balancer.listeners.len(),
            target_group_count: input.load_balancer.target_groups.len(),
            mtls_enabled: input.load_balancer.mtls.is_some(),
            actor: input.actor,
            idempotency_key: input.idempotency_key,
            provider_evidence_ref,
            occurred_at_epoch_seconds: input.requested_at_epoch_seconds,
            schema_version: NETWORK_SCHEMA_VERSION,
        })
    }
}
