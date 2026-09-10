//! The crate-local Cedar seeds must stay byte-identical to the canonical ones
//! in `iam/core/platform-contracts-kernel/cedar/`. The copies exist only
//! because buck2 targets sandbox their srcs; this guard makes the duplication
//! drift-impossible on the cargo lane.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

// option_env!, not env!: CARGO_MANIFEST_DIR is undefined in buck2's hermetic
// sandbox, and this target must still compile there. Cargo enforces the parity;
// buck2 skips with a notice.
fn manifest_dir() -> Option<&'static Path> {
    option_env!("CARGO_MANIFEST_DIR").map(Path::new)
}

fn repo_root() -> Option<PathBuf> {
    let mut dir = manifest_dir()?.to_path_buf();
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs/decisions").is_dir() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[test]
fn crate_local_cedar_seeds_match_canonical() {
    const PAIRS: &[(&str, &str)] = &[
        (
            "cedar/platform.cedarschema",
            "iam/core/platform-contracts-kernel/cedar/platform.cedarschema",
        ),
        (
            "cedar/platform-policies.cedar",
            "iam/core/platform-contracts-kernel/cedar/platform-policies.cedar",
        ),
        (
            "cedar/platform-templates.cedar",
            "iam/core/platform-contracts-kernel/cedar/platform-templates.cedar",
        ),
    ];
    let (Some(crate_dir), Some(root)) = (manifest_dir(), repo_root()) else {
        eprintln!(
            "cedar_seed_parity: repo root marker not reachable (hermetic sandbox); \
             skipped {} pairs — cargo CI lane enforces parity",
            PAIRS.len()
        );
        return;
    };
    let mut mismatches = Vec::new();
    for (local, canonical) in PAIRS {
        let local_bytes = std::fs::read(crate_dir.join(local))
            .unwrap_or_else(|e| panic!("crate-local cedar seed missing: {local}: {e}"));
        let canonical_path = root.join(canonical);
        let canonical_bytes = std::fs::read(&canonical_path).unwrap_or_else(|e| {
            panic!(
                "canonical cedar seed missing: {}: {e}",
                canonical_path.display()
            )
        });
        if local_bytes != canonical_bytes {
            mismatches.push((*local).to_owned());
        }
    }
    assert!(
        mismatches.is_empty(),
        "crate-local cedar seeds drifted from canonical: {mismatches:?} — \
         re-copy from iam/core/platform-contracts-kernel/cedar/"
    );
}
