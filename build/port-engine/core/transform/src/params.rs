//! Parameters and results.
//!
//! Both are POSITIONS, which is what makes them one module: a type's target form depends on where
//! it appears, and a trait in a parameter is a different decision from a trait in a result. The
//! failure convention is the same observation one level up — a trailing result of the failure type
//! is not a result at all, it is the shape of the whole return.

use port_engine_api::Declaration;
use port_engine_rust_ir::{RustParam, RustType};

use crate::error::TransformError;
use crate::naming::to_snake_case;
use crate::ownership::{binds_by_pointer, parameter_target};
use crate::resolve::Resolver;
use crate::vocabulary::{
    CHILD_PARAM, CHILD_RESULT, FLAG_VARIADIC, POSITION_PARAM, POSITION_RESULT,
};

pub(crate) fn refuse_variadic(declaration: &Declaration) -> Result<(), TransformError> {
    if declaration.flags.contains(FLAG_VARIADIC) {
        return Err(TransformError::Unsupported {
            name: declaration.name.clone(),
            detail: "variadic signature: the target has no variadic parameter, so this needs a \
                     rule that chooses a slice or a builder rather than a default"
                .to_owned(),
        });
    }
    Ok(())
}

pub(crate) fn params(
    declaration: &Declaration,
    resolver: &Resolver<'_>,
    owner: &str,
) -> Result<Vec<RustParam>, TransformError> {
    declaration
        .children_of_kind(CHILD_PARAM)
        .into_iter()
        .enumerate()
        .map(|(index, param)| {
            // A POINTER parameter is an ownership question and gets a decision; anything else is
            // just a type. The split is deliberate: a pointer inside a field or a result has no
            // call site to borrow across, so it stays a plain type-map answer.
            let ty = if param.type_ref.kind == "pointer" {
                let pointee =
                    param
                        .type_ref
                        .args
                        .first()
                        .ok_or_else(|| TransformError::Ownership {
                            detail: format!(
                                "pointer parameter `{}` of `{owner}::{}` has no pointee",
                                param.name, declaration.name
                            ),
                        })?;
                let resolved = resolver.resolve(pointee, &declaration.name)?;
                let site = format!("{owner}::{}({})", declaration.name, param.name);
                RustType::path(parameter_target(
                    param,
                    &resolved.spelling(),
                    &site,
                    resolver.ownership,
                )?)
            } else {
                resolver.resolve_in(&param.type_ref, &declaration.name, POSITION_PARAM)?
            };
            // An unnamed parameter is legal in the source and illegal in the target.
            let name = if param.name.is_empty() || param.name == "_" {
                format!("arg{index}")
            } else {
                to_snake_case(&param.name)
            };
            Ok(RustParam { name, ty })
        })
        .collect()
}

pub(crate) fn results(
    declaration: &Declaration,
    resolver: &Resolver<'_>,
) -> Result<Option<RustType>, TransformError> {
    let mut results = declaration.children_of_kind(CHILD_RESULT);

    // A trailing result of the FAILURE type is the source's whole signal that a function can fail,
    // and the target says it in the return type instead. Splitting it off here — rather than
    // resolving it as an ordinary result — is what turns a value the caller may drop into one the
    // caller must handle.
    let fallible = crate::failure::is_fallible(declaration, resolver.failure);
    if fallible {
        results.pop();
    }

    let mut types = Vec::with_capacity(results.len());
    for result in results {
        // A failure type anywhere but last is not the convention. It is a legitimate program and it
        // has no `Result` shape, so it refuses rather than being reordered into one.
        if crate::failure::is_failure_type(&result.type_ref, resolver.failure) {
            return Err(TransformError::Unsupported {
                name: declaration.name.clone(),
                detail: "a failure value in a non-trailing result is not the source's convention, \
                         and the target's fallible return carries exactly one"
                    .to_owned(),
            });
        }
        types.push(resolver.resolve_in(&result.type_ref, &declaration.name, POSITION_RESULT)?);
    }

    let value = match types.len() {
        0 => None,
        // Several results become a tuple. That is the target's own shape for "more than one value
        // out", and it keeps arity and order visible instead of inventing a struct nobody declared.
        1 => types.pop(),
        _ => Some(RustType::Tuple(types)),
    };
    if !fallible {
        return Ok(value);
    }
    let ok = value.unwrap_or_else(|| RustType::path("()"));
    let error = resolver.failure_target(&declaration.name)?;
    Ok(Some(RustType::path(format!(
        "Result<{}, {}>",
        ok.spelling(),
        error
    ))))
}
