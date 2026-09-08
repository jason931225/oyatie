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
