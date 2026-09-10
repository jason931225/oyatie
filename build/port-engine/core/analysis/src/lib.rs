//! # port-engine-analysis — deciding ownership from observed facts.
//!
//! Go is garbage-collected, so a pointer carries no ownership information. The same `*T` may be a
//! borrow that does not outlive the call, an owned value passed by pointer for efficiency, or a
//! shared structure with live aliases; Rust needs the decision made, and it cannot be read off the
//! type. So the front end observes FACTS, the pack declares RULES, and this face pairs them.
//!
//! ## What this does not prove, stated first
//!
//! `docs/programs/k8s-port/census/ownership-escape.md` records the limit precisely: a `does not
//! escape` verdict proves LIFETIME compatibility and **not** the EXCLUSIVITY a Rust borrow needs.
//! Go callers may pass one pointer as two arguments, or retain an alias while the callee mutates
//! through another. That is a caller-side property, so no amount of callee analysis closes it.
//!
//! This face therefore produces a HYPOTHESIS, not a proof. What makes shipping a hypothesis
//! defensible is that the target language checks it: an emitted `&mut` the call sites cannot
//! satisfy is a borrow-check ERROR, caught by the compile proof, loud and located. The failure
//! mode of being wrong here is a red build, not silent corruption — which is exactly the trade the
//! `effect_unknown` fact exists to keep visible, since a disposition chosen on unproven facts is
//! marked as such in the record below rather than blending into the proven ones.
#![forbid(unsafe_code)]

/// This crate's own sources, for the engine-identity axis assembled by the facade.
mod sources;
pub use sources::CRATE_SOURCES;

use std::fmt;

use port_engine_api::{OwnershipFacts, PointerDisposition};

#[must_use]
pub const fn w0_ready() -> bool {
    true
}

/// One decision, with everything a reviewer needs to audit it.
///
/// The justification travels WITH the decision rather than being reconstructible from it. A
/// disposition is an inference over facts the reader cannot see from the emitted code, so a record
/// that omits the facts asks them to trust it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub rule_id: String,
    /// Target type template for a parameter, with `{0}` for the pointee.
    pub target: String,
    /// Target form for a receiver, emitted verbatim — unlike `target` it takes no pointee.
    pub receiver: Option<String>,
    pub reason: String,
    pub facts: OwnershipFacts,
}

impl Decision {
    /// `true` when the decision rests on facts the front end could not prove.
    ///
    /// Not an error and not hidden. It is the difference between "this borrow is safe as far as
    /// anyone looked" and "this borrow is safe as far as anyone looked, and nobody looked past the
    /// first call" — and a reviewer auditing a disposition needs to know which they have.
    #[must_use]
    pub const fn rests_on_unproven_facts(&self) -> bool {
        self.facts.effect_unknown
    }
}

/// Why a disposition could not be decided.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnalysisError {
    /// No declared rule accepts these facts.
    NoRule {
        /// The site being decided.
        site: String,
        /// The facts nothing matched.
        facts: OwnershipFacts,
    },
    /// A rule matched, but it has no form for the position it was asked about.
    NoFormForPosition {
        /// The site being decided.
        site: String,
        /// The rule that matched.
        rule_id: String,
        /// Why the rule declines this position.
        reason: String,
    },
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRule { site, facts } => write!(
                f,
                "no pointer disposition accepts `{site}` (mutated={}, escapes={}, \
                 effect_unknown={}) — the pack needs a rule for these facts, and a catch-all if it \
                 intends every pointer to resolve",
                facts.mutated, facts.escapes, facts.effect_unknown
            ),
            Self::NoFormForPosition {
                site,
                rule_id,
                reason,
            } => write!(
                f,
                "disposition `{rule_id}` has no receiver form, so `{site}` cannot be a method on \
                 the type it points at: {reason}"
            ),
        }
    }
}

impl std::error::Error for AnalysisError {}

/// Decide a pointer's disposition from its facts.
///
/// Rules are evaluated in DECLARED ORDER and the first match wins, so a pack orders from most
/// specific to least. There is no implicit default: facts nothing accepts refuse, because an
/// invented default is a decision nobody wrote down.
///
/// # Errors
/// [`AnalysisError::NoRule`] when no declared rule accepts the facts.
pub fn decide(
    site: &str,
    facts: OwnershipFacts,
    rules: &[PointerDisposition],
) -> Result<Decision, AnalysisError> {
    rules
        .iter()
        .find(|rule| rule.accepts(facts))
        .map(|rule| Decision {
            rule_id: rule.id.clone(),
            target: rule.target.clone(),
            receiver: rule.receiver.clone(),
            reason: rule.reason.clone(),
            facts,
        })
        .ok_or_else(|| AnalysisError::NoRule {
            site: site.to_owned(),
            facts,
        })
}

/// The receiver form for a decision, or a refusal naming why the disposition has none.
///
/// # Errors
/// [`AnalysisError::NoFormForPosition`] when the matched disposition declares no receiver form —
/// which is how "this pointer escapes, so it cannot be a borrow of self" arrives as a refusal
/// rather than as a borrow that does not hold.
pub fn receiver_form<'a>(site: &str, decision: &'a Decision) -> Result<&'a str, AnalysisError> {
    decision
        .receiver
        .as_deref()
        .ok_or_else(|| AnalysisError::NoFormForPosition {
            site: site.to_owned(),
            rule_id: decision.rule_id.clone(),
            reason: decision.reason.clone(),
        })
}
