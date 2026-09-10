//! Stable admission preimages.
//!
//! Decimal length prefixes with a `:` make each field unambiguous; explicit child arity makes the
//! tree unambiguous. That is why the digest does not depend on JSON canonicalization — and why the
//! same preimage can be computed in Go by the extractor and in Rust here, with any drift between
//! the two surfacing as a digest mismatch at admission rather than as a silently accepted
//! snapshot.

use port_engine_api::{Declaration, TypeRef};

/// Stable admission preimage: length-prefixed language, then each length-prefixed unit and
/// producer in model order.
#[must_use]
pub fn snapshot_preimage(language: &str, units_and_producers: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    push_field(&mut out, language);
    for (unit, producer) in units_and_producers {
        push_field(&mut out, unit);
        push_field(&mut out, producer);
    }
    out
}

fn push_field(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(b':');
    out.extend_from_slice(value.as_bytes());
}

/// Stable admission preimage for a v1 artifact, which carries declarations.
///
/// The encoding is the same shape as v0's — decimal length prefixes with a `:` — extended with an
/// explicit child arity per node:
///
/// ```text
/// F(kind) F(name) F(type_ref) F(len(flags)) flags...
///     F(len(attrs)) (F(key) F(value))... F(len(children)) children...
/// ```
///
/// Length prefixes make each field unambiguous; the arity counts make the tree unambiguous. This
/// is mirrored byte-for-byte by the Go extractor's `encodeNode`. That duplication is deliberate:
/// the alternative is trusting the digest the extractor claims, which would let a front-end defect
/// enter the engine carrying a self-consistent receipt. Drift between the two implementations
/// surfaces here as [`crate::AdmitError::DigestMismatch`].
#[must_use]
pub fn snapshot_preimage_v1(
    language: &str,
    packages: &[(&str, &str, Vec<Declaration>)],
) -> Vec<u8> {
    let mut out = Vec::new();
    push_field(&mut out, "snapshot");
    push_field(&mut out, language);
    push_field(&mut out, &packages.len().to_string());
    for (unit, producer, declarations) in packages {
        push_field(&mut out, "package");
        push_field(&mut out, unit);
        push_field(&mut out, producer);
        push_field(&mut out, &declarations.len().to_string());
        for declaration in declarations {
            push_declaration(&mut out, declaration);
        }
    }
    out
}

/// The type TREE, mirrored by the extractor's `encodeType`.
///
/// A present/absent marker leads, so a declaration with no type and one whose type is an empty
/// node cannot encode identically.
fn push_type(out: &mut Vec<u8>, type_ref: &TypeRef) {
    if type_ref.is_empty() {
        push_field(out, "0");
        return;
    }
    push_field(out, "1");
    push_field(out, &type_ref.kind);
    push_field(out, &type_ref.name);
    push_field(out, &type_ref.package);
    push_field(out, &type_ref.args.len().to_string());
    for arg in &type_ref.args {
        push_type(out, arg);
    }
}

fn push_declaration(out: &mut Vec<u8>, declaration: &Declaration) {
    push_field(out, &declaration.kind);
    push_field(out, &declaration.name);
    push_type(out, &declaration.type_ref);
    // `flags` is a BTreeSet and `attrs` a BTreeMap, so both iterate sorted — the same order the
    // extractor sorts into. A set with two orderings would be a set with two digests.
    push_field(out, &declaration.flags.len().to_string());
    for flag in &declaration.flags {
        push_field(out, flag);
    }
    push_field(out, &declaration.attrs.len().to_string());
    for (key, value) in &declaration.attrs {
        push_field(out, key);
        push_field(out, value);
    }
    push_field(out, &declaration.children.len().to_string());
    for child in &declaration.children {
        push_declaration(out, child);
    }
}
