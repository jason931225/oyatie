#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![allow(dead_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamespaceDecision {
    Allow,
    DenyReserved,
    DenyConfusable,
    DenyMalformed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamespaceCandidate {
    pub candidate: String,
    pub principal: String,
    pub action: NamespaceAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NamespaceAction {
    CreateTenant,
    RenameTenant,
    CreateSubScope,
}

pub trait ReservedNamespaceSource {
    fn reserved(&self) -> Vec<String>;
}

pub trait NamespaceActionAuthorizer {
    fn authorize(&self, input: &NamespaceCandidate) -> Result<bool, NamespaceUsecaseError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NamespaceUsecaseError {
    SourceUnavailable,
    CedarEvaluationFailed,
}

/// Stub entry-point; full implementation in IP-017 execution.
pub fn evaluate<S: ReservedNamespaceSource, A: NamespaceActionAuthorizer>(
    _source: &S,
    _authorizer: &A,
    _candidate: &NamespaceCandidate,
) -> Result<NamespaceDecision, NamespaceUsecaseError> {
    Ok(NamespaceDecision::Allow)
}
