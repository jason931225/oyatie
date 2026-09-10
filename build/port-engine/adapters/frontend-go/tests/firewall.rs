//! ADR-0638 D3 architecture fence: the Go firewall, scanned over the WHOLE crate.
//!
//! The scanned set is enumerated, and then PROVEN to be the whole of `src/`. Enumeration alone
//! would rot the first time somebody adds a module and forgets this list; the completeness test is
//! what makes forgetting fail loudly instead of silently widening what may enter the crate.

use std::collections::BTreeSet;
use std::path::Path;

/// Every production source of this crate, read at compile time.
const PRODUCTION_SOURCES: &[(&str, &str)] = &[
    ("convert.rs", include_str!("../src/convert.rs")),
    ("error.rs", include_str!("../src/error.rs")),
    ("lib.rs", include_str!("../src/lib.rs")),
    ("model.rs", include_str!("../src/model.rs")),
    ("vocabulary.rs", include_str!("../src/vocabulary.rs")),
    ("wire.rs", include_str!("../src/wire.rs")),
    ("sources.rs", include_str!("../src/sources.rs")),
];

/// The enumeration above must BE the directory, not a subset somebody once curated.
///
/// Fail-closed on an unreadable directory: a fence that cannot list what it is meant to cover has
/// not found nothing, it has failed to look.
#[test]
fn scanned_sources_are_the_whole_crate() {
    // Two candidate roots because two build systems run this. Cargo sets CARGO_MANIFEST_DIR and
    // is the ADR-0716 merge path; buck2 does not set it and runs from a sandbox whose layout is
    // its own. Trying both and failing when NEITHER resolves keeps the check total under either,
    // rather than silently passing wherever the first guess happens to be wrong.
    let candidates = [
        option_env!("CARGO_MANIFEST_DIR").map(|dir| Path::new(dir).join("src")),
        Some(Path::new("src").to_path_buf()),
        // The crate's REPO-RELATIVE path. buck2 runs a test from the project root with no cargo
        // environment, so neither candidate above resolves there — and this fence guards a property
        // of that build too.
        Some(Path::new("build/port-engine/adapters/frontend-go/src").to_path_buf()),
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

    let embedded: BTreeSet<String> = port_engine_frontend_go::CRATE_SOURCES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    assert_eq!(
        embedded, on_disk,
        "CRATE_SOURCES must BE the crate: the engine-identity axis hashes only what it lists, so \
         a source missing from it is a source an engine change can move without moving the digest"
    );
}

/// The process half: library sources used by verify must never spawn the source toolchain.
#[test]
fn no_production_source_spawns_the_go_toolchain() {
    // Needles built piecewise so this test's own text cannot self-match if it is ever scanned.
    let cmd_new = ["Command", "::", "new"].concat();
    let go_lit = ["\"", "go", "\""].concat();
    let forbidden_call = format!("{cmd_new}({go_lit})");
    let process_cmd = ["std", "::", "process", "::", "Command"].concat();

    for (name, source) in PRODUCTION_SOURCES {
        assert!(
            !source.contains(&forbidden_call),
            "{name} must not invoke the source toolchain via {forbidden_call}"
        );
        assert!(
            !source.contains(&process_cmd),
            "{name} must not import {process_cmd} (Go firewall)"
        );
    }
}

/// The filesystem half: the firewall is not only "do not spawn", it is "do not READ".
///
/// A library source that named the out-of-band corpus tree — to `include_str!` a source file, to
/// walk the corpus, to re-derive anything the snapshot already carries — would make the engine's
/// answer depend on Go source at verify time with no subprocess anywhere. Same defect, arriving
/// through the filesystem instead of through a process.
#[test]
fn no_production_source_reads_the_go_corpus() {
    let corpus_tree = ["go", "src/"].concat();
    let go_extension = [".", "go", "\""].concat();

    for (name, source) in PRODUCTION_SOURCES {
        assert!(
            !source.contains(&corpus_tree),
            "{name} must not name the `{corpus_tree}` out-of-band tree — the engine consumes \
             snapshot artifacts, never Go source"
        );
        assert!(
            !source.contains(&go_extension),
            "{name} must not reference a `{go_extension}` path"
        );
    }
}
