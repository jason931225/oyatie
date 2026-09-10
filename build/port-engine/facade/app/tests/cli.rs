//! CLI dispatch: what each command does with a valid invocation, and usage on anything else.

use std::process::ExitCode;

use port_engine_app::cli::run;

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn help_and_ready_succeed() {
    assert_eq!(run(&args(&["help"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["ready"])), ExitCode::SUCCESS);
}

/// Every command in `cli::COMMANDS` reaches its handler.
///
/// Kept in step with that list by hand, because an integration test cannot see a `pub(crate)`
/// const; `run_dispatches_exactly_the_listed_commands` is what holds the list itself honest.
#[test]
fn every_command_succeeds_on_a_valid_invocation() {
    use std::time::{SystemTime, UNIX_EPOCH};

    assert_eq!(run(&args(&["pin"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["emit-stub"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["emit-syn"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["digest", "port-engine"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["rulepack"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["plan"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["admit-snapshot"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["declarations"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["port-go"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["region-digests"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["dispositions"])), ExitCode::SUCCESS);
    assert_eq!(
        run(&args(&["port-go-source"])),
        ExitCode::SUCCESS,
        "the committed golden must match the current emit"
    );
    assert_eq!(run(&args(&["transform"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["render"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["engine"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["toolchain"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["pipeline"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["receipt"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["delta"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["verify"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["verify-e2e"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["emit-canary"])), ExitCode::SUCCESS);
    assert_eq!(run(&args(&["canary-defect"])), ExitCode::SUCCESS);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let out = std::env::temp_dir()
        .join(format!("pe-cli-canary-{nanos}"))
        .join(port_engine_emit::CANARY_OUT_DIRNAME);
    let out_s = out.to_string_lossy().into_owned();
    assert_eq!(
        run(&args(&["materialize-canary", &out_s])),
        ExitCode::SUCCESS
    );
    let _ = std::fs::remove_dir_all(out.parent().expect("parent"));
}

/// `survey` reaches its handler; an unreadable path is a run failure, not a usage one.
#[test]
fn survey_reaches_its_handler() {
    let missing = std::env::temp_dir().join("pe-cli-no-such-snapshot.json");
    let missing = missing.to_string_lossy().into_owned();
    assert_eq!(run(&args(&["survey", &missing])), ExitCode::from(1));
}

#[test]
fn unknown_command_is_usage() {
    assert_eq!(run(&args(&["not-a-command"])), ExitCode::from(2));
}

#[test]
fn digest_without_arg_is_usage() {
    assert_eq!(run(&args(&["digest"])), ExitCode::from(2));
}

#[test]
fn materialize_canary_without_arg_is_usage() {
    assert_eq!(run(&args(&["materialize-canary"])), ExitCode::from(2));
}

#[test]
fn survey_without_arg_is_usage() {
    assert_eq!(run(&args(&["survey"])), ExitCode::from(2));
}
