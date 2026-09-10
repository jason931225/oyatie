//! A port defined by a change carries its implementation in the same change.

use std::collections::BTreeSet;

use crate::line_budget::{CommentScanner, LineKind};

const STUB_PREFIX: &str = "NotImplemented";

/// One changed Rust source: the path, its bytes at the head commit (empty
/// when the change deletes it), and its bytes at the base commit when the path
/// already held content there.
pub struct ChangedSource<'a> {
    pub path: &'a str,
    pub head: &'a [u8],
    pub base: Option<&'a [u8]>,
}

/// Refuse a trait the change introduces when nothing in the same change
/// implements it for a type that is not a `NotImplemented` stub. A trait any
/// changed path already defined at the base is inherited, not introduced, so
/// neither an edit beside an existing port nor a move of one between files is
/// charged. Implementations are gathered across the whole
/// changed set, so a port and the adapter in another crate still admit each
/// other, and a `cfg(test)` fake counts because a fake means a caller exists.
pub fn port_implementation_violations(changed: &[ChangedSource<'_>]) -> Vec<String> {
    let implemented: BTreeSet<String> = changed
        .iter()
        .filter(|source| source.path.ends_with(".rs"))
        .flat_map(|source| implemented_traits(source.head))
        .collect();
    let inherited: BTreeSet<String> = changed
        .iter()
        .filter(|source| source.path.ends_with(".rs"))
        .filter_map(|source| source.base)
        .flat_map(defined_traits)
        .map(|definition| definition.name)
        .collect();
    let mut violations = Vec::new();
    for source in changed.iter().filter(|s| s.path.ends_with(".rs")) {
        for definition in defined_traits(source.head) {
            if inherited.contains(&definition.name) || implemented.contains(&definition.name) {
                continue;
            }
            let (path, line, name) = (source.path, definition.line, &definition.name);
            violations.push(format!(
                "{path}:{line}: port `{name}` is defined without an \
                 implementation in this change; land the implementation that \
                 serves a caller beside the port, or leave the port unwritten"
            ));
        }
    }
    violations
}

struct Definition {
    line: usize,
    name: String,
}

/// Trait definitions in `contents`, skipping any the comment scanner reports,
/// so a port named inside prose is not a definition.
fn defined_traits(contents: &[u8]) -> Vec<Definition> {
    let text = String::from_utf8_lossy(contents);
    let mut scanner = CommentScanner::default();
    let mut found = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if !matches!(scanner.classify(line), LineKind::Code) {
            continue;
        }
        if let Some(name) = trait_name(line) {
            found.push(Definition {
                line: index + 1,
                name,
            });
        }
    }
    found
}

/// The trait a `trait` item declares, once visibility and the `unsafe` and
/// `auto` modifiers are stripped from the head of the line.
fn trait_name(line: &str) -> Option<String> {
    let mut rest = line;
    loop {
        let trimmed = rest.trim_start();
        let next = trimmed
            .strip_prefix("pub")
            .map(|tail| tail.strip_prefix('(').map_or(tail, close_visibility))
            .or_else(|| trimmed.strip_prefix("unsafe"))
            .or_else(|| trimmed.strip_prefix("auto"));
        match next {
            Some(tail) if tail.starts_with([' ', '(']) || tail.is_empty() => rest = tail,
            _ => break,
        }
    }
    let tail = rest.trim_start().strip_prefix("trait ")?;
    identifier(tail.trim_start())
}

fn close_visibility(scope: &str) -> &str {
    scope.find(')').map_or(scope, |end| &scope[end + 1..])
}

/// Traits implemented in `contents` by something other than a stub. A header
/// rustfmt wraps across lines is joined up to its body or `;` before it is
/// read, so a `for`, a bound or a `where` clause on a continuation line counts.
fn implemented_traits(contents: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(contents);
    let mut scanner = CommentScanner::default();
    let mut found = Vec::new();
    let mut open: Option<String> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if !matches!(scanner.classify(line), LineKind::Code) {
            continue;
        }
        let header = match open.take() {
            Some(_) if starts_impl(line) => line.to_owned(),
            Some(head) => format!("{head} {line}"),
            None if starts_impl(line) => line.to_owned(),
            None => continue,
        };
        if header.contains('{') || header.ends_with(';') {
            found.extend(implemented_trait(&header));
        } else {
            open = Some(header);
        }
    }
    found
}

