//! The deny-by-default refusal taxonomy of the Action writer.

/// The gates a submission passes, in check order. A refusal names its
/// gate, so an operator knows which contract the submission broke
/// without reading a stack trace.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RefusalGate {
    /// The invocation was not authorized for this principal, decision,
    /// and action — nothing is appended to the object log.
    Authorization,
    /// Submitted parameters failed conformance against the action type's
    /// declared schema.
    Parameters,
    /// The edit set was refused — a reserved edit kind, an entity-type
    /// mismatch, or a dry-run conformance failure.
    Admission,
}

impl RefusalGate {
    /// Stable operator-facing label; also the denial record's `gate`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Authorization => "authorization",
            Self::Parameters => "parameters",
            Self::Admission => "admission",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refused {
    pub gate: RefusalGate,   // data_class: INTERNAL_ONLY
    pub cause: &'static str, // data_class: INTERNAL_ONLY
}
