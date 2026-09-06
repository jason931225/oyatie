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

