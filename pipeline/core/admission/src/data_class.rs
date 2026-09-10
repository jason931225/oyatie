//! Where a data classification is allowed to live, and where one is required.
//!
//! Provenance: ADR-0709 carries ADR-0006 forward verbatim — every entity
//! carries a `data_class` per declared property. The vocabulary for that is
//! owned by `data/core/data-boundary-kernel`; these two touched-file rules
//! keep it there and keep it answerable.

mod known;

use known::{CANONICAL_CRATE, GRANDFATHERED, GRANDFATHERED_HOLES};

/// Refuse a NEW `DataClass`-shaped enum declared outside the canonical crate.
///
/// Every parallel definition is a chance for a centrally added variant to go
/// missing locally. A narrower set is legitimate, but it has to be expressed
/// in terms of the canonical vocabulary rather than retyped beside it, so the
/// next parallel definition is refused here.
pub fn data_class_home_violations(path: &str, contents: &[u8]) -> Vec<String> {
    if !path.ends_with(".rs") || path.starts_with(CANONICAL_CRATE) {
        return Vec::new();
    }
    declared_enums(&String::from_utf8_lossy(contents))
        .into_iter()
        .filter(|(_, name)| !GRANDFATHERED.contains(&(path, name.as_str())))
        .map(|(line, name)| {
            format!(
                "{path}:{line}: `enum {name}` declares a data-class vocabulary \
                 outside {CANONICAL_CRATE}; re-export the canonical `DataClass` \
                 or express the narrower set in terms of it"
            )
        })
        .collect()
}

/// Enum declarations whose name carries the data-class vocabulary, as
/// `(line, name)`. What separates a declaration from prose naming one is that
/// `enum ` opens the line once visibility is stripped: a comment, doc line or
/// sentence keeps its marker or its words in front of `enum` and so never
/// opens it. The opening brace is deliberately NOT required — demanding it
/// would miss a declaration whose brace wrapped to the next line, and missing
/// one is the failure that matters here.
fn declared_enums(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let head = strip_visibility(line.trim_start());
            let rest = head.strip_prefix("enum ")?.trim();
            let name = rest.split(['<', ' ', '{']).next().unwrap_or_default();
            (name.contains("DataClass") && is_identifier(name))
                .then(|| (index + 1, name.to_owned()))
        })
        .collect()
}

fn strip_visibility(head: &str) -> &str {
    let Some(rest) = head.strip_prefix("pub") else {
        return head;
    };
    match rest.strip_prefix('(') {
        Some(scoped) => scoped.split_once(')').map_or(head, |(_, tail)| tail),
        None => rest,
    }
    .trim_start()
}

fn is_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(|byte: char| byte.is_ascii_digit())
        && name
            .chars()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == '_')
}

/// The classification-carrying types. A field of one of these states its own
/// class; a field of a domain type delegates to that type's own declaration.
const CLASSIFIED_TYPES: [&str; 3] = ["Classified<", "PrivacyDataClass", "DataClassification"];

/// Refuse an unclassified primitive field inside a struct that already
/// classifies at least one of its fields.
///
/// This is what makes "where does PII live" a query rather than a grep: the
/// answer is only complete if no field in a classified struct is silent.
/// A primitive holds data directly and so has nowhere to delegate a class to,
/// unlike a domain-typed field. Either form of the declaration satisfies the
/// rule — the carrier type or the `data_class:` annotation — because the
/// annotations remain the only record until the type carries them.
pub fn unclassified_field_violations(path: &str, contents: &[u8]) -> Vec<String> {
    if !path.ends_with(".rs") {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(contents);
    let lines: Vec<&str> = text.lines().collect();
    let mut violations = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let Some(name) = struct_head(lines[index]) else {
            index += 1;
            continue;
        };
        let end = (index + 1..lines.len())
            .find(|line| lines[*line].trim_start().starts_with('}'))
            .unwrap_or(lines.len());
        report_holes(
            &mut violations,
            path,
            name,
            &lines[index + 1..end],
            index + 1,
        );
        index = end + 1;
    }
    violations
}

fn report_holes(
    violations: &mut Vec<String>,
    path: &str,
    owner: &str,
    body: &[&str],
    offset: usize,
) {
    let fields: Vec<(usize, &str, &str)> = body
        .iter()
        .enumerate()
        .filter_map(|(index, line)| field(line).map(|(name, ty)| (offset + index + 1, name, ty)))
        .collect();
    if !fields.iter().any(|(_, _, ty)| classified(ty)) {
        return;
    }
    violations.extend(
        fields
            .iter()
            .filter(|(line, name, ty)| {
                is_primitive(ty)
                    && !classified(ty)
                    && !annotated(body, *line - offset - 1)
                    && !GRANDFATHERED_HOLES.contains(&(path, format!("{owner}.{name}").as_str()))
            })
            .map(|(line, name, ty)| {
                format!(
                    "{path}:{line}: `{owner}.{name}: {ty}` carries no data class in a struct \
                     that classifies its other fields; give it a `Classified<{ty}>` or a \
                     `// data_class:` declaration"
                )
            }),
    );
}

/// A declaration is accepted trailing the field, or on a comment line above
/// it. Deleting a doc comment re-pads the trailing-comment column under rustfmt,
/// so an annotation legitimately moves between the two forms without anything
/// being lost; a rule that only saw one of them would call that a deletion.
fn annotated(body: &[&str], index: usize) -> bool {
    let declares = |line: &&&str| {
        line.split_once("//")
            .is_some_and(|(_, comment)| comment.contains("data_class:"))
    };
    body.get(index).filter(declares).is_some()
        || index
            .checked_sub(1)
            .and_then(|above| body.get(above))
            .filter(|line| line.trim_start().starts_with("//"))
            .filter(declares)
            .is_some()
}

fn struct_head(line: &str) -> Option<&str> {
    let head = strip_visibility(line.trim_start())
        .strip_prefix("struct ")?
        .trim();
    let name = head.split(['<', ' ', '{']).next()?;
    is_identifier(name).then_some(name)
}

/// A named field and its declared type, or `None` for anything else in a
/// struct body — a doc line, an attribute, a nested brace.
fn field(line: &str) -> Option<(&str, &str)> {
    let body = strip_visibility(line.trim_start());
    let (name, rest) = body.split_once(':')?;
    let name = name.trim_end();
    if !is_identifier(name) || name.starts_with(char::is_uppercase) {
        return None;
    }
    let ty = rest
        .split("//")
        .next()
        .unwrap_or_default()
        .trim()
        .strip_suffix(',')?
        .trim_end();
    (!ty.is_empty()).then_some((name, ty))
}

fn classified(ty: &str) -> bool {
    CLASSIFIED_TYPES.iter().any(|marker| ty.contains(marker))
}

fn is_primitive(ty: &str) -> bool {
    matches!(
        ty,
        "String"
            | "bool"
            | "char"
            | "f32"
            | "f64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
    )
}
