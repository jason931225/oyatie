//! Query request validation.

use std::collections::BTreeSet;

use crate::contract::*;

/// The caller's consent posture over edge traversal. Consent is an
/// authorization-shaped input, so it has no silent default: a surface
/// where consent semantics do not apply must SAY so, and a grant list
/// that names nothing traverses nothing — deny by default, never open.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EdgeConsent {
    /// Consent does not govern this surface; every edge type may traverse.
    Unrestricted,
    /// Only these `lty_` grants may traverse; an empty list grants none.
    Granted(Vec<String>),
}

impl EdgeConsent {
    pub fn granted(ids: Vec<impl Into<String>>) -> Self {
        Self::Granted(ids.into_iter().map(Into::into).collect())
    }

    /// Whether this posture lets `edge_type_id` traverse — the one place
    /// the consent law lives.
    pub fn permits(&self, edge_type_id: &str) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Granted(grants) => grants.iter().any(|grant| grant == edge_type_id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeGraphQueryRequest {
    pub tenant_id: String,                  // data_class: INTERNAL_ONLY
    pub query_id: String,                   // data_class: INTERNAL_ONLY
    pub root_entity_id: String,             // data_class: INTERNAL_ONLY
    pub edge_type_ids: Vec<String>,         // data_class: INTERNAL_ONLY
    pub max_depth: u32,                     // data_class: INTERNAL_ONLY
    pub freshness_floor_epoch_seconds: u64, // data_class: INTERNAL_ONLY
    pub observed_at_epoch_seconds: u64,     // data_class: INTERNAL_ONLY
    pub edge_consent: EdgeConsent,          // data_class: INTERNAL_ONLY
    pub direction: TraversalDirection,      // data_class: INTERNAL_ONLY
    pub resume_cursor: Option<QueryCursor>, // data_class: INTERNAL_ONLY
    /// Further roots for multi-root search-around (the object-set seam):
    /// the walk seeds from `root_entity_id` plus every id here, all at
    /// depth zero, deduplicated deterministically.
    pub additional_root_entity_ids: Vec<String>, // data_class: INTERNAL_ONLY
}

impl KnowledgeGraphQueryRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: impl Into<String>,
        query_id: impl Into<String>,
        root_entity_id: impl Into<String>,
        edge_type_ids: Vec<impl Into<String>>,
        max_depth: u32,
        freshness_floor_epoch_seconds: u64,
        observed_at_epoch_seconds: u64,
        edge_consent: EdgeConsent,
        direction: TraversalDirection,
    ) -> Result<Self, KnowledgeGraphQueryError> {
        let request = Self {
            tenant_id: tenant_id.into(),
            query_id: query_id.into(),
            root_entity_id: root_entity_id.into(),
            edge_type_ids: edge_type_ids.into_iter().map(Into::into).collect(),
            max_depth,
            freshness_floor_epoch_seconds,
            observed_at_epoch_seconds,
            edge_consent,
            direction,
            resume_cursor: None,
            additional_root_entity_ids: Vec::new(),
        };
        request.validate()?;
        Ok(request)
    }

    /// Seed the walk from an object set: `root_entity_id` plus these.
    /// Re-validates, since roots arrive after construction.
    pub fn with_additional_roots(
        mut self,
        roots: Vec<impl Into<String>>,
    ) -> Result<Self, KnowledgeGraphQueryError> {
        self.additional_root_entity_ids = roots.into_iter().map(Into::into).collect();
        self.validate()?;
        Ok(self)
    }

    pub fn with_resume_cursor(mut self, cursor: QueryCursor) -> Self {
        self.resume_cursor = Some(cursor);
        self
    }

    pub fn validate(&self) -> Result<(), KnowledgeGraphQueryError> {
        validate_tenant_id(&self.tenant_id)?;
        validate_query_id(&self.query_id)?;
        validate_entity_id(&self.root_entity_id)?;
        validate_max_depth(self.max_depth)?;
        for edge_type_id in &self.edge_type_ids {
            validate_edge_type_id(edge_type_id)?;
        }
        if let EdgeConsent::Granted(grants) = &self.edge_consent {
            for grant_id in grants {
                validate_consent_grant_id(grant_id)?;
            }
        }
        for root in &self.additional_root_entity_ids {
            validate_entity_id(root)?;
        }
        Ok(())
    }

    pub(crate) fn edge_filter(&self) -> BTreeSet<&str> {
        self.edge_type_ids.iter().map(String::as_str).collect()
    }
}
