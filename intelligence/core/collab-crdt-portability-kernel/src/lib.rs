//! ADR-0142 — CRDT portability seam trait.
//!
//! The runtime-neutral surface a host depends on instead of a concrete CRDT
//! library. No adapter implements it yet.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

use std::fmt;

/// Document identifier. The adapter never mints one; the host does.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct CrdtDocId(pub String); // data_class: INTERNAL_ONLY

impl fmt::Display for CrdtDocId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A change emitted at one peer and applied at another. The payload encoding
/// is the adapter's; the host signs the bytes without reading them
/// (ADR-0145 Invariant 1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrdtChange {
    pub doc_id: CrdtDocId, // data_class: INTERNAL_ONLY
    pub payload: Vec<u8>,  // data_class: INTERNAL_ONLY
}

/// Stable error surface for adapter failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CrdtPortabilityError {
    /// Encoding mismatch, typically a payload replayed across adapters.
    PayloadInvalid(String),
    UnknownDocument(CrdtDocId),
    /// Anything the underlying runtime reported that the kernel cannot class.
    Adapter(String),
}

impl fmt::Display for CrdtPortabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadInvalid(reason) => write!(f, "crdt payload invalid: {reason}"),
            Self::UnknownDocument(doc) => write!(f, "crdt doc unknown: {doc}"),
            Self::Adapter(reason) => write!(f, "crdt adapter error: {reason}"),
        }
    }
}

impl std::error::Error for CrdtPortabilityError {}

/// The seam a host depends on in place of a concrete CRDT runtime.
pub trait CrdtPortabilityRuntime {
    fn create_doc(&mut self, doc_id: CrdtDocId) -> Result<(), CrdtPortabilityError>;

    fn apply_change(&mut self, change: &CrdtChange) -> Result<(), CrdtPortabilityError>;

    fn snapshot(&self, doc_id: &CrdtDocId) -> Result<CrdtChange, CrdtPortabilityError>;
}

/// Test double only: it appends payloads in arrival order and converges on
/// nothing, so it satisfies the trait without being a CRDT.
#[derive(Default)]
pub struct InMemoryCrdtRuntime {
    docs: std::collections::BTreeMap<CrdtDocId, Vec<u8>>, // data_class: INTERNAL_ONLY
}

impl CrdtPortabilityRuntime for InMemoryCrdtRuntime {
    fn create_doc(&mut self, doc_id: CrdtDocId) -> Result<(), CrdtPortabilityError> {
        self.docs.entry(doc_id).or_default();
        Ok(())
    }

    fn apply_change(&mut self, change: &CrdtChange) -> Result<(), CrdtPortabilityError> {
        match self.docs.get_mut(&change.doc_id) {
            Some(state) => {
                state.extend_from_slice(&change.payload);
                Ok(())
            }
            None => Err(CrdtPortabilityError::UnknownDocument(change.doc_id.clone())),
        }
    }

    fn snapshot(&self, doc_id: &CrdtDocId) -> Result<CrdtChange, CrdtPortabilityError> {
        match self.docs.get(doc_id) {
            Some(state) => Ok(CrdtChange {
                doc_id: doc_id.clone(),
                payload: state.clone(),
            }),
            None => Err(CrdtPortabilityError::UnknownDocument(doc_id.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_runtime_roundtrips_change() {
        let mut runtime = InMemoryCrdtRuntime::default();
        let doc = CrdtDocId("01JABCDEFGHIJKLMNPQRSTUVWX".to_string());
        runtime.create_doc(doc.clone()).expect("create ok");
        runtime
            .apply_change(&CrdtChange {
                doc_id: doc.clone(),
                payload: b"hello".to_vec(),
            })
            .expect("apply ok");
        let snap = runtime.snapshot(&doc).expect("snapshot ok");
        assert_eq!(snap.payload, b"hello");
    }

    #[test]
    fn unknown_doc_surfaces_error() {
        let runtime = InMemoryCrdtRuntime::default();
        let doc = CrdtDocId("01JMISSINGZZZZZZZZZZZZZZZZ".to_string());
        let err = runtime.snapshot(&doc).unwrap_err();
        assert!(matches!(err, CrdtPortabilityError::UnknownDocument(_)));
    }

    #[test]
    fn error_display_renders_human_messages() {
        let payload_err = CrdtPortabilityError::PayloadInvalid("bad-magic".to_string());
        assert!(format!("{payload_err}").contains("bad-magic"));
    }
}
