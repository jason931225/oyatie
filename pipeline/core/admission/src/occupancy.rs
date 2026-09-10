//! Path occupancy. Combine is git; this refuses spawn of overlapping
//! path-sets. Inputs are NUL-delimited git name-status records, not prompts
//! or newline-delimited API projections.
//!
//! Occupancy governs what a lane *authored*. A path `.gitattributes` gives a
//! lane-concurrent merge attribute is exempt: that attribute is a standing
//! statement that independent lanes are expected to edit the path at the same
//! time, whether its content is generated or hand-written.
//!
//! That is a declaration of intent, not a guarantee. What the exemption buys
//! is that two lanes on such a path meet at MERGE, loudly, instead of both
//! being refused at spawn — where the refusal is symmetric, carries no
//! ordering, and bars the merge queue, so neither lane can act on it.
//!
//! Disjointness over authored paths is unchanged, and deliberately so.
//! ADR-0719 D-41 holds that a same-path dual write is an assignment rename
//! rather than a queue; that remains true for content a lane actually wrote.

use std::collections::BTreeSet;

/// Merge attributes by which this repository declares a path open to
/// concurrent lanes. An allowlist, not a pattern: see `declared_mergeable`.
///
/// Only `union` names a driver git implements. The rest name no binary in this
/// tree, so git falls back to its ordinary three-way text merge, which makes
/// the exemption -- not a structural merge -- the whole live effect of listing
/// one. Do not read a name here as evidence that a driver runs.
const LANE_CONCURRENT_MERGE_ATTRIBUTES: &[&str] = &[
    "union",
    "cargo-lock",
    "fixup-ledger",
    "friction-ledger",
    "concurrent-lanes",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccupiedSet {
    pub id: String,
    pub paths: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OccupancyRefused {
    EmptyPathSet,
    Overlap { path: String, other: String },
    UnsupportedMergePattern { pattern: String },
}

impl OccupancyRefused {
    pub fn message(&self) -> String {
        match self {
            Self::EmptyPathSet => "current pull request produced an empty path-set".to_owned(),
            Self::Overlap { path, other } => {
                format!("path {path:?} already occupied by {other}")
            }
            Self::UnsupportedMergePattern { pattern } => format!(
                "`.gitattributes` assigns a merge driver to {pattern:?}, which is not a literal \
                 path; occupancy cannot decide whether a changed path matches it"
            ),
        }
    }
}

/// Paths `.gitattributes` declares structurally mergeable.
///
/// Fails closed on a non-literal pattern: a glob would require occupancy to
/// reimplement gitattributes matching, and guessing wrong in the permissive
/// direction would silently drop authored paths from the set.
///
/// A slash-less pattern is treated as root-anchored where git would match it
/// at any depth. That direction is conservative — a nested `Cargo.lock` stays
/// authored and can only cause an extra refusal, never a missed one.
pub fn declared_mergeable(gitattributes: &str) -> Result<BTreeSet<String>, OccupancyRefused> {
    let mut declared = BTreeSet::new();
    for line in gitattributes.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(pattern) = fields.next() else {
            continue;
        };
        let Some(driver) = fields.find_map(|attribute| attribute.strip_prefix("merge=")) else {
            continue;
        };
        if !LANE_CONCURRENT_MERGE_ATTRIBUTES.contains(&driver) {
            // `merge=` alone proves nothing. Git's own `merge=binary` means
            // "take ours and declare a conflict", the opposite of combining.
            // `merge=text` is git's default merge spelled out, written to
            // defeat binary detection; it says nothing about lanes. Only a
            // listed value is that declaration, so an unknown one leaves the
            // path authored.
            continue;
        }
        if pattern.contains(['*', '?', '[']) || pattern.starts_with('/') {
            return Err(OccupancyRefused::UnsupportedMergePattern {
                pattern: pattern.to_owned(),
            });
        }
        declared.insert(pattern.to_owned());
    }
    Ok(declared)
}

/// The authored subset of a change: what the lane wrote, less what the
/// repository has declared its tooling may regenerate concurrently.
#[must_use]
pub fn authored_paths(
    changed: &BTreeSet<String>,
    mergeable: &BTreeSet<String>,
) -> BTreeSet<String> {
    changed.difference(mergeable).cloned().collect()
}

/// `in_flight` is other open PRs targeting `dev` (not this PR).
pub fn admit(this: &BTreeSet<String>, in_flight: &[OccupiedSet]) -> Result<(), OccupancyRefused> {
    if this.is_empty() {
        return Err(OccupancyRefused::EmptyPathSet);
    }
    for other in in_flight {
        if let Some(path) = this.intersection(&other.paths).next() {
            return Err(OccupancyRefused::Overlap {
                path: path.clone(),
                other: other.id.clone(),
            });
        }
    }
    Ok(())
}

/// Admit a change by its authored paths.
///
/// An empty *raw* change is still refused — a pull request that touched
/// nothing is a collection failure. A change whose every path is declared
/// mergeable is admitted: it authored nothing that another lane can collide
/// with, and the merge driver reconciles the rest.
pub fn admit_authored(
    raw_this: &BTreeSet<String>,
    in_flight: &[OccupiedSet],
    mergeable: &BTreeSet<String>,
) -> Result<(), OccupancyRefused> {
    if raw_this.is_empty() {
        return Err(OccupancyRefused::EmptyPathSet);
    }
    let this = authored_paths(raw_this, mergeable);
    if this.is_empty() {
        return Ok(());
    }
    let others: Vec<OccupiedSet> = in_flight
        .iter()
        .map(|other| OccupiedSet {
            id: other.id.clone(),
            paths: authored_paths(&other.paths, mergeable),
        })
        .filter(|other| !other.paths.is_empty())
        .collect();
    admit(&this, &others)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|s| (*s).to_string()).collect()
    }

    fn occupied(id: &str, paths: &[&str]) -> OccupiedSet {
        OccupiedSet {
            id: id.to_owned(),
            paths: set(paths),
        }
    }

    const ATTRIBUTES: &str = "\
# a comment
evidence/audit-chain.jsonl merge=union
Cargo.lock merge=cargo-lock
*.rs text eol=lf
";

    #[test]
    fn a_merge_driver_declares_a_path_structurally_mergeable() {
        let declared = declared_mergeable(ATTRIBUTES).expect("literal patterns parse");
        assert_eq!(declared, set(&["Cargo.lock", "evidence/audit-chain.jsonl"]));
        // `*.rs` carries no `merge=`, so it is authored content like any other.
        assert!(!declared.contains("*.rs"));
    }

    #[test]
    fn only_this_repositorys_own_drivers_earn_the_exemption() {
        // `merge=binary` is git's own: take ours and DECLARE A CONFLICT. It is
        // the opposite of combining, and accepting any `merge=` value would
        // have exempted a path on the strength of a word.
        let declared = declared_mergeable(
            "shared/thing.rs merge=binary\nother/thing.rs merge=text\nCargo.lock merge=cargo-lock\n",
        )
        .expect("literal patterns parse");
        assert_eq!(declared, set(&["Cargo.lock"]));
    }

    #[test]
    fn a_glob_with_a_merge_driver_fails_closed() {
        // Occupancy does not reimplement gitattributes matching. Guessing
        // permissively would drop authored paths out of the set.
        assert_eq!(
            declared_mergeable("packs/**/*.jsonl merge=union\n"),
            Err(OccupancyRefused::UnsupportedMergePattern {
                pattern: "packs/**/*.jsonl".to_owned()
            })
        );
    }

    #[test]
    fn two_lanes_that_share_only_the_lockfile_both_spawn() {
        // The wedge this fixes: every capability lane births or renames a
        // crate, so every one rewrites `Cargo.lock` and refused every other.
        let mergeable = declared_mergeable(ATTRIBUTES).expect("literal patterns parse");
        let birth = set(&["Cargo.lock", "policy/core/cedar-domain/Cargo.toml"]);
        let other = occupied(
            "pr-2272",
            &["Cargo.lock", "build/adapters/reindeer/src/lib.rs"],
        );
        assert_eq!(admit_authored(&birth, &[other], &mergeable), Ok(()));
    }

    #[test]
    fn a_shared_authored_path_is_still_refused() {
        // The relaxation is scoped to declared-mergeable paths. Two lanes
        // writing one source file remains an assignment error, not a queue.
        let mergeable = declared_mergeable(ATTRIBUTES).expect("literal patterns parse");
        let this = set(&["Cargo.lock", "iam/core/pdp-kernel/src/lib.rs"]);
        let other = occupied("pr-2272", &["Cargo.lock", "iam/core/pdp-kernel/src/lib.rs"]);
        assert_eq!(
            admit_authored(&this, &[other], &mergeable),
            Err(OccupancyRefused::Overlap {
                path: "iam/core/pdp-kernel/src/lib.rs".to_owned(),
                other: "pr-2272".to_owned(),
            })
        );
    }

    #[test]
    fn a_change_that_authored_nothing_collides_with_nothing() {
        let mergeable = declared_mergeable(ATTRIBUTES).expect("literal patterns parse");
        let lock_only = set(&["Cargo.lock"]);
        let other = occupied("pr-2272", &["Cargo.lock"]);
        assert_eq!(admit_authored(&lock_only, &[other], &mergeable), Ok(()));
    }

    #[test]
    fn an_empty_change_is_still_a_collection_failure() {
        let mergeable = declared_mergeable(ATTRIBUTES).expect("literal patterns parse");
        assert_eq!(
            admit_authored(&BTreeSet::new(), &[], &mergeable),
            Err(OccupancyRefused::EmptyPathSet)
        );
    }

    #[test]
    fn disjointness_over_authored_paths_is_unchanged() {
        assert_eq!(admit(&set(&["a"]), &[occupied("pr-1", &["b"])]), Ok(()));
        assert_eq!(
            admit(&set(&["a"]), &[occupied("pr-1", &["a"])]),
            Err(OccupancyRefused::Overlap {
                path: "a".to_owned(),
                other: "pr-1".to_owned()
            })
        );
        assert_eq!(
            admit(&BTreeSet::new(), &[]),
            Err(OccupancyRefused::EmptyPathSet)
        );
    }
}
