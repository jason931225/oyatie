//! Boundaries are pinned on both sides, and specifically at the two bytes
//! that are ambiguous. `.` and `/` continue a name in `docs/a.json.bak` and
//! `vendor/docs/a.json`, and end one in `See docs/a.md.` and `./docs/a.md`. A
//! suite that only tries the continuing half passes against a check that is
//! blind to the most ordinary way prose names a file.

use std::collections::BTreeSet;

use pipeline_retained_reference::{
    Refusal, RetainedReference, refuse_retained, retained_references,
};

const DELETED: &str = "docs/machine-readable/decisions.json";
const PROSE: &str = "docs/standards/code-review.md";

fn deleted(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

fn one(path: &str, contents: &str) -> Vec<RetainedReference> {
    retained_references(&deleted(&[path]), [("docs/ADR-INDEX.md", contents)])
}

#[test]
fn a_surviving_file_naming_a_deleted_path_refuses_at_that_line() {
    let contents = format!("first\nsecond\nsee {DELETED} for detail\nfourth\n");
    let found = one(DELETED, &contents);
    assert_eq!(
        found,
        [RetainedReference {
            referrer: "docs/ADR-INDEX.md".to_owned(),
            deleted: DELETED.to_owned(),
            line: 3,
        }]
    );
    assert!(found[0].reason().contains("docs/ADR-INDEX.md:3"));
}

/// The dominant way prose names a file is at the end of a sentence. Treating
/// `.` as always continuing a path discards exactly those mentions and returns
/// green while the reference stands.
#[test]
fn a_path_ending_a_sentence_or_a_line_is_still_a_mention() {
    for line in [
        format!("See {PROSE}."),
        format!("See {PROSE}. Then stop."),
        PROSE.to_owned(),
        format!("[review]({PROSE})."),
        format!("See {PROSE}?"),
    ] {
        let contents = format!("{line}\n");
        assert_eq!(one(PROSE, &contents).len(), 1, "{line}");
    }
}

/// A `/` before the match joins two parts of one name only when something
/// namelike precedes it. Root-relative and dot-relative forms name the file.
#[test]
fn a_relative_or_rooted_spelling_is_still_a_mention() {
    for line in [
        format!("see ./{PROSE}"),
        format!("see /{PROSE}"),
        format!("[review](./{PROSE})"),
        format!("see ./{PROSE}."),
    ] {
        let contents = format!("{line}\n");
        assert_eq!(one(PROSE, &contents).len(), 1, "{line}");
    }
}

/// The continuing half of the same two bytes must still block.
#[test]
fn a_longer_name_sharing_the_prefix_is_not_a_mention() {
    for suffix in [".bak", "x", "_old", "-2", "/nested", ".json"] {
        let contents = format!("see {DELETED}{suffix}\n");
        assert_eq!(one(DELETED, &contents), [], "{suffix}");
    }
}

#[test]
fn a_path_that_merely_ends_the_same_is_not_a_mention() {
    for prefix in ["vendor/", "x", "a-", "b_", ".", "a/b/"] {
        let contents = format!("see {prefix}{DELETED}\n");
        assert_eq!(one(DELETED, &contents), [], "{prefix}");
    }
}

#[test]
fn ordinary_delimiters_around_a_path_are_still_a_mention() {
    for line in [
        format!("[decisions]({DELETED})"),
        format!("\"{DELETED}\""),
        format!("`{DELETED}`"),
        format!("see {DELETED}, then stop"),
        DELETED.to_owned(),
    ] {
        let contents = format!("{line}\n");
        assert_eq!(one(DELETED, &contents).len(), 1, "{line}");
    }
}

/// Prose that is itself going away cannot retain anything: the candidate that
/// deletes the referrer deletes the pointer with it.
#[test]
fn a_referrer_that_is_itself_deleted_retains_nothing() {
    let both = deleted(&[DELETED, "docs/machine-readable/risks.json"]);
    let contents = format!("this file points at {DELETED}\n");
    let live = [("docs/machine-readable/risks.json", contents.as_str())];
    assert_eq!(retained_references(&both, live), []);
    assert_eq!(refuse_retained(&both, live), Ok(()));
}

/// A check that cannot reach its subject reports that rather than passing.
#[test]
fn a_blank_path_in_the_deletion_set_is_undecidable_not_clean() {
    let contents = format!("see {DELETED}\n");
    for blank in ["", "   ", "\n"] {
        assert_eq!(
            refuse_retained(
                &deleted(&[blank]),
                [("docs/ADR-INDEX.md", contents.as_str())]
            ),
            Err(Refusal::UndecidablePath),
            "{blank:?}"
        );
    }
    assert!(Refusal::UndecidablePath.reason().contains("blank path"));
}

#[test]
fn an_empty_deletion_set_refuses_nothing() {
    let contents = format!("see {DELETED}\n");
    let live = [("docs/ADR-INDEX.md", contents.as_str())];
    assert_eq!(retained_references(&BTreeSet::new(), live), []);
    assert_eq!(refuse_retained(&BTreeSet::new(), live), Ok(()));
}

/// Every (referrer, deleted) pair is reported once, in a stable order, so two
/// runs over the same tree produce the same refusal text.
#[test]
fn every_pair_is_reported_once_in_a_stable_order() {
    let other = "docs/machine-readable/risks.json";
    let set = deleted(&[DELETED, other]);
    let live = [
        ("docs/RISK-REGISTER.md", format!("a\nsee {other}\n")),
        (
            "docs/ADR-INDEX.md",
            format!("see {DELETED}\nand {other}\nand {DELETED} again\n"),
        ),
    ];
    let borrowed: Vec<(&str, &str)> = live
        .iter()
        .map(|(path, text)| (*path, text.as_str()))
        .collect();
    let found = retained_references(&set, borrowed.clone());
    assert_eq!(
        found
            .iter()
            .map(|item| (item.referrer.as_str(), item.deleted.as_str(), item.line))
            .collect::<Vec<_>>(),
        [
            ("docs/ADR-INDEX.md", DELETED, 1),
            ("docs/ADR-INDEX.md", other, 2),
            ("docs/RISK-REGISTER.md", other, 2),
        ],
        "one finding per pair, first mention, sorted"
    );
    assert_eq!(
        refuse_retained(&set, borrowed),
        Err(Refusal::Retained(found))
    );
}
