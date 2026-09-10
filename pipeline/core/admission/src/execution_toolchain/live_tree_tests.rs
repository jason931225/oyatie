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
