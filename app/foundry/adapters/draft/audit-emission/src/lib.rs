//! Foundry's adapter onto the platform audit chain: maps the product's
//! [`FoundryAuditEvent`] onto the `audit-chain.audit-event-emit`
//! producer envelope and forwards through the platform's
//! [`AuditEmitter`] port.
//!
//! Rehearsal-grade by declaration: the platform has no runtime emitter
//! implementation yet (every producer in the tree is contract-only), so
//! this adapter proves the mapping against the port's types — the same
//! posture as the platform's own producers. The payload digest uses an
//! inline FNV-1a over a canonical field rendering until the audit
//! chain's canonical-encoding contract exists; the digest is
//! deterministic, which is all dedup downstream requires.
#![forbid(unsafe_code)]
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use audit_emission_api::AuditEventEmitRequest;
use audit_emission_kernel::{AuditEmitter, ChainCoordinate};
use foundry_audit_draft::{AuditPortError, AuditSink, FoundryAuditEvent};

pub const FOUNDRY_AUDIT_PACK: &str = "foundry";

/// The platform-chain sink: generic over any emitter of the producer
/// envelope, so composition decides the transport and tests use a
/// capturing double.
pub struct ChainAuditSink<E> {
    emitter: E,
}

impl<E> ChainAuditSink<E> {
    pub fn new(emitter: E) -> Self {
        Self { emitter }
    }

    pub fn emitter(&self) -> &E {
        &self.emitter
    }
}

/// The producer envelope for one event — deterministic, clock-free:
/// every field derives from the event's own facts.
pub fn emit_request(event: &FoundryAuditEvent) -> AuditEventEmitRequest {
    let event_id = event.event_id();
    let canonical = format!(
        "{}\n{}\n{}\n{}\n{}\n{:?}\n{}",
        event.tenant_id,
        event.audit_event_type,
        event.principal_id,
        event.decision_id,
        event.object_ref,
        event.disposition,
        event.occurred_at_epoch_ms,
    );
    AuditEventEmitRequest {
        coordinate: ChainCoordinate {
            pack: FOUNDRY_AUDIT_PACK.to_owned(),
            tenant_partition: event.tenant_id.clone(),
            period: civil_period(event.occurred_at_epoch_ms),
        },
        event_id: event_id.clone(),
        payload_digest: format!("fnv1a:{:016x}", fnv1a(canonical.as_bytes())),
        idempotency_key: event_id,
    }
}

/// `YYYY-MM` for an epoch-milliseconds instant, proleptic Gregorian —
/// the civil-from-days algorithm, no clock and no dependency.
fn civil_period(epoch_ms: u64) -> String {
    let days = (epoch_ms / 86_400_000) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}")
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

impl<E> AuditSink for ChainAuditSink<E>
where
    E: AuditEmitter<Envelope = AuditEventEmitRequest>,
    E::Error: core::fmt::Debug,
{
    fn emit(&mut self, event: FoundryAuditEvent) -> Result<(), AuditPortError> {
        self.emitter
            .emit(emit_request(&event))
            .map(|_| ())
            .map_err(|error| AuditPortError::Sink {
                detail: format!("{error:?}"),
            })
    }
}
