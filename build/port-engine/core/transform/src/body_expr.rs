//! Expressions.

use port_engine_api::{Declaration, TypeRef};
use port_engine_rust_ir::RustExpr;

use crate::body::{Body, one_child, two_children, unsupported_source};
use crate::body_index::slice;
use crate::body_ops::{binary_operator, is_receiver, operator_of, reference, unary_operator};
use crate::error::TransformError;
use crate::naming::to_snake_case;
use crate::vocabulary::{ATTR_CALLEE, ATTR_CALLEE_KIND, ATTR_VALUE, CALLEE_KIND_METHOD};

/// Where an expression appears: a value is READ, a place is WRITTEN TO.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Position {
    Value,
    Place,
}

pub(crate) fn expression(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    in_position(node, cx, Position::Value)
}

pub(crate) fn in_position(
    node: &Declaration,
    cx: &Body<'_>,
    position: Position,
) -> Result<RustExpr, TransformError> {
    match node.kind.as_str() {
        // A literal passes through as SOURCE TEXT: a lexical form the target does not
        // share fails the emitted tree's parse rather than being normalised here.
        "literal" => node
            .attr(ATTR_VALUE)
            .map(|value| RustExpr::Literal(value.to_owned()))
            .ok_or_else(|| TransformError::MissingDatum {
                construction: "literal".to_owned(),
                name: cx.owner.to_owned(),
                datum: ATTR_VALUE,
            }),
        "zero" => zero_value(node, cx),
        "ident" if is_receiver(node) => Ok(RustExpr::SelfValue),
        "ident" => Ok(RustExpr::Path(reference(node))),
        // Parentheses are dropped: the IR recomputes precedence, so re-emitting fights it.
        "paren" => expression(one_child(node, cx, "paren")?, cx),
        "binary" => binary(node, cx),
        "unary" => {
            let spelling = operator_of(node, cx)?;
            let op = unary_operator(spelling).ok_or_else(|| TransformError::Unsupported {
                name: cx.owner.to_owned(),
                detail: format!("unary operator `{spelling}` has no direct translation"),
            })?;
            Ok(RustExpr::Unary {
                op,
                operand: Box::new(expression(one_child(node, cx, "unary")?, cx)?),
            })
        }
        "selector" => selector(node, cx, position),
        "call" => call(node, cx),
        "index" => {
            let (base, index) = two_children(node, cx, "index")?;
            Ok(RustExpr::Index {
                base: Box::new(expression(base, cx)?),
                index: Box::new(crate::body_index::index_operand(index, cx)?),
            })
        }
        "composite" => composite(node, cx),
        "convert" => convert(node, cx),
        "slice" => slice(node, cx),
        "unsupported" => Err(unsupported_source(node, cx)),
        other => Err(TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: format!("expression kind `{other}` has no translation"),
        }),
    }
}

fn binary(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    let spelling = operator_of(node, cx)?;
    let op = binary_operator(spelling).ok_or_else(|| TransformError::Unsupported {
        name: cx.owner.to_owned(),
        detail: format!("binary operator `{spelling}` has no direct translation"),
    })?;
    let (lhs, rhs) = two_children(node, cx, "binary")?;
    Ok(RustExpr::Binary {
        op,
        lhs: Box::new(expression(lhs, cx)?),
        rhs: Box::new(expression(rhs, cx)?),
    })
}

/// A field access, cloned when reading it would MOVE.
///
/// In PLACE position there is no read, so cloning there would assign to a temporary.
fn selector(
    node: &Declaration,
    cx: &Body<'_>,
    position: Position,
) -> Result<RustExpr, TransformError> {
    let field = RustExpr::Field {
        base: Box::new(expression(one_child(node, cx, "selector")?, cx)?),
        name: to_snake_case(&node.name),
    };
    if position == Position::Value && moves_on_read(&node.type_ref, cx) {
        return Ok(RustExpr::MethodCall {
            receiver: Box::new(field),
            method: "clone".to_owned(),
            args: Vec::new(),
        });
    }
    Ok(field)
}

/// Whether a plain read of this type moves in the target, and therefore needs a clone.
/// An ABSENT type does NOT move: absence means this is not a resolved field read at all.
fn moves_on_read(type_ref: &TypeRef, cx: &Body<'_>) -> bool {
    if type_ref.is_empty() {
        return false;
    }
    !cx.resolver.copies(type_ref)
}

