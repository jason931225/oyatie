//! ClickHouse [`EventSink`] for the intelligence-app subscription pool
//! (ADR-0384 Path B, D6). One row per gateway event, one table per tenant.
//!
//! The adapter never issues DDL: the receipts table must already exist.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

use std::fmt;
use std::sync::Mutex;

use intelligence_kernel::{EventSink, LlmGatewayEvent};
use shared_olap_clickhouse_adapter::{ClickHouseConfig, ClickHouseOlapClient};
use shared_olap_client_kernel::{
    InsertBatch, OlapClient, QualifiedTable, TableName, TenantId, Value,
};
use tracing::warn;

const TABLE: &str = "intelligence_app_receipts";

/// Errors raised during event emission. Non-fatal per D6 contract.
#[derive(Debug)]
pub enum ClickHouseSinkError {
    /// Row construction failed (e.g. invalid tenant_id format).
    RowBuild(String),
    /// ClickHouse INSERT returned an error.
    Insert(String),
}

impl fmt::Display for ClickHouseSinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClickHouseSinkError::RowBuild(msg) => write!(f, "row build error: {msg}"),
            ClickHouseSinkError::Insert(msg) => write!(f, "clickhouse insert error: {msg}"),
        }
    }
}

/// [`EventSink`] backed by ClickHouse.
///
/// The `Mutex` is what lets the `&self` trait method borrow the client
/// mutably; it is uncontended at one emitter per service instance.
pub struct ClickHouseEventSink {
    client: Mutex<ClickHouseOlapClient>, // data_class: INTERNAL_ONLY
}

impl fmt::Debug for ClickHouseEventSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClickHouseEventSink")
            .finish_non_exhaustive()
    }
}

impl ClickHouseEventSink {
    pub fn new(config: ClickHouseConfig) -> Self {
        Self {
            client: Mutex::new(ClickHouseOlapClient::new(config)),
        }
    }

    fn try_emit(&self, event: &LlmGatewayEvent) -> Result<(), ClickHouseSinkError> {
        let tenant_id = TenantId::try_new(event.tenant_id.as_str())
            .map_err(|e| ClickHouseSinkError::RowBuild(format!("tenant_id invalid: {e:?}")))?;

        let table = TableName::try_new(TABLE)
            .map_err(|e| ClickHouseSinkError::RowBuild(format!("table name invalid: {e:?}")))?;

        let target = QualifiedTable::new(tenant_id, table);

        let (columns, row): (Vec<String>, Vec<Value>) = receipt_row(event).into_iter().unzip();

        let batch = InsertBatch {
            target,
            columns,
            rows: vec![row],
        };

        let mut client = self
            .client
            .lock()
            .map_err(|_| ClickHouseSinkError::Insert("mutex poisoned".to_string()))?;

        client
            .insert(&batch)
            .map_err(|e| ClickHouseSinkError::Insert(e.to_string()))?;

        Ok(())
    }
}

/// Pair each column with its value so the two can never be reordered apart.
fn receipt_row(event: &LlmGatewayEvent) -> Vec<(String, Value)> {
    vec![
        ("request_id", Value::String(event.request_id.clone())),
        (
            "tenant_id",
            Value::String(event.tenant_id.as_str().to_string()),
        ),
        (
            "agent_id",
            Value::String(event.agent_id.as_str().to_string()),
        ),
        ("seat_id", Value::String(event.seat_id.as_str().to_string())),
        ("provider", Value::String(event.provider.to_string())),
        ("model", Value::String(event.model.clone())),
        ("prompt_tokens", Value::UInt(event.prompt_tokens)),
        ("completion_tokens", Value::UInt(event.completion_tokens)),
        ("ms_latency", Value::UInt(event.ms_latency)),
        ("status", Value::String(format!("{:?}", event.status))),
        ("timestamp_unix_ms", Value::UInt(event.timestamp_unix_ms)),
    ]
    .into_iter()
    .map(|(column, value)| (column.to_string(), value))
    .collect()
}

impl EventSink for ClickHouseEventSink {
    /// A failed insert is logged and swallowed: the D6 contract is that a
    /// receipt never fails the request that produced it.
    fn emit(&self, event: LlmGatewayEvent) {
        if let Err(e) = self.try_emit(&event) {
            warn!(
                request_id = %event.request_id,
                error = %e,
                "ClickHouseEventSink: emit failed (non-fatal)"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intelligence_kernel::{AgentId, EventStatus, Provider, SeatId, TenantId as KernelTenantId};

    fn test_config() -> ClickHouseConfig {
        ClickHouseConfig {
            url: "http://clickhouse.test:8123".to_string(),
            user: "default".to_string(),
            password: "test".to_string(),
        }
    }

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
    fn sink_constructs_without_panic() {
        let _sink = ClickHouseEventSink::new(test_config());
    }

    #[test]
    fn emit_non_fatal_on_clickhouse_error() {
        let sink = ClickHouseEventSink::new(test_config());
        sink.emit(test_event());
    }

    #[test]
    fn try_emit_returns_insert_error_on_deferred_backend() {
        let sink = ClickHouseEventSink::new(test_config());
        let result = sink.try_emit(&test_event());
        assert!(
            matches!(result, Err(ClickHouseSinkError::Insert(_))),
            "expected Insert error from deferred backend, got: {result:?}"
        );
    }

    #[test]
    fn try_emit_error_on_invalid_tenant_id() {
        let sink = ClickHouseEventSink::new(test_config());
        let mut event = test_event();
        // The kernel tenant id admits a space; the olap tenant id does not.
        event.tenant_id = KernelTenantId::new("tenant a").unwrap();
        let result = sink.try_emit(&event);
        assert!(
            matches!(result, Err(ClickHouseSinkError::RowBuild(_))),
            "expected RowBuild error for invalid tenant_id, got: {result:?}"
        );
    }

    #[test]
    fn receipt_row_pairs_every_column_with_its_own_value() {
        let row = receipt_row(&test_event());
        let columns: Vec<&str> = row.iter().map(|(column, _)| column.as_str()).collect();
        assert_eq!(
            columns,
            [
                "request_id",
                "tenant_id",
                "agent_id",
                "seat_id",
                "provider",
                "model",
                "prompt_tokens",
                "completion_tokens",
                "ms_latency",
                "status",
                "timestamp_unix_ms",
            ]
        );
        assert_eq!(row[0].1, Value::String("req-001".to_string()));
        assert_eq!(row[6].1, Value::UInt(100));
        assert_eq!(row[7].1, Value::UInt(50));
        assert_eq!(row[8].1, Value::UInt(320));
        assert_eq!(row[10].1, Value::UInt(1_700_000_000_000));
    }

    #[test]
    fn emit_does_not_panic_for_any_event_status() {
        let sink = ClickHouseEventSink::new(test_config());
        for status in [
            EventStatus::Ok,
            EventStatus::UpstreamError,
            EventStatus::RateLimited,
            EventStatus::Forbidden,
            EventStatus::PoolExhausted,
        ] {
            let mut ev = test_event();
            ev.status = status;
            sink.emit(ev);
        }
    }
}
