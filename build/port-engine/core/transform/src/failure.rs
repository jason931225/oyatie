//! The source's failure convention, made structural.
//!
//! The source returns failure as a TRAILING RESULT of a designated type, checked by convention
//! rather than enforced by the type system. The target expresses the same thing as the WHOLE
//! return type. Three shapes carry it, each matched structurally and refused otherwise:
//!
//! - a fallible signature: `(T, error)` becomes `Result<T, E>`, and the failure value stops being a
//!   value the caller may drop;
//! - a return: the trailing operand decides which constructor the whole return becomes;
//! - the CHECK: a bind followed by a guarded early return is the source's propagation idiom, and
//!   it is the one shape that becomes an operator rather than a statement.
//!
//! Everything about the target — `Result`, `Ok`, `Err`, `?` — is decided here rather than declared
//! in the pack, because this face renders Rust. What the pack declares is which SOURCE type carries
//! the convention, because that is the half a second language pair would answer differently.

use port_engine_api::{Declaration, FailureConvention, TypeRef};

use crate::vocabulary::{ATTR_REF, CHILD_BIND, CHILD_RESULT, CHILD_VALUE};

/// Whether a type is the source's failure type.
///
/// Keyed on the type's own identity rather than on its spelling in a signature, so an aliased or
/// re-exported failure type is the same type — and so a locally declared type that happens to share
/// the name is not.
pub(crate) fn is_failure_type(type_ref: &TypeRef, convention: Option<&FailureConvention>) -> bool {
    let Some(convention) = convention else {
        return false;
    };
    let identity = if type_ref.qualified().is_empty() {
        type_ref.kind.clone()
    } else {
        type_ref.qualified()
    };
    convention.is_failure(&identity)
}

/// Whether a declaration's results end in the failure type — the source's whole signal that a
/// function can fail.
pub(crate) fn is_fallible(
    declaration: &Declaration,
    convention: Option<&FailureConvention>,
) -> bool {
    declaration
        .children_of_kind(CHILD_RESULT)
        .last()
        .is_some_and(|result| is_failure_type(&result.type_ref, convention))
}

/// Whether an expression is the source's ABSENT failure value.
///
/// This is what separates a success from a failure in a return with the same arity, so it is
/// recognised from the front end's classification of the identifier rather than from its spelling —
/// a local variable named `nil` is not the nil literal, and only the type-checker knows that.
pub(crate) fn is_absent(node: &Declaration, convention: Option<&FailureConvention>) -> bool {
    let Some(convention) = convention else {
        return false;
    };
    node.attr(ATTR_REF) == Some("nil") && node.name == convention.absent
}

/// The propagation idiom, matched structurally over a pair of statements.
///
/// `v, err := f()` followed by `if err != nil { return …, err }` is what the target spells `?`.
/// Matching it needs BOTH statements, because the bind alone says nothing — a program that binds a
/// failure and handles it some other way is a different program, and one that binds it and ignores
/// it is a third.
pub(crate) struct Propagation<'a> {
    /// The names bound before the failure value, in order.
    pub(crate) values: Vec<&'a str>,
    /// The expression the bind takes its values from.
    pub(crate) source: &'a Declaration,
}

/// Recognise the propagation idiom at `statements[index]`, consuming the check that follows it.
pub(crate) fn propagation<'a>(
    statements: &'a [Declaration],
    index: usize,
    convention: Option<&FailureConvention>,
) -> Option<Propagation<'a>> {
    let bind = statements.get(index)?;
    let (failure, values, source) = match bind.kind.as_str() {
        // `v, err := f()` — the value-and-failure shape.
        "let_tuple" => {
            let binds = bind.children_of_kind(CHILD_BIND);
            let (failure, values) = binds.split_last()?;
            let value = bind.children_of_kind(CHILD_VALUE).first().copied()?;
            (
                failure.name.as_str(),
                values.iter().map(|bound| bound.name.as_str()).collect(),
                value.children.first()?,
            )
        }
        // `err := f()` — the failure-only shape. The bound name IS the failure, so there is no
        // value to carry and the target keeps the call as a statement.
        "let" => (bind.name.as_str(), Vec::new(), bind.children.first()?),
        _ => return None,
    };

    if !checks_and_returns(statements.get(index + 1)?, failure, convention) {
        return None;
    }
    Some(Propagation { values, source })
}

/// Whether a statement is exactly `if <name> != <absent> { return …, <name> }`.
///
/// Every clause is checked. A body that does anything else before returning, or returns something
/// other than the failure it just checked, is a program the operator does not express — and
/// emitting the operator for it would silently drop whatever else the body did.
fn checks_and_returns(
    statement: &Declaration,
    name: &str,
    convention: Option<&FailureConvention>,
) -> bool {
    if statement.kind != "if" || !statement.children_of_kind("else").is_empty() {
        return false;
    }
    let Some(condition) = statement.children_of_kind("cond").first().copied() else {
        return false;
    };
    let Some(test) = condition.children.first() else {
        return false;
    };
    if test.kind != "binary" || test.attr(crate::vocabulary::ATTR_OP) != Some("!=") {
        return false;
    }
    let [lhs, rhs] = test.children.as_slice() else {
        return false;
    };
    if lhs.kind != "ident" || lhs.name != name || !is_absent(rhs, convention) {
        return false;
    }

    let Some(then) = statement.children_of_kind("then").first().copied() else {
        return false;
    };
    let [returned] = then.children.as_slice() else {
        return false;
    };
    returned.kind == "return"
        && returned
            .children
            .last()
            .is_some_and(|value| value.kind == "ident" && value.name == name)
}
