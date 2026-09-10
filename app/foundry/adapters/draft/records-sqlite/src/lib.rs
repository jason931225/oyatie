//! SQLite adapter for the Foundry records port.
//!
//! One file on disk is one durable, per-tenant Action log. The envelope is
//! stored column-per-field; positions are assigned inside a single immediate
//! transaction so ordinals stay dense under interleaved appends; and the
//! idempotency ledger is the same table, enforced by a unique index rather
//! than a second structure that could drift from the log it summarizes.
#![forbid(unsafe_code)]

use std::path::Path;

use foundry_records_draft::{ActionEnvelope, Receipt, RecordsLog, RecordsLogError, SealedEnvelope};
use rusqlite::{Connection, OptionalExtension, params};

pub struct SqliteRecordsLog {
    connection: Connection,
}

impl SqliteRecordsLog {
    /// Open (creating if absent) the log at `path`.
    pub fn open(path: &Path) -> Result<Self, RecordsLogError> {
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 PRAGMA foreign_keys = ON;
                 CREATE TABLE IF NOT EXISTS action_log (
                     tenant_id            TEXT    NOT NULL,
                     ordinal              INTEGER NOT NULL,
                     object_ref           TEXT    NOT NULL,
                     object_sequence      INTEGER NOT NULL,
                     action_type          TEXT    NOT NULL,
                     idempotency_key      TEXT    NOT NULL,
                     schema_revision      INTEGER NOT NULL,
                     payload              BLOB    NOT NULL,
                     observed_at_epoch_ms INTEGER NOT NULL,
                     PRIMARY KEY (tenant_id, ordinal)
                 ) WITHOUT ROWID;
                 CREATE UNIQUE INDEX IF NOT EXISTS action_log_idempotency
                     ON action_log (tenant_id, idempotency_key);
                 CREATE INDEX IF NOT EXISTS action_log_object
                     ON action_log (tenant_id, object_ref);",
            )
            .map_err(storage)?;
        Ok(Self { connection })
    }
}

impl RecordsLog for SqliteRecordsLog {
    fn append(&mut self, envelope: ActionEnvelope) -> Result<Receipt, RecordsLogError> {
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage)?;

        if let Some((receipt, stored)) = spent_key(&transaction, &envelope)? {
            return if stored == envelope {
                Ok(Receipt {
                    deduplicated: true,
                    ..receipt
                })
            } else {
                Err(RecordsLogError::IdempotencyConflict {
                    tenant_id: envelope.tenant_id,
                    idempotency_key: envelope.idempotency_key,
                })
            };
        }

        let ordinal: u64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM action_log WHERE tenant_id = ?1",
                params![envelope.tenant_id],
                |row| row.get(0),
            )
            .map_err(storage)?;
        let object_sequence: u64 = transaction
            .query_row(
                "SELECT COUNT(*) + 1 FROM action_log WHERE tenant_id = ?1 AND object_ref = ?2",
                params![envelope.tenant_id, envelope.object_ref],
                |row| row.get(0),
            )
            .map_err(storage)?;
        transaction
            .execute(
                "INSERT INTO action_log (tenant_id, ordinal, object_ref, object_sequence,
                     action_type, idempotency_key, schema_revision, payload,
                     observed_at_epoch_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    envelope.tenant_id,
                    ordinal,
                    envelope.object_ref,
                    object_sequence,
                    envelope.action_type,
                    envelope.idempotency_key,
                    envelope.schema_revision,
                    envelope.payload,
                    envelope.observed_at_epoch_ms,
                ],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
        Ok(Receipt {
            ordinal,
            object_sequence,
            deduplicated: false,
        })
    }

    fn replay(
        &self,
        tenant_id: &str,
        from_ordinal: u64,
    ) -> Result<Vec<SealedEnvelope>, RecordsLogError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT tenant_id, ordinal, object_ref, object_sequence, action_type,
                        idempotency_key, schema_revision, payload, observed_at_epoch_ms
                 FROM action_log
                 WHERE tenant_id = ?1 AND ordinal >= ?2
                 ORDER BY ordinal",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![tenant_id, from_ordinal], sealed_from_row)
            .map_err(storage)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(storage)
    }

    fn head(&self, tenant_id: &str) -> Result<u64, RecordsLogError> {
        self.connection
            .query_row(
                "SELECT COALESCE(MAX(ordinal), 0) FROM action_log WHERE tenant_id = ?1",
                params![tenant_id],
                |row| row.get(0),
            )
            .map_err(storage)
    }
}

fn spent_key(
    transaction: &rusqlite::Transaction<'_>,
    envelope: &ActionEnvelope,
) -> Result<Option<(Receipt, ActionEnvelope)>, RecordsLogError> {
    transaction
        .query_row(
            "SELECT tenant_id, ordinal, object_ref, object_sequence, action_type,
                    idempotency_key, schema_revision, payload, observed_at_epoch_ms
             FROM action_log
             WHERE tenant_id = ?1 AND idempotency_key = ?2",
            params![envelope.tenant_id, envelope.idempotency_key],
            sealed_from_row,
        )
        .optional()
        .map_err(storage)
        .map(|sealed| sealed.map(|sealed| (sealed.receipt, sealed.envelope)))
}

fn sealed_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SealedEnvelope> {
    let envelope = ActionEnvelope {
        tenant_id: row.get(0)?,
        object_ref: row.get(2)?,
        action_type: row.get(4)?,
        idempotency_key: row.get(5)?,
        schema_revision: row.get(6)?,
        payload: row.get(7)?,
        observed_at_epoch_ms: row.get(8)?,
    };
    Ok(SealedEnvelope {
        envelope,
        receipt: Receipt {
            ordinal: row.get(1)?,
            object_sequence: row.get(3)?,
            deduplicated: false,
        },
    })
}

fn storage(error: rusqlite::Error) -> RecordsLogError {
    RecordsLogError::Storage {
        detail: error.to_string(),
    }
}
