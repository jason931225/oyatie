//! The strings the pack and the front end agreed on.
//!
//! Opaque to this face: it compares them and never interprets them. `struct` reaches here as a
//! kind some rule chose to capture, not as a concept.

/// Unit-level construction: emit one empty region named for the unit.
pub const CONSTRUCTION_PASS_THROUGH: &str = "pass_through";
/// Unit-level construction: emit a minimal empty fn region for fixture coupling.
pub const CONSTRUCTION_EMPTY_CANARY: &str = "empty_canary";

/// Declaration-level construction: a constant with a declared type and value.
pub const CONSTRUCTION_RUST_CONST: &str = "rust_const";
/// Declaration-level construction: a transparent type alias.
pub const CONSTRUCTION_RUST_TYPE_ALIAS: &str = "rust_type_alias";
/// Declaration-level construction: a single-field tuple struct over the underlying type.
pub const CONSTRUCTION_RUST_NEWTYPE: &str = "rust_newtype";
/// Declaration-level construction: a struct with fields, plus an `impl` block whose methods
/// are stubbed.
pub const CONSTRUCTION_RUST_STRUCT: &str = "rust_struct";
/// Declaration-level construction: a struct whose methods carry their translated bodies.
pub const CONSTRUCTION_RUST_STRUCT_BODY: &str = "rust_struct_body";
/// Declaration-level construction: a trait with one signature per method.
pub const CONSTRUCTION_RUST_TRAIT: &str = "rust_trait";
/// Declaration-level construction: a free function, signature only.
pub const CONSTRUCTION_RUST_FN: &str = "rust_fn";
/// Declaration-level construction: a free function with its body translated.
///
/// A SEPARATE construction rather than a smarter [`CONSTRUCTION_RUST_FN`]. One construction that
/// translated a body when it could and fell back to a stub when it could not would degrade
/// silently: the emitted crate would compile either way, and nothing downstream could tell a
/// translated function from an abandoned one. Two constructions make the pack say which it wants,
/// and asking for a body it cannot have is a refusal.
pub const CONSTRUCTION_RUST_FN_BODY: &str = "rust_fn_body";

/// Precondition: the planned unit must exist in the source model.
pub const PRECONDITION_UNIT_PRESENT: &str = "unit_present";

/// Attribute key a construction reads a declared value from.
pub const ATTR_VALUE: &str = "value";
/// Attribute key holding a binary or unary operator.
pub const ATTR_OP: &str = "op";
/// Attribute key naming the source construct an `unsupported` node stands for.
pub const ATTR_SOURCE_NODE: &str = "go_node";
/// The [`ATTR_SOURCE_NODE`] value a `++`/`--` statement arrives as.
pub(crate) const SOURCE_NODE_INCREMENT: &str = "IncDecStmt";
/// The named POSITIONS a type can appear in, which is what decides the form a trait takes there.
///
/// A borrowed trait object is right for a parameter and impossible for a value a function returns,
/// so the pack answers per position rather than once.
pub const POSITION_PARAM: &str = "param";
/// See [`POSITION_PARAM`].
pub const POSITION_RESULT: &str = "result";
/// See [`POSITION_PARAM`].
pub const POSITION_FIELD: &str = "field";
/// The position a trait appears in as another trait's REQUIREMENT.
pub const POSITION_SUPERTRAIT: &str = "supertrait";
/// The position an `impl Trait for Type` names its trait in — the one place a trait appears as
/// itself rather than as something holding it, and still declared rather than special-cased.
pub const POSITION_TRAIT: &str = "trait";

/// Type kind: a named type whose underlying type is an interface.
///
/// Distinguished from a plain named type because the target holds the two differently — a struct
/// is a value and a trait has no size.
pub const TYPE_NAMED_INTERFACE: &str = "named_interface";

/// Attribute key classifying what an identifier resolves to.
pub const ATTR_REF: &str = "ref";
/// Attribute key holding the package-qualified IDENTITY of what a call resolves to.
pub const ATTR_CALLEE: &str = "callee";
/// Attribute key distinguishing a call through a RECEIVER from a call to a free function.
pub const ATTR_CALLEE_KIND: &str = "callee_kind";
/// The one value [`ATTR_CALLEE_KIND`] takes; its absence means a free function.
pub const CALLEE_KIND_METHOD: &str = "method";
/// Attribute key holding the receiver a TRAIT method binds, derived from its observed
/// implementors. Absent means nothing was observed to implement the interface, and the pack's
/// declared decision answers instead.
pub const ATTR_RECEIVER: &str = "receiver";
/// Attribute key holding the dotted FIELD PATH a promoted method is reached through.
pub const ATTR_VIA: &str = "via";
/// Attribute key recording HOW an interface satisfaction was observed.
pub const ATTR_SITE: &str = "site";
/// Attribute key carrying the source declaration's documentation block.
pub const ATTR_DOC: &str = "doc";

/// Flag marking a declaration as part of the source's public surface.
pub const FLAG_EXPORTED: &str = "exported";
/// Flag marking a variadic signature.
pub const FLAG_VARIADIC: &str = "variadic";
/// Flag marking a method bound through a pointer receiver.
pub const FLAG_POINTER_RECEIVER: &str = "pointer_receiver";
/// Ownership fact: the body provably assigns through this pointer.
pub const FLAG_MUTATED: &str = "mutated";
/// Ownership fact: this pointer provably outlives the call.
pub const FLAG_ESCAPES: &str = "escapes";
/// Ownership fact: this pointer reached a call the front end did not analyse, so the other two
/// facts being absent means UNPROVEN rather than false.
pub const FLAG_EFFECT_UNKNOWN: &str = "effect_unknown";

pub(crate) const CHILD_FIELD: &str = "field";
pub(crate) const CHILD_METHOD: &str = "method";
pub(crate) const CHILD_PARAM: &str = "param";
pub(crate) const CHILD_RESULT: &str = "result";
pub(crate) const CHILD_BODY: &str = "body";
/// An OBSERVED interface satisfaction, carrying the trait's full method set.
///
/// The method set rides on the node rather than being looked up from the interface's own
/// declaration, because the interface routinely lives in another unit and the impl is emitted
/// where the type is — a cross-unit lookup is a reference the model does not carry.
pub(crate) const CHILD_IMPLEMENTS: &str = "implements";
/// An interface an interface EMBEDS, which the target spells as a supertrait.
pub(crate) const CHILD_EMBEDS: &str = "embeds";
/// A method a type gains through EMBEDDING rather than declaration.
pub(crate) const CHILD_PROMOTED: &str = "promoted";
/// One name a destructuring bind introduces.
pub(crate) const CHILD_BIND: &str = "bind";
/// The expression a destructuring bind takes its values from.
pub(crate) const CHILD_VALUE: &str = "value";
