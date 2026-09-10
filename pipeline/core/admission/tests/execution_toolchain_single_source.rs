use std::path::{Path, PathBuf};

use pipeline_admission::{
    TOOLCHAIN_PIN_KEYS, channel_literal_violations, declared_channel, execution_channel_violations,
    workflow_toolchain_pins,
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
    let violations = execution_channel_violations("9.9.9", &pins);
    assert_eq!(violations.len(), 1, "the drifted pin must be refused");
    assert!(
        violations[0].starts_with("lint.yml:6:"),
        "the refusal must name file and line: {}",
        violations[0]
    );
    assert!(violations[0].contains("1.97.0") && violations[0].contains("9.9.9"));
}

#[test]
fn an_install_superseded_later_in_the_same_job_is_left_to_its_own_gate() {
    let workflow = concat!(
        "jobs:\n  qualify:\n    steps:\n",
        "      - with: { toolchain: \"nightly-2026-05-22\", components: \"clippy\" }\n",
        "      - with: { toolchain: \"9.9.9\" }\n",
        "  lint:\n    steps:\n",
        "      - with: { toolchain: \"nightly-2026-05-22\" }\n",
    );
    let pins = workflow_toolchain_pins("presubmit.yml", workflow);
    assert_eq!(pins.len(), 3);
    let violations = execution_channel_violations("9.9.9", &pins);
    assert_eq!(
        violations.len(),
        1,
        "only the install that survives as the job default is judged: {violations:?}"
    );
    assert!(violations[0].starts_with("presubmit.yml:8:"));
}

#[test]
fn a_deny_action_rust_version_is_never_shadowed() {
    let workflow = "jobs:\n  deny:\n    steps:\n      - with:\n          rust-version: \"1.97.0\"\n          rust-version: \"9.9.9\"\n";
    let violations =
        execution_channel_violations("9.9.9", &workflow_toolchain_pins("d.yml", workflow));
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert!(violations[0].contains("rust-version:"));
}

#[test]
fn a_dated_nightly_is_a_channel_the_gate_can_carry() {
    let workflow = "jobs:\n  test:\n    steps:\n      - with: { toolchain: nightly-2026-09-09 }\n";
    let pins = workflow_toolchain_pins("t.yml", workflow);
    assert_eq!(
        pins[0].value.as_deref(),
        Some("nightly-2026-09-09"),
        "{pins:?}"
    );
    assert!(execution_channel_violations("nightly-2026-09-09", &pins).is_empty());
    assert_eq!(
        execution_channel_violations("9.9.9", &pins).len(),
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

/// A block-style pin resolves in GitHub's YAML and is invisible to a
/// line-scanner; skipping it would let any channel through unjudged.
#[test]
fn a_pin_whose_value_the_scanner_cannot_read_is_itself_a_violation() {
    let workflow = concat!(
        "jobs:\n  layout:\n    steps:\n      - uses: install\n",
        "        with:\n          toolchain:\n            \"1.42.0\"\n",
    );
    let violations =
        execution_channel_violations("9.9.9", &workflow_toolchain_pins("presubmit.yml", workflow));
    assert!(
        violations
            .iter()
            .any(|line| line.starts_with("presubmit.yml:6:")),
        "a pin the scanner cannot resolve must be refused, not skipped: {violations:?}"
    );
}

#[test]
fn an_inline_comment_on_the_jobs_key_does_not_erase_job_attribution() {
    let workflow =
        "jobs: # every job\n  lint:\n    steps:\n      - with: { toolchain: \"1.97.0\" }\n";
    let pins = workflow_toolchain_pins("p.yml", workflow);
    assert_eq!(
        pins.iter().map(|pin| pin.job.as_str()).collect::<Vec<_>>(),
        ["lint"],
        "{pins:?}"
    );
}

#[test]
fn a_trailing_space_after_a_job_name_does_not_shift_attribution() {
    let workflow =
        "jobs:\n  live-postgres: \n    steps:\n      - with: { toolchain: \"1.97.0\" }\n";
    let pins = workflow_toolchain_pins("l.yml", workflow);
    assert_eq!(
        pins.iter().map(|pin| pin.job.as_str()).collect::<Vec<_>>(),
        ["live-postgres"],
        "{pins:?}"
    );
}

/// An install is only superseded if nothing ran between it and its
/// replacement; work done on the earlier compiler is what the exemption
/// would otherwise hide.
#[test]
fn an_install_that_ran_work_before_being_replaced_is_still_judged() {
    let workflow = concat!(
        "jobs:\n  qualify:\n    steps:\n",
        "      - with: { toolchain: \"1.42.0\" }\n",
        "      - run: cargo build\n",
        "      - with: { toolchain: \"9.9.9\" }\n",
    );
    let violations =
        execution_channel_violations("9.9.9", &workflow_toolchain_pins("p.yml", workflow));
    assert!(
        violations.iter().any(|line| line.contains("1.42.0")),
        "an install used before replacement must be judged: {violations:?}"
    );
}

/// A synthetic channel throughout: writing the live one here would make this
/// file the very duplicate the rule refuses.
#[test]
fn a_rust_literal_equal_to_the_declared_channel_is_refused() {
    let source = "const A: &str = \"9.9.9\";\nconst B: &str = \"9.9.10\";\n";
    let violations = channel_literal_violations("9.9.9", "t.rs", source);
    assert_eq!(
        violations
            .iter()
            .map(|v| v.split(':').nth(1).unwrap())
            .collect::<Vec<_>>(),
        ["1"],
        "only the line equal to the channel is refused: {violations:?}"
    );
}

#[test]
fn a_literal_that_differs_from_the_channel_is_an_oracle_and_is_untouched() {
    let source = "assert_eq!(delta(\"9.9.8\", \"9.9.10\"), Forward);\n";
    let violations = channel_literal_violations("9.9.9", "t.rs", source);
    assert!(
        violations.is_empty(),
        "a comparison oracle must survive the rule: {violations:?}"
    );
}

#[test]
fn a_channel_embedded_in_a_byte_string_declaration_is_still_a_duplicate() {
    let source = "const F: &[u8] = b\"[toolchain]\\nchannel = \\\"9.9.9\\\"\\n\";\n";
    let violations = channel_literal_violations("9.9.9", "h.rs", source);
    assert!(
        !violations.is_empty(),
        "an embedded declaration is the clearest duplicate: {violations:?}"
    );
}

#[test]
fn a_source_that_derives_the_channel_is_clean() {
    let source = "let channel = declared_channel(include_str!(\"../rust-toolchain.toml\"))?;\n";
    let violations = channel_literal_violations("9.9.9", "d.rs", source);
    assert!(violations.is_empty(), "{violations:?}");
}
