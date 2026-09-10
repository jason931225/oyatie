//! Ownership: the facts a front end observes, and the dispositions a pack declares.
//!
//! Go is garbage-collected, so a pointer says nothing about ownership and Rust needs it decided.
//! Nobody can decide it from the type, so it is decided from FACTS, by RULES, and the pairing is
//! recorded.

/// What a front end observed about one pointer receiver or parameter.
///
/// Intra-procedural, and [`OwnershipFacts::effect_unknown`] is what keeps that honest.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct OwnershipFacts {
    /// The body provably assigns through it.
    pub mutated: bool,
    /// It provably outlives the call — returned, or captured by a closure.
    pub escapes: bool,
    /// It was passed to a call the front end did not analyse, so `mutated` and `escapes` being
    /// false means UNPROVEN rather than false.
    pub effect_unknown: bool,
}

impl OwnershipFacts {
    #[must_use]
    pub const fn is_clean(self) -> bool {
        !self.mutated && !self.escapes && !self.effect_unknown
    }
}

/// A rule mapping observed facts onto a target ownership form.
///
/// Rules are DATA and are evaluated in declared order, first match winning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PointerDisposition {
    pub id: String, // data_class: INTERNAL_ONLY
    pub when_mutated: Option<bool>,
    pub when_escapes: Option<bool>,
    pub when_effect_unknown: Option<bool>,
    /// Target type template for a PARAMETER position, with `{0}` for the pointee.
    pub target: String, // data_class: INTERNAL_ONLY
    /// Target form for a RECEIVER position: `&self`, `&mut self`, `self`.
    ///
    /// `None` means this disposition has no receiver form, which is a refusal rather than a
    /// fallback — a pointer that escapes cannot be handed out as any borrow of `self`.
    pub receiver: Option<String>, // data_class: INTERNAL_ONLY
    pub reason: String, // data_class: INTERNAL_ONLY
}

impl PointerDisposition {
    #[must_use]
    pub fn accepts(&self, facts: OwnershipFacts) -> bool {
        matches(self.when_mutated, facts.mutated)
            && matches(self.when_escapes, facts.escapes)
            && matches(self.when_effect_unknown, facts.effect_unknown)
    }
}

const fn matches(required: Option<bool>, observed: bool) -> bool {
    match required {
        None => true,
        Some(value) => value == observed,
    }
}
