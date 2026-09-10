//! Items: what an emitted region contains.
//!
//! Visibility, doc comments and a method's receiver are VALUES here rather than string prefixes.
//! That is the difference that fixed two real defects: a `"pub "` prefix concatenated into a trait
//! body produced `pub fn` on a trait method, which `syn` parses and `rustc` rejects; and a
//! receiver rendered as the literal `&self` made an interface's mutating method unimplementable
//! while the concrete path refused the same guess.

use crate::expr::RustStmt;
use crate::ty::RustType;

/// Whether an item is part of the emitted crate's public surface.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum Visibility {
    /// No qualifier. This is also the ONLY legal choice for a trait item, which is as public as
    /// its trait and may not say so.
    #[default]
    Inherited,
    /// `pub`.
    Public,
}

/// How a method takes its receiver.
///
/// An explicit decision with no default. The source's receiver mode is a fact the front end knows
/// and the transform must not guess: `&self` silently drops the mutation a pointer receiver
/// exists to permit, and `&mut self` claims one the source may never perform.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Receiver {
    /// `&self`
    Shared,
    /// `&mut self`
    Exclusive,
    /// `self`
    Owned,
}

impl Receiver {
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Shared => "&self",
            Self::Exclusive => "&mut self",
            Self::Owned => "self",
        }
    }
}

/// One parameter of a function or method.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustParam {
    /// The parameter's name, already cased for the target.
    pub name: String, // data_class: INTERNAL_ONLY
    pub ty: RustType,
}

/// One field of a struct.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustField {
    pub docs: Vec<String>, // data_class: INTERNAL_ONLY
    pub vis: Visibility,
    /// The field's name, already cased for the target.
    pub name: String, // data_class: INTERNAL_ONLY
    pub ty: RustType,
}

/// A function, a method, or a trait method's signature.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RustFn {
    pub docs: Vec<String>, // data_class: INTERNAL_ONLY
    /// Whether the function is public. Always [`Visibility::Inherited`] for a trait item.
    pub vis: Visibility,
    /// The function's name, already cased for the target.
    pub name: String, // data_class: INTERNAL_ONLY
    pub receiver: Option<Receiver>,
    /// Parameters, in order — which is semantic.
    pub params: Vec<RustParam>,
    /// The return type. `None` and the unit type both render without an arrow.
    pub ret: Option<RustType>,
    /// The body. `None` is a signature with no body, which is what a trait item is.
    pub body: Option<Vec<RustStmt>>,
}

/// The shape of a struct's data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructShape {
    /// `struct Name;`
    Unit,
    /// `struct Name(T);` — a newtype, which is how a distinct source type stays distinct.
    Tuple(Vec<RustField>),
    /// `struct Name { .. }`
    Named(Vec<RustField>),
}

/// A top-level item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RustItem {
    /// `const NAME: T = value;`
    Const {
        docs: Vec<String>, // data_class: INTERNAL_ONLY
        vis: Visibility,
        /// Its name, already cased for the target.
        name: String, // data_class: INTERNAL_ONLY
        ty: RustType,
        /// Its value, carried as a source spelling.
        value: String, // data_class: INTERNAL_ONLY
    },
    /// `type Name = T;` — transparent, for a source alias.
    TypeAlias {
        docs: Vec<String>, // data_class: INTERNAL_ONLY
        vis: Visibility,
        name: String, // data_class: INTERNAL_ONLY
        ty: RustType,
    },
    /// A struct, with any inherent methods that belong to it.
    Struct {
        docs: Vec<String>, // data_class: INTERNAL_ONLY
        vis: Visibility,
        name: String, // data_class: INTERNAL_ONLY
        shape: StructShape,
        /// Methods emitted into an `impl` block beside it.
        methods: Vec<RustFn>,
    },
    /// A trait, from a source interface.
    Trait {
        docs: Vec<String>, // data_class: INTERNAL_ONLY
        vis: Visibility,
        name: String, // data_class: INTERNAL_ONLY
        /// Traits this one REQUIRES, from a source interface's embedded interfaces.
        ///
        /// A requirement rather than a copy of the method set: an implementor must implement these
        /// too, which is what the source means by embedding and what a flattened method list would
        /// silently weaken.
        supertraits: Vec<RustType>,
        /// Its required methods, as signatures.
        methods: Vec<RustFn>,
    },
    /// `impl Trait for Type { .. }`, from an OBSERVED interface satisfaction.
    ///
    /// The trait is a path rather than a name because the interface a type satisfies is routinely
    /// declared in another unit, and the impl is emitted where the type is.
    TraitImpl {
        /// Documentation carried over from the source, plus how the satisfaction was observed.
        docs: Vec<String>, // data_class: INTERNAL_ONLY
        trait_path: RustType,
        self_ty: RustType,
        /// The trait's required methods, each with a body.
        methods: Vec<RustFn>,
    },
    Function(RustFn),
}
