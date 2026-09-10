use crate::NETWORK_SCHEMA_VERSION;
use crate::error::CloudNetworkError;
use crate::interconnect::{
    DirectInterconnect, DirectInterconnectCreate, InterconnectPartner, InterconnectPartnerCreate,
};
use crate::provider::{
    NetworkProviderDirectInterconnectOperation, NetworkProviderKind,
    validate_network_provider_direct_interconnect_ref,
};
use compute_resource::PrincipalId;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderDirectInterconnectCreateRequest {
    pub request_id: String,                   // data_class: INTERNAL_ONLY
    pub provider_virtual_circuit_ref: String, // data_class: INTERNAL_ONLY
    pub interconnect_partners: Vec<InterconnectPartnerCreate>, // data_class: INTERNAL_ONLY
    pub direct_interconnect: DirectInterconnectCreate, // data_class: INTERNAL_ONLY
    pub actor: String,                        // data_class: INTERNAL_ONLY
    pub idempotency_key: String,              // data_class: INTERNAL_ONLY
    pub requested_at_epoch_seconds: u64,      // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkProviderDirectInterconnectReceipt {
    pub provider: NetworkProviderKind, // data_class: PUBLIC
    pub operation: NetworkProviderDirectInterconnectOperation, // data_class: PUBLIC
    pub request_id: String,            // data_class: INTERNAL_ONLY
    pub provider_request_id: String,   // data_class: INTERNAL_ONLY
    pub provider_virtual_circuit_ref: String, // data_class: INTERNAL_ONLY
    pub resource_id: String,           // data_class: INTERNAL_ONLY
    pub tenant_id: String,             // data_class: INTERNAL_ONLY
    pub region: String,                // data_class: PUBLIC
    pub partner_id: String,            // data_class: INTERNAL_ONLY
    pub peering_location: String,      // data_class: PUBLIC
    pub physical_port_id: String,      // data_class: INTERNAL_ONLY
    pub vlan_tag: u16,                 // data_class: INTERNAL_ONLY
    pub bandwidth_mbps: u32,           // data_class: PUBLIC
    pub redundant_port_count: u8,      // data_class: PUBLIC
    pub bgp_session_count: usize,      // data_class: PUBLIC
    pub advertised_prefix_count: usize, // data_class: PUBLIC
    pub actor: String,                 // data_class: INTERNAL_ONLY
    pub idempotency_key: String,       // data_class: INTERNAL_ONLY
    pub provider_evidence_ref: String, // data_class: INTERNAL_ONLY
    pub occurred_at_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    pub schema_version: u32,           // data_class: PUBLIC
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkProviderDirectInterconnectError {
    InvalidProviderVirtualCircuitRef,
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

impl NetworkProviderDirectInterconnectCreateRequest {
    pub fn validate(&self) -> Result<(), NetworkProviderDirectInterconnectError> {
        validate_network_provider_direct_interconnect_ref(
            &self.request_id,
            NetworkProviderDirectInterconnectError::InvalidProviderRequestId,
        )?;
        validate_network_provider_direct_interconnect_ref(
            &self.provider_virtual_circuit_ref,
            NetworkProviderDirectInterconnectError::InvalidProviderVirtualCircuitRef,
        )?;
        validate_network_provider_direct_interconnect_ref(
            &self.idempotency_key,
            NetworkProviderDirectInterconnectError::InvalidIdempotencyKey,
        )?;
        let mut known_partners = BTreeMap::new();
        for partner_input in self.interconnect_partners.clone() {
            let partner = InterconnectPartner::new(partner_input)
                .map_err(NetworkProviderDirectInterconnectError::InvalidRequestShape)?;
            if known_partners
                .insert(partner.id.value.clone(), partner)
                .is_some()
            {
                return Err(NetworkProviderDirectInterconnectError::InvalidRequestShape(
                    CloudNetworkError::DuplicateInterconnectPartner,
                ));
            }
        }
        DirectInterconnect::new(&known_partners, self.direct_interconnect.clone())
            .map_err(NetworkProviderDirectInterconnectError::InvalidRequestShape)?;
        PrincipalId::new(self.actor.clone())
            .map_err(|_| NetworkProviderDirectInterconnectError::InvalidActorRef)?;
        Ok(())
    }
}

impl NetworkProviderDirectInterconnectReceipt {
    pub fn create_direct_interconnect(
        provider: NetworkProviderKind,
        input: NetworkProviderDirectInterconnectCreateRequest,
        provider_request_id: impl Into<String>,
        provider_evidence_ref: impl Into<String>,
    ) -> Result<Self, NetworkProviderDirectInterconnectError> {
        input.validate()?;
        let provider_request_id = provider_request_id.into();
        let provider_evidence_ref = provider_evidence_ref.into();
        validate_network_provider_direct_interconnect_ref(
            &provider_request_id,
            NetworkProviderDirectInterconnectError::InvalidProviderRequestId,
        )?;
        validate_network_provider_direct_interconnect_ref(
            &provider_evidence_ref,
            NetworkProviderDirectInterconnectError::InvalidProviderEvidenceRef,
        )?;
        Ok(Self {
            provider,
            operation: NetworkProviderDirectInterconnectOperation::CreateDirectInterconnect,
            request_id: input.request_id,
            provider_request_id,
            provider_virtual_circuit_ref: input.provider_virtual_circuit_ref,
            resource_id: input.direct_interconnect.resource_id,
            tenant_id: input.direct_interconnect.tenant_id,
            region: input.direct_interconnect.region,
            partner_id: input.direct_interconnect.partner_id,
            peering_location: input.direct_interconnect.peering_location,
            physical_port_id: input.direct_interconnect.physical_port_id,
            vlan_tag: input.direct_interconnect.vlan_tag,
            bandwidth_mbps: input.direct_interconnect.bandwidth_mbps,
            redundant_port_count: input.direct_interconnect.redundant_port_count,
            bgp_session_count: input.direct_interconnect.bgp_sessions.len(),
            advertised_prefix_count: input.direct_interconnect.advertised_prefixes.len(),
            actor: input.actor,
            idempotency_key: input.idempotency_key,
            provider_evidence_ref,
            occurred_at_epoch_seconds: input.requested_at_epoch_seconds,
            schema_version: NETWORK_SCHEMA_VERSION,
        })
    }
}
