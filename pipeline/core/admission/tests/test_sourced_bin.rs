//! A test-support binary is not a face.
//!
//! The manifest grammar refuses explicit `[[bin]]` because a face's entry point
//! is discovered canonically. A binary whose source roots in `tests/` is a test
//! double, and cargo offers no other way to declare one: a package gets a single
//! auto-discovered binary, which a facade spends on `src/main.rs`.
//!
//! These tests pin the boundary in both directions, so a later edit that widens
//! the exemption or drops it fails here rather than in CI.

use pipeline_admission::cargo_manifest_violations;

const CRATE: &str = "build/dependency-declarations/adapters/generation-reindeer/Cargo.toml";

fn refused(manifest: &str) -> bool {
    !cargo_manifest_violations(CRATE, manifest).is_empty()
}

fn base() -> String {
    "[package]\nname = \"dependency-declarations-generation-reindeer\"\n".to_owned()
}

#[test]
fn a_binary_sourced_from_the_test_tree_is_admitted() {
    let manifest =
        base() + "[[bin]]\nname = \"fixture_provider\"\npath = \"tests/fixtures/provider.rs\"\n";
    assert!(
        !refused(&manifest),
        "a test-sourced binary is a fixture, not a face"
    );
}

#[test]
fn two_test_sourced_binaries_are_admitted() {
    let manifest = base()
        + "[[bin]]\nname = \"harness\"\npath = \"tests/fixtures/harness.rs\"\n"
        + "[[bin]]\nname = \"double\"\npath = \"tests/fixtures/double.rs\"\n";
    assert!(!refused(&manifest), "every entry roots in tests/");
}

#[test]
fn a_binary_sourced_from_src_is_still_refused() {
    let manifest = base() + "[[bin]]\nname = \"tool\"\npath = \"src/tool.rs\"\n";
    assert!(
        refused(&manifest),
        "src/ is the face tree; declaring an entry point there is the bypass"
    );
}

#[test]
fn one_non_test_binary_refuses_the_whole_manifest() {
    let manifest = base()
        + "[[bin]]\nname = \"double\"\npath = \"tests/fixtures/double.rs\"\n"
        + "[[bin]]\nname = \"tool\"\npath = \"src/tool.rs\"\n";
    assert!(
        refused(&manifest),
        "the bypass exists as soon as one binary is not a fixture"
    );
}

#[test]
fn a_binary_with_no_path_is_refused() {
    let manifest = base() + "[[bin]]\nname = \"tool\"\n";
    assert!(
        refused(&manifest),
        "no path means cargo discovers it from src/bin/, which is the refusal above"
    );
}

#[test]
fn the_exemption_does_not_reach_example_bench_or_test_targets() {
    for target in ["example", "bench", "test"] {
        let manifest =
            base() + &format!("[[{target}]]\nname = \"x\"\npath = \"tests/fixtures/x.rs\"\n");
        assert!(
            refused(&manifest),
            "`[[{target}]]` is out of scope for this exemption"
        );
    }
}

/// The refusal this exemption must never launder away, named once so a test
/// asserting it cannot be satisfied by an unrelated violation such as a
/// package-name mismatch on a fixture path that is not `CRATE`.
fn refuses_the_bin_target(crate_path: &str, manifest: &str) -> bool {
    cargo_manifest_violations(crate_path, manifest)
        .iter()
        .any(|violation| violation.contains("explicit `[[bin]]`"))
}

/// Declares the path as a TOML *literal* string. A basic string would reject
/// `tests/..\src\tool.rs` as an unknown escape, and the manifest would then be
/// refused for failing to parse rather than by the rule under test.
fn bin_at(path: &str) -> String {
    base() + &format!("[[bin]]\nname = \"tool\"\npath = '{path}'\n")
}

/// Every path the gate admits, rather than the first one. An assertion that
/// panics on the first bad entry witnesses only that entry, so a variant behind
/// it is never measured — and a fix that resolves the literal `..` while letting
/// `./..` or `//..` through would still look pinned.
fn admitted<'a>(crate_path: &str, paths: &[&'a str]) -> Vec<&'a str> {
    paths
        .iter()
        .copied()
        .filter(|path| !refuses_the_bin_target(crate_path, &bin_at(path)))
        .collect()
}

#[test]
fn a_traversal_component_cannot_re_enter_the_face_tree() {
    let escaped = admitted(
        CRATE,
        &[
            "tests/../src/tool.rs",
            "tests/./../src/tool.rs",
            "tests//../src/tool.rs",
            "tests/../../../base/core/x/src/lib.rs",
        ],
    );
    assert!(
        escaped.is_empty(),
        "cargo resolves `..` before rustc sees the source, so these root in the \
         face tree and not in tests/: {escaped:?}"
    );
}

#[test]
fn a_backslash_segment_cannot_carry_the_traversal() {
    let escaped = admitted(CRATE, &["tests/..\\src\\tool.rs"]);
    assert!(
        escaped.is_empty(),
        "a backslash is one filename here and a separator on Windows; refuse both readings"
    );
}

#[test]
fn every_component_after_tests_must_be_a_plain_name() {
    let escaped = admitted(CRATE, &["tests/./x.rs", "tests//x.rs", "tests/"]);
    assert!(
        escaped.is_empty(),
        "these are not plain component walks, so their targets are not \
         demonstrably test-sourced: {escaped:?}"
    );
}

#[test]
fn the_test_tree_prefix_is_an_exact_first_component() {
    let escaped = admitted(
        CRATE,
        &[
            "Tests/x.rs",
            "./tests/x.rs",
            "/tests/x.rs",
            "tests",
            "testsuite/x.rs",
        ],
    );
    assert!(
        escaped.is_empty(),
        "none of these root in the integration-test tree: {escaped:?}"
    );
}

#[test]
fn the_exemption_does_not_reach_crates_outside_dependency_declarations() {
    let fixture = bin_at("tests/fixtures/provider.rs");
    let reached: Vec<&str> = [
        "network/core/route/Cargo.toml",
        "network/facade/edge-app/Cargo.toml",
        "app/drive/adapters/blob-s3/Cargo.toml",
    ]
    .into_iter()
    .filter(|crate_path| !refuses_the_bin_target(crate_path, &fixture))
    .collect();
    assert!(
        reached.is_empty(),
        "ADR-0716's `overturn_when` names dependency-declarations, and the loop \
         refused every explicit `[[bin]]` before this exemption existed, so these \
         are a widening this change did not authorise: {reached:?}"
    );
}

#[test]
fn a_bin_table_rather_than_an_array_of_tables_is_refused() {
    let manifest = base() + "[bin]\nname = \"tool\"\npath = 'tests/fixtures/x.rs'\n";
    assert!(
        refuses_the_bin_target(CRATE, &manifest),
        "a single table declares one target the array walk never inspects"
    );
}

#[test]
fn an_empty_bin_array_is_refused() {
    // Prepended, not appended: after `[package]` this would parse as
    // `package.bin`, there would be no top-level target array at all, and the
    // refusal being asserted would silently be about nothing.
    let manifest = "bin = []\n".to_owned() + &base();
    assert!(
        refuses_the_bin_target(CRATE, &manifest),
        "declaring no binary is not the same as declaring only test-sourced ones"
    );
}

#[test]
fn a_non_string_path_is_refused() {
    let manifest = base() + "[[bin]]\nname = \"tool\"\npath = 42\n";
    assert!(
        refuses_the_bin_target(CRATE, &manifest),
        "a path that is not a string is not demonstrably test-sourced"
    );
}
