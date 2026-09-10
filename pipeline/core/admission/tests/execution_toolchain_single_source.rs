use std::path::{Path, PathBuf};

use pipeline_admission::{
    TOOLCHAIN_PIN_KEYS, declared_channel, execution_channel_violations, workflow_toolchain_pins,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|error| panic!("{relative} must be readable: {error}"))
}

fn channel() -> String {
    declared_channel(&read("rust-toolchain.toml")).expect("declared execution channel")
}

/// The population comes from the tree, so whichever hosted workflows exist
/// when the gate runs are the ones it judges.
fn hosted_workflows() -> Vec<(String, String)> {
    let mut names: Vec<String> = std::fs::read_dir(repo_root().join(".github/workflows"))
        .expect("hosted workflow directory")
        .map(|entry| entry.expect("workflow entry").file_name())
        .filter_map(|name| name.into_string().ok())
        .filter(|name| name.ends_with(".yml") || name.ends_with(".yaml"))
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "no hosted workflow was read from the tree"
    );
    names
        .into_iter()
        .map(|name| {
            let contents = read(&format!(".github/workflows/{name}"));
            (name, contents)
        })
        .collect()
}

#[test]
fn every_hosted_toolchain_pin_agrees_with_the_declared_channel() {
    let channel = channel();
    let violations: Vec<String> = hosted_workflows()
        .iter()
        .flat_map(|(name, contents)| {
            execution_channel_violations(&channel, &workflow_toolchain_pins(name, contents))
        })
        .collect();
    assert!(
        violations.is_empty(),
        "hosted toolchain pins disagree with rust-toolchain.toml:\n{}",
        violations.join("\n")
    );
}

/// Without this, an unreadable pin would make the gate above pass by finding
/// nothing rather than by finding agreement.
#[test]
fn the_scanner_reads_a_pin_from_every_workflow_that_names_one() {
    for (name, contents) in hosted_workflows() {
        if !TOOLCHAIN_PIN_KEYS.iter().any(|key| contents.contains(key)) {
            continue;
        }
        let violations = execution_channel_violations(
            "no-such-channel",
            &workflow_toolchain_pins(&name, &contents),
        );
        assert!(
            !violations.is_empty(),
            "{name} names a toolchain input the scanner cannot read"
        );
    }
}

#[test]
fn a_drifted_pin_is_refused_and_named_by_file_and_line() {
    let workflow = "jobs:\n  lint:\n    steps:\n      - uses: install\n        with:\n          toolchain: \"1.97.0\"\n";
    let pins = workflow_toolchain_pins("lint.yml", workflow);
    assert_eq!(pins.len(), 1, "one pin must be read from the fixture");
    assert_eq!(pins[0].job, "lint");
    let violations = execution_channel_violations("1.98.0", &pins);
    assert_eq!(violations.len(), 1, "the drifted pin must be refused");
    assert!(
        violations[0].starts_with("lint.yml:6:"),
        "the refusal must name file and line: {}",
        violations[0]
    );
    assert!(violations[0].contains("1.97.0") && violations[0].contains("1.98.0"));
}

#[test]
fn an_install_superseded_later_in_the_same_job_is_left_to_its_own_gate() {
    let workflow = concat!(
        "jobs:\n  qualify:\n    steps:\n",
        "      - with: { toolchain: \"nightly-2026-05-22\", components: \"clippy\" }\n",
        "      - with: { toolchain: \"1.98.0\" }\n",
        "  lint:\n    steps:\n",
        "      - with: { toolchain: \"nightly-2026-05-22\" }\n",
    );
    let pins = workflow_toolchain_pins("presubmit.yml", workflow);
    assert_eq!(pins.len(), 3);
    let violations = execution_channel_violations("1.98.0", &pins);
    assert_eq!(
        violations.len(),
        1,
        "only the install that survives as the job default is judged: {violations:?}"
    );
    assert!(violations[0].starts_with("presubmit.yml:8:"));
}

#[test]
fn a_deny_action_rust_version_is_never_shadowed() {
    let workflow = "jobs:\n  deny:\n    steps:\n      - with:\n          rust-version: \"1.97.0\"\n          rust-version: \"1.98.0\"\n";
    let violations =
        execution_channel_violations("1.98.0", &workflow_toolchain_pins("d.yml", workflow));
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(violations[0].contains("rust-version:"));
}

#[test]
fn a_dated_nightly_is_a_channel_the_gate_can_carry() {
    let workflow = "jobs:\n  test:\n    steps:\n      - with: { toolchain: nightly-2026-09-09 }\n";
    let pins = workflow_toolchain_pins("t.yml", workflow);
    assert_eq!(pins[0].value, "nightly-2026-09-09", "{pins:?}");
    assert!(execution_channel_violations("nightly-2026-09-09", &pins).is_empty());
    assert_eq!(
        execution_channel_violations("1.98.0", &pins).len(),
        1,
        "a nightly where a stable is declared is drift"
    );
}

#[test]
fn a_declaration_without_a_channel_is_refused() {
    assert!(declared_channel("[toolchain]\nprofile = \"minimal\"\n").is_err());
    assert!(declared_channel("[toolchain\n").is_err());
    assert!(declared_channel("[toolchain]\nchannel = \"\"\n").is_err());
    assert_eq!(
        declared_channel("[toolchain]\nchannel = \"nightly-2026-09-09\"\n")
            .expect("declared channel"),
        "nightly-2026-09-09"
    );
}
