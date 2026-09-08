#[test]
fn provider_exit_does_not_leave_descendants_holding_output_pipes() {
    let fixture = Fixture::new("exit-descendant");
    let result = qualify(&fixture.request());
    assert!(result.is_ok(), "{result:?}");
    assert!(fixture.first_target.join("descendant-pid").is_file());
}

#[test]
fn provider_timeout_cleans_up_descendant_pipe_holders() {
    let fixture = Fixture::new("wait-descendant");
    let limit = Duration::from_secs(1);
    let result = qualify_with(
        &fixture.request(),
        QualificationLimits {
            runtime: limit,
            ..QualificationLimits::default()
        },
    );
    assert_eq!(
        result,
        Err(CandidateHeadQualificationFailure::ProviderTimeout {
            run: QualificationRun::First,
            limit,
        })
    );
    assert!(fixture.first_target.join("descendant-pid").is_file());
}

#[test]
fn nondeterministic_provider_output_is_refused() {
    let fixture = Fixture::new("nondeterministic");
    assert!(matches!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::NondeterministicOutput {
            first_bytes: 6,
            second_bytes: 7,
            first_difference: 0,
        })
    ));
}

#[test]
fn failed_provider_never_returns_partial_stdout() {
    let fixture = Fixture::new("partial-failure");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::ProviderExit {
            run: QualificationRun::First,
            code: Some(7),
            stdout_bytes: b"partial".len(),
            stderr: Vec::new(),
        })
    );
}

#[test]
fn provider_diagnostic_is_preserved_only_in_a_typed_refusal() {
    let fixture = Fixture::new("exit-diagnostic");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::ProviderExit {
            run: QualificationRun::First,
            code: Some(9),
            stdout_bytes: 0,
            stderr: b"provider refused\n".to_vec(),
        })
    );
}

#[test]
fn successful_provider_with_stderr_is_refused() {
    let fixture = Fixture::new("stderr-success");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::ProviderStderr {
            run: QualificationRun::First,
            stderr: b"unexpected diagnostic\n".to_vec(),
        })
    );
}

#[test]
fn successful_provider_with_empty_stdout_is_refused() {
    let fixture = Fixture::new("empty-success");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::EmptyOutput {
            run: QualificationRun::First,
        })
    );
}
