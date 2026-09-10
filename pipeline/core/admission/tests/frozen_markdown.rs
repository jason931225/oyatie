//! Protected default for frozen, non-authoritative Markdown inventory.

use std::collections::BTreeSet;

use pipeline_admission::{
    changed_layout_violations, git_change_paths_from_name_status_z, layout_violations,
};

fn frozen(input: &[u8]) -> Vec<String> {
    let changes = git_change_paths_from_name_status_z(input).expect("complete Git change record");
    changed_layout_violations(&changes, &BTreeSet::new())
        .into_iter()
        .filter(|violation| violation.contains("frozen non-root Markdown"))
        .collect()
}

#[test]
fn every_changed_markdown_status_and_endpoint_refuses_closed() {
    for (change, path) in [
        (b"A\0policy/new.md\0".as_slice(), "policy/new.md"),
        (b"M\0policy/current.md\0".as_slice(), "policy/current.md"),
        (b"D\0policy/legacy.md\0".as_slice(), "policy/legacy.md"),
        (b"T\0policy/type.md\0".as_slice(), "policy/type.md"),
        (
            b"R100\0policy/source.md\0policy/core/evaluate/src/source.rs\0".as_slice(),
            "policy/source.md",
        ),
        (
            b"R100\0policy/core/evaluate/src/source.rs\0policy/destination.md\0".as_slice(),
            "policy/destination.md",
        ),
        (
            b"C100\0policy/source.md\0policy/core/evaluate/src/copy.rs\0".as_slice(),
            "policy/source.md",
        ),
        (
            b"C100\0policy/core/evaluate/src/source.rs\0policy/copy.md\0".as_slice(),
            "policy/copy.md",
        ),
    ] {
        let violations = frozen(change);
        assert!(
            violations
                .iter()
                .any(|violation| violation.starts_with(path)),
            "{path}: {violations:#?}"
        );
    }
}

#[test]
fn only_three_exact_root_markdown_paths_remain_mutable() {
    assert!(
        layout_violations(&["README.md", "AGENTS.md", "CLAUDE.md"].map(str::to_owned)).is_empty()
    );
    for path in [
        "README.MD",
        "policy/README.md",
        "docs/decisions/INDEX.md",
        "docs/decisions/ADR-0654-consolidated.md",
        ".github/SECURITY.md",
        "app/README.md",
        "policy/notes.markdown",
    ] {
        let violations = layout_violations(&[path.to_owned()]);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("frozen non-root Markdown")),
            "{path}: {violations:#?}"
        );
    }
}

#[test]
fn historical_path_provenance_does_not_leak_into_the_diagnostic() {
    let violation = layout_violations(&["docs/decisions/ADR-0654-consolidated.md".to_owned()])
        .into_iter()
        .next()
        .expect("frozen historical path violation");
    assert_eq!(
        violation,
        "docs/decisions/ADR-0654-consolidated.md: frozen non-root Markdown cannot be changed or used as a copy source"
    );
}

#[test]
fn new_owner_is_admitted_by_native_source_but_not_tracked_prose() {
    let native = git_change_paths_from_name_status_z(
        b"A\0policy/OWNERS\0A\0policy/core/evaluate/Cargo.toml\0A\0policy/core/evaluate/src/lib.rs\0",
    )
    .expect("native owner change");
    assert!(changed_layout_violations(&native, &BTreeSet::new()).is_empty());

    let with_prose = git_change_paths_from_name_status_z(
        b"A\0policy/OWNERS\0A\0policy/README.md\0A\0policy/core/evaluate/Cargo.toml\0A\0policy/core/evaluate/src/lib.rs\0",
    )
    .expect("owner change with prose");
    assert!(!frozen_change(&with_prose).is_empty());
}

fn frozen_change(changes: &pipeline_admission::GitChangePaths) -> Vec<String> {
    changed_layout_violations(changes, &BTreeSet::new())
        .into_iter()
        .filter(|violation| violation.contains("frozen non-root Markdown"))
        .collect()
}

const APEX: &str = "docs/decisions/ADR-0716-cargo-merge-path.md";

#[test]
fn a_live_apex_decision_record_is_amendable_in_place() {
    for change in [
        format!("M\0{APEX}\0"),
        format!("A\0{APEX}\0"),
        format!("T\0{APEX}\0"),
    ] {
        let violations = frozen(change.as_bytes());
        assert!(violations.is_empty(), "{change:?}: {violations:#?}");
    }
    assert!(layout_violations(&[APEX.to_owned()]).is_empty());
}

#[test]
fn a_live_apex_decision_record_cannot_be_retired_by_erasure() {
    for change in [
        format!("D\0{APEX}\0"),
        format!("R100\0{APEX}\0docs/decisions/ADR-0717-renamed.md\0"),
        format!("C100\0{APEX}\0docs/decisions/ADR-0717-copied.md\0"),
    ] {
        let violations = frozen(change.as_bytes());
        assert!(
            violations.iter().any(|item| item.starts_with(APEX)),
            "{change:?}: {violations:#?}"
        );
    }
}

#[test]
fn the_exemption_is_spelled_exactly_like_the_live_apex_corpus() {
    for path in [
        "docs/decisions/ADR-0654-consolidated.md",
        "docs/decisions/ADR-0800-future.md",
        "docs/decisions/ADR-07evil.md",
        "docs/decisions/INDEX.md",
        "docs/adr-archive/ADR-0716-cargo-merge-path.md",
        "docs/decisions/nested/ADR-0716-cargo-merge-path.md",
    ] {
        let violations = layout_violations(&[path.to_owned()]);
        assert!(
            violations
                .iter()
                .any(|violation| violation.contains("frozen non-root Markdown")),
            "{path}: {violations:#?}"
        );
    }
}
