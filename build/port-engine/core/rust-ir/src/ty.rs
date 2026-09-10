//! Target types, as a tree.
//!
//! Today a type reaches the IR as a resolved target SPELLING — the rule pack's type map answers
//! with `i64`, `String`, `&'static str`, and the transform resolves a locally declared name to its
//! emitted identifier. So [`RustType::Path`] carries that spelling and the lowering parses it.
//!
//! That is deliberately a floor rather than a ceiling. The variants below exist so a structured
//! source type model can lower into a structured target type without this seam changing shape:
//! `[]T` becomes [`RustType::Generic`], `*T` becomes [`RustType::Reference`] or a `Box`, and a
//! multi-value result becomes [`RustType::Tuple`]. Until that phase lands, a pack that answers
//! with a spelling gets a `Path` and nothing pretends otherwise.

/// A type in the emitted language.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RustType {
    /// An already-resolved spelling: a primitive, a path, or anything the pack's map answered with.
    Path(String), // data_class: INTERNAL_ONLY
    /// A reference, shared or exclusive.
    Reference {
        /// `true` for `&mut`.
        mutable: bool,
        inner: Box<RustType>,
    },
    /// A tuple. Empty is the unit type.
    Tuple(Vec<RustType>),
    /// A path applied to type arguments: `Vec<T>`, `Option<T>`, `BTreeMap<K, V>`.
    Generic {
        path: String, // data_class: INTERNAL_ONLY
        args: Vec<RustType>,
    },
}

impl RustType {
    pub fn path(spelling: impl Into<String>) -> Self {
        Self::Path(spelling.into())
    }

    /// The unit type, `()`.
    #[must_use]
    pub const fn unit() -> Self {
        Self::Tuple(Vec::new())
    }

    /// `true` when this is the unit type, which a signature renders by omitting the return arrow.
    #[must_use]
    pub fn is_unit(&self) -> bool {
        matches!(self, Self::Tuple(args) if args.is_empty())
    }

    /// Render to the spelling the lowering parses.
    ///
    /// Producing text here and parsing it in the lowering looks like the round-trip this IR exists
    /// to remove, and the distinction is that the round-trip is over a TYPE rather than over an
    /// item. A type spelling has no statement structure, no operator precedence and no visibility
    /// to get wrong, and it arrives as a spelling from the pack in the first place — so there is
    /// no structure here to lose. The item tree, where those hazards live, never becomes text.
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            Self::Path(path) => path.clone(),
            Self::Reference { mutable, inner } => {
                let prefix = if *mutable { "&mut " } else { "&" };
                format!("{prefix}{}", inner.spelling())
            }
            Self::Tuple(args) => {
                let rendered: Vec<String> = args.iter().map(Self::spelling).collect();
                format!("({})", rendered.join(", "))
            }
            Self::Generic { path, args } => {
                let rendered: Vec<String> = args.iter().map(Self::spelling).collect();
                format!("{path}<{}>", rendered.join(", "))
            }
        }
    }
}
