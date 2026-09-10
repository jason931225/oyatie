//! Closed grammars for repository-level metadata and data roots.

use super::{CARGO_CONFIG_PATHS, inner::validate_owner_path};

const PACK_NAMESPACES: &[&str] = &["us", "eu", "jp", "kr"];
const ROOT_METADATA: &[&str] = &["OWNERS", "README.md", "BUCK"];
const GITHUB_ROOT_FILES: &[&str] = &[
    "CODEOWNERS",
    "CODE_OF_CONDUCT.md",
    "CONTRIBUTING.md",
    "OWNERS",
    "PULL_REQUEST_TEMPLATE.md",
    "SECURITY.md",
    "branch-protection.yaml",
];

pub(super) fn validate_base_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    if parts.len() == 2 && ROOT_METADATA.contains(&parts[1]) {
        return;
    }
    if parts.get(1) != Some(&"core") {
        violations.push(format!(
            "{file}: base admits only root metadata and `core/` primitives"
        ));
        return;
    }
    validate_owner_path(file, parts, 1, violations);
}

pub(super) fn validate_cargo_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    let config_name = parts.get(1).is_some_and(|name| {
        CARGO_CONFIG_PATHS
            .iter()
            .filter_map(|path| path.strip_prefix(".cargo/"))
            .any(|admitted| admitted == *name)
    });
    if parts.len() != 2 || !(parts[1] == "BUCK" || config_name) {
        violations.push(format!(
            "{file}: `.cargo/` admits only BUCK and the canonical Cargo configuration"
        ));
    }
}

pub(super) fn validate_config_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    if !matches!(parts, [".config", "nextest.toml"]) {
        violations.push(format!(
            "{file}: `.config/` admits only the active nextest profile"
        ));
    }
}

pub(super) fn validate_githook_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    if !matches!(parts, [".githooks", "pre-commit" | "pre-push"]) {
        violations.push(format!(
            "{file}: `.githooks/` admits only the required pre-commit and pre-push hooks"
        ));
    }
}

pub(super) fn validate_github_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    let valid = matches!(parts, [".github", name] if GITHUB_ROOT_FILES.contains(name))
        || matches!(parts, [".github", "workflows", "OWNERS"])
        || matches!(parts, [".github", "workflows", name] if yaml_file(name))
        || matches!(parts, [".github", "ISSUE_TEMPLATE", name] if yaml_file(name))
        || matches!(parts, [".github", "scripts", rest @ ..] if valid_glue_path(rest));
    if !valid {
        violations.push(format!(
            "{file}: `.github/` admits root metadata, issue templates, workflow YAML, and self-contained `scripts/` glue only"
        ));
    }
}

pub(super) fn validate_docs_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    if parts.len() == 2 && matches!(parts[1], "AGENTS.md" | "OWNERS" | "README.md" | "BUCK") {
        return;
    }
    let valid = match parts.get(1).copied() {
        Some("decisions") if parts.len() == 3 => decision_file(parts[2]),
        Some("standards") if parts.len() == 3 => markdown_or_metadata(parts[2]),
        _ => false,
    };
    if !valid {
        violations.push(format!(
            "{file}: root docs are limited to AGENTS.md, live 07xx decisions, and direct standards"
        ));
    }
}

pub(super) fn validate_packs_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    if parts.len() == 2 && ROOT_METADATA.contains(&parts[1]) {
        return;
    }
    let valid = parts.len() == 4
        && PACK_NAMESPACES.contains(&parts[1])
        && kebab_case(parts[2])
        && pack_payload(parts[3]);
    if !valid {
        violations.push(format!(
            "{file}: packs require `<us|eu|jp|kr>/<package>/<name>.cedar|proto|textproto`"
        ));
    }
}

pub(super) fn validate_templates_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    let valid = matches!(parts, ["templates", "OWNERS" | "README.md" | "BUCK"])
        || matches!(parts, ["templates", "adr-template.md"])
        || matches!(parts, ["templates", "checklists", "swarm-agent-ritual.md"]);
    if !valid {
        violations.push(format!(
            "{file}: templates are limited to the ADR template and swarm ritual"
        ));
    }
}

