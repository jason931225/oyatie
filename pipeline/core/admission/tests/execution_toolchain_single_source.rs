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
        // Counted the way the scanner counts: one pin per key occurrence per
        // line, comment-only lines skipped like the scanner skips them.
        //
        // The population size is the assertion, not the refusal count.
        // Asserting only that SOMETHING was refused detects total blindness,
        // and total blindness is not the case that happens: a file with six
        // readable pins and one silently dropped pin satisfied that while a
        // wrong compiler reached a protected job. Refusals legitimately number
        // fewer than pins, because a shadowed install is left to its own gate.
        let named: usize = contents
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            // Comment-stripped, mirroring the scanner: a key named inside a
            // trailing comment is not a pin, so counting it here would make
            // this assertion fail over prose.
            .map(|line| line.split('#').next().unwrap_or(line))
            .map(|line| {
                TOOLCHAIN_PIN_KEYS
                    .iter()
                    .filter(|key| line.contains(**key))
                    .count()
            })
            .sum();
        if named == 0 {
            continue;
        }
        let pins = workflow_toolchain_pins(&name, &contents);
        assert_eq!(
            pins.len(),
            named,
            "{name} names {named} toolchain input(s); the scanner read {}",
            pins.len()
        );
        let violations = execution_channel_violations("no-such-channel", &pins);
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

/// The shadow exemption belongs to `toolchain:` alone, because only a rustup
/// default install is replaced by the next one. This is the assertion that
/// makes `can_be_shadowed` load-bearing: with the exemption widened to every
/// key, the earlier pin here is superseded by the next step and the drift goes
/// unreported. The existing `rust-version` case does not test it -- its two
/// pins share one step, so the step rule refuses the shadow first, and the
/// RUSTUP case has two different keys.
#[test]
fn a_non_default_install_key_is_never_shadowed_by_the_next_step() {
    let workflow = concat!(
        "jobs:\n  deny:\n    steps:\n",
        "      - with: { rust-version: \"1.42.0\" }\n",
        "      - with: { rust-version: \"9.9.9\" }\n",
    );
    let pins = workflow_toolchain_pins("d.yml", workflow);
    assert_eq!(pins.len(), 2, "both pins must be read: {pins:?}");
    assert_eq!(pins[1].step, pins[0].step + 1, "consecutive steps: {pins:?}");
    let violations = execution_channel_violations("9.9.9", &pins);
    assert_eq!(
        violations.len(),
        1,
        "a rust-version pin is not replaced by the next one: {violations:?}"
    );
    assert!(violations[0].contains("1.42.0"), "{violations:?}");
}

/// A key inside a trailing comment is prose, not an input. Read from the raw
/// line it became a pin whose value parsed to whatever followed, and the gate
/// refused a workflow for a sentence about a toolchain.
#[test]
fn a_toolchain_named_only_in_a_trailing_comment_is_not_a_pin() {
    let workflow = "jobs:\n  lint:\n    steps:\n      - uses: install # also sets toolchain: later\n";
    let pins = workflow_toolchain_pins("l.yml", workflow);
    assert!(pins.is_empty(), "a comment is not a pin: {pins:?}");
}

/// An unattributed pin is never shadowed. Legal YAML this scanner cannot parse
/// -- four-space job indentation here -- gives every pin the same empty job,
/// while `step` keeps counting across the boundary, so two installs in
/// different jobs would shadow each other.
#[test]
fn pins_in_unparsed_jobs_do_not_shadow_each_other() {
    let workflow = concat!(
        "jobs:\n",
        "    first:\n        steps:\n            - with: { toolchain: \"1.42.0\" }\n",
        "    second:\n        steps:\n            - with: { toolchain: \"9.9.9\" }\n",
    );
    let pins = workflow_toolchain_pins("w.yml", workflow);
    assert_eq!(pins.len(), 2, "{pins:?}");
    assert!(
        pins.iter().all(|pin| pin.job.is_empty()),
        "this fixture exists because attribution fails here: {pins:?}"
    );
    let violations = execution_channel_violations("9.9.9", &pins);
    assert_eq!(
        violations.len(),
        1,
        "the drifted pin must be refused, not treated as replaced: {violations:?}"
    );
    assert!(violations[0].contains("1.42.0"), "{violations:?}");
}

/// `RUSTUP_TOOLCHAIN` in a job `env:` picks the compiler before any
/// declaration is consulted, so no later install can shadow it and it must be
/// judged wherever it appears. It is also the only pin key spelled in upper
/// case, which is how it escaped a case-sensitive scan entirely.
#[test]
fn a_rustup_toolchain_environment_override_is_judged_and_never_shadowed() {
    let workflow = concat!(
        "jobs:\n  qualify:\n",
        "    env:\n      RUSTUP_TOOLCHAIN: \"1.42.0\"\n",
        "    steps:\n      - uses: install\n        with:\n          toolchain: \"9.9.9\"\n",
    );
    let pins = workflow_toolchain_pins("q.yml", workflow);
    assert_eq!(pins.len(), 2, "both inputs must be read: {pins:?}");
    let violations = execution_channel_violations("9.9.9", &pins);
    assert_eq!(
        violations.len(),
        1,
        "the override disagrees and the install agrees: {violations:?}"
    );
    assert!(
        violations[0].contains("RUSTUP_TOOLCHAIN:") && violations[0].contains("1.42.0"),
        "the refusal must name the override it measured: {violations:?}"
    );
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
