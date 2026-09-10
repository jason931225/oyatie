//! # port-engine-rulepack — fixture-gated neutral `RulePack` loader.
//!
//! ADR-0637 D1: rule SEMANTICS live in data under the forever home `specs/port-rules/**`; this
//! adapter embeds package-local mirrors of that tree.
#![forbid(unsafe_code)]

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

#[must_use]
pub const fn w0_ready() -> bool {
    true
}
