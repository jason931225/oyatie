//! Parameters, results, and method signatures — as IR nodes.

use port_engine_api::Declaration;
use port_engine_rust_ir::{Receiver, RustFn, RustParam, RustType, Visibility};

use crate::error::TransformError;
use crate::naming::to_snake_case;
use crate::ownership::{binds_by_pointer, facts_of, parameter_target, receiver_for};
use crate::params::{params, refuse_variadic, results};
use crate::resolve::Resolver;
use crate::vocabulary::{
    ATTR_RECEIVER, CHILD_BODY, CHILD_METHOD, CHILD_PARAM, CHILD_RESULT, FLAG_VARIADIC,
    POSITION_PARAM, POSITION_RESULT,
};

/// Whether a method's body is translated, stubbed, or absent.
#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum Body {
    /// Emit `todo!()`.
    Stub,
    /// Translate the source body, refusing anything outside the supported subset.
    Translate,
    /// Emit no body at all — a trait item is a signature.
    None,
}

/// The methods a type carries in its inherent `impl` block.
///
/// Declared methods and PROMOTED ones, in one block, because the target draws no distinction
/// between them: a caller of the source cannot tell whether a method was declared on the type or
/// lifted from an embedded field, and the emit should not make them tell either.
pub(crate) fn inherent_methods(
    declaration: &Declaration,
    resolver: &Resolver<'_>,
    body: Body,
) -> Result<Vec<RustFn>, TransformError> {
    let mut methods = declaration
        .children_of_kind(CHILD_METHOD)
        .into_iter()
        .map(|method| {
            method_signature(
                method,
                resolver,
                Visibility::Public,
                body,
                &declaration.name,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    methods.extend(crate::promote::promoted_methods(declaration, resolver)?);
    Ok(methods)
}

/// Parse a pack-declared receiver form into the IR's receiver.
fn parse_receiver(form: &str, site: &str) -> Result<Receiver, TransformError> {
    match form {
        "&self" => Ok(Receiver::Shared),
        "&mut self" => Ok(Receiver::Exclusive),
        "self" => Ok(Receiver::Owned),
        other => Err(TransformError::Ownership {
            detail: format!("`{other}` is not a receiver form the target has, at `{site}`"),
        }),
    }
}

/// The methods a trait requires, as bodiless signatures.
///
/// Visibility is [`Visibility::Inherited`] and not a choice: a trait item is as public as its
/// trait and may not say so. The previous renderer concatenated the source's `pub ` prefix into
/// the trait body, which `syn` parses and `rustc` rejects — and which a golden over the parsed
/// output would have frozen in place.
///
/// The RECEIVER is a choice, and the pack makes it. It cannot be recovered from the source: an
/// interface says nothing about how an implementation binds its receiver, and the implementations
/// are not all in view. `&self` was being emitted for every method, which is why the fixture's
/// `Rename(next string)` — a mutator — became a signature no mutating implementation can satisfy.
pub(crate) fn trait_methods(
    declaration: &Declaration,
    resolver: &Resolver<'_>,
) -> Result<Vec<RustFn>, TransformError> {
    declaration
        .children_of_kind(CHILD_METHOD)
        .into_iter()
        .map(|method| {
            let mut rendered = method_signature(
                method,
                resolver,
                Visibility::Inherited,
                Body::None,
                &declaration.name,
            )?;
            rendered.receiver = Some(method_receiver(method, resolver, &declaration.name)?);
            Ok(rendered)
        })
        .collect()
}

/// The receiver one trait method binds.
///
/// The OBSERVED mode wins. A source interface says nothing about how an implementation binds its
/// receiver, so P1 made this a declared pack decision — one mode for every trait method, which put
/// `&mut self` on getters. With the implementors observed, the mode is derived per method:
/// exclusive exactly when some implementor mutates through it. The pack's decision is the fallback
/// and now covers only the interfaces nothing was seen to implement.
///
/// # Errors
/// [`TransformError::Unsupported`] when neither answers, because emitting a guessed receiver is
/// what this whole path exists to stop.
pub(crate) fn method_receiver(
    method: &Declaration,
    resolver: &Resolver<'_>,
    owner: &str,
) -> Result<Receiver, TransformError> {
    if let Some(observed) = method.attr(ATTR_RECEIVER) {
        return declared_receiver(observed, owner);
    }
    let (mode, _reason) = resolver
        .trait_receiver()
        .ok_or_else(|| TransformError::Unsupported {
            name: owner.to_owned(),
            detail: "no implementor of this interface was observed and the pack declares no \
                     trait-receiver mode, so the receiver would be a guess"
                .to_owned(),
        })?;
    declared_receiver(mode, owner)
}

/// Map the pack's declared mode onto the IR's receiver.
pub(crate) fn declared_receiver(mode: &str, owner: &str) -> Result<Receiver, TransformError> {
    match mode {
        "shared" => Ok(Receiver::Shared),
        "exclusive" => Ok(Receiver::Exclusive),
        "owned" => Ok(Receiver::Owned),
        other => Err(TransformError::Unsupported {
            name: owner.to_owned(),
            detail: format!("`{other}` is not a receiver mode the target has"),
        }),
    }
}

pub(crate) fn method_signature(
    method: &Declaration,
    resolver: &Resolver<'_>,
    vis: Visibility,
    body: Body,
    owner: &str,
) -> Result<RustFn, TransformError> {
    refuse_variadic(method)?;

    // A pointer receiver used to be refused outright, because `&self` drops the mutation it
    // permits and `&mut self` claims one the source may not perform, and nothing reported which.
    // The front end now reports it, so the guess became a decision — made by the pack over
    // observed facts, and recorded per site.
    //
    // A VALUE receiver is not an aliasing question — the source already copied, so there are no
    // aliases to reason about — but it is not `self` either. Go's value receiver COPIES and the
    // caller's value survives the call; Rust's `self` CONSUMES, so calling the method twice would
    // stop compiling. `&self` is the form with the source's permissions.
    //
    // A value receiver the body mutates is a different shape again: the mutation is on the copy
    // and needs a local binding, which is body-translation work rather than a receiver form. It
    // refuses instead of silently picking one of the two wrong answers.
    let receiver = if binds_by_pointer(method) {
        let site = format!("{owner}::{}", method.name);
        parse_receiver(&receiver_for(method, &site, resolver.ownership)?, &site)?
    } else if facts_of(method).mutated {
        return Err(TransformError::Unsupported {
            name: method.name.clone(),
            detail: "value receiver mutated in the body: the source mutates its own COPY, which \
                     needs a local binding rather than a receiver form — `self` would consume the \
                     caller's value and `&mut self` would claim a mutation the caller can see"
                .to_owned(),
        });
    } else {
        Receiver::Shared
    };

    Ok(RustFn {
        // A method's documentation is emitted, like every other declaration's. It was being
        // dropped here while the front end captured it — the same silent loss as the interface
        // methods, and invisible for the same reason: nothing looks for prose that is absent.
        docs: crate::docs::docs_of(method),
        vis,
        name: to_snake_case(&method.name),
        receiver: Some(receiver),
        params: params(method, resolver, owner)?,
        ret: results(method, resolver)?,
        body: match body {
            Body::Stub => Some(vec![port_engine_rust_ir::RustStmt::Tail(
                port_engine_rust_ir::RustExpr::Todo,
            )]),
            Body::Translate => {
                let source = method
                    .children_of_kind(CHILD_BODY)
                    .first()
                    .copied()
                    .ok_or_else(|| TransformError::MissingDatum {
                        construction: "rust_struct_body".to_owned(),
                        name: method.name.clone(),
                        datum: "body",
                    })?;
                Some(crate::body::statements(&source.children, method, resolver)?)
            }
            Body::None => None,
        },
    })
}
