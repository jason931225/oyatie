//! Semantic-name regressions for repository admission and workflow labels.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pipeline_admission::{
    cargo_manifest_violations, changed_layout_violations, file_budget_violations,
    git_change_paths_from_name_status_z, layout_violations,
};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

fn has_decision_identifier(value: &str) -> bool {
    ["ADR-", "D-"]
        .into_iter()
        .any(|marker| decision_marker_has_number(value, marker))
}

fn decision_marker_has_number(mut value: &str, marker: &str) -> bool {
    while let Some((_, tail)) = value.split_once(marker) {
        if tail
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_digit())
        {
            return true;
        }
        value = tail;
    }
    false
}

fn assert_semantic(surface: &str, value: &str) {
    assert!(
        !has_decision_identifier(value),
        "{surface} exposes a decision identifier as an operational name: {value}"
    );
}

fn delivery_sequence(relative: &str) -> Vec<String> {
    let document = std::fs::read_to_string(repo_root().join(relative)).expect("delivery law");
    delivery_sequence_items(&document)
}

fn delivery_sequence_items(document: &str) -> Vec<String> {
    let start = "<!-- agent-instructions:start -->";
    let end = "<!-- agent-instructions:end -->";
    assert_eq!(
        document.matches(start).count(),
        1,
        "expected exactly one agent-instructions block start"
    );
    assert_eq!(
        document.matches(end).count(),
        1,
        "expected exactly one agent-instructions block end"
    );
    let (_, tail) = document
        .split_once(start)
        .expect("agent-instructions block start");
    let (block, _) = tail.split_once(end).expect("agent-instructions block end");
    assert_eq!(
        block.matches("required_sequence:").count(),
        1,
        "expected exactly one required delivery sequence"
    );
    let (_, sequence) = block
        .split_once("required_sequence:")
        .expect("required delivery sequence");
    let mut items = Vec::new();
    for line in sequence.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(item) = line.strip_prefix("- ") else {
            break;
        };
        items.push(item.split_whitespace().collect::<Vec<_>>().join(" "));
    }
    assert!(!items.is_empty(), "required delivery sequence is empty");
    items
}

fn semantic_naming_rule(relative: &str) -> String {
    let document = std::fs::read_to_string(repo_root().join(relative)).expect("semantic-name law");
    let (_, section) = document
        .split_once("Semantic operational names\n")
        .expect("semantic operational names section");
    section
        .lines()
        .take_while(|line| !line.starts_with('#') && !line.starts_with("<!--"))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn presubmit_display_names_are_semantic_and_required_fan_in_is_stable() {
    let workflow = std::fs::read_to_string(repo_root().join(".github/workflows/presubmit.yml"))
        .expect("presubmit workflow");
    let cache_path = repo_root().join(".github/workflows/build-cache-qualification.yml");
    let qualification = std::fs::read_to_string(cache_path).expect("cache workflow");
    let display_names: Vec<&str> = workflow
        .lines()
        .chain(qualification.lines())
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix("name: ")
                .or_else(|| line.strip_prefix("- name: "))
        })
        .collect();
    for name in display_names {
        assert_semantic("workflow display name", name);
    }
    assert!(workflow.contains("\n  layout:\n    name: repository layout\n"));
    assert!(workflow.contains("\n  presubmit:\n    name: presubmit\n"));
    assert!(
        workflow.contains(
            "needs: [layout, occupancy, lint, clippy, test, deny, change-gates, commit-signing, reindeer-source-qualification, live-postgres, build-cache-qualification]"
        )
    );
}

#[test]
fn real_reindeer_qualification_is_pinned_offline_and_fail_closed() {
    let workflow = std::fs::read_to_string(repo_root().join(".github/workflows/presubmit.yml"))
        .expect("presubmit workflow");
    let job = workflow
        .split_once("\n  reindeer-source-qualification:\n")
        .and_then(|(_, tail)| tail.split_once("\n  change-gates:\n"))
        .map(|(job, _)| job)
        .expect("bounded Reindeer qualification job");
    let toolchain_action = "dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17";
    let ordered_toolchains = concat!(
        "      - uses: dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17\n",
        "        with: { toolchain: \"nightly-2026-05-22\", components: \"clippy\" }\n",
        "      - uses: dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17\n",
        "        with: { toolchain: \"1.98.0\" }",
    );

    assert_eq!(
        job.matches(toolchain_action).count(),
        2,
        "Reindeer qualification must invoke the pinned toolchain action exactly twice"
    );
    assert_eq!(
        job.matches("toolchain: \"nightly-2026-05-22\"").count(),
        1,
        "Reindeer qualification must declare the pinned nightly exactly once"
    );
    assert_eq!(
        job.matches(ordered_toolchains).count(),
        1,
        "Reindeer qualification must install nightly before stable"
    );
    assert!(
        !workflow.contains("RUSTUP_TOOLCHAIN"),
        "the workflow must not override the stable default toolchain"
    );

    for fact in [
        "needs: [layout, change-gates]",
        "timeout-minutes: 50",
        "repository: facebookincubator/reindeer",
        "ref: bb681570d2bc47d1446080c12b8681a50a95f628",
        "153f32a846b5e1f460e61fff1cecbbf5177c8c90",
        "d4644db6bee4fce06425c6802dfc5b3c2d2a12ba93ea3d635e076700bc34d614",
        "CARGO_NET_OFFLINE: \"true\"",
        "--run-ignored only",
        "--no-tests=fail",
        "--test-threads 1",
        "--max-fail 1:immediate",
    ] {
        assert!(job.contains(fact), "Reindeer qualification missing {fact}");
    }
    assert_eq!(
        job.matches("actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9")
            .count(),
        1
    );
    assert_eq!(job.matches("cargo fetch --locked").count(), 2);
    assert!(job.contains("timeout --signal=TERM --kill-after=30s 40m"));
    assert!(!job.contains("restore-keys:") && !job.contains("latest"));
}