/// A type CONVERSION, which the source spells exactly like a call.
fn convert(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    let operand = expression(one_child(node, cx, "convert")?, cx)?;
    let target = &node.type_ref;

    if target.kind == "named" {
        let path = cx.resolver.resolve(target, cx.owner)?;
        return Ok(RustExpr::Call {
            callee: Box::new(RustExpr::Path(path.spelling())),
            args: vec![operand],
        });
    }

    if target.kind == "basic" && cx.resolver.converts_by_cast(target) {
        let rendered = cx.resolver.resolve(target, cx.owner)?;
        return Ok(RustExpr::Cast {
            expr: Box::new(operand),
            ty: rendered,
        });
    }

    Err(TransformError::Unsupported {
        name: cx.owner.to_owned(),
        detail: format!(
            "converting to `{}` has no declared target form — the source's conversion is \
             infallible and the target's is not, so what happens to input the target rejects is a \
             decision the pack has to make rather than a spelling",
            target.describe()
        ),
    })
}

/// A struct literal, with every field named.
fn composite(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    let path = cx.resolver.resolve(&node.type_ref, cx.owner).map_err(|_| {
        TransformError::MissingDatum {
            construction: "composite".to_owned(),
            name: cx.owner.to_owned(),
            datum: "type",
        }
    })?;

    let keyed = node.children_of_kind("keyed");
    let mut fields = Vec::with_capacity(keyed.len());
    for entry in keyed {
        fields.push((
            to_snake_case(&entry.name),
            expression(one_child(entry, cx, "keyed")?, cx)?,
        ));
    }
    Ok(RustExpr::StructLiteral {
        path: path.spelling(),
        fields,
    })
}

/// The target's zero for a source type the literal left out.
///
/// Refuses BY NAME rather than `Default::default()`, whose impl need not be the zero.
fn zero_value(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    cx.resolver
        .zero_value(&node.type_ref)
        .map(RustExpr::Literal)
        .ok_or_else(|| TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: format!(
                "a struct literal omits field `{}` of type `{}`, and the pack declares no zero \
                 value for it — Go fills the field with that type's zero and the target must \
                 spell it out",
                node.name,
                node.type_ref.describe()
            ),
        })
}

/// A call, dispatched on the RECORDED callee kind rather than on the callee's syntax.
fn call(node: &Declaration, cx: &Body<'_>) -> Result<RustExpr, TransformError> {
    let callee = node
        .children
        .first()
        .ok_or_else(|| TransformError::MissingDatum {
            construction: "call".to_owned(),
            name: cx.owner.to_owned(),
            datum: "callee",
        })?;
    let args = node.children[1..]
        .iter()
        .map(|arg| expression(arg, cx))
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(rendered) = mapped_call(node, &args, cx)? {
        return Ok(rendered);
    }

    if node.attr(ATTR_CALLEE_KIND) == Some(CALLEE_KIND_METHOD) {
        return Ok(RustExpr::MethodCall {
            // The receiver is a PLACE: `x.m()` borrows, so a clone here calls on a temporary.
            receiver: Box::new(in_position(
                one_child(callee, cx, "selector")?,
                cx,
                Position::Place,
            )?),
            method: to_snake_case(&callee.name),
            args,
        });
    }

    let path = cx
        .resolver
        .function_path(node.attr(ATTR_CALLEE), cx.owner)?;
    Ok(RustExpr::Call {
        callee: Box::new(RustExpr::Path(path)),
        args,
    })
}

/// A call the pack answers for by the callee's IDENTITY, rendered from its template.
fn mapped_call(
    node: &Declaration,
    args: &[RustExpr],
    cx: &Body<'_>,
) -> Result<Option<RustExpr>, TransformError> {
    let Some(identity) = node.attr(ATTR_CALLEE) else {
        return Ok(None);
    };
    let Some(template) = cx.resolver.function_map.get(identity) else {
        return Ok(None);
    };

    let mut rendered = template.clone();
    for (index, arg) in args.iter().enumerate() {
        let operand = render_operand(arg).ok_or_else(|| TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: format!(
                "an argument to `{identity}` is a compound expression, and the pack answers for \
                 that call with a TEXT template — substituting one would need parentheses the \
                 template cannot ask for"
            ),
        })?;
        rendered = rendered.replace(&format!("{{{index}}}"), &operand);
    }
    if rendered.contains('{') {
        return Err(TransformError::Unsupported {
            name: cx.owner.to_owned(),
            detail: format!(
                "the pack's template for `{identity}` expects more arguments than the call has"
            ),
        });
    }
    Ok(Some(RustExpr::Literal(rendered)))
}

/// An argument, as target text for a template to interpolate.
fn render_operand(arg: &RustExpr) -> Option<String> {
    match arg {
        RustExpr::Literal(text) | RustExpr::Path(text) => Some(text.clone()),
        _ => None,
    }
}
