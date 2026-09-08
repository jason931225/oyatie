//! Cargo identity checks for changed manifests. Provenance: ADR-0719 D-30/D-41.

mod entrypoint;
mod test_sourced;

pub use entrypoint::{
    cargo_entrypoint, cargo_entrypoints, cargo_manifest_for_crate_path,
    cargo_manifest_for_entrypoint,
};

use super::{is_capability_root, path_parts};

pub(super) type PackagePolicy = (&'static str, bool);
pub(super) fn dependency_declarations_package(face: &str, leaf: &str) -> Option<PackagePolicy> {
    match (face, leaf) {
        ("core", "reconcile") => Some(("dependency-declarations-reconcile", true)),
        ("ports", "generation") => Some(("dependency-declarations-generation", false)),
        ("ports", "publication") => Some(("dependency-declarations-publication", false)),
        ("adapters", "generation-reindeer") => {
            Some(("dependency-declarations-generation-reindeer", true))
        }
        ("adapters", "publication-filesystem") => {
            Some(("dependency-declarations-publication-filesystem", true))
        }
        ("facade", "reconciler-app") => Some(("dependency-declarations-reconciler-app", true)),
        _ => None,
    }
}

pub fn cargo_manifest_violations(path: &str, contents: &str) -> Vec<String> {
    let Some((expected_name, face, allows_build_script)) = expected_manifest_identity(path) else {
        return Vec::new();
    };
    let manifest = match contents.parse::<toml::Value>() {
        Ok(manifest) => manifest,
        Err(error) => return vec![format!("{path}: invalid Cargo manifest: {error}")],
    };
    let mut violations = Vec::new();
    let package_name = manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str);
    if package_name != Some(expected_name.as_str()) {
        violations.push(format!(
            "{path}: package name must be `{expected_name}`, got {}",
            package_name.unwrap_or("<missing or non-string>")
        ));
    }
    if manifest
        .get("lib")
        .and_then(|library| library.get("name"))
        .is_some()
    {
        violations.push(format!(
            "{path}: `[lib].name` must be omitted so Cargo derives the crate name"
        ));
    }
    if path.starts_with("build/dependency-declarations/") {
        let library = manifest.get("lib");
        for target_kind in ["proc-macro", "crate-type"] {
            if library.is_some_and(|library| library.get(target_kind).is_some()) {
                violations.push(format!("{path}: `[lib].{target_kind}` must be omitted"));
            }
        }
    }
    if let Some(lib_path) = manifest
        .get("lib")
        .and_then(|library| library.get("path"))
        .and_then(toml::Value::as_str)
        && lib_path != "src/lib.rs"
    {
        violations.push(format!(
            "{path}: `[lib].path` must be `src/lib.rs`, got `{lib_path}`"
        ));
    }
    test_sourced::explicit_target_violations(path, &manifest, &mut violations);
    let package = manifest.get("package");
    // A facade may root at either `src/main.rs` or `src/lib.rs`, so BOTH
    // discovery switches guard a canonical target for it. Checking only
    // `autobins` let a lib-rooted facade disable its own root with
    // `autolib = false`. Both directions still fail closed, so a facade
    // that sets either switch is refused whether or not it has that target.
    let discovery_switches: &[&str] = if face == "facade" {
        &["autobins", "autolib"]
    } else {
        &["autolib"]
    };
    for discovery_switch in discovery_switches {
        if package
            .and_then(|package| package.get(*discovery_switch))
            .and_then(toml::Value::as_bool)
            == Some(false)
        {
            violations.push(format!(
                "{path}: package `{discovery_switch} = false` disables the canonical face target"
            ));
        }
    }
    if package
        .and_then(|package| package.get("autotests"))
        .and_then(toml::Value::as_bool)
        == Some(false)
    {
        violations.push(format!(
            "{path}: package `autotests = false` disables required integration-test discovery"
        ));
    }
    for switch in ["test", "harness"] {
        if manifest
            .get("lib")
            .and_then(|library| library.get(switch))
            .and_then(toml::Value::as_bool)
            == Some(false)
        {
            violations.push(format!(
                "{path}: `[lib].{switch} = false` disables required test discovery"
            ));
        }
    }
    if let Some(build) = manifest
        .get("package")
        .and_then(|package| package.get("build"))
    {
        match build.as_str() {
            Some("build.rs") if allows_build_script => {}
            Some("build.rs") => {
                violations.push(format!("{path}: std-only port forbids package `build`"))
            }
            Some(build_path) => violations.push(format!(
                "{path}: package build target must be the stable item-scanner `build.rs`, got `{build_path}`"
            )),
            None => violations.push(format!(
                "{path}: package build target must be the stable item-scanner `build.rs`; boolean or non-string overrides are forbidden"
            )),
        }
    }
    violations
}

pub(super) fn expected_manifest_identity(path: &str) -> Option<(String, &str, bool)> {
    let parts = path_parts(path);
    if parts.first() == Some(&"build") && parts.get(1) == Some(&"dependency-declarations") {
        if parts.len() != 5 || parts.last() != Some(&"Cargo.toml") {
            return None;
        }
        let face = *parts.get(2)?;
        let leaf = *parts.get(3)?;
        return dependency_declarations_package(face, leaf)
            .map(|(package, allows_build_script)| (package.to_owned(), face, allows_build_script));
    }
    let (owner, face_index) = if parts.first() == Some(&"app") {
        (*parts.get(1)?, 2)
    } else {
        let owner = *parts.first()?;
        if owner != "base" && !is_capability_root(owner) {
            return None;
        }
        (owner, 1)
    };
    let face = *parts.get(face_index)?;
    if !matches!(face, "core" | "ports" | "adapters" | "facade") {
        return None;
    }
    let mut leaf_index = face_index + 1;
    let draft = matches!(face, "ports" | "adapters") && parts.get(leaf_index) == Some(&"draft");
    if draft {
        leaf_index += 1;
    }
    if parts.len() != leaf_index + 2 || parts.last() != Some(&"Cargo.toml") {
        return None;
    }
    let leaf = parts[leaf_index];
    if !super::inner::crate_leaf_ok(face, leaf) {
        return None;
    }
    let suffix = if draft { "-draft" } else { "" };
    Some((format!("{owner}-{leaf}{suffix}"), face, true))
}

