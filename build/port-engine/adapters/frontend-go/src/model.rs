//! The decoded Go source model: artifact bytes in, identity and declarations out.

use std::collections::BTreeSet;

use port_engine_api::{Declaration, Digest, UnitId};

use crate::convert::convert_declarations;
use crate::error::SnapshotError;
use crate::vocabulary::{
    PRODUCER_BOOTSTRAP_GO, PRODUCER_OWNED_RUST, SCHEMA_VERSION_DECLARATIONS,
    SCHEMA_VERSION_IDENTITY_ONLY,
};
use crate::wire::SnapshotDocument;

#[derive(Clone, Debug, Eq, PartialEq)]
struct Package {
    unit: UnitId,
    producer: String,
    declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoSourceModel {
    schema_version: u32,
    language: String,
    snapshot_digest: Digest,
    packages: Vec<Package>,
}

impl GoSourceModel {
    /// Decode snapshot JSON bytes into an unadmitted Go model.
    ///
    /// # Errors
    /// [`SnapshotError`] on parse failure, schema violation, unknown producer, or duplicate unit.
    pub fn decode(bytes: &[u8]) -> Result<Self, SnapshotError> {
        let text = std::str::from_utf8(bytes).map_err(|err| SnapshotError::Parse {
            detail: format!("utf-8: {err}"),
        })?;
        Self::decode_str(text)
    }

    /// Decode snapshot JSON from an in-memory string (test hook and future adapter input).
    ///
    /// # Errors
    /// Same refusals as [`Self::decode`].
    pub fn decode_str(json: &str) -> Result<Self, SnapshotError> {
        let doc: SnapshotDocument =
            serde_json::from_str(json).map_err(|err| SnapshotError::Parse {
                detail: err.to_string(),
            })?;
        if doc.language.is_empty() {
            return Err(SnapshotError::Schema { field: "language" });
        }
        if doc.snapshot_digest.is_empty() {
            return Err(SnapshotError::Schema {
                field: "snapshot_digest",
            });
        }
        if doc.schema_version != SCHEMA_VERSION_IDENTITY_ONLY
            && doc.schema_version != SCHEMA_VERSION_DECLARATIONS
        {
            return Err(SnapshotError::UnknownSchemaVersion {
                actual: doc.schema_version,
            });
        }

        let mut packages = Vec::with_capacity(doc.packages.len());
        let mut seen = BTreeSet::new();
        for pkg in doc.packages {
            if pkg.unit_id.is_empty() || pkg.unit_id.contains('\0') {
                return Err(SnapshotError::Schema {
                    field: "packages.unit_id",
                });
            }
            if pkg.producer != PRODUCER_BOOTSTRAP_GO && pkg.producer != PRODUCER_OWNED_RUST {
                return Err(SnapshotError::UnknownProducer {
                    actual: pkg.producer,
                });
            }
            if !seen.insert(pkg.unit_id.clone()) {
                return Err(SnapshotError::DuplicateUnit {
                    unit_id: pkg.unit_id,
                });
            }
            if doc.schema_version == SCHEMA_VERSION_IDENTITY_ONLY && !pkg.declarations.is_empty() {
                return Err(SnapshotError::VersionPayloadMismatch {
                    detail: "schema_version 0 carries declarations",
                });
            }
            packages.push(Package {
                declarations: convert_declarations(&pkg.unit_id, &pkg.declarations)?,
                unit: UnitId(pkg.unit_id),
                producer: pkg.producer,
            });
        }
        Ok(Self {
            schema_version: doc.schema_version,
            language: doc.language,
            snapshot_digest: Digest(doc.snapshot_digest),
            packages,
        })
    }

    #[must_use]
    pub fn language(&self) -> &str {
        &self.language
    }

    #[must_use]
    pub fn snapshot_digest(&self) -> Digest {
        self.snapshot_digest.clone()
    }

    #[must_use]
    pub fn units(&self) -> Vec<UnitId> {
        self.packages.iter().map(|pkg| pkg.unit.clone()).collect()
    }

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub fn producer_for(&self, unit: &UnitId) -> Option<&str> {
        self.package_for(unit).map(|pkg| pkg.producer.as_str())
    }

    /// Declaration tree for `unit`, or `None` when the model does not carry that unit.
    #[must_use]
    pub fn declarations_for(&self, unit: &UnitId) -> Option<Vec<Declaration>> {
        self.package_for(unit).map(|pkg| pkg.declarations.clone())
    }

    fn package_for(&self, unit: &UnitId) -> Option<&Package> {
        self.packages.iter().find(|pkg| &pkg.unit == unit)
    }
}
