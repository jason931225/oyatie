//! Injecting one fault into a candidate, and proving every deletion refuses.
//!
//! The proof is [`crate::verdict`] over a perturbed candidate. It returns the
//! (deletion, fault) pairs it covered, so a caller can check that it reached
//! every deletion rather than trusting that it ran at all.

use std::collections::BTreeSet;

use pipeline_revision_view::RevisionView;

use crate::{Candidate, REQUIRED_CONSUMERS, Refusal, verdict};

/// A fault a candidate must be shown to refuse.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Fault {
    ConsumerFailed,
    ConsumerMissing,
    MissingInput,
    RevisionMismatch,
    ViewUnavailable,
}

/// Every class, so a proof cannot skip one by enumerating a subset.
#[rustfmt::skip]
pub const FAULTS: [Fault; 5] = [
    Fault::ConsumerFailed, Fault::ConsumerMissing, Fault::MissingInput,
    Fault::RevisionMismatch, Fault::ViewUnavailable,
];

impl Fault {
    pub fn name(self) -> &'static str {
        match self {
            Self::ConsumerFailed => "failed consumer",
            Self::ConsumerMissing => "missing consumer",
            Self::MissingInput => "missing input",
            Self::RevisionMismatch => "revision mismatch",
            Self::ViewUnavailable => "view unavailable",
        }
    }
}

/// `None` when withholding `input` leaves the view with nothing bound.
fn withhold(view: Option<&RevisionView>, input: &str) -> Option<RevisionView> {
    let view = view?;
    let kept = view.inputs().filter(|(name, _)| *name != input);
    view.identity().clone().bind(kept).ok()
}

/// A different object name. The last digit lands on `1` or `2`, never on the
/// all-zero name, so the result is a mismatch and not a malformed revision.
fn perturb(revision: &str) -> String {
    let mut bytes = revision.as_bytes().to_vec();
    if let Some(last) = bytes.last_mut() {
        *last = if *last == b'1' { b'2' } else { b'1' };
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// The candidate with one fault injected against one deletion.
///
/// Only [`Fault::MissingInput`] varies with `deletion`; the rest perturb
/// candidate-wide state and are proved per deletion because clause 5 requires
/// the proof per deletion, not because the injected candidate differs.
pub fn inject(candidate: &Candidate, deletion: &str, fault: Fault) -> Candidate {
    let mut injected = candidate.clone();
    let first = REQUIRED_CONSUMERS[0];
    match fault {
        Fault::ConsumerFailed => {
            injected.consumers.insert(first.to_owned(), false);
        }
        Fault::ConsumerMissing => {
            injected.consumers.remove(first);
        }
        Fault::MissingInput => injected.view = withhold(candidate.view.as_ref(), deletion),
        Fault::RevisionMismatch => injected.revision = perturb(&candidate.revision),
        Fault::ViewUnavailable => injected.view = None,
    }
    injected
}

/// A reason the injection proof does not hold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProofGap {
    /// The un-injected candidate was refused, so every injection refuses for
    /// free and the proof attests to nothing.
    BaselineRefused(Vec<Refusal>),
    NotRefused {
        deletion: String,
        fault: Fault,
    },
    /// Refused, but for a class other than the one injected: the check that
    /// should have caught this fault could be absent and this stay green.
    WrongClass {
        deletion: String,
        fault: Fault,
        got: Vec<Refusal>,
    },
}

fn listed(got: &[Refusal]) -> String {
    got.iter()
        .map(Refusal::reason)
        .collect::<Vec<_>>()
        .join("; ")
}

impl ProofGap {
    pub fn reason(&self) -> String {
        match self {
            Self::BaselineRefused(got) => {
                format!("un-injected candidate was refused: {}", listed(got))
            }
            Self::NotRefused { deletion, fault } => {
                format!("`{deletion}` admitted under injected {}", fault.name())
            }
            Self::WrongClass {
                deletion,
                fault,
                got,
            } => format!(
                "`{deletion}` under injected {} refused for another reason: {}",
                fault.name(),
                listed(got)
            ),
        }
    }
}

/// Every (deletion, fault) pair proved to refuse, so a caller can check that
/// the proof reached every deletion rather than trusting that it ran.
///
/// # Errors
/// Every [`ProofGap`], so the whole proof is repaired at once. A refused
/// baseline is reported alone: with it, no injection means anything.
pub fn prove(candidate: &Candidate) -> Result<Vec<(String, Fault)>, Vec<ProofGap>> {
    if let Err(refused) = verdict(candidate) {
        return Err(vec![ProofGap::BaselineRefused(refused)]);
    }
    let (mut gaps, mut covered) = (Vec::new(), Vec::new());
    for deletion in &candidate.deletions {
        for fault in FAULTS {
            let Err(got) = verdict(&inject(candidate, deletion, fault)) else {
                gaps.push(ProofGap::NotRefused {
                    deletion: deletion.clone(),
                    fault,
                });
                continue;
            };
            if got.iter().map(Refusal::class).collect::<BTreeSet<_>>()
                != BTreeSet::from([Some(fault)])
            {
                gaps.push(ProofGap::WrongClass {
                    deletion: deletion.clone(),
                    fault,
                    got,
                });
                continue;
            }
            covered.push((deletion.clone(), fault));
        }
    }
    if gaps.is_empty() {
        Ok(covered)
    } else {
        Err(gaps)
    }
}
