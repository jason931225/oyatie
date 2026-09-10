//! A source type, as a tree rather than as a flat spelling.

const KIND_BASIC: &str = "basic";
const KIND_NAMED: &str = "named";

/// One node of a source type.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct TypeRef {
    /// What kind of type this is, as an opaque slug the pack decides the meaning of.
    pub kind: String, // data_class: INTERNAL_ONLY
    /// The type's own name, for the kinds that have one. Empty otherwise.
    pub name: String, // data_class: INTERNAL_ONLY
    /// The package that declares this type, for the kinds that have one.
    ///
    /// Without it two packages declaring the same name are indistinguishable, so the resolution
    /// silently picks one.
    pub package: String, // data_class: INTERNAL_ONLY
    /// Type arguments, in significant order: an element type, a key and a value, a parameter list.
    pub args: Vec<TypeRef>,
}

impl TypeRef {
    pub fn of(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            ..Self::default()
        }
    }

    /// A primitive named type: `int`, `bool`, `string`.
    pub fn basic(name: impl Into<String>) -> Self {
        Self {
            kind: KIND_BASIC.to_owned(),
            name: name.into(),
            ..Self::default()
        }
    }

    pub fn named(package: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            kind: KIND_NAMED.to_owned(),
            name: name.into(),
            package: package.into(),
            ..Self::default()
        }
    }

    /// A composite over its arguments: a slice, a map, a pointer.
    pub fn composite(kind: impl Into<String>, args: Vec<Self>) -> Self {
        Self {
            kind: kind.into(),
            args,
            ..Self::default()
        }
    }

    /// `true` when `kind`, `name` and `args` are all empty — the shape a declaration without a type
    /// has.
    ///
    /// `package` is deliberately NOT consulted. This predicate decides the present/absent marker in
    /// the snapshot digest preimage, so widening it would move every digest already recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kind.is_empty() && self.name.is_empty() && self.args.is_empty()
    }

    /// The type's fully-qualified identity, for the kinds that have one: `package.Name`.
    ///
    /// Empty when the type has no name. A bare name is deliberately NOT returned for an unpackaged
    /// type: a qualified key and an unqualified one must not collide in the same table.
    #[must_use]
    pub fn qualified(&self) -> String {
        match (self.package.is_empty(), self.name.is_empty()) {
            (_, true) => String::new(),
            (true, false) => self.name.clone(),
            (false, false) => format!("{}.{}", self.package, self.name),
        }
    }

    /// A readable rendering, for refusal messages. Never used to make a decision.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut out = self.kind.clone();
        let qualified = self.qualified();
        if !qualified.is_empty() {
            out.push(' ');
            out.push_str(&qualified);
        }
        if !self.args.is_empty() {
            let rendered: Vec<String> = self.args.iter().map(Self::describe).collect();
            out.push('<');
            out.push_str(&rendered.join(", "));
            out.push('>');
        }
        out
    }
}
