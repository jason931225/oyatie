//! The seam traits: what the engine asks of the adapters that implement them.
//!
//! They live on the ports face because they are implemented OUTSIDE the core. An adapter depends on
//! the contract it implements; it must never depend on the engine behind that contract.

use std::collections::{BTreeMap, BTreeSet};

use crate::declaration::Declaration;
use crate::error::PortError;
use crate::failure::FailureConvention;
use crate::identity::{Digest, LanguagePair, RegionId, RuleId, UnitId};
use crate::ownership::PointerDisposition;

/// The canonical semantic model of the source corpus, as produced by a front end.
///
/// `units` is order-significant and MUST be deterministic for a given input —
/// `port-engine-kernel::plan` rejects a duplicate id because that is the shape in which a
/// non-deterministic model reaches the engine.
pub trait SourceModel {
    fn language(&self) -> &str;
    /// The receipt's `snapshot_digest`.
    fn snapshot_digest(&self) -> Digest;
    fn units(&self) -> Vec<UnitId>;
    /// What `unit` declares, in deterministic order.
    ///
    /// `None` means the model does not carry that unit at all; `Some(vec![])` means it carries the
    /// unit and the unit declares nothing.
    ///
    /// Deliberately NOT defaulted. A default returning "no declarations" would let a front end that
    /// forgot to implement it produce a green, empty translation of a populated corpus.
    fn declarations(&self, unit: &UnitId) -> Option<Vec<Declaration>>;
}

/// Neutral rule data, addressed by [`LanguagePair`].
///
/// Rule SEMANTICS live in the data, not here. The kernel needs exactly two things: which pair the
/// pack serves, and which of its declared rules apply to a unit — in pack order, because rule order
/// is part of the transform.
pub trait RulePack {
    fn pair(&self) -> &LanguagePair;
    /// The receipt's `rulepack_digest`.
    fn digest(&self) -> Digest;
    fn rules(&self) -> Vec<RuleId>;
    /// The declared rules that apply to `unit`, in pack order. Returning a rule absent from
    /// [`RulePack::rules`] is a pack defect and `port-engine-kernel::plan` refuses it.
    fn rules_for(&self, unit: &UnitId) -> Vec<RuleId>;
}

/// Everything a transform needs from a loaded rule pack.
///
/// Distinct from [`RulePack`], which answers WHICH rules apply. This answers what a rule MEANS.
pub trait PackSemantics {
    fn construction(&self, rule: &RuleId) -> Option<&str>;
    fn precondition(&self, rule: &RuleId) -> Option<&str>;
    /// Declaration kinds `rule` captures. Empty means the rule is unit-level.
    fn captures(&self, rule: &RuleId) -> Option<&[String]>;
    fn type_map(&self) -> &BTreeMap<String, String>;
    /// Target-type templates keyed by source type KIND, with `{0}`, `{1}` for the arguments.
    fn type_constructors(&self) -> &BTreeMap<String, String>;
    /// Per-construction overrides of [`PackSemantics::type_map`], keyed by construction id.
    ///
    /// One source type does not always map to one target type: an owned type is right for a field
    /// and impossible for a constant.
    fn type_map_overrides(&self, construction: &str) -> Option<&BTreeMap<String, String>>;
    /// SOURCE types whose target counterpart copies, so reading one by value needs nothing.
    ///
    /// Everything else MOVES on a plain read, which does not compile out of a borrow — so the read
    /// is cloned, because the source copied and the target would not.
    fn copy_types(&self) -> &BTreeSet<String>;
    /// SOURCE type identity → the target expression for that type's zero value.
    ///
    /// Go fills a struct literal's omitted fields with the zero value of their type; the target
    /// rejects an incomplete literal, so the engine has to spell the omitted fields out.
    fn zero_values(&self) -> &BTreeMap<String, String>;
    /// The target form a TRAIT takes in each position, with `{0}` for the trait's path.
    ///
    /// Keyed by position — `param`, `result`, `field` — because one form does not answer for all of
    /// them: a borrowed trait object is right for a parameter and impossible for a value the
    /// function returns. A position with no entry REFUSES, because the choice between borrowing,
    /// boxing and sharing is an ownership decision and the engine has no basis to make it.
    fn trait_object_forms(&self) -> &BTreeMap<String, String>;
    /// How the source spells failure, or `None` when it has no such convention.
    ///
    /// `None` means every result is an ordinary value, and a source language without the convention
    /// needs no rule to say so.
    fn failure_convention(&self) -> Option<&FailureConvention>;
    /// Source function identity → a target expression template, with `{0}`, `{1}` for arguments.
    ///
    /// A call the pack does not answer for emits the source's own name, which the target does not
    /// have.
    fn function_map(&self) -> &BTreeMap<String, String>;
    /// SOURCE types a conversion reaches by a plain cast.
    ///
    /// A conversion the pack does not list is one where the two languages disagree about what
    /// conversion MEANS — infallible and lossy on one side, fallible on the other — and those
    /// refuse.
    fn cast_types(&self) -> &BTreeSet<String>;
    /// Ownership rules, in declared order — first match wins.
    fn pointer_dispositions(&self) -> &[PointerDisposition];
    fn deferred_kinds(&self) -> &BTreeSet<String>;
    /// How a trait method binds its receiver, and why the pack chose that.
    ///
    /// `None` is a REFUSAL, not a default. A source interface says nothing about how an
    /// implementation binds its receiver, and the implementations are not all in view, so this
    /// cannot be recovered — it can only be decided.
    fn trait_receiver(&self) -> Option<(&str, &str)>;
}

/// The neutral intermediate representation handed to a [`Renderer`].
///
/// As with [`SourceModel`], the kernel sees identity and order, never content.
pub trait TargetIr {
    fn target_language(&self) -> &str;
    fn regions(&self) -> Vec<RegionId>;
}

/// Turns a [`TargetIr`] into emitted bytes, one blob per region.
pub trait Renderer {
    fn target_language(&self) -> &str;
    /// The receipt's `formatter_digest`.
    fn formatter_digest(&self) -> Digest;
    /// Render every region of `ir`. The returned key set MUST equal `ir.regions()`;
    /// `port-engine-kernel::emit` enforces that rather than trusting it.
    ///
    /// # Errors
    /// Whatever the implementation refuses with — [`PortError::Render`] exists so that sentence is
    /// true of this closed enum. `port-engine-kernel::emit` adds the region-set proof on top.
    fn render(&self, ir: &dyn TargetIr) -> Result<BTreeMap<RegionId, Vec<u8>>, PortError>;
}
