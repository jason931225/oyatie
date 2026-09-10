//! SQLite adapter for the Foundry projection port: one database file is
//! one durable, indexed projection. Object rows carry the adapter's
//! canonical bytes as the round-trip source of truth; a per-property
//! index table carries the value kind plus StorageClass-affinity
//! columns, so predicates prune through real secondary indexes.
#![forbid(unsafe_code)]

mod codec;
mod key_law;
mod reset;
mod scan;
mod write;

use std::path::Path;

use foundry_projection_draft::{
    AppliedEntry, ApplyReceipt, KeyDesignations, Page, PageRequest, ProjectedLink, ProjectedObject,
    ProjectionStore, ProjectionStoreError, PropertyPredicate,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::codec::decode_object;

/// The name of the property index; pinned by the adapter's tests via
/// `EXPLAIN QUERY PLAN`.
pub const PROPERTY_INDEX_NAME: &str = "projection_property_kind";

pub struct SqliteProjectionStore {
    connection: Connection,
}

impl SqliteProjectionStore {
    /// Open (creating if absent) the projection at `path`.
    pub fn open(path: &Path) -> Result<Self, ProjectionStoreError> {
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 CREATE TABLE IF NOT EXISTS projection_heads (
                     tenant_id       TEXT    NOT NULL PRIMARY KEY,
                     applied_ordinal INTEGER NOT NULL
                 ) WITHOUT ROWID;
                 CREATE TABLE IF NOT EXISTS projection_entries (
                     tenant_id   TEXT    NOT NULL,
                     ordinal     INTEGER NOT NULL,
                     entry_bytes BLOB    NOT NULL,
                     PRIMARY KEY (tenant_id, ordinal)
                 ) WITHOUT ROWID;
                 CREATE TABLE IF NOT EXISTS projection_objects (
                     tenant_id    TEXT NOT NULL,
                     object_ref   TEXT NOT NULL,
                     entity_type  TEXT NOT NULL,
                     object_bytes BLOB NOT NULL,
                     PRIMARY KEY (tenant_id, object_ref)
                 ) WITHOUT ROWID;
                 CREATE INDEX IF NOT EXISTS projection_objects_type
                     ON projection_objects (tenant_id, entity_type, object_ref);
                 CREATE TABLE IF NOT EXISTS projection_property_index (
                     tenant_id   TEXT NOT NULL,
                     object_ref  TEXT NOT NULL,
                     entity_type TEXT NOT NULL,
                     property    TEXT NOT NULL,
                     value_kind  TEXT NOT NULL,
                     int_value   INTEGER,
                     real_value  REAL,
                     text_value  TEXT,
                     PRIMARY KEY (tenant_id, object_ref, property)
                 ) WITHOUT ROWID;
                 CREATE INDEX IF NOT EXISTS projection_property_kind
                     ON projection_property_index
                     (tenant_id, entity_type, property, value_kind, object_ref);
                 CREATE TABLE IF NOT EXISTS projection_links (
                     tenant_id  TEXT NOT NULL,
                     from_ref   TEXT NOT NULL,
                     link_type  TEXT NOT NULL,
                     to_ref     TEXT NOT NULL,
                     observed_at_epoch_ms INTEGER NOT NULL,
                     PRIMARY KEY (tenant_id, from_ref, link_type, to_ref)
                 ) WITHOUT ROWID;
                 CREATE INDEX IF NOT EXISTS projection_links_inbound
                     ON projection_links (tenant_id, to_ref, link_type, from_ref);
                 CREATE TABLE IF NOT EXISTS projection_poisons (
                     tenant_id TEXT    NOT NULL,
                     ordinal   INTEGER NOT NULL,
                     reason    TEXT    NOT NULL,
                     PRIMARY KEY (tenant_id, ordinal)
                 ) WITHOUT ROWID;",
            )
            .map_err(storage)?;
        Ok(Self { connection })
    }
}

