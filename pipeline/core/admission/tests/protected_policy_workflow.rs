use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repo root")
}

fn workflow() -> String {
    std::fs::read_to_string(repo_root().join(".github/workflows/presubmit.yml"))
        .expect("presubmit workflow")
}

const CHECKOUT: &str = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
const TOOLCHAIN: &str = "dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17";
const BACKBONE_OUTPUT: &str = "${{ steps.g.outputs.backbone_postgres }}";
const COMPUTE_OUTPUT: &str = "${{ steps.g.outputs.compute_lifecycle_postgres }}";

#[derive(Clone, Copy)]
struct JobSpec {
    id: &'static str,
    next_id: &'static str,
    build_name: &'static str,
    admit_name: &'static str,
    target_dir: &'static str,
    package: &'static str,
    admission: Admission,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Admission {
    Layout,
    Occupancy,
    ChangeGates,
}

const JOBS: [JobSpec; 3] = [
    JobSpec {
        id: "layout",
        next_id: "occupancy",
        build_name: "Build protected path-layout application",
        admit_name: "Admit candidate tree",
        target_dir: "oyatie-layout-admission",
        package: "pipeline-path-layout-app",
        admission: Admission::Layout,
    },
    JobSpec {
        id: "occupancy",
        next_id: "lint",
        build_name: "Build path occupancy application",
        admit_name: "Admit complete Git path-set",
        target_dir: "oyatie-occupancy-admission",
        package: "pipeline-path-occupancy-app",
        admission: Admission::Occupancy,
    },
    JobSpec {
        id: "change-gates",
        next_id: "live-postgres",
        build_name: "Build protected change-gates application",
        admit_name: "Classify typed qualification paths",
        target_dir: "oyatie-change-gates-admission",
        package: "pipeline-change-gates-app",
        admission: Admission::ChangeGates,
    },
];

fn ensure(ok: bool, message: impl Into<String>) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}

fn job_body(yaml: &str, spec: JobSpec) -> Result<&str, String> {
    let start = format!("\n  {}:\n", spec.id);
    let end = format!("\n  {}:\n", spec.next_id);
    let rest = yaml
        .split_once(&start)
        .map(|(_, rest)| rest)
        .ok_or_else(|| format!("missing {} job", spec.id))?;
    rest.split_once(&end)
        .map(|(body, _)| body)
        .ok_or_else(|| format!("{} must precede {}", spec.id, spec.next_id))
}