#[cfg(test)]
fn expected_package_name(path: &str) -> Option<String> {
    expected_manifest_identity(path).map(|(name, _, _)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_names_follow_owner_face_and_draft_identity() {
        let cases = [
            ("network/core/route/Cargo.toml", "network-route"),
            ("network/ports/blob/Cargo.toml", "network-blob"),
            ("network/ports/draft/blob/Cargo.toml", "network-blob-draft"),
            ("app/drive/adapters/blob-s3/Cargo.toml", "drive-blob-s3"),
            ("app/drive/facade/app/Cargo.toml", "drive-app"),
        ];
        for (path, expected) in cases {
            assert_eq!(
                expected_package_name(path).as_deref(),
                Some(expected),
                "{path}"
            );
        }
    }

    #[test]
    fn package_name_and_lib_override_are_fail_closed() {
        let path = "network/ports/blob/Cargo.toml";
        assert!(cargo_manifest_violations(path, "[package]\nname='network-blob'\n").is_empty());
        let violations =
            cargo_manifest_violations(path, "[package]\nname='anything'\n[lib]\nname='alias'\n");
        assert!(violations.iter().any(|item| item.contains("network-blob")));
        assert!(violations.iter().any(|item| item.contains("[lib].name")));
        assert!(!cargo_manifest_violations(path, "not = [valid").is_empty());
    }

    #[test]
    fn custom_target_paths_are_fail_closed() {
        let path = "network/core/route/Cargo.toml";
        for manifest in [
            "[package]\nname='network-route'\n[lib]\npath='src/other.rs'\n",
            "[package]\nname='network-route'\n[[bin]]\nname='route'\npath='src/route.rs'\n",
            "[package]\nname='network-route'\nbuild='tools/generate.rs'\n",
            "[package]\nname='network-route'\nbuild=false\n",
            "[package]\nname='network-route'\nautolib=false\n",
            "[package]\nname='network-route'\nautotests=false\n",
            "[package]\nname='network-route'\n[lib]\ntest=false\n",
            "[package]\nname='network-route'\n[lib]\nharness=false\n",
        ] {
            assert!(!cargo_manifest_violations(path, manifest).is_empty());
        }
    }

    #[test]
    fn dependency_declarations_target_kind_is_canonical() {
        let path = "build/dependency-declarations/core/reconcile/Cargo.toml";
        for field in ["proc-macro=true", "crate-type=['cdylib']"] {
            let target_kind = field.split_once('=').expect("target field").0;
            let contents =
                format!("[package]\nname='dependency-declarations-reconcile'\n[lib]\n{field}\n");
            let violations = cargo_manifest_violations(path, &contents);
            assert!(violations.join("\n").contains(target_kind), "{target_kind}");
        }
    }

    #[test]
    fn both_discovery_switches_guard_a_facade() {
        // A facade may root at either file, so disabling either discovery
        // switch can disable its canonical target. Checking only `autobins`
        // let a lib-rooted facade turn its own root off.
        for switch in ["autobins", "autolib"] {
            let manifest = format!("[package]\nname = \"network-edge-app\"\n{switch} = false\n");
            let violations =
                cargo_manifest_violations("network/facade/edge-app/Cargo.toml", &manifest);
            assert!(
                violations
                    .iter()
                    .any(|v| v.contains(&format!("`{switch} = false`"))),
                "facade must refuse {switch} = false, got {violations:?}"
            );
        }
    }

    #[test]
    fn canonical_entrypoint_follows_the_face() {
        let core = cargo_entrypoint("network/core/route/Cargo.toml");
        assert_eq!(core.as_deref(), Some("network/core/route/src/lib.rs"));
        let facade = cargo_entrypoint("network/facade/edge-app/Cargo.toml");
        let facade_path = Some("network/facade/edge-app/src/main.rs");
        assert_eq!(facade.as_deref(), facade_path);
        let manifest = cargo_manifest_for_entrypoint("network/facade/edge-app/src/main.rs");
        let manifest_path = Some("network/facade/edge-app/Cargo.toml");
        assert_eq!(manifest.as_deref(), manifest_path);
    }

    #[test]
    fn every_path_below_a_face_leaf_maps_to_its_manifest() {
        for path in [
            "network/ports/blob/tests/fixture.rs",
            "network/ports/blob/OWNERS",
            "network/ports/blob/build.rs",
            "network/ports/draft/blob/src/lib.rs",
            "app/drive/adapters/blob-s3/tests/live.rs",
        ] {
            assert!(cargo_manifest_for_crate_path(path).is_some(), "{path}");
        }
        let proto = cargo_manifest_for_crate_path("network/facade/proto/network/api/v1/a.proto");
        assert!(proto.is_none());
        let port_engine =
            cargo_manifest_for_crate_path("build/port-engine/core/analysis/src/lib.rs");
        assert!(port_engine.is_none());
    }
}
