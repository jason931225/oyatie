use crate::NETWORK_SCHEMA_VERSION;
use crate::dns::{DnsZone, DnsZoneCreate, DnsZoneKind};
use crate::error::CloudNetworkError;
use crate::provider::{
    NetworkProviderDnsZoneOperation, NetworkProviderKind, validate_network_provider_dns_zone_ref,
};
use crate::vpc::{Vpc, VpcCreate};
use compute_resource::PrincipalId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderDnsZoneCreateRequest {
    pub request_id: String,              // data_class: INTERNAL_ONLY
    pub provider_dns_zone_ref: String,   // data_class: INTERNAL_ONLY
    pub vpc: Option<VpcCreate>,          // data_class: INTERNAL_ONLY
    pub dns_zone: DnsZoneCreate,         // data_class: INTERNAL_ONLY
    pub actor: String,                   // data_class: INTERNAL_ONLY
    pub idempotency_key: String,         // data_class: INTERNAL_ONLY
    pub requested_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderDnsZoneReceipt {
    pub provider: NetworkProviderKind, // data_class: PUBLIC
    pub operation: NetworkProviderDnsZoneOperation, // data_class: PUBLIC
    pub request_id: String,            // data_class: INTERNAL_ONLY
    pub provider_request_id: String,   // data_class: INTERNAL_ONLY
    pub provider_dns_zone_ref: String, // data_class: INTERNAL_ONLY
    pub resource_id: String,           // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub name: String,                  // data_class: PUBLIC
    pub kind: DnsZoneKind,             // data_class: PUBLIC
    pub vpc_id: Option<String>,        // data_class: INTERNAL_ONLY
    pub dnssec_enabled: bool,          // data_class: PUBLIC
    pub actor: String,                 // data_class: INTERNAL_ONLY
    pub idempotency_key: String,       // data_class: INTERNAL_ONLY
    pub provider_evidence_ref: String, // data_class: INTERNAL_ONLY
    pub occurred_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    pub schema_version: u32,           // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkProviderDnsZoneError {
    InvalidProviderDnsZoneRef,
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

impl NetworkProviderDnsZoneCreateRequest {
    pub fn validate(&self) -> Result<(), NetworkProviderDnsZoneError> {
        validate_network_provider_dns_zone_ref(
            &self.request_id,
            NetworkProviderDnsZoneError::InvalidProviderRequestId,
        )?;
        validate_network_provider_dns_zone_ref(
            &self.provider_dns_zone_ref,
            NetworkProviderDnsZoneError::InvalidProviderDnsZoneRef,
        )?;
        validate_network_provider_dns_zone_ref(
            &self.idempotency_key,
            NetworkProviderDnsZoneError::InvalidIdempotencyKey,
        )?;
        let vpc = self
            .vpc
            .clone()
            .map(Vpc::new)
            .transpose()
            .map_err(NetworkProviderDnsZoneError::InvalidRequestShape)?;
        DnsZone::new(vpc.as_ref(), self.dns_zone.clone())
            .map_err(NetworkProviderDnsZoneError::InvalidRequestShape)?;
        PrincipalId::new(self.actor.clone())
            .map_err(|_| NetworkProviderDnsZoneError::InvalidActorRef)?;
        Ok(())
    }
}

impl NetworkProviderDnsZoneReceipt {
    pub fn create_dns_zone(
        provider: NetworkProviderKind,
        input: NetworkProviderDnsZoneCreateRequest,
        provider_request_id: impl Into<String>,
        provider_evidence_ref: impl Into<String>,
    ) -> Result<Self, NetworkProviderDnsZoneError> {
        input.validate()?;
        let provider_request_id = provider_request_id.into();
        let provider_evidence_ref = provider_evidence_ref.into();
        validate_network_provider_dns_zone_ref(
            &provider_request_id,
            NetworkProviderDnsZoneError::InvalidProviderRequestId,
        )?;
        validate_network_provider_dns_zone_ref(
            &provider_evidence_ref,
            NetworkProviderDnsZoneError::InvalidProviderEvidenceRef,
        )?;
        Ok(Self {
            provider,
            operation: NetworkProviderDnsZoneOperation::CreateDnsZone,
            request_id: input.request_id,
            provider_request_id,
            provider_dns_zone_ref: input.provider_dns_zone_ref,
            resource_id: input.dns_zone.resource_id,
            tenant_id: input.dns_zone.tenant_id,
            region: input.dns_zone.region,
            name: input.dns_zone.name,
            kind: input.dns_zone.kind,
            vpc_id: input.dns_zone.vpc_id,
            dnssec_enabled: input.dns_zone.dnssec_key_ref.is_some(),
            actor: input.actor,
            idempotency_key: input.idempotency_key,
            provider_evidence_ref,
            occurred_at_epoch_seconds: input.requested_at_epoch_seconds,
            schema_version: NETWORK_SCHEMA_VERSION,
        })
    }
}
