//! Two fences on the `engine_digest` axis, each proving one half, because neither is worth anything
//! alone: the manifest is the whole engine, and the digest moves when the manifest does. A complete
//! manifest hashed by an insensitive function reports a constant; a sensitive function over a
//! partial manifest reports a constant for every change to the part it cannot see.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use port_engine_app::engine::{engine_crates, engine_digest};

#[test]
fn the_manifest_is_the_whole_engine() {
    let root = engine_root()
        .expect("the engine tree must be locatable — a fence that cannot look has not looked");

    let mut on_disk = BTreeSet::new();
    collect(&root, &root, &mut on_disk);

    let mut embedded = BTreeSet::new();
    for (name, sources) in engine_crates() {
        for (path, _) in sources {
            embedded.insert(format!("{name}/{path}"));
        }
    }

    assert_eq!(
        embedded, on_disk,
        "an engine source exists that `engine_digest` does not hash — regenerate the crate \
         manifests, because a change to that file would alter output with no receipt axis to \
         account for it"
    );
}

/// Every crate is represented, and none twice.
///
/// The count is separate from the file set on purpose: a crate whose manifest is empty contributes
/// no paths, so the set comparison above would pass while an entire crate went unhashed.
#[test]
fn every_crate_contributes_and_none_twice() {
    let crates = engine_crates();
    let names: BTreeSet<&str> = crates.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names.len(),
        crates.len(),
        "a crate is listed twice, which would hash its sources twice and mean nothing"
    );
    for (name, sources) in &crates {
        assert!(
            !sources.is_empty(),
            "crate `{name}` contributes no sources, so nothing in it moves the axis"
        );
    }
}

/// The digest moves when the engine does.
///
/// Perturbs a COPY of the preimage rather than a file on disk, so the fence is hermetic.
#[test]
fn a_changed_source_moves_the_digest() {
    let baseline = port_engine_identity::engine_preimage(&engine_crates());
    assert!(
        baseline.len() > 1024,
        "the preimage must cover real sources"
    );

    for index in [
        0,
        baseline.len() / 3,
        baseline.len() / 2,
        baseline.len() - 1,
    ] {
        let mut perturbed = baseline.clone();
        perturbed[index] = perturbed[index].wrapping_add(1);
        assert_ne!(
            port_engine_hash::digest_bytes(&baseline),
            port_engine_hash::digest_bytes(&perturbed),
            "perturbing byte {index} of the engine preimage must move the digest"
        );
    }
}

#[test]
fn the_digest_is_a_stable_sha256() {
    let digest = engine_digest();
    assert!(digest.0.starts_with("sha256:"));
    assert_eq!(digest.0.len(), "sha256:".len() + 64);
    assert_eq!(
        digest,
        engine_digest(),
        "the digest must not vary between calls"
    );
}

/// The manifest order is sorted by crate name, and stays that way.
///
/// The order is part of the preimage, so a reordering moves the digest with no engine change behind
/// it. Any fixed order would do; this one is checkable, which is why it is the one asserted rather
/// than described.
#[test]
fn engine_crates_are_sorted_by_crate_name() {
    let names: Vec<&str> = engine_crates().iter().map(|(name, _)| *name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(
        names, sorted,
        "the crate list must stay sorted, or the digest moves without the engine changing"
    );
}

fn engine_root() -> Option<PathBuf> {
    let manifest = option_env!("CARGO_MANIFEST_DIR")?;
    // .../build/port-engine/facade/app → .../build/port-engine
    let root = Path::new(manifest).parent()?.parent()?;
    root.is_dir().then(|| root.to_path_buf())
}

/// Every production `.rs` under the engine, keyed the way the manifests key them:
/// `<crate-name>/<path-within-src>`.
fn collect(root: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            // `target/` is build output. `gosrc/` is the out-of-band front end: it is not compiled
            // into this binary, so it is not part of what produced the output — the SNAPSHOT axis
            // is what covers a front-end change, which is the correct axis for it.
            if name != "target" && name != "gosrc" {
                collect(root, &path, out);
            }
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        // Production sources only: `tests/` are not linked into the engine.
        let Some((crate_dir, within)) = relative.split_once("/src/") else {
            continue;
        };
        let Some(leaf) = crate_dir.rsplit('/').next() else {
            continue;
        };
        out.insert(format!("{}/{within}", crate_name(leaf)));
    }
}

fn crate_name(leaf: &str) -> String {
    format!("port-engine-{leaf}")
}
