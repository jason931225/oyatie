//! Identity binding for a derived view of one immutable revision.
//!
//! A view is never tracked. What makes it trustworthy is that it names the
//! revision it was derived from and every input digest it consumed, so a
//! consumer can prove the view it holds and the tree it is about are the same
//! thing. This crate owns that binding and nothing else: no I/O, no
//! materialization, no digest computation.
//!
//! The revision is checked to be an object name. Digests are **opaque**: the
//! caller supplies them, and this refuses a blank one but cannot tell a real
//! digest from any other string. A view therefore proves which revision and
//! which input NAMES it covers, and carries the caller's word for the rest.

use std::collections::BTreeMap;

/// The all-zero object name. Git writes it to mean "no object", so it names
/// nothing a view could have been derived from.
const NULL_OBJECT_NAME: &str = "0000000000000000000000000000000000000000";

/// Refusal to bind or to answer. Every variant stops dispatch rather than
/// yielding a view a consumer could mistake for a complete one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Unknown {
    /// A field of the view's identity was absent or blank.
    MissingIdentity(&'static str),
    /// A revision that names no single immutable object. A branch, tag,
    /// `HEAD`, or abbreviated hash can name a different tree later, and the
    /// null object name names nothing at all.
    MutableRevision(String),
    /// An input was named with no digest to bind it to.
    MissingDigest(String),
    /// One input name was classified twice. Whether the digests agree is not
    /// the question: a name classified twice means the caller's input set is
    /// itself undecided, and picking either entry would invent an answer.
    DuplicateInput(String),
    /// The view describes a different revision than the one presented.
    RevisionMismatch { bound: String, presented: String },
    /// The view consumed no inputs, so it attests to nothing.
    NoInputs,
}

impl Unknown {
    pub fn reason(&self) -> String {
        match self {
            Self::MissingIdentity(field) => format!("view identity `{field}` is absent or blank"),
            Self::MutableRevision(revision) => {
                format!("revision `{revision}` is not an immutable object name")
            }
            Self::MissingDigest(input) => format!("input `{input}` has no digest"),
            Self::DuplicateInput(input) => format!("input `{input}` is classified twice"),
            Self::RevisionMismatch { bound, presented } => {
                format!("view is bound to revision {bound}, presented {presented}")
            }
            Self::NoInputs => "view consumed no inputs".to_owned(),
        }
    }
}

/// What a view is derived from and by what. All four are required: a digest
/// set alone cannot say which tree, which producer, or which schema shaped it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewIdentity {
    pub repository: String,
    pub revision: String,
    pub producer: String,
    pub schema: String,
}

/// A bound view. Construction is the proof; there is no way to hold one whose
/// identity or input set was never checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevisionView {
    identity: ViewIdentity,
    inputs: BTreeMap<String, String>,
}

fn is_object_name(value: &str) -> bool {
    value.len() == 40
        && value != NULL_OBJECT_NAME
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

impl ViewIdentity {
    /// Bind this identity to the digests of every input the view consumed.
    ///
    /// Identity fields are stored trimmed, so two callers that disagree only
    /// about surrounding space produce the same view rather than two that
    /// compare unequal.
    ///
    /// # Errors
    /// [`Unknown`] on a blank identity field, a revision that is not an
    /// immutable object name, an empty input set, a blank digest, or an input
    /// named more than once.
    pub fn bind<I, N, D>(self, inputs: I) -> Result<RevisionView, Unknown>
    where
        I: IntoIterator<Item = (N, D)>,
        N: Into<String>,
        D: Into<String>,
    {
        let identity = ViewIdentity {
            repository: self.repository.trim().to_owned(),
            revision: self.revision.trim().to_owned(),
            producer: self.producer.trim().to_owned(),
            schema: self.schema.trim().to_owned(),
        };
        for (field, value) in [
            ("repository", &identity.repository),
            ("revision", &identity.revision),
            ("producer", &identity.producer),
            ("schema", &identity.schema),
        ] {
            if value.is_empty() {
                return Err(Unknown::MissingIdentity(field));
            }
        }
        if !is_object_name(&identity.revision) {
            return Err(Unknown::MutableRevision(identity.revision));
        }

        let mut bound: BTreeMap<String, String> = BTreeMap::new();
        for (name, digest) in inputs {
            let (name, digest) = (name.into(), digest.into());
            if digest.trim().is_empty() {
                return Err(Unknown::MissingDigest(name));
            }
            if bound.insert(name.clone(), digest).is_some() {
                return Err(Unknown::DuplicateInput(name));
            }
        }
        if bound.is_empty() {
            return Err(Unknown::NoInputs);
        }
        Ok(RevisionView {
            identity,
            inputs: bound,
        })
    }
}

impl RevisionView {
    pub fn identity(&self) -> &ViewIdentity {
        &self.identity
    }

    /// Digests in a deterministic order, so two views of the same inputs
    /// compare and serialize identically.
    pub fn inputs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.inputs
            .iter()
            .map(|(name, digest)| (name.as_str(), digest.as_str()))
    }

    pub fn digest_of(&self, input: &str) -> Option<&str> {
        self.inputs.get(input).map(String::as_str)
    }

    /// Confirm this view describes `revision`.
    ///
    /// # Errors
    /// [`Unknown::MutableRevision`] when `revision` is not an object name at
    /// all, which is reported apart from a mismatch: an abbreviated or
    /// uppercase spelling of the SAME commit would otherwise be reported as a
    /// stale view, sending an operator to re-derive something already current.
    /// [`Unknown::RevisionMismatch`] when the view describes a different one.
    pub fn assert_revision(&self, revision: &str) -> Result<(), Unknown> {
        if !is_object_name(revision) {
            return Err(Unknown::MutableRevision(revision.to_owned()));
        }
        if self.identity.revision == revision {
            return Ok(());
        }
        Err(Unknown::RevisionMismatch {
            bound: self.identity.revision.clone(),
            presented: revision.to_owned(),
        })
    }
}
