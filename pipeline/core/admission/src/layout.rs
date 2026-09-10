//! Changed-path repository-layout admission. Provenance: ADR-0719 D-8.
//!
//! Unknown top-level names are red. Absence of a listed name is not red
//! (BUILD vs DONE). Capability and `app/<product>/` children are hexagonal
//! faces.

use std::collections::BTreeSet;

mod base;
mod build;
mod cargo_config;
mod change;
mod classification;
mod dependency;
mod inner;
mod manifest;
mod payload;
mod port_implementation;
mod proto;
mod root_meta;
mod test_fixture;
mod workspace;

pub use base::base_admission_violations;
pub use cargo_config::{CARGO_CONFIG_PATHS, cargo_config_violations};
pub use change::{changed_layout_violations, owner_core_regression_violations};
pub use classification::deleted_classification_violations;
pub use dependency::{draft_dependency_violations, workspace_draft_dependency_violations};
use inner::validate_owner_path;
pub use manifest::{
    cargo_entrypoint, cargo_entrypoints, cargo_manifest_for_crate_path,
    cargo_manifest_for_entrypoint, cargo_manifest_violations,
};
pub use port_implementation::{ChangedSource, port_implementation_violations};
pub use proto::proto_package_violations;
pub use workspace::{WORKSPACE_EXCLUDES, WORKSPACE_MEMBER_GLOBS, workspace_membership_violations};

/// Admitted root directories that are present on `dev` and therefore require
/// OWNERS/CODEOWNERS coverage.
pub const ALLOWED_ROOT_DIRS: &[&str] = &[
    "app",
    "audit",
    "billing",
    "build",
    "bus",
    "cell",
    "compliance",
    "compute",
    "data",
    "docs",
    "flags",
    "gateway",
    "iac",
    "iam",
    "intelligence",
    "k8s",
    "marketplace",
    "network",
    "observability",
    "packs",
    "pipeline",
    "secrets",
    "storage",
    "templates",
    "tenancy",
    "third-party",
];

/// Conditional roots that are valid only once a real core lands. `base` also
/// requires the ADR-0719 three-capability review; absence never requires an
/// empty OWNERS scaffold.
pub const BUILD_ROOT_DIRS: &[&str] = &["base", "notify", "policy", "workflow"];

/// ADR-0719 D-22's closed v1 product roster. Missing products are BUILD, not
/// membership ghosts, and must arrive with implementation content.
pub const APP_PRODUCT_DIRS: &[&str] = &[
    "accounting",
    "application",
    "calendar",
    "community",
    "drive",
    "foundry",
    "hr",
    "ledger",
    "mail",
    "messenger",
    "payments",
    "payroll",
];

/// Closed dot-directory set. Existing dot-root debt is removal-only rather
/// than a wildcard exemption for new tracked paths.
pub const ALLOWED_DOT_ROOT_DIRS: &[&str] = &[".cargo", ".config", ".github", ".githooks"];

pub const ALLOWED_ROOT_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    "LICENSE",
    "OWNERS",
    "AGENTS.md",
    "CLAUDE.md",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "deny.toml",
    "reindeer.toml",
    ".buckconfig",
    ".buckroot",
    ".gitattributes",
    ".gitignore",
];

pub const FACES: &[&str] = &["core", "ports", "adapters", "facade"];

pub const CAP_EXTRAS: &[&str] = &["cedar", "observability", "iac"];

pub const FORBIDDEN_NAMES: &[&str] = &[
    "plan",
    "tasks",
    "contracts",
    "specs",
    "libs",
    "tools",
    "infra",
    "kernel",
    "os",
    "governance",
    "console",
    "tests",
    "e2e",
];

pub const META_ROOTS: &[&str] = &["app", "build", "docs", "templates", "third-party"];

/// Root data loaded by capabilities but not itself a capability or Cargo face.
pub const DATA_ROOTS: &[&str] = &["packs"];

pub(crate) fn path_parts(path: &str) -> Vec<&str> {
    path.split('/').filter(|part| !part.is_empty()).collect()
}

fn is_meta_root(root: &str) -> bool {
    META_ROOTS.contains(&root)
}

