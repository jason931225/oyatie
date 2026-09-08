//! Whether a manifest's declared binaries are all test doubles.
//!
//! The manifest grammar refuses explicit `[[bin]]` because a face's entry point
//! is discovered canonically. A binary whose source lives under `tests/` is not
//! a face: it is a test double that has to be an executable because the thing it
//! impersonates is one.
//!
//! Cargo leaves no other way to declare such a double. A package gets exactly
//! one auto-discovered binary, a facade spends that on `src/main.rs`, and
//! `src/bin/` is refused by the sibling rule — so a package needing both a
//! harness and a fixture provider cannot express the second at all.
//!
//! ADR-0716's `overturn_when` names `manifest/reindeer` as a domain where cargo
//! execute stays even after buck2 takes the merge path, because
//! dependency-declarations is the buckifier's own bootstrap and cannot be
//! buckified by what it produces. This admits what that named exception
//! requires, and nothing broader.

/// True when every declared `[[bin]]` roots in the integration-test tree.
///
/// Conservative by construction: an array with no entries, an entry with no
/// `path`, or a single non-array value all return false, so the refusal stands
/// unless every binary is demonstrably test-sourced. A partially test-sourced
/// manifest is refused with the rest, because the face-bypass this loop guards
/// is present as soon as one binary is not a fixture.
/// Refuses explicit `[[bin]]`, `[[example]]`, `[[bench]]` and `[[test]]`, except
/// a `[[bin]]` array whose every entry roots in the integration-test tree.
pub(super) fn explicit_target_violations(
    path: &str,
    manifest: &toml::Value,
    violations: &mut Vec<String>,
) {
    for target in ["bin", "example", "bench", "test"] {
        let admitted = |declared| target == "bin" && every_bin_is_test_sourced(declared);
        if manifest.get(target).is_some_and(|d| !admitted(d)) {
            violations.push(format!(
                "{path}: explicit `[[{target}]]` targets bypass the canonical face entry point"
            ));
        }
    }
}

fn every_bin_is_test_sourced(declared: &toml::Value) -> bool {
    let Some(entries) = declared.as_array() else {
        return false;
    };
    if entries.is_empty() {
        return false;
    }
    entries.iter().all(|entry| {
        entry
            .get("path")
            .and_then(toml::Value::as_str)
            .is_some_and(|path| path.starts_with("tests/"))
    })
}
