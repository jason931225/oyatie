//! Per-deletion failure injection for an owner-prose migration candidate.
//!
//! The proof is the candidate's own [`verdict`] applied to a perturbed
//! candidate, so there is no second model of what refusal means.
//!
//! [`prove`] admits only when the un-injected candidate is admitted and every
//! (deletion, fault) pair refuses **for the class that was injected**. Refusing
//! is not enough: a candidate with no view refuses a missing-input injection
//! too, and treating that as proof would let the view-unavailable check be
//! deleted without a single test noticing.

use std::collections::{BTreeMap, BTreeSet};

use pipeline_retained_reference::{Refusal as RetainedRefusal, refuse_retained};
use pipeline_revision_view::{RevisionView, Unknown};

mod proof;

pub use proof::{FAULTS, Fault, ProofGap, inject, prove};

/// The consumers an exact candidate must pass.
#[rustfmt::skip]
pub const REQUIRED_CONSUMERS: [&str; 9] = [
    "compiler", "test", "runtime", "pdp", "slo-controller",
    "reconciler", "cargo", "buck", "ownership",
];

/// Why a candidate may not land.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// Deletes nothing, so per-deletion proof would cover nothing.
    NoDeletions,
    ConsumerMissing(String),
    ConsumerFailed(String),
    /// A deleted path whose bytes the view never bound: nothing offline can
    /// answer what that file said once the candidate lands.
    UnattestedDeletion(String),
    /// Carries the view's own reason, separating a genuine mismatch from a
    /// revision that names no immutable object at all.
    RevisionMismatch(Unknown),
    ViewUnavailable,
    Retained(RetainedRefusal),
}

impl Refusal {
    pub fn reason(&self) -> String {
        match self {
            Self::NoDeletions => "candidate deletes nothing".to_owned(),
            Self::ConsumerMissing(name) => format!("consumer `{name}` reported nothing"),
            Self::ConsumerFailed(name) => format!("consumer `{name}` failed"),
            Self::UnattestedDeletion(path) => format!("no view digest attests `{path}`"),
            Self::RevisionMismatch(unknown) => unknown.reason(),
            Self::ViewUnavailable => "no offline view is available".to_owned(),
            Self::Retained(retained) => retained.reason(),
        }
    }

    /// The fault class this refusal answers, or `None` for a refusal no
    /// injection produces.
    pub(crate) fn class(&self) -> Option<Fault> {
        Some(match self {
            Self::ConsumerMissing(_) => Fault::ConsumerMissing,
            Self::ConsumerFailed(_) => Fault::ConsumerFailed,
            Self::UnattestedDeletion(_) => Fault::MissingInput,
            Self::RevisionMismatch(_) => Fault::RevisionMismatch,
            Self::ViewUnavailable => Fault::ViewUnavailable,
            Self::NoDeletions | Self::Retained(_) => return None,
        })
    }
}

/// One migration candidate.
#[derive(Clone, Debug)]
pub struct Candidate {
    deletions: BTreeSet<String>,
    survivors: BTreeMap<String, String>,
    view: Option<RevisionView>,
    revision: String,
    consumers: BTreeMap<String, bool>,
}

impl Candidate {
    /// Paths are trimmed here, so a deletion spelled with surrounding space is
    /// the same deletion for the digest lookup and for retained-reference
    /// scanning rather than a path that matches neither.
    pub fn new(
        deletions: &[&str],
        survivors: &[(&str, &str)],
        view: Option<RevisionView>,
        revision: &str,
        consumers: &[(&str, bool)],
    ) -> Self {
        Self {
            deletions: deletions
                .iter()
                .map(|path| path.trim().to_owned())
                .collect(),
            survivors: survivors
                .iter()
                .map(|(path, body)| (path.trim().to_owned(), (*body).to_owned()))
                .collect(),
            view,
            revision: revision.trim().to_owned(),
            consumers: consumers
                .iter()
                .map(|(name, passed)| ((*name).to_owned(), *passed))
                .collect(),
        }
    }
}

/// Whether this candidate may land. A consumer reported under a name the
/// protocol does not require is ignored: the required nine are what must pass,
/// and a superset weakens nothing.
///
/// # Errors
/// Every [`Refusal`] the candidate earns, so an operator repairs the whole
/// candidate rather than rediscovering one fault per run.
pub fn verdict(candidate: &Candidate) -> Result<(), Vec<Refusal>> {
    let mut found = Vec::new();
    if candidate.deletions.is_empty() {
        found.push(Refusal::NoDeletions);
    }
    match &candidate.view {
        None => found.push(Refusal::ViewUnavailable),
        Some(view) => {
            if let Err(unknown) = view.assert_revision(&candidate.revision) {
                found.push(Refusal::RevisionMismatch(unknown));
            }
            for deletion in &candidate.deletions {
                if view.digest_of(deletion).is_none() {
                    found.push(Refusal::UnattestedDeletion(deletion.clone()));
                }
            }
        }
    }
    for name in REQUIRED_CONSUMERS {
        match candidate.consumers.get(name) {
            None => found.push(Refusal::ConsumerMissing(name.to_owned())),
            Some(false) => found.push(Refusal::ConsumerFailed(name.to_owned())),
            Some(true) => {}
        }
    }
    let live = candidate
        .survivors
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()));
    if let Err(retained) = refuse_retained(&candidate.deletions, live) {
        found.push(Refusal::Retained(retained));
    }
    if found.is_empty() { Ok(()) } else { Err(found) }
}
