#[test]
fn provider_runtime_and_output_are_natively_bounded() {
    let fixture = Fixture::new("timeout");
    let limits = QualificationLimits {
        runtime: Duration::from_millis(50),
        ..QualificationLimits::default()
    };
    assert_eq!(
        qualify_with(&fixture.request(), limits),
        Err(CandidateHeadQualificationFailure::ProviderTimeout {
            run: QualificationRun::First,
            limit: Duration::from_millis(50),
        })
    );

    let fixture = Fixture::new("stdout-limit");
    let limits = QualificationLimits {
        stdout_bytes: 32,
        ..QualificationLimits::default()
    };
    assert!(matches!(
        qualify_with(&fixture.request(), limits),
        Err(CandidateHeadQualificationFailure::OutputLimitExceeded {
            run: QualificationRun::First,
            stream: QualificationStream::Stdout,
            limit: 32,
            ..
        })
    ));

    let fixture = Fixture::new("stderr-limit");
    let limits = QualificationLimits {
        stderr_bytes: 32,
        ..QualificationLimits::default()
    };
    assert!(matches!(
        qualify_with(&fixture.request(), limits),
        Err(CandidateHeadQualificationFailure::OutputLimitExceeded {
            run: QualificationRun::First,
            stream: QualificationStream::Stderr,
            limit: 32,
            ..
        })
    ));
}

#[test]
fn tree_work_and_configuration_are_natively_bounded() {
    let fixture = Fixture::new("file-limit");
    let limits = QualificationLimits {
        tree_file_bytes: 4,
        ..QualificationLimits::default()
    };
    assert!(matches!(
        qualify_with(&fixture.request(), limits),
        Err(
            CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                root: CandidateRoot::First,
                scope: CandidateTreeScope::Semantic,
                limit: QualificationLimit::TreeFileBytes,
                ..
            }
        )
    ));

    let fixture = Fixture::new("entry-limit");
    let limits = QualificationLimits {
        tree_entries: 1,
        ..QualificationLimits::default()
    };
    assert!(matches!(
        qualify_with(&fixture.request(), limits),
        Err(
            CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                root: CandidateRoot::First,
                scope: CandidateTreeScope::Semantic,
                limit: QualificationLimit::TreeEntries,
                ..
            }
        )
    ));

    let fixture = Fixture::new("aggregate-limit");
    let limits = QualificationLimits {
        tree_total_bytes: 100,
        ..QualificationLimits::default()
    };
    assert!(matches!(
        qualify_with(&fixture.request(), limits),
        Err(
            CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                root: CandidateRoot::First,
                scope: CandidateTreeScope::Semantic,
                limit: QualificationLimit::TreeTotalBytes,
                ..
            }
        )
    ));

    let fixture = Fixture::new("depth-limit");
    let limits = QualificationLimits {
        tree_depth: 1,
        ..QualificationLimits::default()
    };
    assert!(matches!(
        qualify_with(&fixture.request(), limits),
        Err(
            CandidateHeadQualificationFailure::CandidateTreeLimitExceeded {
                root: CandidateRoot::First,
                scope: CandidateTreeScope::Semantic,
                limit: QualificationLimit::TreeDepth,
                ..
            }
        )
    ));

    let fixture = Fixture::new("invalid-limit");
    let limits = QualificationLimits {
        stdout_bytes: 0,
        ..QualificationLimits::default()
    };
    assert_eq!(
        qualify_with(&fixture.request(), limits),
        Err(CandidateHeadQualificationFailure::InvalidLimit {
            limit: QualificationLimit::StdoutBytes,
            value: 0,
            maximum: dependency_declarations_generation_reindeer::MAX_GENERATED_OUTPUT_BYTES
                as u128,
        })
    );
}

#[cfg(unix)]
#[test]
fn independently_materialized_roots_must_not_share_file_storage() {
    let fixture = Fixture::new("shared-storage");
    let second = fixture.second_root.join("src/lib.rs");
    fs::remove_file(&second).unwrap();
    fs::hard_link(fixture.first_root.join("src/lib.rs"), &second).unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(
            CandidateHeadQualificationFailure::CandidateTreesShareStorage {
                scope: CandidateTreeScope::Semantic,
                path: PathBuf::from("src/lib.rs"),
            }
        )
    );
}

