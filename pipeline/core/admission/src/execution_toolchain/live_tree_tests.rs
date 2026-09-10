use std::path::{Path, PathBuf};

use super::*;

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

/// A YAML comment opens at a `#` that starts the line or follows whitespace.
/// Written from that rule rather than from the scanner's code.
fn structure_of(line: &str) -> &str {
    match line
        .match_indices('#')
        .find(|(at, _)| *at == 0 || line[..*at].ends_with(char::is_whitespace))
    {
        Some((at, _)) => &line[..at],
        None => line,
    }
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
            // Comment-stripped by this file's own implementation of the YAML
            // rule, deliberately not by the scanner's helper: a mirror that
            // calls the code it checks agrees with it by construction, and the
            // transform is exactly where the scanner lost a pin before.
            .map(structure_of)
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

/// The rule's own module must satisfy it. It did not: an explanatory comment
/// quoted the live channel, so this rule refused the file that defines it and
/// the next change to that file would have been blocked by it. The gate is
/// built from the trusted revision, so this pull request's own green was not
/// evidence either way.
#[test]
fn the_source_literal_rule_admits_its_own_module() {
    let channel = channel();
    let violations: Vec<String> = [
        "execution_toolchain.rs",
        "execution_toolchain/channel_literal_tests.rs",
        "execution_toolchain/live_tree_tests.rs",
        "execution_toolchain/workflow_pin_tests.rs",
    ]
    .into_iter()
    .flat_map(|name| {
        let path = format!("pipeline/core/admission/src/{name}");
        let contents = read(&path);
        channel_literal_violations(&channel, &path, &contents)
    })
    .collect();
    assert!(
        violations.is_empty(),
        "the rule refuses its own source:\n{}",
        violations.join("\n")
    );
}
