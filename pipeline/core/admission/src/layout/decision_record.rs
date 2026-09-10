//! Frontmatter admission for a newly added live-apex decision record.
//!
//! Amending in place is the anti-sprawl move. Filing the amendment as a new
//! document is how one decision becomes a chain of files that contradict each
//! other, so a new record that cites a still-amendable one is refused and told
//! which file to edit.

use std::collections::BTreeMap;

/// Refuse a newly added decision record whose provenance names a decision file
/// the author could have amended instead. `amendable` maps decision id to the
/// live path that carries it.
pub fn new_decision_record_violations(
    path: &str,
    contents: &str,
    amendable: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut violations = Vec::new();
    for field in ["supersedes", "amends"] {
        for id in frontmatter_decision_ids(contents, field) {
            let Some(target) = amendable.get(&id) else {
                continue;
            };
            if target == path {
                continue;
            }
            violations.push(format!(
                "{path}: `{field}:` names {target}, which is amendable in place; amend \
                 {target} rather than recording one decision in a second file"
            ));
        }
    }
    violations
}

fn frontmatter_decision_ids(contents: &str, field: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut inside_field = false;
    for line in frontmatter(contents) {
        if !line.starts_with(char::is_whitespace) {
            inside_field = line.split_once(':').is_some_and(|(key, _)| key == field);
        }
        if inside_field {
            ids.extend(decision_ids(line));
        }
    }
    ids
}

/// Lines of the leading YAML block. A document without one yields nothing,
/// which is the same answer as a document that cites nothing.
fn frontmatter(contents: &str) -> impl Iterator<Item = &str> {
    let mut lines = contents.lines();
    let opened = lines.next().is_some_and(|line| line.trim_end() == "---");
    lines.take_while(move |line| opened && line.trim_end() != "---")
}

fn decision_ids(line: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = line;
    while let Some((_, tail)) = rest.split_once("ADR-") {
        let digits: String = tail.chars().take(4).collect();
        if digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            ids.push(format!("ADR-{digits}"));
        }
        rest = tail;
    }
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    const APEX: &str = "docs/decisions/ADR-0716-cargo-merge-path.md";
    const NEW: &str = "docs/decisions/ADR-0720-buck2-is-canonical.md";

    fn amendable() -> BTreeMap<String, String> {
        [("ADR-0716".to_owned(), APEX.to_owned())].into()
    }

    #[test]
    fn both_yaml_spellings_of_a_citation_are_read() {
        for contents in [
            "---\nid: ADR-0720\nsupersedes: [ADR-0716]\n---\n",
            "---\nid: ADR-0720\nsupersedes:\n  - ADR-0716\n---\n",
            "---\nid: ADR-0720\namends: [ADR-0001, ADR-0716]\n---\n",
        ] {
            let violations = new_decision_record_violations(NEW, contents, &amendable());
            assert!(
                violations.iter().any(|violation| violation.contains(APEX)),
                "{contents:?}: {violations:#?}"
            );
        }
    }

    #[test]
    fn provenance_that_is_not_amendable_is_admitted() {
        for contents in [
            "---\nid: ADR-0720\nsupersedes: [ADR-0560]\namends: []\n---\n",
            "---\nid: ADR-0720\nsuperseded_by: [ADR-0716]\namended_by: [ADR-0716]\n---\n",
            "---\nid: ADR-0720\nrelated: [ADR-0716]\n---\n",
            "# ADR-0720\n\nsupersedes: [ADR-0716]\n",
            "---\nid: ADR-0720\n---\n\nsupersedes: [ADR-0716]\n",
        ] {
            let violations = new_decision_record_violations(NEW, contents, &amendable());
            assert!(violations.is_empty(), "{contents:?}: {violations:#?}");
        }
    }

    #[test]
    fn a_record_never_asks_to_amend_itself() {
        let contents = "---\nid: ADR-0716\nsupersedes: [ADR-0716]\n---\n";
        assert!(new_decision_record_violations(APEX, contents, &amendable()).is_empty());
    }
}
