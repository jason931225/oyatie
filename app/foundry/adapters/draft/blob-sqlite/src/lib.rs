//! SQLite adapter for the Foundry blob port.
//!
//! One database file is one tenant-scoped, content-addressed blob store. The
//! address is verified on BOTH sides of the boundary: recomputed before insert
//! (so a bug between hash and write cannot store bytes under the wrong name)
//! and recomputed on every read (so silent corruption surfaces as a loud
//! error, never as wrong bytes handed to a caller).
#![forbid(unsafe_code)]

use std::path::Path;

use foundry_blob_draft::{BlobRef, BlobStore, BlobStoreError};
use rusqlite::{Connection, OptionalExtension, params};

pub struct SqliteBlobStore {
    connection: Connection,
}

impl SqliteBlobStore {
    /// Open (creating if absent) the store at `path`.
    pub fn open(path: &Path) -> Result<Self, BlobStoreError> {
        let connection = Connection::open(path).map_err(storage)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 PRAGMA synchronous = FULL;
                 CREATE TABLE IF NOT EXISTS blobs (
                     tenant_id  TEXT NOT NULL,
                     digest_hex TEXT NOT NULL,
                     bytes      BLOB NOT NULL,
                     PRIMARY KEY (tenant_id, digest_hex)
                 ) WITHOUT ROWID;",
            )
            .map_err(storage)?;
        Ok(Self { connection })
    }

    /// Test-only fault injection: flip one stored byte behind an address so
    /// the read-side verification can be proven rather than trusted.
    pub fn corrupt_for_test(&mut self, reference: &BlobRef) -> Result<(), BlobStoreError> {
        self.connection
            .execute(
                "UPDATE blobs SET bytes = X'DEAD' WHERE digest_hex = ?1",
                params![reference.digest_hex()],
            )
            .map_err(storage)
            .map(|_| ())
    }
}

impl BlobStore for SqliteBlobStore {
    fn put(&mut self, tenant_id: &str, bytes: &[u8]) -> Result<BlobRef, BlobStoreError> {
        let reference = BlobRef::for_bytes(bytes);
        self.connection
            .execute(
                "INSERT OR IGNORE INTO blobs (tenant_id, digest_hex, bytes)
                 VALUES (?1, ?2, ?3)",
                params![tenant_id, reference.digest_hex(), bytes],
            )
            .map_err(storage)?;
        Ok(reference)
    }

    fn get(&self, tenant_id: &str, reference: &BlobRef) -> Result<Option<Vec<u8>>, BlobStoreError> {
        let bytes: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT bytes FROM blobs WHERE tenant_id = ?1 AND digest_hex = ?2",
                params![tenant_id, reference.digest_hex()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        match bytes {
            None => Ok(None),
            Some(bytes) => {
                if BlobRef::for_bytes(&bytes) != *reference {
                    return Err(BlobStoreError::Storage {
                        detail: format!("stored bytes no longer hash to their address {reference}"),
                    });
                }
                Ok(Some(bytes))
            }
        }
    }
}

fn storage(error: rusqlite::Error) -> BlobStoreError {
    BlobStoreError::Storage {
        detail: error.to_string(),
    }
}