#[test]
fn emitted_admission_diagnostics_are_semantic() {
    let budget = file_budget_violations(
        "network/core/route/src/lib.rs",
        "line\n".repeat(301).as_bytes(),
    )
    .join("\n");
    assert!(budget.contains("repository 300-line file budget"));
    assert_semantic("file-budget diagnostic", &budget);

    let manifest = cargo_manifest_violations(
        "network/core/route/Cargo.toml",
        "[package]\nname='network-route'\nautotests=false\nbuild='other.rs'\n",
    )
    .join("\n");
    assert!(manifest.contains("required integration-test discovery"));
    assert!(manifest.contains("stable item-scanner `build.rs`"));
    assert_semantic("manifest diagnostic", &manifest);

    let change =
        git_change_paths_from_name_status_z(b"A\0app/ledger/OWNERS\0A\0app/ledger/README.md\0")
            .expect("change paths");
    let new_owner = changed_layout_violations(&change, &BTreeSet::new()).join("\n");
    assert!(new_owner.contains("requires one core crate"));
    assert_semantic("new-owner diagnostic", &new_owner);
}

#[test]
fn provenance_citations_and_decision_files_remain_valid() {
    let workflow = std::fs::read_to_string(repo_root().join(".github/workflows/presubmit.yml"))
        .expect("presubmit workflow");
    assert!(workflow.contains("ADR-0719"));
    assert!(
        file_budget_violations(
            "docs/decisions/ADR-0719-example.md",
            "line\n".repeat(301).as_bytes(),
        )
        .is_empty()
    );

    let change =
        std::fs::read_to_string(repo_root().join("pipeline/core/admission/src/layout/change.rs"))
            .expect("change admission source");
    let manifest = std::fs::read_to_string(
        repo_root().join("pipeline/core/admission/src/layout/manifest/mod.rs"),
    )
    .expect("manifest admission source");
    let entrypoint = std::fs::read_to_string(
        repo_root().join("pipeline/core/admission/src/layout/manifest/entrypoint.rs"),
    )
    .expect("entry-point admission source");
    assert!(change.contains("Provenance: ADR-0719 D-8/D-36"));
    assert!(manifest.contains("Provenance: ADR-0719 D-30/D-41"));
    // Entry-point resolution split out of `manifest.rs`; it carries the same
    // provenance because it is the same rule, not a new one.
    assert!(entrypoint.contains("Provenance: ADR-0719 D-30/D-41"));
}

#[test]
fn semantic_naming_rule_is_identical_across_current_root_sources() {
    let agents = semantic_naming_rule("AGENTS.md");
    let claude = semantic_naming_rule("CLAUDE.md");

    assert_eq!(agents, claude);
    assert!(agents.contains("legitimate ADR content amendments remain allowed"));
    assert!(agents.contains("recorded challenge demonstrably shows"));
    assert!(!agents.contains("records remain unchanged"));
}

#[test]
fn delivery_sequence_is_identical_across_instruction_sources() {
    let agents = delivery_sequence("AGENTS.md");
    let claude = delivery_sequence("CLAUDE.md");
    let expected = [
        "isolated worktree branch per lane",
        "SSH-signed commit and push on that lane",
        "draft pull request against dev",
        "required context presubmit green",
        "independent reviewer APPROVE; threads resolved; conflict-free protected squash merge",
    ]
    .map(str::to_owned)
    .to_vec();

    assert_eq!(agents, claude);
    assert_eq!(agents, expected);
    assert_semantic("delivery sequence", &agents.join(" "));
}

#[test]
fn delivery_sequence_ignores_unrelated_harness_metadata() {
    let git = r#"<!-- agent-instructions:start -->
sanctioned_primitives:
  - git
required_sequence:
  - first step
  - second step
<!-- agent-instructions:end -->"#;
    let forge = r#"<!-- agent-instructions:start -->
sanctioned_primitives:
  - forge
required_sequence:
    - first   step
    - second step
<!-- agent-instructions:end -->"#;

    assert_eq!(delivery_sequence_items(git), delivery_sequence_items(forge));
}
