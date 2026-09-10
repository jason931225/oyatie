//! Valkey Stream [`EventSink`] for the intelligence-app subscription pool
//! (ADR-0384 Path B, D6). One append-only receipt stream per tenant.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;
use std::sync::Mutex;

use intelligence_kernel::{EventSink, LlmGatewayEvent};
use redis::Commands;
use tracing::warn;

/// Valkey generates the entry id, which is what an append-only receipt log
/// wants; a caller-chosen id would have to be monotonic and unique.
const AUTO_GENERATED_ENTRY_ID: &str = "*";

fn stream_key(tenant_id: &str) -> String {
    format!("intelligence-app-receipts:{tenant_id}")
}

/// Errors raised during event emission. Non-fatal per D6 contract.
#[derive(Debug)]
pub enum ValkeySinkError {
    /// Redis/Valkey connection error.
    Connection(String),
    /// XADD command failed.
    Xadd(String),
}

impl fmt::Display for ValkeySinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValkeySinkError::Connection(msg) => write!(f, "valkey connection error: {msg}"),
            ValkeySinkError::Xadd(msg) => write!(f, "valkey XADD error: {msg}"),
        }
    }
}

/// [`EventSink`] backed by a Valkey Stream via XADD.
///
/// The `Mutex` is what lets the `&self` trait method borrow the connection
/// mutably; it is uncontended at one emitter per service instance.
pub struct ValkeyEventSink {
    conn: Mutex<redis::Connection>, // data_class: INTERNAL_ONLY
}

impl fmt::Debug for ValkeyEventSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValkeyEventSink").finish_non_exhaustive()
    }
}

impl ValkeyEventSink {
    /// Connect to Valkey at `url`; `rediss://` selects TLS. Fails eagerly so
    /// the composition root surfaces a bad endpoint at start-up rather than
    /// on the first swallowed emit.
    pub fn connect(url: &str) -> Result<Self, ValkeySinkError> {
        let client =
            redis::Client::open(url).map_err(|e| ValkeySinkError::Connection(e.to_string()))?;
        let conn = client
            .get_connection()
            .map_err(|e| ValkeySinkError::Connection(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn try_emit(&self, event: &LlmGatewayEvent) -> Result<(), ValkeySinkError> {
        let key = stream_key(event.tenant_id.as_str());

        let fields: Vec<(&str, String)> = vec![
            ("request_id", event.request_id.clone()),
            ("tenant_id", event.tenant_id.as_str().to_string()),
            ("agent_id", event.agent_id.as_str().to_string()),
            ("seat_id", event.seat_id.as_str().to_string()),
            ("provider", event.provider.to_string()),
            ("model", event.model.clone()),
            ("prompt_tokens", event.prompt_tokens.to_string()),
            ("completion_tokens", event.completion_tokens.to_string()),
            ("ms_latency", event.ms_latency.to_string()),
            ("status", format!("{:?}", event.status)),
            ("timestamp_unix_ms", event.timestamp_unix_ms.to_string()),
        ];

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| ValkeySinkError::Connection("mutex poisoned".to_string()))?;

        let _: redis::Value = conn
            .xadd(&key, AUTO_GENERATED_ENTRY_ID, &fields)
            .map_err(|e| ValkeySinkError::Xadd(e.to_string()))?;

        Ok(())
    }
}

impl EventSink for ValkeyEventSink {
    /// A transport failure is logged and swallowed: the D6 contract is that a
    /// receipt never fails the request that produced it.
    fn emit(&self, event: LlmGatewayEvent) {
        if let Err(e) = self.try_emit(&event) {
            warn!(
                request_id = %event.request_id,
                error = %e,
                "ValkeyEventSink: emit failed (non-fatal)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intelligence_kernel::{AgentId, EventStatus, Provider, SeatId, TenantId as KernelTenantId};

    fn test_event() -> LlmGatewayEvent {
        LlmGatewayEvent {
            request_id: "req-001".to_string(),
            tenant_id: KernelTenantId::new("tenant-a").unwrap(),
            agent_id: AgentId::new("agent-1").unwrap(),
            seat_id: SeatId::new("seat-1").unwrap(),
            provider: Provider::Anthropic,
            model: "claude-3-5-sonnet".to_string(),
            prompt_tokens: 100,
            completion_tokens: 50,
            ms_latency: 320,
            status: EventStatus::Ok,
            timestamp_unix_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn stream_key_format() {
        assert_eq!(stream_key("tenant-a"), "intelligence-app-receipts:tenant-a");
    }

    #[test]
    fn connect_fails_on_bad_url() {
        let result = ValkeyEventSink::connect("not-a-url");
        assert!(
            matches!(result, Err(ValkeySinkError::Connection(_))),
            "expected Connection error for bad URL, got: {result:?}"
        );
    }

    #[test]
    fn connect_fails_on_unreachable_host() {
        // A privileged port nothing in the test environment binds.
        let result = ValkeyEventSink::connect("redis://127.0.0.1:1");
        assert!(
            matches!(result, Err(ValkeySinkError::Connection(_))),
            "expected Connection error for unreachable host, got: {result:?}"
        );
    }

    #[test]
    fn error_display_connection() {
        let e = ValkeySinkError::Connection("timeout".to_string());
        assert!(e.to_string().contains("valkey connection error"));
    }

    #[test]
    fn error_display_xadd() {
        let e = ValkeySinkError::Xadd("WRONGTYPE".to_string());
        assert!(e.to_string().contains("valkey XADD error"));
    }

    /// `try_emit` renders `status` through `Debug`; a variant without a
    /// distinct repr would collapse two outcomes into one receipt field.
    #[test]
    fn event_status_variants_have_distinct_debug_reprs() {
        let mut seen = std::collections::HashSet::new();
        for status in [
            EventStatus::Ok,
            EventStatus::UpstreamError,
            EventStatus::RateLimited,
            EventStatus::Forbidden,
            EventStatus::PoolExhausted,
        ] {
            let mut ev = test_event();
            ev.status = status;
            assert!(
                seen.insert(format!("{:?}", ev.status)),
                "duplicate Debug repr for {status:?}"
            );
        }
    }
}
