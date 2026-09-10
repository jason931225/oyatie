//! End-to-end admission for the live decision corpus.
//!
//! A decision log that refuses both amendment and supersession preserves
//! nothing. These fixtures pin the one carved exemption and the refusal that
//! keeps it from becoming a chain of contradicting successor files.

mod support;

use support::{admit, commit, fixture, write};

const APEX: &str = "docs/decisions/ADR-0716-cargo-merge-path.md";
const SUCCESSOR: &str = "docs/decisions/ADR-0720-buck2-is-canonical.md";

fn record(id: &str, supersedes: &str, amends: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\nstatus: Accepted\nsupersedes: [{supersedes}]\nsuperseded_by: []\n\
         amends: [{amends}]\namended_by: []\n---\n\n# {id}\n\n{body}\n"
    )
}

fn apex_base() -> (std::path::PathBuf, String) {
    let root = fixture();
    write(
        &root,
        APEX,
        &record("ADR-0716", "ADR-0560", "", "cargo wins"),
    );
    let base = commit(&root, "base with a live apex decision");
    (root, base)
}

fn refusal(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 diagnostic")
}

#[test]
fn amending_a_live_apex_decision_record_in_place_is_admitted() {
    let (root, base) = apex_base();
    write(
        &root,
        APEX,
        &record("ADR-0716", "ADR-0560", "", "buck2 is canonical"),
    );
    let head = commit(&root, "amend the apex in place");
    let output = admit(&root, &base, &head);
    assert!(
        output.status.success(),
        "amending a live apex decision must be admitted: {}",
        refusal(&output)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_decision_record_that_cites_no_live_file_is_admitted() {
    let (root, base) = apex_base();
    write(
        &root,
        SUCCESSOR,
        &record("ADR-0720", "ADR-0560", "", "an archived predecessor"),
    );
    let head = commit(&root, "new decision over archived provenance");
    let output = admit(&root, &base, &head);
    assert!(
        output.status.success(),
        "a decision whose provenance is archived has nothing to amend: {}",
        refusal(&output)
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_decision_record_cannot_supersede_an_amendable_one() {
    let (root, base) = apex_base();
    write(
        &root,
        SUCCESSOR,
        &record("ADR-0720", "ADR-0716", "", "buck2 is canonical"),
    );
    let head = commit(&root, "successor instead of an amendment");
    let output = admit(&root, &base, &head);
    let error = refusal(&output);
    assert!(!output.status.success(), "{error}");
    assert!(
        error.contains(&format!("{SUCCESSOR}: `supersedes:` names {APEX}")),
        "the refusal must name the file to amend: {error}"
    );
    assert!(error.contains("amendable in place"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_new_decision_record_cannot_amend_an_amendable_one() {
    let (root, base) = apex_base();
    write(
        &root,
        SUCCESSOR,
        &record("ADR-0720", "", "ADR-0716", "buck2 is canonical"),
    );
    let head = commit(&root, "amendment filed as a second file");
    let output = admit(&root, &base, &head);
    let error = refusal(&output);
    assert!(!output.status.success(), "{error}");
    assert!(
        error.contains(&format!("{SUCCESSOR}: `amends:` names {APEX}")),
        "the refusal must name the file to amend: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_block_sequence_citation_does_not_evade_the_refusal() {
    let (root, base) = apex_base();
    write(
        &root,
        SUCCESSOR,
        "---\nid: ADR-0720\nstatus: Accepted\nsupersedes:\n  - ADR-0716\nsuperseded_by: []\n\
         amends: []\namended_by: []\n---\n\n# ADR-0720\n",
    );
    let head = commit(&root, "successor with a block-sequence citation");
    let output = admit(&root, &base, &head);
    let error = refusal(&output);
    assert!(!output.status.success(), "{error}");
    assert!(error.contains(APEX), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn deleting_a_live_apex_decision_record_is_still_refused() {
    let (root, base) = apex_base();
    std::fs::remove_file(root.join(APEX)).expect("remove the apex record");
    let head = commit(&root, "delete the apex");
    let output = admit(&root, &base, &head);
    let error = refusal(&output);
    assert!(!output.status.success(), "{error}");
    assert!(error.contains("frozen non-root Markdown"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn prose_beside_the_corpus_is_still_frozen() {
    let (root, base) = apex_base();
    write(&root, "docs/decisions/INDEX.md", "# updated index\n");
    let head = commit(&root, "touch the corpus index");
    let output = admit(&root, &base, &head);
    let error = refusal(&output);
    assert!(!output.status.success(), "{error}");
    assert!(
        error.contains("docs/decisions/INDEX.md: frozen non-root Markdown"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(root);
}
