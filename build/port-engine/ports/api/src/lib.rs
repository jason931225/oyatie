//! # port-engine-api — the ports face of the owned deterministic port engine.
//!
//! Seam types only. `port-engine-kernel` owns neutrality enforcement and the `plan` / `emit` /
//! `verify` entry points; the adapters implement the seams (ADR-0637 D1, archived; live via apex
//! ADR-0704).
#![forbid(unsafe_code)]

mod sources;
pub use sources::CRATE_SOURCES;

mod declaration;
mod error;
mod failure;
mod identity;
mod ownership;
mod plan;
mod receipt;
mod seams;
mod type_ref;

pub use declaration::Declaration;
pub use error::PortError;
pub use failure::FailureConvention;
pub use identity::{Digest, LanguagePair, PAIR_SEPARATOR, RegionId, RuleId, UnitId};
pub use ownership::{OwnershipFacts, PointerDisposition};
pub use plan::{PlanStep, TransformPlan};
pub use receipt::{RECEIPT_AXES, Receipt, ReceiptAxis};
pub use seams::{PackSemantics, Renderer, RulePack, SourceModel, TargetIr};
pub use type_ref::TypeRef;

#[must_use]
pub const fn w0_ready() -> bool {
    true
}
