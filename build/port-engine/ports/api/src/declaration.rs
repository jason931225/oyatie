//! What a unit declares, as one uniform recursive node.

use std::collections::{BTreeMap, BTreeSet};

use crate::type_ref::TypeRef;

/// One node of a unit's declaration tree: what the unit declares, as the front end saw it.
///
/// UNIFORM BY DESIGN: what tells a constant, a struct field, a function parameter and an interface
/// method apart is [`Declaration::kind`] — a value, not a field name and not an enum variant.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Declaration {
    /// What this node is, as an opaque slug the rule pack selects on.
    pub kind: String, // data_class: INTERNAL_ONLY
    /// The declared identifier. Empty is legal — an unnamed result is a real declaration.
    pub name: String, // data_class: INTERNAL_ONLY
    /// The declared type. Empty when the node declares no type.
    pub type_ref: TypeRef,
    /// Boolean facts, as a set of opaque slugs rather than named booleans.
    pub flags: BTreeSet<String>, // data_class: INTERNAL_ONLY
    /// Key→value facts that do not fit a set: a constant's value, and whatever a later front end
    /// needs to record.
    pub attrs: BTreeMap<String, String>, // data_class: INTERNAL_ONLY
    /// Nested declarations in significant order. A front end that sorts what must stay positional
    /// has produced a defective model.
    pub children: Vec<Declaration>, // data_class: INTERNAL_ONLY
}

impl Declaration {
    #[must_use]
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    #[must_use]
    pub fn children_of_kind(&self, kind: &str) -> Vec<&Self> {
        self.children.iter().filter(|c| c.kind == kind).collect()
    }

    #[must_use]
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }
}
