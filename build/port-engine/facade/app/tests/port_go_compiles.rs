//! The compile proof: the Rust this engine emits for the hermetic Go corpus is fed to `rustc`.
//!
//! Every other check in this crate proves the pipeline is STABLE. None of them proves the bytes are
//! CORRECT — `syn` parses things `rustc` rejects, and a golden over broken output pins the breakage.
//!
//! This spawns a subprocess, which the engine's own libraries may never do. The ADR-0638 D3
//! firewall (archived; live via apex ADR-0704) is about the engine's LIBRARY sources and about the
//! source-language toolchain; this is a test binary invoking the TARGET-language compiler, on the
//! output, after the fact. Nothing here feeds back into what the engine emits.

use std::process::Command;

use port_engine_app::driver;

#[test]
fn emitted_rust_compiles() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    let out_dir =
        std::env::temp_dir().join(format!("port-engine-compile-proof-{}", std::process::id()));
    std::fs::create_dir_all(&out_dir).expect("scratch dir");
    let source_path = out_dir.join("emitted.rs");
    std::fs::write(&source_path, &source).expect("write emitted source");

    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let output = Command::new(rustc)
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--crate-name=port_engine_emitted")
        // Metadata only: this proves the code type-checks, which is the claim. Producing an
        // artifact would prove the same thing more slowly.
        .arg("--emit=metadata")
        .arg("-o")
        .arg(out_dir.join("emitted.rmeta"))
        // Both allowances are about the SOURCE, not the translation. Go warns on neither an
        // unexported declaration nobody calls nor a parameter a function ignores — `geometry.Lookup`
        // genuinely ignores its table — so denying them here would fail the proof for output that
        // is faithful.
        .arg("--allow=dead_code")
        .arg("--allow=unused_variables")
        .arg(&source_path)
        .output()
        .expect("rustc must be runnable — this test runs under cargo, which found one");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "emitted Rust must compile.\n\n--- rustc ---\n{stderr}\n--- source ---\n{source}"
    );

    let _ = std::fs::remove_dir_all(&out_dir);
}

/// The emitted modules must carry the corpus's declarations, named as Rust names them. Compiling
/// proves the output is valid Rust; this proves it is valid Rust ABOUT SOMETHING — an empty file
/// compiles too, and the engine's whole prior state was output that compiled and said nothing.
#[test]
fn emitted_rust_carries_the_corpus() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    for expected in [
        "pub mod basic",
        "pub mod shapes",
        "MAX_RETRIES",
        "DEFAULT_NAME",
        "fn add",
        "fn scale",
        "struct Celsius",
        "type ID",
        "struct Point",
        "impl Point",
        "fn shift",
        "trait Named",
    ] {
        assert!(
            source.contains(expected),
            "emitted source must carry `{expected}`:\n{source}"
        );
    }

    // The deferred kind emits nothing, and that is the pack's recorded decision rather than a
    // silent drop — `Enabled` and `Threshold` are Go vars.
    assert!(
        !source.contains("ENABLED") && !source.contains("THRESHOLD"),
        "a deferred kind must not be emitted:\n{source}"
    );
}

/// Asserted over the whole emit rather than per declaration on purpose: a stub reintroduced
/// anywhere reds it, including in a declaration nobody thought to name here.
#[test]
fn no_method_body_is_a_stub() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    let source = driver::assemble_modules(&report);

    assert!(
        !source.contains("todo!"),
        "every body in the corpus must translate:\n{source}"
    );
}

#[test]
fn porting_twice_is_byte_identical() {
    let verification = driver::port_go_delta().expect("two runs must be identical");
    assert_eq!(verification.verdict, port_engine_kernel::Verdict::Green);
}

/// A planted defect must be RED, and the reason matters: the two receipts are IDENTICAL, so no axis
/// moved, so nothing explains the changed bytes. `Unexplained` is the verdict ADR-0637 D2
/// (archived; live via apex ADR-0704) assigns that situation.
#[test]
fn a_planted_defect_in_the_ported_corpus_is_unexplained() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");

    let mut defective = report.emitted.clone();
    let target = defective
        .keys()
        .next()
        .cloned()
        .expect("the port emits at least one region");
    defective.insert(target.clone(), b"pub fn planted_defect() {}".to_vec());

    let verification = port_engine_kernel::verify(
        &report.receipt,
        &report.emitted,
        &report.receipt,
        &defective,
    );

    assert_eq!(verification.verdict, port_engine_kernel::Verdict::Red);
    match verification.delta {
        port_engine_kernel::Delta::Unexplained { regions } => {
            assert!(
                regions.contains(&target),
                "the changed region must be named: {regions:?}"
            );
        }
        other => panic!("a defect with no moved axis must be Unexplained, got {other:?}"),
    }
}

#[test]
fn every_receipt_axis_carries_a_value() {
    let report = driver::port_go_pipeline().expect("the Go corpus must port");
    assert!(
        report.receipt.incomplete_axes().is_empty(),
        "receipt axes left empty: {:?}",
        report.receipt.incomplete_axes()
    );
    assert!(report.receipt.snapshot_digest.0.starts_with("sha256:"));
    assert!(report.receipt.rulepack_digest.0.starts_with("sha256:"));
    assert!(report.receipt.engine_digest.0.starts_with("sha256:"));
    assert!(!report.receipt.pin.is_empty());
}
