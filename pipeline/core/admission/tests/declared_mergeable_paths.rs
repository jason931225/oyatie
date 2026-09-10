//! The exemption set, checked against the repository's real `.gitattributes`.
//!
//! The unit tests for `declared_mergeable` use a synthetic fixture, which is
//! enough to pin the parser and nothing else. Two things it cannot catch: the
//! real file drifting away from the driver allowlist, and the whole feature
//! being inert. The first version of the trunk read shipped dead — it went
//! through an object-id validator that rejects any non-hex byte, so
//! `.gitattributes` failed at its first character and the exemption set was
//! always empty — while every unit test, the substring freeze on the call
//! site, clippy, fmt and the layout gate stayed green.

use std::path::{Path, PathBuf};

use std::collections::BTreeSet;

use pipeline_admission::{OccupancyRefused, OccupiedSet, admit_authored, declared_mergeable};

/// The admission crate root: one `pub mod` line and one `pub use` group per
/// module, so every lane that adds a module must append to it.
const CRATE_ROOT: &str = "pipeline/core/admission/src/lib.rs";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

fn attributes() -> String {
    std::fs::read_to_string(repo_root().join(".gitattributes")).expect("repository .gitattributes")
}

#[test]
fn the_real_gitattributes_declares_the_hub_files() {
    // Equality, not containment. A listed name buys an exemption and nothing
    // else -- no driver runs behind any of these -- so a new `merge=` line
    // silently lifts its path out of occupancy. Pinning the whole set makes a
    // fifth exempt path cost a deliberate edit here.
    let declared = declared_mergeable(&attributes()).expect("the real file parses");
    assert_eq!(
        declared,
        paths(&[
            "Cargo.lock",
            "ci/facade/action-item-accounting/friction-ledger.jsonl",
            "evidence/audit-chain.jsonl",
            CRATE_ROOT,
            "registry/fixuptasks.jsonl",
        ]),
        "the lockfile and the three ledgers carry merge drivers and the crate \
         root is this rule's newest exemption; an unexpected member is a path \
         that quietly stopped bearing occupancy"
    );
}

#[test]
fn every_declared_merge_attribute_is_one_occupancy_recognises() {
    // Drift guard. A future `merge=` line naming a driver outside the allowlist
    // leaves its path occupancy-bearing, which is the safe direction but a
    // silent one — the author would see refusals and no explanation. Fail here
    // instead, at the place that decides.
    let text = attributes();
    let mut undeclared = Vec::new();
    let mut unsupported = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split_whitespace();
        let Some(pattern) = fields.next() else {
            continue;
        };
        if let Some(driver) = fields.find_map(|a| a.strip_prefix("merge=")) {
            match declared_mergeable(&format!("{pattern} merge={driver}\n")) {
                // The driver is recognised and the pattern is literal.
                Ok(set) if !set.is_empty() => {}
                // Recognised driver, but the pattern is not one occupancy can
                // match. Distinct from an unknown driver: same safe direction,
                // different cause, and conflating them sends the next reader
                // to the allowlist when the problem is the glob.
                Err(reason) => unsupported.push(format!("{pattern}: {}", reason.message())),
                Ok(_) => undeclared.push(format!("{pattern} -> merge={driver}")),
            }
        }
    }
    assert!(
        unsupported.is_empty(),
        "`.gitattributes` assigns a merge driver to a pattern occupancy cannot \
         match, so the path stays occupancy-bearing for a reason that has \
         nothing to do with the allowlist: {unsupported:?}"
    );
    assert!(
        undeclared.is_empty(),
        "`.gitattributes` names merge drivers occupancy does not recognise, so \
         these paths silently stay occupancy-bearing: {undeclared:?}"
    );
}

fn paths(paths: &[&str]) -> BTreeSet<String> {
    paths.iter().map(|path| (*path).to_owned()).collect()
}

fn lane(id: &str, paths_of: &[&str]) -> OccupiedSet {
    OccupiedSet {
        id: id.to_owned(),
        paths: paths(paths_of),
    }
}

#[test]
fn the_crate_root_every_lane_appends_to_is_declared() {
    let declared = declared_mergeable(&attributes()).expect("the real file parses");
    assert!(
        declared.contains(CRATE_ROOT),
        "{CRATE_ROOT} is the file two lanes adding unrelated modules both have \
         to touch; undeclared, they refuse each other and neither can merge"
    );
}

#[test]
fn two_lanes_adding_different_modules_both_admit() {
    // The wedge, from the real pair: #2449 added `execution_toolchain`, #2456
    // added `data_class`, the crate root was the only path they shared, and
    // the exit was closing #2449.
    let mergeable = declared_mergeable(&attributes()).expect("the real file parses");
    let this = paths(&[
        CRATE_ROOT,
        "pipeline/core/admission/src/execution_toolchain.rs",
    ]);
    let other = lane(
        "pr-2456",
        &[CRATE_ROOT, "pipeline/core/admission/src/data_class.rs"],
    );
    assert_eq!(admit_authored(&this, &[other], &mergeable), Ok(()));
}

#[test]
fn two_lanes_sharing_a_module_body_are_still_refused() {
    // The other side. Declaring the crate root declares the crate root: two
    // lanes writing one module remain an assignment error, not a queue.
    let mergeable = declared_mergeable(&attributes()).expect("the real file parses");
    let shared = "pipeline/core/admission/src/occupancy.rs";
    let this = paths(&[CRATE_ROOT, shared]);
    assert_eq!(
        admit_authored(&this, &[lane("pr-2456", &[CRATE_ROOT, shared])], &mergeable),
        Err(OccupancyRefused::Overlap {
            path: shared.to_owned(),
            other: "pr-2456".to_owned(),
        })
    );
}

#[test]
fn a_lane_that_only_touches_the_crate_root_collides_with_nothing() {
    // The cost of the declaration, stated. Two lanes whose whole change is the
    // crate root both spawn and meet in the merge queue -- a dequeue plus a
    // rebase, and a rebase voids a running suite. Still the better side of the
    // trade: a conflict an author can resolve, over a refusal neither can.
    let mergeable = declared_mergeable(&attributes()).expect("the real file parses");
    let this = paths(&[CRATE_ROOT]);
    assert_eq!(
        admit_authored(&this, &[lane("pr-2456", &[CRATE_ROOT])], &mergeable),
        Ok(())
    );
}
