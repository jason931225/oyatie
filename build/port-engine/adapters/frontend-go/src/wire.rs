//! Serde wire shapes for the snapshot envelope.
//!
//! Separate from the decoded model so the shape the artifact is WRITTEN in and the shape the
//! engine REASONS over can diverge without either pretending to be the other.

use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct TypeEntry {
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) package: String,
    #[serde(default)]
    pub(crate) args: Vec<TypeEntry>,
}

#[derive(Deserialize)]
pub(crate) struct SnapshotDocument {
    /// Absent in v0 artifacts, which predate the field.
    #[serde(default)]
    pub(crate) schema_version: u32,
    pub(crate) language: String,
    pub(crate) snapshot_digest: String,
    pub(crate) packages: Vec<PackageEntry>,
}

#[derive(Deserialize)]
pub(crate) struct PackageEntry {
    pub(crate) unit_id: String,
    pub(crate) producer: String,
    #[serde(default)]
    pub(crate) declarations: Vec<DeclarationEntry>,
}

/// Wire shape of one declaration node. Recursive and uniform, matching the extractor.
#[derive(Deserialize)]
pub(crate) struct DeclarationEntry {
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "type")]
    pub(crate) type_ref: Option<TypeEntry>,
    #[serde(default)]
    pub(crate) flags: Vec<String>,
    #[serde(default)]
    pub(crate) attrs: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) children: Vec<DeclarationEntry>,
}
