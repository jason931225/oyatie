use super::*;

fn mutate_job(yaml: &str, spec: JobSpec, from: &str, to: &str) -> String {
    let body = job_body(yaml, spec).expect("protected job body");
    assert!(body.contains(from), "{} mutation fixture", spec.id);
    let mutated = body.replacen(from, to, 1);
    yaml.replacen(body, &mutated, 1)
}

#[test]
fn every_protected_job_rejects_source_selector_mutations() {
    let yaml = workflow();
    let protected_ref = "          ref: ${{ github.workflow_sha }}";
    for spec in JOBS {
        for wrong in [
            "${{ github.event.pull_request.base.sha }}",
            "${{ github.event.merge_group.base_sha }}",
            "refs/heads/dev",
            "${{ github.sha }}",
            "${{ inputs.policy_sha }}",
        ] {
            let mutated = mutate_job(
                &yaml,
                spec,
                protected_ref,
                &format!("          ref: {wrong}"),
            );
            assert!(validate_workflow(&mutated).is_err(), "{}: {wrong}", spec.id);
        }
        for replacement in [
            "          # ref: ${{ github.workflow_sha }}\n          ref: ${{ github.sha }}",
            "          ref: ${{ github.workflow_sha }}\n          ref: ${{ github.sha }}",
        ] {
            let mutated = mutate_job(&yaml, spec, protected_ref, replacement);
            assert!(validate_workflow(&mutated).is_err(), "{}", spec.id);
        }
    }
}

#[test]
fn every_protected_job_rejects_source_overwrite_and_candidate_builds() {
    let yaml = workflow();
    for spec in JOBS {
        let build = format!("      - name: {}", spec.build_name);
        for injected in [
            format!(
                "      - name: Overwrite protected source\n        uses: {CHECKOUT}\n        with:\n          ref: ${{{{ github.sha }}}}\n          path: trusted\n          persist-credentials: false\n          fetch-depth: 1\n{build}"
            ),
            format!(
                "      - name: Mutate protected source\n        run: cp -R candidate/. trusted/\n{build}"
            ),
        ] {
            let mutated = mutate_job(&yaml, spec, &build, &injected);
            assert!(validate_workflow(&mutated).is_err(), "{}", spec.id);
        }
        let candidate = mutate_job(
            &yaml,
            spec,
            "$GITHUB_WORKSPACE/trusted/Cargo.toml",
            "$GITHUB_WORKSPACE/candidate/Cargo.toml",
        );
        assert!(validate_workflow(&candidate).is_err(), "{}", spec.id);
    }
}

#[test]
fn change_gates_rejects_input_execution_and_output_mutations() {
    let yaml = workflow();
    let spec = JOBS
        .into_iter()
        .find(|spec| spec.admission == Admission::ChangeGates)
        .expect("change-gates job");
    for (from, to) in [
        (
            "backbone_postgres: ${{ steps.g.outputs.backbone_postgres }}",
            "backbone_postgres: false",
        ),
        (
            "compute_lifecycle_postgres: ${{ steps.g.outputs.compute_lifecycle_postgres }}",
            "compute_lifecycle_postgres: false",
        ),
        (
            "reindeer: ${{ steps.g.outputs.reindeer }}",
            "reindeer: false",
        ),
        ("EVENT: ${{ github.event_name }}", "EVENT: pull_request"),
        (
            "base_sha=\"$(git rev-parse --verify 'HEAD^1^{commit}')\"",
            "base_sha=\"${{ github.event.pull_request.base.sha }}\"",
        ),
        (
            "head_sha=\"$(git rev-parse --verify 'HEAD^{commit}')\"",
            "head_sha=\"${{ github.sha }}\"",
        ),
        (
            " >> \"$GITHUB_OUTPUT\"",
            " > /tmp/untrusted-change-gates-output",
        ),
        (
            "\"$RUNNER_TEMP/oyatie-change-gates-admission/x86_64-unknown-linux-gnu/debug/pipeline-change-gates-app\"",
            "cargo run --manifest-path candidate/Cargo.toml -p pipeline-change-gates-app --",
        ),
    ] {
        let mutated = mutate_job(&yaml, spec, from, to);
        assert!(validate_workflow(&mutated).is_err(), "{from}");
    }
}

/// GitHub's YAML resolves a block-style value; a line-oriented reader sees an
/// empty entry. The value is derived, never named, so this stays true across
/// a channel bump.
#[test]
fn every_protected_job_rejects_a_block_style_toolchain_pin() {
    let yaml = workflow();
    for spec in JOBS {
        let body = job_body(&yaml, spec).expect("protected job body");
        let pin = body
            .lines()
            .find(|line| line.starts_with("          toolchain: "))
            .expect("protected job pins a toolchain")
            .to_owned();
        let channel = pin
            .trim_start()
            .strip_prefix("toolchain: ")
            .expect("pin value");
        let blocked = format!("          toolchain:\n            {channel}");
        let mutated = mutate_job(&yaml, spec, &pin, &blocked);
        assert!(
            validate_workflow(&mutated).is_err(),
            "{}: a block-style pin must not read as pinned",
            spec.id
        );
    }
}
