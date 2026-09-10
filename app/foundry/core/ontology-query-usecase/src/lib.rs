//! Ontology query execution usecase: idempotent execution over a
//! precomputed policy decision, emitting metadata-only audit events and
//! never carrying raw property values or credentials.
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod execution;
mod types;

pub use execution::OntologyQueryExecutionUsecase;
pub use types::*;

#[cfg(test)]
mod tests;
