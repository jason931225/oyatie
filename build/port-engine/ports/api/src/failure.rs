//! The source's failure convention.
//!
//! Deliberately NOT here: `Result`, `Ok`, `Err`. Those are target spellings, and the face that
//! renders the target owns them.

/// How the source spells failure, so the engine can recognise it without knowing the language.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureConvention {
    pub source_type: String, // data_class: INTERNAL_ONLY
    pub target_type: String, // data_class: INTERNAL_ONLY
    /// How the source spells the ABSENT failure value, which is what a success returns and what a
    /// check compares against.
    pub absent: String, // data_class: INTERNAL_ONLY
}

impl FailureConvention {
    #[must_use]
    pub fn is_failure(&self, identity: &str) -> bool {
        identity == self.source_type
    }
}
