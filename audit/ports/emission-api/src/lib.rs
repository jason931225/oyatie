#![allow(dead_code)]

use audit_emission_kernel::ChainCoordinate;

pub const AUDIT_EVENT_EMIT_SURFACE: &str = "audit-chain.audit-event-emit";

/// Outbox topic for emitted audit events per ADR-0145 inter-µservice contract.
pub const AUDIT_EVENT_TOPIC: &str = "oyatie.platform.audit";

#[derive(Clone, Debug)]
pub struct AuditEventEmitRequest {
    pub coordinate: ChainCoordinate,
    pub event_id: String,
    pub payload_digest: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug)]
pub struct AuditEventEmitResponse {
    pub audit_id: String,
    pub period_id: String,
    pub merkle_root_ref: String,
}
