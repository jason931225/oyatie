//! # port-engine-rust-ir — the typed target IR and its renderer.
//!
//! ADR-0637 D1 core face: holds `TargetIr` rendering with stable ordering and normalized
//! formatting.
//!
//! The IR is a TREE. Items, statements and expressions are data; `quote!` lowers them to tokens,
//! `syn` parses those tokens, and `prettyplease` formats the result. Nothing in the emit path
//! builds Rust by string formatting, so precedence, visibility and doc comments are decided
//! structurally rather than by a `format!` that happens to read correctly.
//!
//! There is no scan over EMITTED bytes here; `tests/fences.rs` records why.
#![forbid(unsafe_code)]

/// This crate's own sources, for the engine-identity axis assembled by the facade.
mod sources;
pub use sources::CRATE_SOURCES;

mod expr;
mod item;
mod lower;
mod lower_body;
mod lower_parts;
mod ops;
mod render;
mod ty;

pub use expr::{MatchArm, RustExpr, RustStmt};
pub use item::{Receiver, RustField, RustFn, RustItem, RustParam, StructShape, Visibility};
pub use lower::lower_file;
pub use ops::{BinaryOp, Precedence, UnaryOp};
pub use render::{EmptyRenderer, FORMATTER_ID, RustIr, RustRenderer};
pub use ty::RustType;

#[must_use]
pub const fn w0_ready() -> bool {
    true
}
