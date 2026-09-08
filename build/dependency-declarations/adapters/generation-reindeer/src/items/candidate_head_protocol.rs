const REQUIRED_FILES: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "reindeer.toml",
    "rust-toolchain.toml",
    "third-party/BUCK",
];

fn qualify_candidate_head_with_limits(
    request: &CandidateHeadQualificationRequest,
    limits: QualificationLimits,
) -> Result<CandidateHeadQualificationArtifact, CandidateHeadQualificationFailure> {
    ensure_supported_platform()?;
    validate_limits(limits)?;
    let validated = ValidatedRequest::new(request)?;
    validate_required_candidate_inputs(&validated)?;
    reject_ancestor_cargo_config(CandidateRoot::First, &validated.first_root)?;
    reject_ancestor_cargo_config(CandidateRoot::Second, &validated.second_root)?;

    let tool_identities = external_tool_identities(&validated)?;
    let mut storage = IndependentStorage::default();
    let semantic_snapshot = compare_trees(
        &validated.first_root,
        &validated.second_root,
        CandidateTreeScope::Semantic,
        limits,
        &tool_identities,
        &mut storage,
    )?;
    let cargo_seed_snapshot = compare_trees(
        &validated.first_cargo_home,
        &validated.second_cargo_home,
        CandidateTreeScope::CargoSeed,
        limits,
        &tool_identities,
        &mut storage,
    )?;

    create_target(QualificationRun::First, &validated.first_target)?;
    create_target(QualificationRun::Second, &validated.second_target)?;

    let first_execution = execute_provider(
        QualificationRun::First,
        &validated,
        &validated.first_root,
        &validated.first_cargo_home,
        &validated.first_target,
        limits,
    );
    verify_tree_unchanged(
        CandidateRoot::First,
        &validated.first_root,
        CandidateTreeScope::Semantic,
        &semantic_snapshot,
        limits,
        &tool_identities,
    )?;
    verify_tree_unchanged(
        CandidateRoot::Second,
        &validated.second_root,
        CandidateTreeScope::Semantic,
        &semantic_snapshot,
        limits,
        &tool_identities,
    )?;
    verify_tree_unchanged(
        CandidateRoot::Second,
        &validated.second_cargo_home,
        CandidateTreeScope::CargoSeed,
        &cargo_seed_snapshot,
        limits,
        &tool_identities,
    )?;
    let first_cache_after_run = snapshot_tree(
        CandidateRoot::First,
        &validated.first_cargo_home,
        CandidateTreeScope::CargoSeed,
        limits,
        &tool_identities,
    )?;
    let first_output = accept_provider_output(first_execution?)?;

    let second_execution = execute_provider(
        QualificationRun::Second,
        &validated,
        &validated.second_root,
        &validated.second_cargo_home,
        &validated.second_target,
        limits,
    );
    verify_tree_unchanged(
        CandidateRoot::First,
        &validated.first_root,
        CandidateTreeScope::Semantic,
        &semantic_snapshot,
        limits,
        &tool_identities,
    )?;
    verify_tree_unchanged(
        CandidateRoot::Second,
        &validated.second_root,
        CandidateTreeScope::Semantic,
        &semantic_snapshot,
        limits,
        &tool_identities,
    )?;
    verify_tree_unchanged(
        CandidateRoot::First,
        &validated.first_cargo_home,
        CandidateTreeScope::CargoSeed,
        &first_cache_after_run,
        limits,
        &tool_identities,
    )?;
    let second_output = accept_provider_output(second_execution?)?;

    if first_output.stdout != second_output.stdout {
        return Err(CandidateHeadQualificationFailure::NondeterministicOutput {
            first_bytes: first_output.stdout.len(),
            second_bytes: second_output.stdout.len(),
            first_difference: first_difference(&first_output.stdout, &second_output.stdout),
        });
    }
    compare_published_output(
        CandidateRoot::First,
        &validated.first_root.join("third-party/BUCK"),
        &first_output.stdout,
    )?;
    compare_published_output(
        CandidateRoot::Second,
        &validated.second_root.join("third-party/BUCK"),
        &second_output.stdout,
    )?;

    Ok(CandidateHeadQualificationArtifact {
        generated_buck: first_output.stdout,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn ensure_supported_platform() -> Result<(), CandidateHeadQualificationFailure> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn ensure_supported_platform() -> Result<(), CandidateHeadQualificationFailure> {
    Err(CandidateHeadQualificationFailure::UnsupportedPlatform)
}
fn validate_required_candidate_inputs(
    request: &ValidatedRequest,
) -> Result<(), CandidateHeadQualificationFailure> {
    for (root, path) in [
        (CandidateRoot::First, request.first_root.as_path()),
        (CandidateRoot::Second, request.second_root.as_path()),
    ] {
        for relative in REQUIRED_FILES {
            require_candidate_input(root, &path.join(relative), false)?;
        }
        require_candidate_input(root, &path.join("third-party/.cargo"), true)?;
    }
    Ok(())
}

fn require_candidate_input(
    root: CandidateRoot,
    path: &Path,
    expected_directory: bool,
) -> Result<(), CandidateHeadQualificationFailure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(CandidateHeadQualificationFailure::MissingCandidateInput {
                root,
                path: path.to_path_buf(),
            });
        }
        Err(error) => {
            return Err(CandidateHeadQualificationFailure::CandidateTreeRead {
                root,
                scope: CandidateTreeScope::Semantic,
                path: path.to_path_buf(),
                kind: error.kind(),
            });
        }
    };
    let valid = if expected_directory {
        metadata.is_dir() && !metadata.file_type().is_symlink()
    } else {
        metadata.is_file() && !metadata.file_type().is_symlink()
    };
    if valid {
        Ok(())
    } else {
        Err(CandidateHeadQualificationFailure::InvalidCandidateInput {
            root,
            path: path.to_path_buf(),
            expected_directory,
        })
    }
}

fn create_target(
    run: QualificationRun,
    path: &Path,
) -> Result<(), CandidateHeadQualificationFailure> {
    fs::create_dir(path).map_err(|error| CandidateHeadQualificationFailure::TargetCreate {
        run,
        path: path.to_path_buf(),
        kind: error.kind(),
    })
}
