//! HR employment domain foundation.
//!
//! This crate owns pure HR invariants for employee and employment records:
//! legal-entity-scoped employees, audit-backed lifecycle events, and
//! Korea-first labor-compliance threshold obligations.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]
#![forbid(unsafe_code)]

include!(concat!(env!("OUT_DIR"), "/lib.generated.rs"));

#[cfg(test)]
include!(concat!(env!("OUT_DIR"), "/tests.generated.rs"));
