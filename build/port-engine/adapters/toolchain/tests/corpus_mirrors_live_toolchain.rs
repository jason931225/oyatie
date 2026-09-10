//! The toolchain receipt axis must notice its subject changed.
//!
//! `toolchain_digest` binds package-local `.txt` mirrors of `build/toolchains/**` rather than the
//! live files: the digest is a compile-time constant and `include_str!` outside the crate is not
//! visible to `rust_library`'s srcs glob. That embed is the only reason the receipt is hermetic --
//! and, unenforced, the only reason it can lie. The axis is documented as the toolchain IN FORCE,
//! so a mirror that stops equalling its live file makes an emitted-byte change caused by the
//! toolchain unattributable to any receipt axis.
//!
//! Nothing compared them until this fence, and `cache/OWNERS` had already drifted on `dev`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use port_engine_toolchain::{CORPUS_MIRRORS, CORPUS_PATHS};

/// Locate the live `build/toolchains/` this crate mirrors.
///
/// Three candidates because three runners look for it: cargo sets `CARGO_MANIFEST_DIR`, buck2 sets
/// nothing and runs from the project root, and a plain run starts in the crate directory.
fn live_toolchains() -> PathBuf {
    [
        option_env!("CARGO_MANIFEST_DIR").map(|dir| Path::new(dir).join("../../../toolchains")),
        Some(PathBuf::from("../../../toolchains")),
        Some(PathBuf::from("build/toolchains")),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.join("BUCK").is_file())
    .expect("build/toolchains/ must be locatable -- a fence that cannot look has not looked")
}

/// Every relative file path under the live toolchain tree.
fn live_paths(root: &Path, dir: &Path, out: &mut BTreeSet<String>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("{} must be readable to be scanned: {err}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            live_paths(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.insert(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Every mirrored byte the digest binds still equals the live file it claims to mirror.
#[test]
fn every_mirror_equals_the_live_toolchain_file() {
    let root = live_toolchains();
    for (path, mirrored) in CORPUS_MIRRORS {
        let live = std::fs::read_to_string(root.join(path))
            .unwrap_or_else(|err| panic!("build/toolchains/{path} must be readable: {err}"));
        assert_eq!(
            live, mirrored,
            "build/toolchains/{path} no longer equals the mirror `toolchain_digest` binds, so the \
             receipt would keep attesting to a toolchain that no longer exists. Copy the live \
             bytes into src/corpus/ and re-pin the frozen digest in the same change."
        );
    }
}

/// The bound set IS the toolchain tree, not a subset somebody once curated.
///
/// Byte-parity alone cannot see a file the corpus never mirrored: the loop has nothing to compare.
/// A toolchain that GROWS a file is the same defect as one that changes a file.
#[test]
fn the_bound_corpus_is_the_whole_toolchain_tree() {
    let root = live_toolchains();
    let mut on_disk = BTreeSet::new();
    live_paths(&root, &root, &mut on_disk);
    let bound: BTreeSet<String> = CORPUS_PATHS.iter().map(|path| (*path).to_owned()).collect();
    assert_eq!(
        bound, on_disk,
        "build/toolchains/ holds a file the toolchain receipt axis does not bind, so a change to \
         it would move no digest. Mirror it into src/corpus/, add it to CORPUS_PATHS and \
         CORPUS_MIRRORS, and re-pin the frozen digest in the same change."
    );
}
