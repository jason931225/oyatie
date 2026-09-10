//! OpenAPI reference source fitness kernel.
//!
//! Public REST contracts become tenant- and ISV-facing promises. This pure
//! kernel validates the minimum OpenAPI 3.2 source shape that the OpenAPI publish surface
//! may emit: contract paths are versioned, `info.version` agrees with the
//! path suffix, every document declares paths, and every operation carries an
//! operation id plus at least one response. Adapters own filesystem discovery
//! and semver metadata parsing.
// ADR-0083 Tier 3: tests legitimately use `.unwrap()` / `.expect()` /
// `panic!()` to assert invariants under the `cfg(test)` exemption.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod contract_mirror;
mod data_class;
mod document;
mod error;
mod ingress;
mod operation;
mod parameter;
mod path_validation;
mod runtime_binding_source;
mod runtime_parity;
mod runtime_test_scan;
mod rust_lexer;
mod rust_source;
mod rust_status_mapping;
mod rust_struct_shape;
mod schema_match;
mod schema_parity;
mod schema_shape;
mod security;
mod serde_attribute;
mod yaml;

#[cfg(test)]
mod tests;

pub use contract_mirror::*;
pub use data_class::*;
pub use document::*;
pub use error::*;
pub use ingress::*;
pub use operation::*;
pub use parameter::*;
pub use path_validation::*;
pub use runtime_binding_source::*;
pub use runtime_parity::*;
pub use runtime_test_scan::*;
pub use rust_lexer::*;
pub use rust_source::*;
pub use rust_status_mapping::*;
pub use rust_struct_shape::*;
pub use schema_match::*;
pub use schema_parity::*;
pub use schema_shape::*;
pub use security::*;
pub use serde_attribute::*;
pub use yaml::*;
