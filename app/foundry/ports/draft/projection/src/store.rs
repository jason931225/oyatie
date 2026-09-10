use data_ontology_kernel::ObjectEntity;

use crate::keys::KeyDesignations;
use crate::predicate::PropertyPredicate;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedObject {
    pub entity: ObjectEntity, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    pub schema_revision: u32, // data_class: INTERNAL_ONLY
    pub last_ordinal: u64,    // data_class: INTERNAL_ONLY
    pub last_actor: String,   // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProjectedLink {
    pub link_type: String,       // data_class: INTERNAL_ONLY
    pub from_object_ref: String, // data_class: INTERNAL_ONLY
    pub to_object_ref: String,   // data_class: INTERNAL_ONLY
    /// When the edge was observed. Traversal filters on a freshness
    /// floor, so an edge store that forgot this would silently serve
    /// stale edges a caller explicitly asked to exclude. Identity is
    /// (tenant, from, link_type, to) — this is a VALUE on that key, so
    /// a later observation updates the edge rather than duplicating it.
    pub observed_at_epoch_ms: u64, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntryOutcome {
    /// Both land together or neither does.
    Applied {
        objects: Vec<ProjectedObject>,
        links: Vec<ProjectedLink>,
    },
    /// The reason is a static label, never a classified value.
    Poisoned { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedEntry {
    pub tenant_id: String,     // data_class: INTERNAL_ONLY
    pub ordinal: u64,          // data_class: INTERNAL_ONLY
    pub outcome: EntryOutcome, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyReceipt {
    pub ordinal: u64,       // data_class: INTERNAL_ONLY
    pub deduplicated: bool, // data_class: INTERNAL_ONLY
}

/// Why the store refused. A store failure is infrastructure — never a
/// poison, because it is not derivable from (log bytes, registry).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectionStoreError {
    /// Applies are dense per tenant; a gap or skip is refused so a
    /// crashed runner can never silently hole the projection.
    NonDenseOrdinal { expected: u64, found: u64 },
    /// A re-apply at an applied ordinal carried different content.
    DivergentReplay { ordinal: u64 },
    /// The entry is malformed (blank tenant, tenant-mismatched object,
    /// blank poison reason, zero page limit, ...).
    Entry { detail: &'static str },
    /// A range predicate met a stored value of a different kind —
    /// schema drift surfaces loudly, never as a silent false.
    KindMismatch { property: String },
    /// Another object of this entity type already holds this key. The
    /// refusal names the property and the HOLDER — never the key value,
    /// which carries the property's own privacy class.
    DuplicatePrimaryKey { property: String, held_by: String },
    /// An object of a keyed entity type does not carry its key
    /// property; an unidentifiable object never enters the store.
    MissingPrimaryKey { property: String },
    /// A key value is an array or struct. Identity must be a scalar:
    /// composite values have no index affinity, so an adapter could
    /// only compare them by decoding every row — and one that tried to
    /// use its typed columns would collide every composite with every
    /// other. Refused in BOTH planes so they cannot diverge.
    NonScalarPrimaryKey { property: String },
    /// The backing store failed; the detail is diagnostic.
    Storage { detail: String },
}

/// A typed resume point: pages continue strictly after this object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionCursor {
    pub after_object_ref: String, // data_class: INTERNAL_ONLY
}

/// `limit` must be non-zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRequest {
    pub limit: usize,                     // data_class: INTERNAL_ONLY
    pub cursor: Option<ProjectionCursor>, // data_class: INTERNAL_ONLY
}

impl PageRequest {
    pub fn first(limit: usize) -> Self {
        Self {
            limit,
            cursor: None,
        }
    }

    pub fn after(limit: usize, cursor: ProjectionCursor) -> Self {
        Self {
            limit,
            cursor: Some(cursor),
        }
    }
}

/// One page of objects in `object_ref` order; `next` is present exactly
/// when more objects remain past this page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page {
    pub objects: Vec<ProjectedObject>, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
    pub next: Option<ProjectionCursor>, // data_class: INTERNAL_ONLY
}

/// Reads are deterministic — `object_ref` ascending — so pages
/// partition the full result.
pub trait ProjectionStore {
    fn apply(
        &mut self,
        entry: AppliedEntry,
        keys: &KeyDesignations,
    ) -> Result<ApplyReceipt, ProjectionStoreError>;

    /// The highest ordinal applied for the tenant (0 = nothing yet).
    fn applied_head(&self, tenant_id: &str) -> Result<u64, ProjectionStoreError>;

    fn get(
        &self,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Option<ProjectedObject>, ProjectionStoreError>;

    fn objects_of_type(
        &self,
        tenant_id: &str,
        entity_type: &str,
        page: &PageRequest,
    ) -> Result<Page, ProjectionStoreError>;

    fn filter(
        &self,
        tenant_id: &str,
        entity_type: &str,
        predicate: &PropertyPredicate,
        page: &PageRequest,
    ) -> Result<Page, ProjectionStoreError>;

    /// Edges leaving `object_ref`, in deterministic order.
    fn links_from(
        &self,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError>;

    /// Edges arriving at `object_ref` — traversal reads both directions,
    /// so the store must serve both.
    fn links_to(
        &self,
        tenant_id: &str,
        object_ref: &str,
    ) -> Result<Vec<ProjectedLink>, ProjectionStoreError>;

    fn poisoned(&self, tenant_id: &str) -> Result<Vec<(u64, String)>, ProjectionStoreError>;

    /// Discard everything this store holds for ONE tenant, returning the
    /// head it discarded so a caller can record what it destroyed rather
    /// than losing that number along with the rows.
    ///
    /// Destructive and deliberate: never the response to a transient
    /// failure, because a store outage is retried rather than reset.
    /// Afterwards the tenant is indistinguishable from one never
    /// written, so the dense-ordinal law starts again at 1.
    fn reset_tenant(&mut self, tenant_id: &str) -> Result<u64, ProjectionStoreError> {
        let _ = tenant_id;
        Err(ProjectionStoreError::Storage {
            detail: "this store does not support reset".to_owned(),
        })
    }
}
