//! Loop and switch statements.
//!
//! Every shape here is matched STRUCTURALLY and refused otherwise. A loop that merely resembles a
//! translated one is a different loop, and getting it wrong changes how many times a body runs —
//! which is the class of defect no golden and no parse check would ever surface.

use port_engine_api::Declaration;
use port_engine_rust_ir::{MatchArm, RustExpr, RustStmt};

use crate::body::{Body, TailPosition, branch, named_child, one_child, translate, two_children};
use crate::body_expr::expression;
use crate::error::TransformError;
use crate::naming::to_snake_case;
use crate::vocabulary::{ATTR_OP, ATTR_SOURCE_NODE, SOURCE_NODE_INCREMENT};

/// A three-clause or condition-only `for`.
///
/// The CANONICAL ascending-integer form becomes a range loop, because that is what it means and
/// what a reader expects. Everything else with a post-statement REFUSES rather than becoming a
/// `while`: a `while` runs its post-statement only on the paths that reach the end of the body, so
/// a `continue` — or any early exit added later — silently skips it. That is a different program,
/// and it is different in a way no test of the current corpus would catch.
pub(crate) fn counted_loop(node: &Declaration, cx: &Body<'_>) -> Result<RustStmt, TransformError> {
    let body = branch(node, "then", cx)?;
    let condition = node.children_of_kind("cond").first().copied();
    let init = node.children_of_kind("init").first().copied();
    let post = node.children_of_kind("post").first().copied();

    match (init, condition, post) {
        (None, None, None) => Ok(RustStmt::Loop(translate(
            &body.children,
            cx,
            TailPosition::No,
        )?)),
        (None, Some(cond), None) => Ok(RustStmt::While {
            cond: expression(one_child(cond, cx, "cond")?, cx)?,
            body: translate(&body.children, cx, TailPosition::No)?,
        }),
        (Some(init), Some(cond), Some(post)) => counted_range(init, cond, post, body, cx),
        _ => Err(TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: "a `for` with only some of its clauses has no direct target form".to_owned(),
        }),
    }
}

/// Recognise `for i := A; i < B; i++` and emit `for i in A..B`.
///
/// Matched STRUCTURALLY and refused otherwise. A loop that merely resembles this one — a different
/// variable in the condition, a decrement, a bound that changes inside the body — is a different
/// loop, and emitting a range for it would silently change how many times the body runs.
fn counted_range(
    init: &Declaration,
    cond: &Declaration,
    post: &Declaration,
    body: &Declaration,
    cx: &Body<'_>,
) -> Result<RustStmt, TransformError> {
    let init = one_child(init, cx, "init")?;
    let cond = one_child(cond, cx, "cond")?;
    let post = one_child(post, cx, "post")?;

    let refuse = |why: &str| TransformError::Unsupported {
        name: cx.owner.to_owned(),
        detail: format!(
            "`for` loop is not the canonical ascending-integer form ({why}), and a `while` would \
             run its post-statement only on the paths that reach the end of the body"
        ),
    };

    if init.kind != "let" {
        return Err(refuse("the init clause does not bind a new name"));
    }
    let counter = &init.name;
    if cond.kind != "binary" || cond.attr(ATTR_OP) != Some("<") {
        return Err(refuse("the condition is not `<`"));
    }
    let (lhs, rhs) = two_children(cond, cx, "cond")?;
    if lhs.kind != "ident" || &lhs.name != counter {
        return Err(refuse("the condition does not test the counter"));
    }
    if post.attr(ATTR_SOURCE_NODE) != Some(SOURCE_NODE_INCREMENT) {
        return Err(refuse("the post clause is not an increment"));
    }

    Ok(RustStmt::ForIn {
        binding: to_snake_case(counter),
        iter: RustExpr::Range {
            start: Box::new(expression(one_child(init, cx, "let")?, cx)?),
            end: Box::new(expression(rhs, cx)?),
        },
        body: translate(&body.children, cx, TailPosition::No)?,
    })
}

/// A `range` loop over a sequence.
///
/// Only the value-only form translates. `for i, v := range xs` binds two names, and the target's
/// equivalent — `.iter().enumerate()` — changes what `v` IS from a copy to a reference; that is a
/// rule about element ownership rather than a loop shape, so it refuses here.
pub(crate) fn range_loop(node: &Declaration, cx: &Body<'_>) -> Result<RustStmt, TransformError> {
    let over = named_child(node, "over", cx, "range")?;
    let body = branch(node, "then", cx)?;

    let key = node.attr("key").unwrap_or_default();
    let value = node.attr("value").unwrap_or_default();

    let binding = match (key.is_empty() || key == "_", value.is_empty()) {
        (true, false) => value,
        _ => {
            return Err(TransformError::Unsupported {
                name: cx.owner.to_owned(),
                detail: "only `for _, v := range xs` translates: binding the index too needs a \
                         rule for whether the element is a copy or a reference"
                    .to_owned(),
            });
        }
    };

    Ok(RustStmt::ForIn {
        binding: to_snake_case(binding),
        // Iterate by REFERENCE. Go's range copies the element and leaves the sequence usable;
        // consuming it here would end the sequence's life at the loop.
        iter: RustExpr::Reference {
            mutable: false,
            inner: Box::new(expression(one_child(over, cx, "over")?, cx)?),
        },
        body: translate(&body.children, cx, TailPosition::No)?,
    })
}

/// An expression switch becomes a `match`.
///
/// The target's `match` does not fall through and neither does the source's switch, so the two
/// agree on the one semantic that usually differs between languages here.
pub(crate) fn switch(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    let cases = node.children_of_kind("case");
    let Some(tag) = node.children_of_kind("tag").first().copied() else {
        return Err(TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: "a switch with no tag is a condition chain rather than a match, and needs a \
                     rule for how an empty case list becomes an `else`"
                .to_owned(),
        });
    };

    let mut arms = Vec::with_capacity(cases.len());
    let mut wildcard_seen = false;
    for case in cases {
        let patterns_node = named_child(case, "patterns", cx, "switch")?;
        let body = branch(case, "then", cx)?;
        let patterns = patterns_node
            .children
            .iter()
            .map(|pattern| expression(pattern, cx))
            .collect::<Result<Vec<_>, _>>()?;
        wildcard_seen |= patterns.is_empty();
        arms.push(MatchArm {
            patterns,
            body: translate(&body.children, cx, TailPosition::No)?,
        });
    }

    // A `match` must be exhaustive and a Go switch need not be. Adding the arm silently would
    // invent a branch the source does not have, so the absence is a refusal — with the fix named.
    if !wildcard_seen {
        return Err(TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: "switch has no `default`, and the target's match must be exhaustive — adding \
                     the missing arm here would invent a branch the source does not have"
                .to_owned(),
        });
    }

    Ok(RustExpr::Match {
        scrutinee: Box::new(expression(one_child(tag, cx, "tag")?, cx)?),
        arms,
    })
}
