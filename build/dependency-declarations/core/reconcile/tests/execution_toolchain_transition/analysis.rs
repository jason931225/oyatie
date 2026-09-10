use std::collections::BTreeSet;

use dependency_declarations_reconcile::{
    DeclarationFieldDelta, DeclarationRefusal, ExecutionToolchainAnalysisRefusal,
    ExecutionToolchainProfile, ExecutionVersionDelta, ToolchainSide,
    analyze_execution_toolchain_transition,
};

use crate::support::{analyze, custom_declaration, declaration, workspace};

#[test]
fn analyzer_classifies_every_version_direction() {
    for (protected, candidate, expected) in [
        ("1.98.0", "1.98.0", ExecutionVersionDelta::Unchanged),
        ("1.98.0", "1.98.1", ExecutionVersionDelta::ForwardPatch),
        ("1.98.0", "1.99.0", ExecutionVersionDelta::ForwardMinor),
        ("1.98.0", "2.0.0", ExecutionVersionDelta::ForwardMajor),
        ("1.98.1", "1.98.0", ExecutionVersionDelta::Downgrade),
    ] {
        let analysis = analyze(
            &declaration(protected),
            &declaration(candidate),
            "1.96.0",
            "1.96.0",
        )
        .unwrap();
        assert_eq!(analysis.delta().execution(), expected);
    }
}

#[test]
fn analyzer_retains_valid_declaration_changes_as_typed_facts() {
    let candidate = custom_declaration(
        "1.99.0",
        "[\"rustfmt\", \"rust-src\"]",
        "default",
        "targets = [\"wasm32-unknown-unknown\"]\n",
    );
    let analysis = analyze(&declaration("1.98.0"), &candidate, "1.96.0", "1.97.0").unwrap();

    assert_eq!(
        analysis.delta().execution(),
        ExecutionVersionDelta::ForwardMinor
    );
    assert_eq!(analysis.delta().msrv(), DeclarationFieldDelta::Changed);
    assert_eq!(analysis.delta().profile(), DeclarationFieldDelta::Changed);
    assert_eq!(
        analysis.delta().components(),
        DeclarationFieldDelta::Changed
    );
    assert_eq!(analysis.delta().targets(), DeclarationFieldDelta::Changed);
    assert_eq!(analysis.protected().execution().to_string(), "1.98.0");
    assert_eq!(analysis.candidate().msrv().to_string(), "1.97.0");
    assert_eq!(
        analysis.candidate().profile(),
        ExecutionToolchainProfile::Default
    );
    assert_eq!(
        analysis.candidate().components(),
        &BTreeSet::from(["rust-src".to_owned(), "rustfmt".to_owned()])
    );
    assert_eq!(
        analysis.candidate().targets(),
        &BTreeSet::from(["wasm32-unknown-unknown".to_owned()])
    );
}