impl SqliteProjectionStore {
    fn read_links(
        &self,
        sql: &str,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError> {
        require_trimmed(tenant_id, "blank tenant")?;
        let mut statement = self.connection.prepare(sql).map_err(storage)?;
        let rows = statement
            .query_map(params![tenant_id, object_ref], |row| {
                Ok(ProjectedLink {
                    link_type: row.get(0)?,
                    from_object_ref: row.get(1)?,
                    to_object_ref: row.get(2)?,
                    observed_at_epoch_ms: row.get(3)?,
                })
            })
            .map_err(storage)?;
        let mut edges = Vec::new();
        for row in rows {
            edges.push(row.map_err(storage)?);
        }
        Ok(edges)
    }
}

impl ProjectionStore for SqliteProjectionStore {
    fn reset_tenant(&mut self, tenant_id: &str) -> Result<u64, ProjectionStoreError> {
        reset::reset_tenant(&mut self.connection, tenant_id)
    }

    fn apply(
        &mut self,
        entry: AppliedEntry,
        keys: &KeyDesignations,
    ) -> Result<ApplyReceipt, ProjectionStoreError> {
        write::apply(&mut self.connection, entry, keys)
    }

    fn applied_head(&self, tenant_id: &str) -> Result<u64, ProjectionStoreError> {
        require_trimmed(tenant_id, "blank tenant")?;
        self.connection
            .query_row(
                "SELECT applied_ordinal FROM projection_heads WHERE tenant_id = ?1",
                params![tenant_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)
            .map(|head| head.unwrap_or(0))
    }

    fn get(
        &self,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Option<ProjectedObject>, ProjectionStoreError> {
        require_trimmed(tenant_id, "blank tenant")?;
        let bytes: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT object_bytes FROM projection_objects
                 WHERE tenant_id = ?1 AND object_ref = ?2",
                params![tenant_id, object_ref],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        bytes
            .map(|bytes| decode_object(&bytes).map_err(codec_error))
            .transpose()
    }

    fn objects_of_type(
        &self,
        tenant_id: &str,
        entity_type: &str,
        page: &PageRequest,
    ) -> Result<Page, ProjectionStoreError> {
        scan::scan(&self.connection, tenant_id, entity_type, None, page)
    }

    fn filter(
        &self,
        tenant_id: &str,
        entity_type: &str,
        predicate: &PropertyPredicate,
        page: &PageRequest,
    ) -> Result<Page, ProjectionStoreError> {
        scan::scan(
            &self.connection,
            tenant_id,
            entity_type,
            Some(predicate),
            page,
        )
    }

    fn links_from(
        &self,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError> {
        self.read_links(
            "SELECT link_type, from_ref, to_ref, observed_at_epoch_ms FROM projection_links
             WHERE tenant_id = ?1 AND from_ref = ?2 ORDER BY from_ref, link_type, to_ref",
            tenant_id,
            object_ref,
        )
    }

    fn links_to(
        &self,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError> {
        self.read_links(
            "SELECT link_type, from_ref, to_ref, observed_at_epoch_ms FROM projection_links
             WHERE tenant_id = ?1 AND to_ref = ?2 ORDER BY from_ref, link_type, to_ref",
            tenant_id,
            object_ref,
        )
    }

    fn poisoned(&self, tenant_id: &str) -> Result<Vec<(u64, String)>, ProjectionStoreError> {
        require_trimmed(tenant_id, "blank tenant")?;
        let mut statement = self
            .connection
            .prepare(
                "SELECT ordinal, reason FROM projection_poisons
                 WHERE tenant_id = ?1 ORDER BY ordinal",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![tenant_id], |row| {
                Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?;
        let mut poisons = Vec::new();
        for row in rows {
            poisons.push(row.map_err(storage)?);
        }
        Ok(poisons)
    }
}

pub(crate) fn storage(error: rusqlite::Error) -> ProjectionStoreError {
    ProjectionStoreError::Storage {
        detail: error.to_string(),
    }
}

pub(crate) fn codec_error(error: codec::CodecError) -> ProjectionStoreError {
    ProjectionStoreError::Storage {
        detail: format!("codec: {}", error.detail),
    }
}

pub(crate) fn require_trimmed(
    value: &str,
    detail: &'static str,
) -> Result<(), ProjectionStoreError> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(ProjectionStoreError::Entry { detail });
    }
    Ok(())
}
