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

use super::super::invalid_git_path;

/// Refuses explicit `[[bin]]`, `[[example]]`, `[[bench]]` and `[[test]]`, except
/// a `[[bin]]` array under `build/dependency-declarations/` whose every entry
/// roots in the integration-test tree.
///
/// The domain gate is the `overturn_when` above spelled in code. That exception
/// is argued from one fact — the buckifier's own bootstrap cannot be buckified
/// by what it produces — and the fact holds for no crate outside that tree, so
/// neither does the exemption. It scopes on the same prefix as the sibling
/// `[lib]` rule at the call site.
pub(super) fn explicit_target_violations(
    path: &str,
    manifest: &toml::Value,
    violations: &mut Vec<String>,
) {
    let bootstrap = path.starts_with("build/dependency-declarations/");
    for target in ["bin", "example", "bench", "test"] {
        let admitted =
            |declared| target == "bin" && bootstrap && every_bin_is_test_sourced(declared);
        if manifest.get(target).is_some_and(|d| !admitted(d)) {
            violations.push(format!(
                "{path}: explicit `[[{target}]]` targets bypass the canonical face entry point"
            ));
        }
    }
}

/// True when every declared `[[bin]]` roots in the integration-test tree.
///
/// Conservative by construction: an array with no entries, an entry with no
/// `path`, or a single non-array value all return false, so the refusal stands
/// unless every binary is demonstrably test-sourced. A partially test-sourced
/// manifest is refused with the rest, because the face-bypass this loop guards
/// is present as soon as one binary is not a fixture.
///
/// "Roots in" is a walk over path components, not a text prefix. Cargo resolves
/// `..` before handing the source to rustc, so `tests/../src/tool.rs` reads as a
/// fixture and compiles as a face. `invalid_git_path` is the component walk this
/// crate already uses against exactly that: it refuses an empty, `.` or `..`
/// component, a backslash, and a leading or trailing separator. Behind it the
/// `tests/` prefix is an exact first component, every later component is a plain
/// name, and there is always at least one of them, so bare `tests` stays refused.
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
            .is_some_and(|path| path.starts_with("tests/") && !invalid_git_path(path))
    })
}
