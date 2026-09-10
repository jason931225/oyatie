//! Neutrality and toolchain fences for the target IR, scanned over the WHOLE crate.
//!
//! ## Why there is no scan of EMITTED bytes here
//!
//! There used to be. `render_rust_ir` refused any rendered output containing one of six fixed
//! strings — four corpus identifiers, plus the source language's package and function keywords.
//! It is gone, because it conflated two different claims:
//!
//! 1. The ENGINE must not know about the corpus. That is ADR-0637 D1, it is real, and it is what
//!    the scans below enforce over this crate's own sources.
//! 2. The engine's OUTPUT must not mention the corpus. That is not a rule anywhere and cannot be:
//!    the program exists to emit a Rust translation OF that corpus, so its output carries the
//!    corpus's identifiers in every doc comment, string literal and type name. A fence that
//!    reddens on the program succeeding is not a fence.
//!
//! The property the source-keyword needles reached for — "we emitted the target language, not the
//! source" — is now carried by something stronger than a substring list. The IR holds typed items,
//! `syn` parses the assembled tokens, and `rustc` compiles the result in the port-go compile
//! proof. Source-language text survives none of those steps, and it could easily have survived a
//! scan for six fixed strings.

use std::collections::BTreeSet;
use std::path::Path;

use port_engine_rust_ir::CRATE_SOURCES;

/// The enumeration must BE the directory, not a subset somebody once curated.
#[test]
fn scanned_sources_are_the_whole_crate() {
    let candidates = [
        option_env!("CARGO_MANIFEST_DIR").map(|dir| Path::new(dir).join("src")),
        Some(Path::new("src").to_path_buf()),
        // The crate's REPO-RELATIVE path. buck2 runs a test from the project root with no cargo
        // environment, so neither candidate above resolves there — and this fence guards a property
        // of that build too.
        Some(Path::new("build/port-engine/core/rust-ir/src").to_path_buf()),
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

    let scanned: BTreeSet<String> = CRATE_SOURCES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();

    assert_eq!(
        scanned, on_disk,
        "a source file exists that no architecture fence reads — regenerate src/sources.rs"
    );
}

/// A CANARY SET, not a decision procedure — no finite list can decide "corpus-specific". The
/// structural properties carry the weight; this is the cheap backstop on top.
#[test]
fn no_production_source_carries_corpus_vocabulary() {
    let needles = [
        ["k", "8", "s", ".", "i", "o"].concat(),
        ["k", "ube", "rnete", "s"].concat(),
        ["k", "ube", "let"].concat(),
        ["api", "machin", "ery"].concat(),
    ];

    for (name, source) in CRATE_SOURCES {
        for needle in &needles {
            assert!(
                !source.contains(needle),
                "{name} must not embed corpus needle `{needle}`"
            );
        }
    }
}

/// The formatter is a receipt axis, not an in-process shell-out. A renderer that spawned a host
/// formatter would make the emitted bytes depend on whichever binary is on PATH, and the axis
/// would attest to a version it never saw.
#[test]
fn no_production_source_spawns_a_host_toolchain() {
    let cmd_new = ["Command", "::", "new"].concat();
    let process_cmd = ["std", "::", "process", "::", "Command"].concat();

    for (name, source) in CRATE_SOURCES {
        assert!(
            !source.contains(&cmd_new),
            "{name} must not spawn a process via {cmd_new}"
        );
        assert!(
            !source.contains(&process_cmd),
            "{name} must not import {process_cmd}"
        );
    }
}
