#![allow(dead_code)]

#[derive(Clone, Debug, Default)]
pub struct AuditQuery {
    pub tenant_id: String,
    pub pack: Option<String>,
    pub event_type: Option<String>,
    pub principal: Option<String>,
    pub entity: Option<String>,
    pub period: Option<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultSealState {
    Unsealed,
    Sealed,
    Published,
    Redacted,
}

#[derive(Clone, Debug, Default)]
pub struct QueryResult {
    pub rows: Vec<QueryRow>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug)]
pub struct QueryRow {
    pub audit_id: String,
    pub period_id: String,
    pub seal_state: ResultSealState,
}

#[derive(Clone, Debug)]
pub struct ExportBundle {
    pub engagement_id: String,
    pub root_ref: String,
    pub public_key_ref: String,
    pub bundle_uri: String,
}

#[derive(Clone, Debug)]
pub struct AuditorEngagement {
    pub engagement_id: String,
    pub tenant_id: String,
    pub expires_at: String,
}
