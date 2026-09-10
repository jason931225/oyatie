//! What this engine can and cannot do with a source tree it has never seen.
//!
//! [`crate::apply`] is fail-closed: the first construct it cannot translate ends the run. That is
//! the right behaviour for PRODUCING a port — a partial port that compiles is worse than no port,
//! because nothing downstream can tell which half is real.
//!
//! It is the wrong behaviour for MEASURING one. `apply` reports the first refusal and nothing
//! about the declarations behind it, so the engine's own maturity can only be discovered one
//! construct at a time. A survey attempts every declaration independently and reports all of it,
//! which turns "what is missing" from a guess into a ranked list.
//!
//! Two things this deliberately does NOT do, because both would make the number flattering:
//!
//! - it does not consult the pack's `applies` map. That map is policy — which units this programme
//!   has decided to port — and a survey asks a capability question instead: what COULD the engine
//!   do with this source. A survey restricted to units somebody already listed would measure the
//!   list.
//! - it does not count a refusal as a partial success. A declaration is translated or it is not,
//!   and the reason is carried verbatim so the ranking is by real cause rather than by category.

use std::collections::BTreeMap;

use port_engine_api::{Declaration, PackSemantics, RulePack, SourceModel, UnitId};

use crate::error::TransformError;
use crate::items::build_item;
use crate::ownership::{DispositionLog, OwnershipContext};
use crate::resolve::{LocalScope, Resolver};

/// What a survey found, per declaration.
#[derive(Clone, Debug)]
pub struct SurveyReport {
    /// Declarations the engine translated.
    pub translated: Vec<SurveyEntry>,
    /// Declarations it refused, with the reason it gave.
    pub refused: Vec<SurveyEntry>,
    /// Declarations no rule captures and no policy defers.
    ///
    /// Distinct from a refusal on purpose: a refusal is the engine saying it understood the
    /// construct and will not guess, and this is the pack saying nothing about it. The two need
    /// different work — a rule, versus a decision about what the rule should say.
    pub uncaptured: Vec<SurveyEntry>,
    /// Declarations the pack DEFERS, with the reason it recorded.
    ///
    /// Kept apart from the uncaptured because they are opposite states. A deferral is a decision
    /// somebody made and wrote down; an uncaptured kind is a hole nobody has looked at. Counting
    /// them together understates how finished the engine is and hides the decision.
    pub deferred: Vec<SurveyEntry>,
}

/// One declaration's outcome.
#[derive(Clone, Debug)]
pub struct SurveyEntry {
    /// The unit that declares it.
    pub unit: String, // data_class: INTERNAL_ONLY
    /// Its name in the source.
    pub name: String, // data_class: INTERNAL_ONLY
    /// Its source kind.
    pub kind: String, // data_class: INTERNAL_ONLY
    /// The refusal, when there was one.
    pub reason: Option<String>, // data_class: INTERNAL_ONLY
}

impl SurveyReport {
    /// How many declarations were examined.
    #[must_use]
    pub fn total(&self) -> usize {
        self.translated.len() + self.refused.len() + self.uncaptured.len() + self.deferred.len()
    }

    /// The share of declarations translated, as a percentage.
    #[must_use]
    pub fn coverage(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "a declaration count large enough to lose precision in an f64 is far beyond \
                      any corpus, and this is a reported percentage rather than a decision input"
        )]
        {
            self.translated.len() as f64 * 100.0 / total as f64
        }
    }

    /// A declaration this reason blocked, for a reader who wants to go and look at one.
    #[must_use]
    pub fn example_of(&self, reason: &str) -> Option<&SurveyEntry> {
        self.refused
            .iter()
            .chain(&self.uncaptured)
            .chain(&self.deferred)
            .find(|entry| self.reason_of(entry) == reason)
    }

    fn reason_of(&self, entry: &SurveyEntry) -> String {
        entry
            .reason
            .clone()
            .unwrap_or_else(|| format!("no rule captures `{}`", entry.kind))
    }

    /// Refusal reasons, most frequent first — the work list, ranked by what it would unblock.
    #[must_use]
    pub fn ranked_reasons(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for entry in self
            .refused
            .iter()
            .chain(&self.uncaptured)
            .chain(&self.deferred)
        {
            *counts.entry(self.reason_of(entry)).or_default() += 1;
        }
        let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
        // By count descending, then by reason, so the report is stable across runs.
        ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        ranked
    }
}

