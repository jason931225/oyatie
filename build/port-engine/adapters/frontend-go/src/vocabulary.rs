//! Closed vocabularies the Go snapshot may use.
//!
//! CLOSED, and closed HERE rather than in `port-engine-api`, on purpose. The neutral seam treats
//! `kind`, `flags` and attribute keys as opaque slugs because a second language pair must not need
//! a second seam. This adapter is the Go half, so this is exactly where Go's taxonomy is allowed
//! to be named — and where an extractor emitting something the engine has never heard of gets
//! refused instead of translated into silence.

/// Canonical bootstrap extractor identity (ADR-0638 D3).
pub const PRODUCER_BOOTSTRAP_GO: &str = "bootstrap-go-packages-go-types";

/// Owned Rust front-end producer identity (authorized only after W2 equivalence).
pub const PRODUCER_OWNED_RUST: &str = "owned-rust-go-front-end";

pub const SCHEMA_VERSION_IDENTITY_ONLY: u32 = 0;

/// Flat type spellings. NOT ACCEPTED: refusing names the fix (regenerate) instead of
/// half-answering with a spelling the engine would have to re-resolve.
pub const SCHEMA_VERSION_FLAT_TYPES: u32 = 1;

pub const SCHEMA_VERSION_DECLARATIONS: u32 = 2;

/// Closed for the same reason as [`KNOWN_DECLARATION_KINDS`].
pub const KNOWN_TYPE_KINDS: &[&str] = &[
    "array",
    "basic",
    "chan",
    "func",
    "interface",
    "map",
    "named",
    // A named type whose underlying type is an interface. Same identity as `named`, separate kind
    // because the target holds the two differently: a struct is a value and a trait has no size,
    // so a trait reaches a position as a reference, a box or a generic parameter — and which of
    // those is an ownership decision the pack makes rather than a spelling the engine infers.
    "named_interface",
    "pointer",
    "slice",
    "struct",
    "tuple",
    "type_param",
    "unsupported",
];

/// Declaration kinds this Go adapter admits, at package scope.
pub const KNOWN_DECLARATION_KINDS: &[&str] = &[
    "alias",
    "const",
    // An observed interface satisfaction whose concrete type this corpus does not declare, so
    // there is nowhere to emit the impl. Admitted because the front end RECORDS it rather than
    // dropping it — a satisfaction the engine cannot host must be distinguishable from one that
    // does not exist — and deferred by the pack, which is where the decision belongs.
    "foreign_satisfaction",
    "func",
    // The package's `init` work. The source keeps it out of package scope — it is not addressable
    // and several may exist — so it reached the model nowhere until it was collected explicitly. A
    // construct that is neither translated nor refused is the one outcome this engine cannot answer
    // for, so it is admitted here and deferred by the pack.
    "package_init",
    "interface",
    "named",
    "struct",
    "var",
];

/// The kinds whose children form a NAMESPACE — a scope in which one name means one thing.
///
/// Package scope, a struct's fields, a signature's parameters. Below these the tree is SYNTAX, and
/// syntax repeats names freely: `c.total + other.total` has two sibling nodes called `total` and
/// both are correct.
pub const NAMESPACE_KINDS: &[&str] = &[
    "alias",
    "const",
    "func",
    "interface",
    "method",
    "named",
    "struct",
    "var",
];

/// Declaration kinds admitted below package scope, as children of a declaration.
///
/// One flat list rather than a per-level one. The body vocabulary nests arbitrarily — a `binary`
/// inside a `return` inside a `then` inside an `if` — so a level-indexed grammar would have to
/// enumerate the nesting rules of the source language here, in the adapter that is supposed to
/// carry only its taxonomy. The precision this gives up is that a `param` could nominally contain
/// a `field`; the precision it keeps is the one that matters, which is that no PACKAGE-scope kind
/// can appear as a member.
pub const KNOWN_MEMBER_KINDS: &[&str] = &[
    "assign",
    "binary",
    "break",
    "call",
    "case",
    "composite",
    "convert",
    "for",
    // An interface an interface EMBEDS. The target has no embedding, so this becomes a
    // supertrait — which is a requirement rather than a copy of the method set, and is why it is
    // recorded as a relation rather than flattened into the outer interface's methods.
    "embeds",
    // A method a type gains through EMBEDDING rather than declaration. The target has no
    // promotion, so what is implicit in the source becomes a forwarding method — and recording it
    // is what closes `census/interfaces.md` §11 item 7, where types have method sets larger than
    // the census could measure.
    "promoted",
    // An observed interface satisfaction, hung on the concrete type that satisfies it. It is a
    // MEMBER kind rather than a declaration kind because the impl belongs to the type: emitting it
    // anywhere else would need the orphan rule answered by the front end, which is a target
    // question the source cannot see.
    "implements",
    "index",
    "init",
    "keyed",
    "over",
    "patterns",
    "post",
    "range",
    "selector",
    "slice",
    "absent",
    "switch",
    "tag",
    "block",
    "body",
    "cond",
    "else",
    "expr_stmt",
    "field",
    "ident",
    "if",
    // A destructuring bind and its two child kinds. Recorded rather than refused because it is
    // the shape every fallible call in the source has, and a rule cannot fire on a shape the
    // snapshot never carries.
    "bind",
    "let",
    "let_tuple",
    "literal",
    "method",
    "paren",
    "param",
    "result",
    "return",
    "then",
    "unary",
    "value",
    // A field a struct literal LEFT OUT, carrying the type whose zero fills it. Recorded
    // rather than omitted because the target needs every field named, and an absent entry
    // would be indistinguishable from a field the front end failed to see.
    "zero",
    // `unsupported` is how the snapshot stays a faithful model of the source while the engine
    // stays fail-closed: a construct the translator cannot handle is RECORDED as present, and
    // refused by name at transform. Omitting it would make an untranslatable function
    // indistinguishable from an empty one.
    "unsupported",
];

