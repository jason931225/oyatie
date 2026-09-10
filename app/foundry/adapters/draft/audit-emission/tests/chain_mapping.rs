//! The chain mapping, pinned: coordinate (pack/tenant/period), identity
//! and idempotency from the event's own facts, a deterministic digest,
//! and loud sink failure.

use std::cell::RefCell;

use audit_emission_api::AuditEventEmitRequest;
use audit_emission_kernel::AuditEmitter;
use foundry_audit_draft::{AuditDisposition, AuditPortError, AuditSink, FoundryAuditEvent};
use foundry_audit_emission_draft::{ChainAuditSink, FOUNDRY_AUDIT_PACK, emit_request};

const APPLIED_AT_2023_11_14_MS: u64 = 1_700_000_000_000;
const LEAP_DAY_2000_02_29_MS: u64 = 951_782_400_000;

#[derive(Default)]
struct CapturingEmitter {
    requests: RefCell<Vec<AuditEventEmitRequest>>,
    fail: bool,
}

impl AuditEmitter for CapturingEmitter {
    type Envelope = AuditEventEmitRequest;
    type Receipt = ();
    type Error = String;

    fn emit(&self, envelope: AuditEventEmitRequest) -> Result<(), String> {
        if self.fail {
            return Err("audit chain unreachable".into());
        }
        self.requests.borrow_mut().push(envelope);
        Ok(())
    }
}

fn applied() -> FoundryAuditEvent {
    FoundryAuditEvent::new(
        "ten_test",
        "reading.calibrated",
        "prn_alice",
        "dec_1",
        "ent_r1",
        AuditDisposition::Applied { ordinal: 4 },
        APPLIED_AT_2023_11_14_MS,
    )
    .unwrap()
}

#[test]
fn the_producer_envelope_is_deterministic_and_complete() {
    let request = emit_request(&applied());
    assert_eq!(request.coordinate.pack, FOUNDRY_AUDIT_PACK);
    assert_eq!(request.coordinate.tenant_partition, "ten_test");
    assert_eq!(request.coordinate.period, "2023-11");
    assert_eq!(request.event_id, applied().event_id());
    assert_eq!(request.idempotency_key, request.event_id);
    assert!(
        request.payload_digest.starts_with("fnv1a:"),
        "digest scheme is named: {}",
        request.payload_digest,
    );
    assert_eq!(
        request.payload_digest,
        emit_request(&applied()).payload_digest,
        "the digest is deterministic",
    );

    let mut other = applied();
    other.principal_id = "prn_other".into();
    assert_ne!(
        request.payload_digest,
        emit_request(&other).payload_digest,
        "different facts, different digest",
    );
}

#[test]
fn period_derivation_is_civil_and_clock_free() {
    let mut event = applied();
    event.occurred_at_epoch_ms = 0;
    assert_eq!(emit_request(&event).coordinate.period, "1970-01");
    event.occurred_at_epoch_ms = LEAP_DAY_2000_02_29_MS;
    assert_eq!(emit_request(&event).coordinate.period, "2000-02");
}

#[test]
fn emission_forwards_and_identical_retries_share_one_key() {
    let mut sink = ChainAuditSink::new(CapturingEmitter::default());
    sink.emit(applied()).unwrap();
    sink.emit(applied()).unwrap();
    let requests = sink.emitter().requests.borrow();
    assert_eq!(
        requests.len(),
        2,
        "the adapter forwards; dedup is the chain's"
    );
    assert_eq!(requests[0].idempotency_key, requests[1].idempotency_key);
    assert_eq!(requests[0].payload_digest, requests[1].payload_digest);
}

#[test]
fn a_chain_failure_is_loud() {
    let mut sink = ChainAuditSink::new(CapturingEmitter {
        fail: true,
        ..Default::default()
    });
    assert!(matches!(
        sink.emit(applied()),
        Err(AuditPortError::Sink { .. })
    ));
}
