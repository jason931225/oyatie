//! Cedar-style policy obligations and advice annotations.
//!
//! Annotations are key/value pairs attached to [`crate::PolicyRule`] at authoring
//! time and collected onto [`crate::AuthorizationDecision`] at evaluation time.
//!
//! # Cedar semantics
//!
//! Cedar distinguishes **obligations** (must-execute side effects the PEP is required
//! to act on) from **advice** (informational hints the PEP may use).  Both are
//! represented as `(kind, key, value)` triples.  [`AnnotationKind`] encodes the
//! distinction.

use serde::{Deserialize, Serialize};

/// Whether an annotation is a required obligation or an informational advice hint.
///
/// Cedar terminology:
/// - `Obligation` — the PEP *must* execute this side effect (step-up auth, redaction, …).
/// - `Advice`     — the PEP *may* use this hint (audit context, UI labels, …).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationKind {
    Obligation,
    Advice,
}

/// A single Cedar-style policy annotation attached to a rule or surfaced on a decision.
///
/// Annotations are pure value types.  `key` and `value` are plain strings; semantic
/// validation is the policy author's responsibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PolicyAnnotation {
    /// Whether this annotation is an obligation or advice.
    pub kind: AnnotationKind,
    /// Annotation key (e.g. `"require_mfa"`, `"audit_event"`, `"redact_fields"`).
    pub key: String,
    /// Annotation value (e.g. `"true"`, `"pii_access"`, `"email,phone"`).
    pub value: String,
}
