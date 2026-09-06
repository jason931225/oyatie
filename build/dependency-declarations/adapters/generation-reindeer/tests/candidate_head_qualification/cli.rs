#[test]
fn qualification_cli_modes_refuse_missing_environment_without_stdout() {
    for mode in ["real-provider-fixture", "candidate"] {
        let output = Command::new(qualification_cli())
            .arg(mode)
            .env_clear()
            .output()
            .expect("qualification CLI must start");
        assert!(!output.status.success(), "{mode}");
        assert!(output.stdout.is_empty(), "{mode}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("REINDEER_QUALIFICATION_EXECUTABLE is required"),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn candidate_cli_refuses_each_required_input_before_qualification() {
    for (omitted, diagnostic) in [
        (
            "REINDEER_QUALIFICATION_EXECUTABLE",
            "REINDEER_QUALIFICATION_EXECUTABLE is required",
        ),
        (
            "REINDEER_QUALIFICATION_CARGO",
            "REINDEER_QUALIFICATION_CARGO is required",
        ),
        (
            "REINDEER_QUALIFICATION_RUSTC",
            "REINDEER_QUALIFICATION_RUSTC is required",
        ),
        (
            "REINDEER_QUALIFICATION_FIRST_CANDIDATE_ROOT",
            "REINDEER_QUALIFICATION_FIRST_CANDIDATE_ROOT is required",
        ),
        (
            "REINDEER_QUALIFICATION_SECOND_CANDIDATE_ROOT",
            "REINDEER_QUALIFICATION_SECOND_CANDIDATE_ROOT is required",
        ),
        (
            "REINDEER_QUALIFICATION_FIRST_TARGET_DIR",
            "REINDEER_QUALIFICATION_FIRST_TARGET_DIR is required",
        ),
        (
            "REINDEER_QUALIFICATION_SECOND_TARGET_DIR",
            "REINDEER_QUALIFICATION_SECOND_TARGET_DIR is required",
        ),
    ] {
        let fixture = Fixture::new("exact-contract");
        let output = candidate_cli_omitting(&fixture, omitted);
        assert!(!output.status.success(), "{omitted}");
        assert!(output.stdout.is_empty(), "{omitted}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "{omitted}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!fixture.first_target.exists(), "{omitted}");
        assert!(!fixture.second_target.exists(), "{omitted}");
        assert!(
            !fixture.first_target.join("provider-ran").exists(),
            "{omitted}"
        );
        assert!(
            !fixture.second_target.join("provider-ran").exists(),
            "{omitted}"
        );
    }
}

#[test]
fn candidate_cli_emits_only_raw_admitted_buck_bytes() {
    let fixture = Fixture::new("exact-contract");
    let output = candidate_cli(&fixture);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"generated\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn candidate_cli_refusal_withholds_generated_bytes() {
    let fixture = Fixture::new("published-mismatch");
    for root in [&fixture.first_root, &fixture.second_root] {
        fs::write(root.join("third-party/BUCK"), b"stale\n").unwrap();
    }
    let output = candidate_cli(&fixture);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("generated output differs from published third-party/BUCK"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn candidate_cli(fixture: &Fixture) -> std::process::Output {
    candidate_cli_command(fixture)
        .output()
        .expect("candidate qualification CLI must start")
}

fn candidate_cli_omitting(fixture: &Fixture, omitted: &str) -> std::process::Output {
    let mut command = candidate_cli_command(fixture);
    command.env_remove(omitted);
    command
        .output()
        .expect("candidate qualification CLI must start")
}

fn candidate_cli_command(fixture: &Fixture) -> Command {
    let mut command = Command::new(qualification_cli());
    command
        .arg("candidate")
        .env_clear()
        .env("REINDEER_QUALIFICATION_EXECUTABLE", &fixture.provider)
        .env("REINDEER_QUALIFICATION_CARGO", &fixture.cargo)
        .env("REINDEER_QUALIFICATION_RUSTC", &fixture.rustc)
        .env(
            "REINDEER_QUALIFICATION_FIRST_CANDIDATE_ROOT",
            &fixture.first_root,
        )
        .env(
            "REINDEER_QUALIFICATION_SECOND_CANDIDATE_ROOT",
            &fixture.second_root,
        )
        .env(
            "REINDEER_QUALIFICATION_FIRST_TARGET_DIR",
            &fixture.first_target,
        )
        .env(
            "REINDEER_QUALIFICATION_SECOND_TARGET_DIR",
            &fixture.second_target,
        );
    command
}

fn qualification_cli() -> PathBuf {
    std::env::var_os("REINDEER_QUALIFICATION_CLI")
        .map(PathBuf::from)
        .or_else(|| {
            option_env!("CARGO_BIN_EXE_reindeer_candidate_head_qualification").map(PathBuf::from)
        })
        .expect("qualification CLI path must be supplied by Cargo or Buck")
}