/// The closed flag vocabulary. Same argument as [`KNOWN_DECLARATION_KINDS`]: a flag the engine
/// does not know is a flag nothing will ever select on, and accepting it would let a misspelled
/// `exported` silently unexport a declaration.
pub const KNOWN_FLAGS: &[&str] = &[
    "effect_unknown",
    "embedded",
    "escapes",
    "exported",
    "mutated",
    "pointer_receiver",
    "variadic",
];

/// The closed attribute-key vocabulary, closed for the same reason as the flags.
pub const KNOWN_ATTR_KEYS: &[&str] = &[
    ATTR_CALLEE,
    ATTR_CALLEE_KIND,
    ATTR_DOC,
    ATTR_GO_NODE,
    ATTR_LIT_KIND,
    ATTR_OP,
    ATTR_RANGE_KEY,
    ATTR_RECEIVER,
    ATTR_REF,
    ATTR_SITE,
    ATTR_VALUE,
    ATTR_VIA,
];

/// Attribute key holding the dotted FIELD PATH a promoted method is reached through.
///
/// The target has no method promotion, so a forwarding method has to name the field it forwards to.
pub const ATTR_VIA: &str = "via";

/// Attribute key holding the name a `range` loop binds as its key.
pub const ATTR_RANGE_KEY: &str = "key";

/// A `range` loop's value binding and a constant's value deliberately share one key.
pub const ATTR_RANGE_VALUE: &str = ATTR_VALUE;

/// Attribute key holding the package-qualified IDENTITY of what a call resolves to.
///
/// The identity rather than the spelling, because a rule keyed on text would answer for a local
/// variable that shares a package's name.
pub const ATTR_CALLEE: &str = "callee";

/// Attribute key distinguishing a call through a RECEIVER from a call to a free function.
///
/// The source spells `value.Method()` and `package.Function()` identically; the target does not.
/// Only the type-checker can tell which name is a package, so the front end records it rather than
/// leaving the transform to guess from syntax.
pub const ATTR_CALLEE_KIND: &str = "callee_kind";

/// Attribute key holding the receiver a TRAIT method binds, derived from its observed
/// implementors.
///
/// A source interface says nothing about receiver mode, so this is the one answer the declaration
/// cannot give and the corpus can. Its absence means nothing was observed to implement the
/// interface, and the pack's declared decision answers instead.
pub const ATTR_RECEIVER: &str = "receiver";

/// Attribute key recording HOW an interface satisfaction was observed.
///
/// A declared assertion is compile-checked by the source language; a flow-derived one is the front
/// end's inference. An impl emitted from either looks identical, so the distinction is recorded
/// rather than left to be reconstructed.
pub const ATTR_SITE: &str = "site";

/// Attribute key holding a constant's or literal's value, spelled as source.
pub const ATTR_VALUE: &str = "value";

/// Attribute key holding a binary or unary operator, spelled as source.
pub const ATTR_OP: &str = "op";

/// Attribute key naming the source AST node an `unsupported` placeholder stands for, so a refusal
/// can say WHAT it refused rather than only that it refused.
pub const ATTR_GO_NODE: &str = "go_node";

/// Attribute key classifying what an identifier resolves to — a constant, a function, a local.
/// The target cases each differently, and the identifier alone cannot say which it is.
pub const ATTR_REF: &str = "ref";

pub const ATTR_LIT_KIND: &str = "lit_kind";

/// Attribute key carrying a declaration's documentation block, newline-separated.
///
/// Recorded because the target emits it. Dropping documentation is a SILENT loss — coverage
/// proves every declaration was translated, not that everything about a declaration survived —
/// and no downstream check looks for prose that is simply absent.
pub const ATTR_DOC: &str = "doc";
