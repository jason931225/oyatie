use std::collections::{BTreeMap, BTreeSet};

use pipeline_admission::layout::live_apex_adr;
use pipeline_admission::{
    ALLOWED_ROOT_DIRS, APP_PRODUCT_DIRS, BUILD_ROOT_DIRS, CARGO_CONFIG_PATHS,
    cargo_config_violations, comment_run_violations, file_budget_violations, is_capability_root,
};
use pipeline_repository_draft::{RepositoryEntryKind, RepositoryRead};

#[derive(Default)]
pub(super) struct OwnerTreeState {
    pub(super) live: BTreeSet<String>,
    pub(super) complete: BTreeSet<String>,
}

pub(super) fn owner_tree_state(
    repository: &impl RepositoryRead,
    commit: &str,
) -> Result<OwnerTreeState, String> {
    let mut state = OwnerTreeState::default();
    let capability_owners = ALLOWED_ROOT_DIRS
        .iter()
        .chain(BUILD_ROOT_DIRS)
        .copied()
        .filter(|owner| *owner == "base" || is_capability_root(owner))
        .map(str::to_owned);
    let app_owners = APP_PRODUCT_DIRS
        .iter()
        .map(|product| format!("app/{product}"));
    for owner in capability_owners.chain(app_owners) {
        if !repository.directory_exists(commit, &owner)? {
            continue;
        }
        state.live.insert(owner.clone());
        if owner_has_complete_core(repository, commit, &owner)? {
            state.complete.insert(owner.clone());
        }
    }
    Ok(state)
}

fn owner_has_complete_core(
    repository: &impl RepositoryRead,
    commit: &str,
    owner: &str,
) -> Result<bool, String> {
    let prefix = format!("{owner}/core/");
    let files: BTreeSet<String> = repository
        .files_under(commit, &format!("{owner}/core"))?
        .into_iter()
        .collect();
    for manifest in files.iter().filter(|path| {
        path.strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix("/Cargo.toml"))
            .is_some_and(|leaf| !leaf.is_empty() && !leaf.contains('/'))
    }) {
        let directory = manifest
            .strip_suffix("/Cargo.toml")
            .expect("filtered manifest suffix");
        let entrypoint = format!("{directory}/src/lib.rs");
        if files.contains(&entrypoint)
            && regular_blob(repository.entry_kind(commit, manifest)?)
            && regular_blob(repository.entry_kind(commit, &entrypoint)?)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn repository_cargo_config_violations(
    repository: &impl RepositoryRead,
    head: &str,
) -> Result<Vec<String>, String> {
    let mut violations = Vec::new();
    for path in CARGO_CONFIG_PATHS {
        match repository.entry_kind(head, path)? {
            None => {}
            Some(kind) if regular_blob(Some(kind)) => {
                let contents = repository.blob_text(head, path)?;
                violations.extend(cargo_config_violations(path, &contents));
            }
            Some(kind) => violations.push(format!(
                "{path}: Cargo configuration must be a regular Git blob, got {kind:?}"
            )),
        }
    }
    Ok(violations)
}

/// Budgets charged against the content of every changed live path. Both are
/// path-keyed, so both are judged at a rename's source.
type TouchedContentRule = fn(&str, &[u8]) -> Vec<String>;
const TOUCHED_CONTENT_RULES: [TouchedContentRule; 2] =
    [file_budget_violations, comment_run_violations];

pub(super) fn live_candidate_violations(
    repository: &impl RepositoryRead,
    head: &str,
    candidates: &BTreeSet<String>,
    exact_rename_sources: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let mut violations = Vec::new();
    for path in candidates {
        match repository.entry_kind(head, path)? {
            Some(kind) if regular_blob(Some(kind)) => {
                let contents = repository.blob_bytes(head, path)?;
                // Relocating a file that ALREADY broke a budget charges its
                // debt to whoever moved it, which is why the exception exists.
                // But both budgets are path-keyed, so the exemption must be
                // judged at the SOURCE: grading the same bytes where they came
                // from is the difference between forgiving pre-existing debt
                // and laundering oversized content out of an exempt path into
                // a budgeted one, which no longer costs anybody anything.
                for rule in TOUCHED_CONTENT_RULES {
                    let already_owed = exact_rename_sources
                        .get(path)
                        .is_some_and(|source| !rule(source, &contents).is_empty());
                    if !already_owed {
                        violations.extend(rule(path, &contents));
                    }
                }
            }
            Some(kind) => violations.push(format!(
                "{path}: live changed content must be a regular Git blob, got {kind:?}"
            )),
            None => violations.push(format!(
                "{path}: live changed path is absent at the head commit"
            )),
        }
    }
    Ok(violations)
}

/// Decision id to live path for every decision record the corpus still serves
/// as current law, and can therefore amend in place.
pub(super) fn amendable_decision_paths(
    repository: &impl RepositoryRead,
    commit: &str,
) -> Result<BTreeMap<String, String>, String> {
    const DECISIONS: &str = "docs/decisions";
    if !repository.directory_exists(commit, DECISIONS)? {
        return Ok(BTreeMap::new());
    }
    Ok(repository
        .files_under(commit, DECISIONS)?
        .into_iter()
        .filter(|path| live_apex_adr(path))
        .filter_map(|path| Some((decision_id(&path)?, path)))
        .collect())
}

fn decision_id(path: &str) -> Option<String> {
    let (_, name) = path.rsplit_once('/')?;
    name.get(..8).map(str::to_owned)
}

pub(super) fn regular_blob(kind: Option<RepositoryEntryKind>) -> bool {
    matches!(
        kind,
        Some(RepositoryEntryKind::Blob | RepositoryEntryKind::ExecutableBlob)
    )
}

pub(super) fn reject_indirect_dependency_components(
    repository: &impl RepositoryRead,
    head: &str,
    visited: &[String],
) -> Result<(), String> {
    for path in visited {
        match repository.entry_kind(head, path)? {
            Some(RepositoryEntryKind::Symlink) => {
                return Err(format!("tracked symlink component `{path}` is forbidden"));
            }
            Some(RepositoryEntryKind::Gitlink) => {
                return Err(format!("tracked gitlink component `{path}` is forbidden"));
            }
            _ => {}
        }
    }
    Ok(())
}
