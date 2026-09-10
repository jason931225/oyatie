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

/// Traits implemented for a concrete type in `contents`. A stub target is not
/// an implementation: it is the absence of one, spelled out.
fn implemented_traits(contents: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(contents);
    let mut scanner = CommentScanner::default();
    let mut found = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if !matches!(scanner.classify(line), LineKind::Code) {
            continue;
        }
        if let Some((name, target)) = implemented_trait(line)
            && !target.starts_with(STUB_PREFIX)
        {
            found.push(name);
        }
    }
    found
}

/// The trait and target of an `impl <Trait> for <Type>` header. An inherent
/// `impl Type {` has no `for` and implements nothing.
fn implemented_trait(line: &str) -> Option<(String, String)> {
    let rest = line
        .strip_prefix("unsafe ")
        .unwrap_or(line)
        .strip_prefix("impl")?;
    let rest = rest.strip_prefix('<').map_or(rest, close_generics);
    let (head, tail) = rest.split_once(" for ")?;
    Some((last_segment(head)?, last_segment(tail)?))
}

/// Skip a balanced generic parameter list so a lifetime or bound holding
/// `for` cannot be read as the separator.
fn close_generics(parameters: &str) -> &str {
    let mut depth = 1usize;
    for (index, byte) in parameters.bytes().enumerate() {
        match byte {
            b'<' => depth += 1,
            b'>' => {
                depth -= 1;
                if depth == 0 {
                    return &parameters[index + 1..];
                }
            }
            _ => {}
        }
    }
    parameters
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
