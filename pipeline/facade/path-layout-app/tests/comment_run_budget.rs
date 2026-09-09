//! End-to-end: the comment-run ceiling charged through the layout facade.

mod support;

use std::path::Path;

use support::{admit, commit, fixture, git, write};

const SOURCE: &str = "network/core/engine/src/legacy.rs";
const MOVED: &str = "network/core/engine/src/moved.rs";
const CEILING: &str = "comment-run ceiling";

fn write_complete_owner(root: &Path) {
    write(
        root,
        "network/core/engine/Cargo.toml",
        "[package]\nname='network-engine'\nversion='0.1.0'\nedition='2024'\n",
    );
    write(
        root,
        "network/core/engine/src/lib.rs",
        "pub fn engine() {}\n",
    );
}

fn commented(lines: usize) -> String {
    format!("{}pub fn legacy() {{}}\n", "// note\n".repeat(lines))
}

fn assert_admitted(root: &Path, base: &str, head: &str) {
    let output = admit(root, base, head);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_refused(root: &Path, base: &str, head: &str) -> String {
    let output = admit(root, base, head);
    assert!(
        !output.status.success(),
        "admitted an oversized comment run"
    );
    let error = String::from_utf8(output.stderr).expect("admission error text");
    assert!(error.contains(CEILING), "{error}");
    error
}

#[test]
fn a_changed_source_carrying_an_oversized_run_is_refused_and_named() {
    let root = fixture();
    write_complete_owner(&root);
    write(&root, SOURCE, &commented(3));
    let base = commit(&root, "a source that explains itself briefly");
    write(&root, SOURCE, &commented(21));
    let head = commit(&root, "grow the preamble past the ceiling");

    let error = assert_refused(&root, &base, &head);
    assert!(error.contains(&format!("{SOURCE}:1:")), "{error}");
    assert!(error.contains("21 consecutive comment lines"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_changed_source_at_the_ceiling_is_admitted() {
    let root = fixture();
    write_complete_owner(&root);
    write(&root, SOURCE, &commented(3));
    let base = commit(&root, "a source that explains itself briefly");
    write(&root, SOURCE, &commented(20));
    let head = commit(&root, "grow the preamble to the ceiling");

    assert_admitted(&root, &base, &head);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_exact_rename_out_of_third_party_is_charged_the_ceiling() {
    let root = fixture();
    write_complete_owner(&root);
    write(&root, "third-party/vendor/huge.rs", &commented(21));
    let base = commit(&root, "vendored prose, exempt where it sits");
    git(&root, &["mv", "third-party/vendor/huge.rs", MOVED]);
    let head = commit(&root, "relocate vendored prose into a budgeted path");

    let error = assert_refused(&root, &base, &head);
    assert!(
        error.contains(MOVED),
        "the refusal must name the destination that now owes the debt: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_exact_rename_of_an_existing_run_is_not_recharged() {
    let root = fixture();
    write_complete_owner(&root);
    write(&root, SOURCE, &commented(21));
    let base = commit(&root, "existing comment debt");
    git(&root, &["mv", SOURCE, MOVED]);
    let head = commit(&root, "move existing comment debt");

    assert_admitted(&root, &base, &head);
    let _ = std::fs::remove_dir_all(root);
}