fn indentation(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn direct_entries(block: &str, indent: usize) -> Vec<(&str, &str)> {
    block
        .lines()
        .filter_map(|line| {
            if indentation(line) != indent {
                return None;
            }
            let direct = &line[indent..];
            if direct.is_empty() || direct.starts_with('#') || direct.starts_with("- ") {
                return None;
            }
            direct
                .split_once(':')
                .map(|(key, value)| (key.trim(), value.trim()))
        })
        .collect()
}

fn content_lines(block: &str, indent: usize) -> Vec<String> {
    block
        .lines()
        .filter(|line| indentation(line) >= indent)
        .map(|line| line[indent..].to_owned())
        .collect()
}

fn validate_checkout(
    step: &str,
    label: &str,
    reference: &str,
    path: &str,
    fetch_depth: &str,
) -> Result<(), String> {
    ensure(
        direct_entries(step, 8) == [("uses", CHECKOUT), ("with", "")],
        format!("{label} must have one pinned checkout action and one with block"),
    )?;
    ensure(
        direct_entries(step, 10)
            == [
                ("ref", reference),
                ("path", path),
                ("persist-credentials", "false"),
                ("fetch-depth", fetch_depth),
            ],
        format!("{label} checkout inputs must be exact and unique"),
    )
}

fn validate_toolchain(step: &str, job: &str) -> Result<(), String> {
    ensure(
        direct_entries(step, 8) == [("with", "")]
            && matches!(direct_entries(step, 10).as_slice(), [("toolchain", _)]),
        format!("{job} toolchain step must be pinned and closed"),
    )
}

fn validate_build(step: &str, spec: JobSpec) -> Result<(), String> {
    ensure(
        direct_entries(step, 8) == [("working-directory", "${{ runner.temp }}"), ("run", ">-")],
        format!("{} build step fields must be closed", spec.id),
    )?;
    let actual = content_lines(step, 10);
    let expected = vec![
        "cargo build --locked".to_owned(),
        "--manifest-path \"$GITHUB_WORKSPACE/trusted/Cargo.toml\"".to_owned(),
        "--target x86_64-unknown-linux-gnu".to_owned(),
        format!("--target-dir \"$RUNNER_TEMP/{}\"", spec.target_dir),
        format!("-p {}", spec.package),
    ];
    ensure(
        actual == expected,
        format!("{} must build only the protected source", spec.id),
    )
}

fn validate_admit(step: &str, spec: JobSpec) -> Result<(), String> {
    if spec.admission == Admission::Layout {
        ensure(
            direct_entries(step, 8) == [("working-directory", "candidate"), ("run", "|")],
            "layout admission fields must be closed",
        )?;
        let expected = [
            "set -euo pipefail",
            "base_sha=\"$(git rev-parse --verify 'HEAD^1^{commit}')\"",
            "head_sha=\"$(git rev-parse --verify 'HEAD^{commit}')\"",
            "\"$RUNNER_TEMP/oyatie-layout-admission/x86_64-unknown-linux-gnu/debug/pipeline-path-layout-app\" \"$base_sha\" \"$head_sha\"",
        ];
        return ensure(
            content_lines(step, 10) == expected,
            "layout admission must execute the protected binary over candidate HEAD^1 and HEAD",
        );
    }
    if spec.admission == Admission::Occupancy {
        ensure(
            direct_entries(step, 8)
                == [
                    ("working-directory", "candidate"),
                    ("env", ""),
                    (
                        "run",
                        "\"$RUNNER_TEMP/oyatie-occupancy-admission/x86_64-unknown-linux-gnu/debug/pipeline-path-occupancy-app\"",
                    ),
                ],
            "occupancy admission fields must be closed",
        )?;
        return ensure(
            direct_entries(step, 10)
                == [
                    ("GH_TOKEN", "${{ github.token }}"),
                    (
                        "OYATIE_PULL_REQUEST",
                        "${{ github.event.pull_request.number }}",
                    ),
                    ("OYATIE_REPOSITORY", "${{ github.repository }}"),
                ],
            "occupancy admission environment must be exact",
        );
    }
    ensure(
        direct_entries(step, 8)
            == [
                ("name", "Classify typed qualification paths"),
                ("working-directory", "candidate"),
                ("env", ""),
                ("run", "|"),
            ],
        "change-gates admission fields must be closed",
    )?;
    ensure(
        content_lines(step, 10)
            == [
                "EVENT: ${{ github.event_name }}",
                "set -euo pipefail",
                "base_sha=\"$(git rev-parse --verify 'HEAD^1^{commit}')\"",
                "head_sha=\"$(git rev-parse --verify 'HEAD^{commit}')\"",
                "\"$RUNNER_TEMP/oyatie-change-gates-admission/x86_64-unknown-linux-gnu/debug/pipeline-change-gates-app\" \"$EVENT\" \"$base_sha\" \"$head_sha\" >> \"$GITHUB_OUTPUT\"",
            ],
        "change-gates must append exact protected output for candidate HEAD^1 and HEAD",
    )
}

fn validate_job(yaml: &str, spec: JobSpec) -> Result<(), String> {
    let body = job_body(yaml, spec)?;
    if spec.admission == Admission::ChangeGates {
        ensure(
            direct_entries(body, 4)
                == [
                    ("name", "change gates"),
                    ("runs-on", "ubuntu-24.04"),
                    ("timeout-minutes", "10"),
                    ("outputs", ""),
                    ("steps", ""),
                ]
                && direct_entries(body, 6)
                    == [
                        ("backbone_postgres", BACKBONE_OUTPUT),
                        ("compute_lifecycle_postgres", COMPUTE_OUTPUT),
                        ("reindeer", "${{ steps.g.outputs.reindeer }}"),
                    ],
            "change-gates job fields and outputs must be closed",
        )?;
    }
    let steps: Vec<_> = body.split("\n      - ").skip(1).collect();
    ensure(steps.len() == 5, "protected job must have five steps")?;
    let actual_headers: Vec<_> = steps
        .iter()
        .map(|step| step.lines().next().unwrap_or_default().to_owned())
        .collect();
    let admit_header = if spec.admission == Admission::ChangeGates {
        "id: g".to_owned()
    } else {
        format!("name: {}", spec.admit_name)
    };
    let expected_headers = [
        "name: Check out candidate tree".to_owned(),
        "name: Check out protected admission source".to_owned(),
        format!("uses: {TOOLCHAIN}"),
        format!("name: {}", spec.build_name),
        admit_header,
    ];
    ensure(
        actual_headers == expected_headers,
        format!("{} step occupants or order drifted", spec.id),
    )?;
    validate_checkout(steps[0], "candidate", "${{ github.sha }}", "candidate", "0")?;
    validate_checkout(
        steps[1],
        "protected policy",
        "${{ github.workflow_sha }}",
        "trusted",
        "1",
    )?;
    validate_toolchain(steps[2], spec.id)?;
    validate_build(steps[3], spec)?;
    validate_admit(steps[4], spec)
}

fn validate_workflow(yaml: &str) -> Result<(), String> {
    for spec in JOBS {
        validate_job(yaml, spec)?;
    }
    Ok(())
}

#[test]
fn protected_admission_jobs_have_closed_step_graphs() {
    validate_workflow(&workflow()).unwrap_or_else(|error| panic!("{error}"));
}

#[path = "protected_policy_workflow/refutations.rs"]
mod refutations;