/// Attempt every declaration in `model` independently and report what happened to each.
#[must_use]
pub fn survey<P>(model: &dyn SourceModel, pack: &P) -> SurveyReport
where
    P: RulePack + PackSemantics,
{
    let mut report = SurveyReport {
        translated: Vec::new(),
        refused: Vec::new(),
        uncaptured: Vec::new(),
        deferred: Vec::new(),
    };

    let rules = pack.rules();
    for unit in model.units() {
        let Some(declarations) = model.declarations(&unit) else {
            continue;
        };
        let scope = LocalScope::of(&declarations);
        for declaration in &declarations {
            survey_declaration(&unit, declaration, &scope, &rules, pack, &mut report);
        }
    }
    report
}

fn survey_declaration<P>(
    unit: &UnitId,
    declaration: &Declaration,
    scope: &LocalScope,
    rules: &[port_engine_api::RuleId],
    pack: &P,
    report: &mut SurveyReport,
) where
    P: RulePack + PackSemantics,
{
    let entry = |reason: Option<String>| SurveyEntry {
        unit: unit.0.clone(),
        name: declaration.name.clone(),
        kind: declaration.kind.clone(),
        reason,
    };

    // The LAST rule in pack order that captures this kind. Pack order is precedence order, so the
    // later rule is the more specific one — `rust_struct_body` over `rust_struct`, which is the
    // difference between measuring what the engine can do and what it could do before bodies.
    let Some(construction) = rules
        .iter()
        .filter(|rule| {
            pack.captures(rule)
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == &declaration.kind))
        })
        .filter_map(|rule| pack.construction(rule))
        .next_back()
    else {
        if pack.deferred_kinds().contains(&declaration.kind) {
            report.deferred.push(entry(Some(format!(
                "deferred by policy: `{}`",
                declaration.kind
            ))));
        } else {
            report.uncaptured.push(entry(None));
        }
        return;
    };

    let log = DispositionLog::new();
    let ownership = OwnershipContext {
        rules: pack.pointer_dispositions(),
        log: &log,
    };
    let resolver = Resolver {
        scope,
        type_map: pack.type_map(),
        overrides: pack.type_map_overrides(construction),
        constructors: pack.type_constructors(),
        copy_types: pack.copy_types(),
        cast_types: pack.cast_types(),
        zero_values: pack.zero_values(),
        trait_object_forms: pack.trait_object_forms(),
        failure: pack.failure_convention(),
        function_map: pack.function_map(),
        receiver: pack.trait_receiver(),
        ownership: &ownership,
        unit,
    };

    match build_item(construction, declaration, &resolver) {
        Ok(_) => report.translated.push(entry(None)),
        Err(error) => report.refused.push(entry(Some(refusal_of(&error)))),
    }
}

/// A refusal, reduced to what identifies its CAUSE.
///
/// Two competing pressures, and getting either wrong makes the ranking useless. A reason that keeps
/// the DECLARATION's name counts once per site and ranks nothing — two hundred functions blocked by
/// one missing rule must read as one row of two hundred. A reason that drops the SUBJECT ranks
/// everything into one row and names nothing to add: "the type map does not carry it", twenty
/// times, for twenty different types.
///
/// So the subject is kept and the site is dropped, per variant, rather than by cutting the rendered
/// string at a delimiter it was never designed to have.
fn refusal_of(error: &TransformError) -> String {
    match error {
        TransformError::UnmappedType { type_ref, .. } => {
            format!("unmapped type `{type_ref}`")
        }
        TransformError::MissingDatum {
            construction,
            datum,
            ..
        } => format!("`{construction}` needs `{datum}`, which the front end did not record"),
        TransformError::ConstructionKindMismatch {
            construction, kind, ..
        } => format!("construction `{construction}` does not fit a `{kind}`"),
        TransformError::Unsupported { detail, .. } => detail.clone(),
        other => other.to_string(),
    }
}