/// An `impl` item begins here, including the bare `impl` rustfmt leaves when
/// it wraps a long generic header.
fn starts_impl(line: &str) -> bool {
    let rest = line.strip_prefix("unsafe ").unwrap_or(line);
    rest.strip_prefix("impl")
        .is_some_and(|tail| tail.is_empty() || tail.starts_with([' ', '<']))
}

/// The trait an `impl <Trait> for <Type>` header implements. `None` when the
/// header is inherent, names a standard-library trait by path, has a
/// `NotImplemented` stub anywhere in its target type, or only forwards to
/// types that must already implement the same trait.
fn implemented_trait(header: &str) -> Option<String> {
    let rest = header
        .strip_prefix("unsafe ")
        .unwrap_or(header)
        .strip_prefix("impl")?;
    let (generics, rest) = match rest.trim_start().strip_prefix('<') {
        Some(parameters) => split_generics(parameters),
        None => ("", rest),
    };
    let (signature, bounds) = rest.split_once(" where ").unwrap_or((rest, ""));
    let (trait_path, target) = signature.split_once(" for ")?;
    let trait_path = trait_path.trim().trim_start_matches("::");
    if ["std::", "core::", "alloc::"]
        .iter()
        .any(|root| trait_path.starts_with(root))
    {
        return None;
    }
    let name = last_segment(trait_path)?;
    let target = target.split('{').next().unwrap_or_default();
    let stub = words(target).any(|word| word.starts_with(STUB_PREFIX));
    let forwards = unqualified_words(generics)
        .into_iter()
        .chain(unqualified_words(bounds))
        .chain(unqualified_words(target))
        .any(|word| word == name);
    (!stub && !forwards).then_some(name)
}

/// Split a generic parameter list at its balancing `>`, so a lifetime or a
/// bound holding `for` cannot be read as the separator.
fn split_generics(parameters: &str) -> (&str, &str) {
    let bytes = parameters.as_bytes();
    let mut depth = 1usize;
    for (index, byte) in parameters.bytes().enumerate() {
        match byte {
            b'<' => depth += 1,
            b'>' if index == 0 || bytes[index - 1] != b'-' => {
                depth -= 1;
                if depth == 0 {
                    return (&parameters[..index], &parameters[index + 1..]);
                }
            }
            _ => {}
        }
    }
    (parameters, "")
}

/// Identifier words in `text` that no `::` qualifies, so `std::io::Write`
/// contributes `std` and not `Write`. A bound naming another crate's trait
/// therefore cannot be read as a mention of the trait being implemented.
fn unqualified_words(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        match (character.is_ascii_alphanumeric() || character == '_', start) {
            (true, None) => start = Some(index),
            (false, Some(begin)) => {
                push_unqualified(text, begin, index, &mut found);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(begin) = start {
        push_unqualified(text, begin, text.len(), &mut found);
    }
    found
}

fn push_unqualified<'text>(
    text: &'text str,
    begin: usize,
    end: usize,
    found: &mut Vec<&'text str>,
) {
    if !text[..begin].ends_with("::") {
        found.push(&text[begin..end]);
    }
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|word| !word.is_empty())
}

/// The bare name of a path, with generic arguments and module qualification
/// removed, so `crate::port::Store<T>` and `Store` are one name.
fn last_segment(path: &str) -> Option<String> {
    let bare = path.trim().split('<').next()?.trim();
    identifier(bare.rsplit("::").next()?.trim_start())
}

fn identifier(text: &str) -> Option<String> {
    let name: String = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect();
    let starts_alpha = name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
    (starts_alpha && !name.is_empty()).then_some(name)
}

#[cfg(test)]
#[path = "port_implementation_tests.rs"]
mod tests;
