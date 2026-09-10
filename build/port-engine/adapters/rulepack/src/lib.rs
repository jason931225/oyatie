//! # port-engine-rulepack — fixture-gated neutral `RulePack` loader.
//!
//! ADR-0637 D1: rule SEMANTICS live in data under the forever home `specs/port-rules/**`. This
//! adapter embeds package-local mirrors and implements the `RulePack` and `PackSemantics` seams
//! from the ports face.
//!
//! **Every loaded rule MUST carry ≥1 positive selecting fixture**, and every fixture MUST agree
//! with the selection derived from `applies`. Missing, empty, or false fixtures cannot manufacture
//! coverage. Digest is SHA-256 of the embedded JSON bytes. Neutral only — no corpus vocabulary.
#![forbid(unsafe_code)]

/// This crate's own sources, for the engine-identity axis assembled by the facade.
mod sources;
pub use sources::CRATE_SOURCES;

mod error;
mod pack;
mod policy;
mod rule;
mod seams;
mod wire;

pub use error::RulepackError;
pub use pack::LoadedRulePack;
pub use rule::{DeferredKind, DispositionRule, LoadedRule, SelectingFixture, TraitReceiver};

/// Embedded v0 mirror of forever `specs/port-rules/**` (integ/specs owns the live tree).
pub(crate) const RULEPACK_V0_JSON: &str = include_str!("rulepack-v0.json");

/// Embedded go→rust pack v1: the declaration-level rules, type map, and deferral policy that
/// translate the hermetic Go corpus. Same forever home as v0.
pub(crate) const RULEPACK_GO_RUST_V1_JSON: &str = include_str!("rulepack-go-rust-v1.json");

/// The only conflict policy the engine implements. A pack may not declare another: the kernel
/// refuses a duplicate rule or region outright, and there is no code path that would do anything
/// else with a different value.
pub const CONFLICT_REFUSE: &str = "refuse";

/// Fail-closed readiness gate. `true` once fixture-gated load is present.
#[must_use]
pub const fn w0_ready() -> bool {
    true
}