pub fn is_capability_root(root: &str) -> bool {
    (ALLOWED_ROOT_DIRS.contains(&root) || BUILD_ROOT_DIRS.contains(&root))
        && root != "base"
        && !is_meta_root(root)
        && !DATA_ROOTS.contains(&root)
}

pub fn cap_root_file_ok(name: &str) -> bool {
    matches!(name, "OWNERS" | "BUCK")
}

pub fn face_dir_ok(child: &str) -> bool {
    FACES.contains(&child) || CAP_EXTRAS.contains(&child)
}

/// Report violations on live changed paths. A current-tree walk is separate
/// reorganization follow-through; this is the born-blocking pattern engine.
pub fn layout_violations(changed_files: &[String]) -> Vec<String> {
    let allowed_dirs: BTreeSet<&str> = ALLOWED_ROOT_DIRS
        .iter()
        .chain(BUILD_ROOT_DIRS)
        .chain(ALLOWED_DOT_ROOT_DIRS)
        .copied()
        .collect();
    let allowed_files: BTreeSet<&str> = ALLOWED_ROOT_FILES.iter().copied().collect();
    let forbidden: BTreeSet<&str> = FORBIDDEN_NAMES.iter().copied().collect();
    let mut violations = Vec::new();
    for file in changed_files {
        if invalid_git_path(file) {
            violations.push(format!(
                "{file}: invalid Git path spelling; `/` is the only separator"
            ));
            continue;
        }
        if is_frozen_non_root_markdown(file) {
            violations.push(frozen_non_root_markdown_message(file));
            continue;
        }
        let parts = path_parts(file);
        let Some(root) = parts.first().copied() else {
            continue;
        };
        if forbidden.contains(root) {
            violations.push(format!("{file}: forbidden root `{root}`"));
            continue;
        }
        if parts.len() == 1 {
            if !allowed_files.contains(root) {
                violations.push(format!("{file}: unknown root file `{root}`"));
            }
            continue;
        }
        if !allowed_dirs.contains(root) {
            violations.push(format!("{file}: unknown root `{root}`"));
            continue;
        }
        if root == ".cargo" {
            root_meta::validate_cargo_path(file, &parts, &mut violations);
        } else if root == ".config" {
            root_meta::validate_config_path(file, &parts, &mut violations);
        } else if root == ".github" {
            root_meta::validate_github_path(file, &parts, &mut violations);
        } else if root == ".githooks" {
            root_meta::validate_githook_path(file, &parts, &mut violations);
        } else if root == "app" {
            validate_app_path(file, &parts, &mut violations);
        } else if root == "base" {
            root_meta::validate_base_path(file, &parts, &mut violations);
        } else if root == "docs" {
            root_meta::validate_docs_path(file, &parts, &mut violations);
        } else if root == "packs" {
            root_meta::validate_packs_path(file, &parts, &mut violations);
        } else if root == "templates" {
            root_meta::validate_templates_path(file, &parts, &mut violations);
        } else if root == "build" {
            build::validate_build_path(file, &parts, &mut violations);
        } else if is_meta_root(root) && parts.get(1).is_some_and(|child| FACES.contains(child)) {
            violations.push(format!(
                "{file}: meta root `{root}` cannot contain owner Cargo face `{}`",
                parts[1]
            ));
        } else if is_capability_root(root) {
            validate_owner_path(file, &parts, 1, &mut violations);
        }
    }
    violations
}

fn invalid_git_path(path: &str) -> bool {
    path.is_empty()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
}

pub(crate) fn is_frozen_non_root_markdown(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    (lower.ends_with(".md") || lower.ends_with(".markdown"))
        && !matches!(path, "README.md" | "AGENTS.md" | "CLAUDE.md")
}

pub(crate) fn frozen_non_root_markdown_message(path: &str) -> String {
    format!("{path}: frozen non-root Markdown cannot be changed or used as a copy source")
}

fn validate_app_path(file: &str, parts: &[&str], violations: &mut Vec<String>) {
    if parts.len() == 2 {
        if parts[1] != "OWNERS" {
            violations.push(format!("{file}: app-root file `{}` not allowed", parts[1]));
        }
        return;
    }
    let product = parts[1];
    if !APP_PRODUCT_DIRS.contains(&product) {
        violations.push(format!("{file}: unknown app product `{product}`"));
        return;
    }
    validate_owner_path(file, parts, 2, violations);
}
