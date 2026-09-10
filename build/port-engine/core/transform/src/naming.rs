//! Target-language identifiers: sanitising, region ids, and casing.
//!
//! This is the one place the target language's conventions are owned. That is legitimate — this
//! face RENDERS Rust, so Rust's casing is its business; the source language it must stay ignorant
//! of.

use port_engine_api::{Declaration, RuleId, UnitId};
use port_engine_rust_ir::Visibility;

use crate::vocabulary::FLAG_EXPORTED;

/// Sanitize a unit or rule id into a Rust-safe region / fn name segment.
#[must_use]
pub fn sanitize_ident(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("unit");
    }
    if out.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Region id for one unit-level plan step: `{sanitized_unit}__{sanitized_rule}`.
#[must_use]
pub fn region_id_for(unit: &UnitId, rule: &RuleId) -> String {
    format!("{}__{}", sanitize_ident(&unit.0), sanitize_ident(&rule.0))
}

/// Region id for one declaration-level plan step, extended with the declaration.
///
/// The declaration segment is what keeps two rules capturing different kinds from colliding on one
/// region — and one region silently overwriting another is exactly the loss the kernel's
/// duplicate-region refusal exists to catch, arriving one layer earlier.
#[must_use]
pub fn region_id_for_declaration(unit: &UnitId, rule: &RuleId, declaration: &str) -> String {
    format!(
        "{}__{}__{}",
        sanitize_ident(&unit.0),
        sanitize_ident(&rule.0),
        sanitize_ident(declaration)
    )
}

/// `MaxRetries` → `MAX_RETRIES`. Rust's constant convention.
#[must_use]
pub fn to_screaming_snake(raw: &str) -> String {
    to_snake_case(raw).to_ascii_uppercase()
}

/// `MaxRetries` → `max_retries`. Rust's function and binding convention.
///
/// Runs of capitals are kept together, so `ParseURL` becomes `parse_url` and not `parse_u_r_l`.
#[must_use]
pub fn to_snake_case(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len() + 4);
    for (index, ch) in chars.iter().enumerate() {
        if !ch.is_ascii_alphanumeric() {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }
            continue;
        }
        if ch.is_ascii_uppercase() && index > 0 {
            let previous_is_lower =
                chars[index - 1].is_ascii_lowercase() || chars[index - 1].is_ascii_digit();
            let next_is_lower = chars
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_lowercase());
            if (previous_is_lower || next_is_lower) && !out.ends_with('_') {
                out.push('_');
            }
        }
        out.push(ch.to_ascii_lowercase());
    }
    if out.is_empty() {
        out.push_str("item");
    }
    if out.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        out.insert(0, '_');
    }
    escape_keyword(&out)
}

/// `point` → `Point`. Rust's type convention; already-capitalized names pass through.
#[must_use]
pub fn to_pascal_case(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut capitalize_next = true;
    for ch in raw.chars() {
        if !ch.is_ascii_alphanumeric() {
            capitalize_next = true;
            continue;
        }
        if capitalize_next {
            out.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            out.push(ch);
        }
    }
    if out.is_empty() {
        out.push_str("Item");
    }
    if out.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        out.insert(0, '_');
    }
    out
}

/// A unit id reduced to one target module identifier.
///
/// Lives HERE, in the face that owns target naming, because two callers need it and they must
/// agree: the assembler groups regions into modules by it, and the resolver addresses another
/// unit's types through it. Deriving it twice is how a cross-unit reference ends up pointing at a
/// module nobody emitted.
///
/// The last path segment is the package's own name; the leading segments say where it lives.
/// Using the whole path produces module names nobody can read; using the tail alone collides
/// across two packages of the same name — and that collision becomes a rustc duplicate-definition
/// error rather than a silent overwrite, which is why the readable form is worth its risk today.
#[must_use]
pub fn module_name(unit: &str) -> String {
    let tail = unit.rsplit('/').next().unwrap_or(unit);
    let name = sanitize_ident(tail);
    if name.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        format!("_{name}")
    } else {
        name
    }
}

/// Target keywords a source identifier may collide with.
///
/// Every one of these is a legal Go identifier, so a translator that cannot emit them cannot
/// translate Go. Strict and reserved keywords both: a reserved one is not a keyword yet, and
/// emitting it would make the output depend on which edition compiles it.
const TARGET_KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "static",
    "struct", "trait", "true", "try", "type", "typeof", "union", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];

/// Keywords that cannot be raw identifiers, so a collision is resolved by RENAMING — a real change
/// to the identifier, which is why they are listed apart from the escapable ones above.
const UNRAWABLE_KEYWORDS: [&str; 4] = ["crate", "self", "Self", "super"];

/// Make an identifier emittable, escaping a target keyword rather than refusing it.
#[must_use]
pub fn escape_keyword(name: &str) -> String {
    if UNRAWABLE_KEYWORDS.contains(&name) {
        return format!("{name}_");
    }
    if TARGET_KEYWORDS.contains(&name) {
        return format!("r#{name}");
    }
    name.to_owned()
}

/// The absolute module path another unit's items are addressed by.
///
/// Absolute, because the emitted unit modules are SIBLINGS: a relative `shapes::Point` written
/// inside `geometry` resolves to nothing, since there is no `shapes` in `geometry`'s scope.
///
/// The `crate::` prefix is a claim about the emitted LAYOUT — that unit modules sit at the crate
/// root — and the assembler in the facade is what makes that claim true. If the layout ever nests
/// them, this function and that assembler move together or cross-unit references break.
#[must_use]
pub fn module_path(unit: &str) -> String {
    format!("crate::{}", module_name(unit))
}

/// Target visibility for a declaration.
///
/// A VALUE, not a `"pub "` string prefix. The prefix form is what let `pub` be concatenated into a
/// trait body, where it is not legal Rust; the IR decides where a visibility may appear at all.
pub(crate) fn visibility(declaration: &Declaration) -> Visibility {
    if declaration.flags.contains(FLAG_EXPORTED) {
        Visibility::Public
    } else {
        Visibility::Inherited
    }
}
