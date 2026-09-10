//! Retention of declared data classes across a change.

/// Classes that only restate the default, and so may be deleted freely.
const DEFAULT_CLASSES: [&str; 2] = ["INTERNAL_ONLY", "PUBLIC"];

/// A declared class: the line it sits on, the class, and whether its carrier
/// is a comment-only line rather than a declaration's trailing comment.
type Declared<'a> = (usize, &'a str, bool);

/// Refuse deleting a non-default `data_class:` annotation from a changed file.
///
/// Classes are compared as a multiset per file, not by diff position: rustfmt
/// re-pads a trailing comment column when a neighbouring line changes width,
/// so an annotation moves without anything being lost. The multiset is keyed
/// by carrier as well as class, because a comment-only mention that survives
/// is not the judgment that a declaration carried.
///
/// A file with no head bytes is not judged. Whether a `.rs` file may be
/// deleted, or moved by a rename this caller could not resolve, is a question
/// for the gate that reads paths; answering it here would refuse every
/// annotation such a file ever carried.
pub fn deleted_classification_violations(path: &str, before: &[u8], after: &[u8]) -> Vec<String> {
    if !path.ends_with(".rs") || after.is_empty() {
        return Vec::new();
    }
    let (before, after) = (
        String::from_utf8_lossy(before),
        String::from_utf8_lossy(after),
    );
    let mut surviving: Vec<(&str, bool)> = declared_classes(&after)
        .into_iter()
        .map(|(_, class, comment_only)| (class, comment_only))
        .collect();
    declared_classes(&before)
        .into_iter()
        .filter(|(_, class, comment_only)| {
            match surviving
                .iter()
                .position(|kept| *kept == (*class, *comment_only))
            {
                Some(index) => {
                    surviving.swap_remove(index);
                    false
                }
                None => true,
            }
        })
        .map(|(line, class, _)| {
            format!(
                "{path}:{line}: `// data_class: {class}` was deleted; only {} restate the \
                 default and may go",
                DEFAULT_CLASSES.join(" and ")
            )
        })
        .collect()
}

/// Every class a comment on this line declares. One annotation may name
/// several, separated by `,` or `+`; the list ends at the first token that is
/// not SCREAMING_SNAKE, which is where trailing prose begins. Requiring `//`
/// ahead of the marker is what separates an annotation from a field named
/// `data_class`.
fn declared_classes_on(line: &str) -> Vec<&str> {
    let Some((head, mut rest)) = line.split_once("data_class:") else {
        return Vec::new();
    };
    if !head.contains("//") {
        return Vec::new();
    }
    let mut classes = Vec::new();
    loop {
        let token = rest.trim_start();
        let end = token
            .find(|byte: char| !byte.is_ascii_uppercase() && !byte.is_ascii_digit() && byte != '_')
            .unwrap_or(token.len());
        if !token[..end].starts_with(|byte: char| byte.is_ascii_uppercase()) {
            return classes;
        }
        classes.push(&token[..end]);
        rest = token[end..].trim_start();
        match rest.strip_prefix([',', '+']) {
            Some(tail) => rest = tail,
            None => return classes,
        }
    }
}

/// Every non-default class a file declares.
fn declared_classes(text: &str) -> Vec<Declared<'_>> {
    let lines: Vec<&str> = text.lines().collect();
    (0..lines.len())
        .flat_map(|index| {
            let comment_only = lines[index].trim_start().starts_with("//");
            declared_classes_on(lines[index])
                .into_iter()
                .filter(|class| !DEFAULT_CLASSES.contains(class))
                .filter(|class| {
                    !(comment_only && restates_the_declaration_below(&lines, index, class))
                })
                .map(|class| (index + 1, class, comment_only))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether a comment-only line repeats a class of the declaration it
/// introduces, which is one judgment written twice.
fn restates_the_declaration_below(lines: &[&str], index: usize, class: &str) -> bool {
    lines[index + 1..]
        .iter()
        .find(|line| {
            let text = line.trim();
            !text.is_empty() && !text.starts_with("//") && !text.starts_with("#[")
        })
        .is_some_and(|line| declared_classes_on(line).contains(&class))
}

#[cfg(test)]
#[path = "classification_tests.rs"]
mod tests;