#[test]
fn invalid_toolchain_shapes_fail_analysis_with_side_and_field_context() {
    for candidate in [
        "[toolchain",
        "",
        "[unknown]\nvalue = 1\n",
        "toolchain = \"stable\"\n",
        "[toolchain]\ncomponents = [\"rustfmt\", \"clippy\"]\nprofile = \"minimal\"\n",
        "[toolchain]\nchannel = \"1.98.0\"\ncomponents = \"clippy\"\nprofile = \"minimal\"\n",
        "[toolchain]\nchannel = \"1.98.0\"\ncomponents = [\"clippy\", \"clippy\"]\nprofile = \"minimal\"\n",
        "[toolchain]\nchannel = \"1.98.0\"\ncomponents = [\"rustfmt\", \"clippy\"]\nprofile = \"custom\"\n",
        "[toolchain]\nchannel = \"1.98.0\"\ncomponents = [\"rustfmt\", \"clippy\"]\nprofile = \"minimal\"\npath = \"/tmp/rust\"\n",
    ] {
        assert!(matches!(
            analyze(&declaration("1.98.0"), candidate, "1.96.0", "1.96.0"),
            Err(ExecutionToolchainAnalysisRefusal::InvalidToolchain(
                ToolchainSide::Candidate,
                _
            ))
        ));
    }
    for channel in [
        "1.98",
        "stable",
        "beta",
        "custom-toolchain",
        "1.98.0-aarch64-apple-darwin",
        "1.98.0-alpha.1",
        "1.98.0+build",
    ] {
        assert!(matches!(
            analyze(
                &declaration("1.98.0"),
                &declaration(channel),
                "1.96.0",
                "1.96.0",
            ),
            Err(ExecutionToolchainAnalysisRefusal::InvalidToolchain(
                ToolchainSide::Candidate,
                DeclarationRefusal::InvalidStableVersion("toolchain.channel", _)
            ))
        ));
    }
    let duplicate_targets = custom_declaration(
        "1.98.0",
        "[\"rustfmt\", \"clippy\"]",
        "minimal",
        "targets = [\"wasm32-unknown-unknown\", \"wasm32-unknown-unknown\"]\n",
    );
    assert!(matches!(
        analyze(
            &declaration("1.98.0"),
            &duplicate_targets,
            "1.96.0",
            "1.96.0"
        ),
        Err(ExecutionToolchainAnalysisRefusal::InvalidToolchain(
            ToolchainSide::Candidate,
            DeclarationRefusal::Duplicate("toolchain.targets", _)
        ))
    ));
    assert!(matches!(
        analyze("[toolchain", &declaration("1.98.0"), "1.96.0", "1.96.0"),
        Err(ExecutionToolchainAnalysisRefusal::InvalidToolchain(
            ToolchainSide::Protected,
            DeclarationRefusal::MalformedToml(_)
        ))
    ));
    assert!(matches!(
        analyze_execution_toolchain_transition(
            &declaration("1.98.0"),
            &declaration("1.98.1"),
            "[workspace",
            &workspace("1.96.0"),
        ),
        Err(ExecutionToolchainAnalysisRefusal::InvalidMsrv(
            ToolchainSide::Protected,
            DeclarationRefusal::MalformedToml(_)
        ))
    ));
}

#[test]
fn invalid_candidate_msrv_table_is_compact_and_fail_closed() {
    for (candidate_workspace, expected) in [
        ("[workspace", "malformed"),
        ("[workspace]\n[workspace.package]\n", "missing"),
        (
            "[workspace]\n[workspace.package]\nrust-version = 198\n",
            "wrong-type",
        ),
        (
            "[workspace]\n[workspace.package]\nrust-version = '1.98'\n",
            "invalid-version",
        ),
        (
            "[workspace]\n[workspace.package]\nrust-version = '1.98.0-alpha.1'\n",
            "invalid-version",
        ),
        (
            "[workspace]\n[workspace.package]\nrust-version = '1.98.0+build'\n",
            "invalid-version",
        ),
        (
            "[workspace]\n[workspace.package]\nrust-version = 'nightly-2026-09-09'\n",
            "invalid-version",
        ),
        (
            "[workspace]\n[workspace.package]\nrust-version = '1.98.0'\nrust-version = '1.98.1'\n",
            "malformed",
        ),
    ] {
        let refusal = analyze_execution_toolchain_transition(
            &declaration("1.98.0"),
            &declaration("1.98.1"),
            &workspace("1.96.0"),
            candidate_workspace,
        )
        .unwrap_err();
        let ExecutionToolchainAnalysisRefusal::InvalidMsrv(ToolchainSide::Candidate, reason) =
            refusal
        else {
            panic!("expected candidate MSRV refusal, got {refusal:?}")
        };
        let actual = match reason {
            DeclarationRefusal::MalformedToml(_) => "malformed",
            DeclarationRefusal::Missing("workspace.package.rust-version") => "missing",
            DeclarationRefusal::WrongType("workspace.package.rust-version", "string") => {
                "wrong-type"
            }
            DeclarationRefusal::InvalidStableVersion("workspace.package.rust-version", _) => {
                "invalid-version"
            }
            other => panic!("unexpected candidate MSRV refusal: {other:?}"),
        };
        assert_eq!(actual, expected);
    }
}
