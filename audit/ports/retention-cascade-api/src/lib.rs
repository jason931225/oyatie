#![allow(dead_code)]

#[derive(Clone, Debug)]
pub struct RetentionPolicy {
    pub pack: String,
    pub data_class: String,
    pub retention_seconds: u64,
}

#[derive(Clone, Debug)]
pub struct DsrCascade {
    pub tenant_id: String,
    pub subject_id: String,
    pub source_microservice: String,
}

#[derive(Clone, Debug)]
pub struct RedactionToken {
    pub audit_id: String,
    pub reason: String,
    pub lawful_basis: String,
}

#[derive(Clone, Debug)]
pub struct RetentionRun {
    pub run_id: String,
    pub pack: String,
    pub started_at: String,
}