fn decision_file(name: &str) -> bool {
    matches!(name, "OWNERS" | "README.md" | "INDEX.md" | "BUCK") || super::live_apex_adr_name(name)
}

fn markdown_or_metadata(name: &str) -> bool {
    matches!(name, "OWNERS" | "README.md" | "INDEX.md" | "BUCK")
        || name.strip_suffix(".md").is_some_and(kebab_case)
}

fn yaml_file(name: &str) -> bool {
    name.strip_suffix(".yml")
        .or_else(|| name.strip_suffix(".yaml"))
        .is_some_and(kebab_case)
}

fn valid_glue_path(parts: &[&str]) -> bool {
    !parts.is_empty()
        && parts.iter().all(|part| {
            part.as_bytes()
                .first()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        })
        && !parts.contains(&"Cargo.toml")
}

fn pack_payload(name: &str) -> bool {
    [".cedar", ".proto", ".textproto"]
        .iter()
        .any(|suffix| name.strip_suffix(suffix).is_some_and(lower_identifier))
}

fn kebab_case(name: &str) -> bool {
    lower_identifier(name) && !name.contains('_')
}

fn lower_identifier(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && name
            .as_bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && !name.contains("--")
        && !name.contains("__")
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejected(path: &str, check: fn(&str, &[&str], &mut Vec<String>)) -> bool {
        let parts: Vec<&str> = path.split('/').collect();
        let mut violations = Vec::new();
        check(path, &parts, &mut violations);
        !violations.is_empty()
    }

    #[test]
    fn root_docs_are_thin() {
        assert!(!rejected(
            "docs/decisions/ADR-0720-example.md",
            validate_docs_path
        ));
        assert!(!rejected(
            "docs/standards/code-style.md",
            validate_docs_path
        ));
        assert!(rejected("docs/scratch/Cargo.toml", validate_docs_path));
        assert!(rejected("docs/decisions/ADR.md", validate_docs_path));
    }

    #[test]
    fn packs_are_typed_cedar_and_ir_packages() {
        assert!(!rejected("packs/eu/gdpr/policy.cedar", validate_packs_path));
        assert!(!rejected(
            "packs/kr/csap/data_residency.textproto",
            validate_packs_path
        ));
        assert!(rejected("packs/eu/plan/todo.md", validate_packs_path));
        assert!(rejected("packs/eu/new-overlay.yaml", validate_packs_path));
    }

    #[test]
    fn active_dot_roots_are_closed_to_their_loaded_inputs() {
        for path in [
            ".config/nextest.toml",
            ".githooks/pre-commit",
            ".githooks/pre-push",
            ".github/workflows/presubmit.yml",
            ".github/scripts/check.py",
            ".github/ISSUE_TEMPLATE/bug-report.yml",
        ] {
            let check = if path.starts_with(".config/") {
                validate_config_path
            } else if path.starts_with(".githooks/") {
                validate_githook_path
            } else {
                validate_github_path
            };
            assert!(!rejected(path, check), "{path}");
        }
        for path in [
            ".config/other.toml",
            ".githooks/install",
            ".github/core/shadow/Cargo.toml",
            ".github/helper.py",
            ".github/scripts/Cargo.toml",
        ] {
            let check = if path.starts_with(".config/") {
                validate_config_path
            } else if path.starts_with(".githooks/") {
                validate_githook_path
            } else {
                validate_github_path
            };
            assert!(rejected(path, check), "{path}");
        }
    }

    #[test]
    fn cargo_root_derives_only_exact_config_names_from_owner_authority() {
        for path in CARGO_CONFIG_PATHS {
            assert!(!rejected(path, validate_cargo_path), "{path}");
        }
        for path in [
            ".cargo/config/child.toml",
            ".cargo/config.toml/child",
            ".cargo/config.local",
        ] {
            assert!(rejected(path, validate_cargo_path), "{path}");
        }
    }
}
