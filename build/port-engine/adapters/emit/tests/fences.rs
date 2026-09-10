//! ADR-0638 (archived; live via apex ADR-0704) D1 puts generated Kubernetes output under `k8s/`,
//! and ADR-0637 (archived; live via apex ADR-0704) D4 places corpus expansion expressly outside
//! the W0 authorization. So emit refuses that destination root rather than writing into it.
//!
//! Scanned over the WHOLE crate: the set is enumerated and then PROVEN to be the whole of `src/`.

use std::collections::BTreeSet;
use std::path::Path;

/// Every production source of this crate, read at compile time.
const PRODUCTION_SOURCES: &[(&str, &str)] = &[
    ("canary.rs", include_str!("../src/canary.rs")),
    ("error.rs", include_str!("../src/error.rs")),
    ("lib.rs", include_str!("../src/lib.rs")),
    ("materialize.rs", include_str!("../src/materialize.rs")),
    ("sources.rs", include_str!("../src/sources.rs")),
];

/// The enumeration must BE the directory, not a subset somebody once curated.
#[test]
fn scanned_sources_are_the_whole_crate() {
    let candidates = [
        option_env!("CARGO_MANIFEST_DIR").map(|dir| Path::new(dir).join("src")),
        Some(Path::new("src").to_path_buf()),
        // The crate's REPO-RELATIVE path. buck2 runs a test from the project root with no cargo
        // environment, so neither candidate above resolves there — and this fence guards a property
        // of that build too.
        Some(Path::new("build/port-engine/adapters/emit/src").to_path_buf()),
    ];
    let src = candidates
        .into_iter()
        .flatten()
        .find(|path| path.is_dir())
        .expect("this crate's src/ must be locatable — a fence that cannot look has not looked");

    let on_disk: BTreeSet<String> = std::fs::read_dir(&src)
        .unwrap_or_else(|err| panic!("src/ must be readable to be scanned: {err}"))
        .map(|entry| entry.expect("readable dir entry").file_name())
        .filter_map(|name| name.to_str().map(ToOwned::to_owned))
        .filter(|name| name.ends_with(".rs"))
        .collect();

    let scanned: BTreeSet<String> = PRODUCTION_SOURCES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();

    assert_eq!(
        scanned, on_disk,
        "a source file exists that no architecture fence reads — add it to PRODUCTION_SOURCES"
    );

    let embedded: BTreeSet<String> = port_engine_emit::CRATE_SOURCES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(
        embedded, on_disk,
        "CRATE_SOURCES must BE the crate: the engine-identity axis hashes only what it lists, so \
         a source missing from it is a source an engine change can move without moving the digest"
    );
}

#[test]
fn a_production_source_refuses_the_corpus_destination() {
    let destination = ["k", "8", "s"].concat();
    let under_corpus = Path::new("/tmp")
        .join(&destination)
        .join(port_engine_emit::EMIT_OUT_DIRNAME);
    assert!(
        port_engine_emit::validate_emit_out_dir(&under_corpus).is_err(),
        "a destination under `{destination}/` must be refused by the validator itself, not \
         merely mentioned somewhere in the sources"
    );
}
