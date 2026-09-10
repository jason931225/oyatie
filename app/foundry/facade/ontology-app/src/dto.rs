//! The wire shapes of the write surface.
//!
//! `occurred_at_epoch_seconds` is the caller's, not this process's, and
//! that is deliberate: the writer derives every payload byte from the
//! request so a retry is byte-identical and deduplicates. If the facade
//! stamped its own clock, the same request sent twice would produce
//! different bytes under one idempotency key — a loud conflict where the
//! caller expected a dedup. The tenant is absent from this shape by
//! design; it comes from the credential and nothing in the body can move
//! it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitRequest {
    pub object_ref: String,                   // data_class: TENANT_SCOPED
    pub action_type: String,                  // data_class: INTERNAL_ONLY
    pub idempotency_key: String,              // data_class: INTERNAL_ONLY
    pub occurred_at_epoch_seconds: u64,       // data_class: INTERNAL_ONLY
    pub properties: BTreeMap<String, String>, // data_class: PROPERTY_VALUE_PRIVACY_CLASS
}

#[derive(Clone, Debug, Serialize)]
pub struct SubmitResponse {
    pub outcome: &'static str, // data_class: INTERNAL_ONLY
    pub ordinal: u64,          // data_class: INTERNAL_ONLY
    pub deduplicated: bool,    // data_class: INTERNAL_ONLY
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poison_reason: Option<String>, // data_class: INTERNAL_ONLY
}

#[derive(Clone, Debug, Serialize)]
pub struct RefusalBody {
    pub gate: String,  // data_class: INTERNAL_ONLY
    pub cause: String, // data_class: INTERNAL_ONLY
}
