//! Cross-path completeness checks for new repository owners. Provenance: ADR-0719 D-8/D-36.

use std::collections::BTreeSet;

use crate::GitChangePaths;

use super::{
    ALLOWED_ROOT_DIRS, APP_PRODUCT_DIRS, BUILD_ROOT_DIRS, frozen_non_root_markdown_message,
    is_capability_root, is_frozen_non_root_markdown, layout_violations, live_apex_adr,
};

/// Apply repository-layout rules only to changed paths that remain after the Git diff. A new owner
/// must land as an implemented unit, never as paperwork or an unbuilt source
/// dump. The implemented unit is the proof; prose beside it is not.
pub fn changed_layout_violations(
    changes: &GitChangePaths,
    existing_owner_dirs: &BTreeSet<String>,
) -> Vec<String> {
    let mut violations = layout_violations(
        &changes
            .layout_candidates
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
    );
    // A live apex decision record is exempt from the Markdown freeze so the
    // repository can change its own mind in place. Amendment leaves the file
    // where it is: an endpoint that is occupied but does not survive the diff
    // is a delete or a copy/rename source, which erases the record instead.
    violations.extend(
        changes
            .occupied
            .iter()
            .filter(|path| {
                is_frozen_non_root_markdown(path)
                    || (live_apex_adr(path) && !changes.layout_candidates.contains(*path))
            })
            .map(|path| frozen_non_root_markdown_message(path)),
    );
    if owner_is_new_and_touched("base", changes, existing_owner_dirs) {
        require_core_crate("base", "BUILD root", changes, &mut violations);
    }
    for root in ALLOWED_ROOT_DIRS
        .iter()
        .chain(BUILD_ROOT_DIRS)
        .copied()
        .filter(|root| is_capability_root(root))
    {
        if owner_is_new_and_touched(root, changes, existing_owner_dirs) {
            require_core_crate(root, "capability owner", changes, &mut violations);
        }
    }
    for product in APP_PRODUCT_DIRS {
        let owner = format!("app/{product}");
        if owner_is_new_and_touched(&owner, changes, existing_owner_dirs) {
            require_core_crate(&owner, "BUILD product", changes, &mut violations);
        }
    }
    violations.sort();
    violations.dedup();
    violations
}

/// A conditional owner that had a complete core at the merge base may be
/// removed as a unit, but it cannot survive as paperwork-only scaffolding.
pub fn owner_core_regression_violations(
    changes: &GitChangePaths,
    complete_before: &BTreeSet<String>,
    live_after: &BTreeSet<String>,
    complete_after: &BTreeSet<String>,
) -> Vec<String> {
    complete_before
        .iter()
        .filter(|owner| owner_touched(owner, changes))
        .filter(|owner| live_after.contains(*owner) && !complete_after.contains(*owner))
        .map(|owner| {
            format!(
                "{owner}: deleting the last complete core crate while retaining the owner is forbidden"
            )
        })
        .collect()
}

fn owner_is_new_and_touched(
    owner: &str,
    changes: &GitChangePaths,
    existing_owner_dirs: &BTreeSet<String>,
) -> bool {
    !existing_owner_dirs.contains(owner) && owner_touched(owner, changes)
}

fn owner_touched(owner: &str, changes: &GitChangePaths) -> bool {
    changes
        .occupied
        .iter()
        .any(|path| path == owner || path.starts_with(&format!("{owner}/")))
}

fn require_core_crate(
    owner: &str,
    kind: &str,
    changes: &GitChangePaths,
    violations: &mut Vec<String>,
) {
    let prefix = format!("{owner}/core/");
    let has_crate = changes.layout_candidates.iter().any(|path| {
        let Some(crate_name) = path
            .strip_prefix(&prefix)
            .and_then(|rest| rest.strip_suffix("/src/lib.rs"))
        else {
            return false;
        };
        !crate_name.is_empty()
            && !crate_name.contains('/')
            && changes
                .layout_candidates
                .contains(&format!("{prefix}{crate_name}/Cargo.toml"))
    });
    if !has_crate {
        violations.push(format!(
            "{owner}: new {kind} requires one core crate with `Cargo.toml` and `src/lib.rs` in the same change"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_owner_cannot_lose_its_last_core_crate() {
        let changes = crate::git_change_paths_from_name_status_z(
            b"D\0policy/core/evaluate/Cargo.toml\0D\0policy/core/evaluate/src/lib.rs\0",
        )
        .unwrap();
        let complete_before = ["policy".to_owned()].into();
        let live_after = ["policy".to_owned()].into();
        assert!(
            !owner_core_regression_violations(
                &changes,
                &complete_before,
                &live_after,
                &BTreeSet::new(),
            )
            .is_empty()
        );
        assert!(
            owner_core_regression_violations(
                &changes,
                &complete_before,
                &BTreeSet::new(),
                &BTreeSet::new(),
            )
            .is_empty()
        );
    }
}
