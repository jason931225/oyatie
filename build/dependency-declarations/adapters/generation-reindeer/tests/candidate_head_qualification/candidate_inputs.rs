#[test]
fn every_semantic_candidate_mutation_is_refused() {
    for mode in [
        "mutate-cargo-lock",
        "mutate-third-party-buck",
        "mutate-config",
        "mutate-manifest",
        "mutate-source",
        "add-input",
    ] {
        let fixture = Fixture::new(mode);
        assert_eq!(
            qualify(&fixture.request()),
            Err(CandidateHeadQualificationFailure::CandidateTreeChanged {
                root: CandidateRoot::First,
                scope: CandidateTreeScope::Semantic,
            }),
            "{mode}"
        );
    }
}

#[test]
fn cross_root_semantic_and_cache_mutations_are_refused() {
    let fixture = Fixture::new("mutate-second-semantic");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::CandidateTreeChanged {
            root: CandidateRoot::Second,
            scope: CandidateTreeScope::Semantic,
        })
    );

    let fixture = Fixture::new("mutate-second-cache");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::CandidateTreeChanged {
            root: CandidateRoot::Second,
            scope: CandidateTreeScope::CargoSeed,
        })
    );

    let fixture = Fixture::new("mutate-first-cache-from-second");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::CandidateTreeChanged {
            root: CandidateRoot::First,
            scope: CandidateTreeScope::CargoSeed,
        })
    );
}

#[test]
fn each_run_may_mutate_only_its_own_derived_cargo_home() {
    let fixture = Fixture::new("mutate-own-cache");
    assert_eq!(
        qualify(&fixture.request())
            .expect("run-local Cargo cache mutations must qualify")
            .generated_buck(),
        b"generated\n"
    );
}

#[test]
fn candidate_roots_and_cargo_seeds_must_begin_byte_identical() {
    let fixture = Fixture::new("root-difference");
    fs::write(fixture.second_root.join("src/lib.rs"), b"different\n").unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
            scope: CandidateTreeScope::Semantic,
            path: PathBuf::from("src/lib.rs"),
        })
    );

    let fixture = Fixture::new("seed-difference");
    fs::write(
        fixture.second_root.join("third-party/.cargo/seed"),
        b"different\n",
    )
    .unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::CandidateTreesDiffer {
            scope: CandidateTreeScope::CargoSeed,
            path: PathBuf::from("seed"),
        })
    );
}

#[test]
fn generated_output_must_equal_each_published_buck_file() {
    let fixture = Fixture::new("published-mismatch");
    for root in [&fixture.first_root, &fixture.second_root] {
        fs::write(root.join("third-party/BUCK"), b"stale\n").unwrap();
    }
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::PublishedOutputMismatch {
            root: CandidateRoot::First,
            generated_bytes: b"generated\n".len(),
            published_bytes: b"stale\n".len(),
            first_difference: 0,
        })
    );
}

#[test]
fn required_candidate_inputs_are_checked_before_execution() {
    for relative in [
        "Cargo.toml",
        "Cargo.lock",
        "reindeer.toml",
        "rust-toolchain.toml",
        "third-party/BUCK",
    ] {
        let fixture = Fixture::new("missing-input");
        let missing = fixture.first_root.join(relative);
        fs::remove_file(&missing).unwrap();
        assert_eq!(
            qualify(&fixture.request()),
            Err(CandidateHeadQualificationFailure::MissingCandidateInput {
                root: CandidateRoot::First,
                path: missing,
            }),
            "{relative}"
        );
    }

    let fixture = Fixture::new("missing-cargo-seed");
    let missing = fixture.first_root.join("third-party/.cargo");
    fs::remove_dir_all(&missing).unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::MissingCandidateInput {
            root: CandidateRoot::First,
            path: missing,
        })
    );
}

#[test]
fn every_external_execution_path_is_checked_before_execution() {
    for (label, field) in [
        ("provider", QualificationPath::ProviderExecutable),
        ("cargo", QualificationPath::CargoExecutable),
        ("rustc", QualificationPath::RustcExecutable),
        ("first-root", QualificationPath::FirstCandidateRoot),
        ("second-root", QualificationPath::SecondCandidateRoot),
    ] {
        let fixture = Fixture::new("missing-external-input");
        match label {
            "provider" => fs::remove_file(&fixture.provider).unwrap(),
            "cargo" => fs::remove_file(&fixture.cargo).unwrap(),
            "rustc" => fs::remove_file(&fixture.rustc).unwrap(),
            "first-root" => fs::remove_dir_all(&fixture.first_root).unwrap(),
            "second-root" => fs::remove_dir_all(&fixture.second_root).unwrap(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                qualify(&fixture.request()),
                Err(CandidateHeadQualificationFailure::InvalidPath {
                    field: actual,
                    reason: PathRefusal::Missing,
                    ..
                }) if actual == field
            ),
            "{label}"
        );
    }
}

#[test]
fn execution_paths_must_be_disjoint_and_targets_must_not_exist() {
    let fixture = Fixture::new("overlap");
    let mut request = fixture.request();
    request.second_target_dir = request.first_target_dir.clone();
    assert!(matches!(
        qualify(&request),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            reason: PathRefusal::OverlapsAnotherInput,
            ..
        })
    ));

    let fixture = Fixture::new("target-inside-root");
    let mut request = fixture.request();
    request.first_target_dir = fixture.first_root.join("target");
    assert_eq!(
        qualify(&request),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            field: QualificationPath::FirstCandidateRoot,
            path: fixture.first_root.clone(),
            reason: PathRefusal::OverlapsAnotherInput,
        })
    );

    let fixture = Fixture::new("existing-target");
    fs::create_dir(&fixture.first_target).unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            field: QualificationPath::FirstTargetDirectory,
            path: fixture.first_target.clone(),
            reason: PathRefusal::AlreadyExists,
        })
    );
}

#[test]
fn uncontrolled_ancestor_cargo_configuration_is_refused() {
    let fixture = Fixture::new("ancestor-config");
    fs::create_dir(fixture.root.join(".cargo")).unwrap();
    let config = fixture.root.join(".cargo/config.toml");
    fs::write(&config, b"[net]\noffline = false\n").unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::AncestorCargoConfig {
            root: CandidateRoot::First,
            path: config,
        })
    );
}

#[cfg(unix)]
#[test]
fn symlinks_in_candidate_trees_are_refused() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new("symlink");
    symlink("lib.rs", fixture.first_root.join("src/alias.rs")).unwrap();
    symlink("lib.rs", fixture.second_root.join("src/alias.rs")).unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(
            CandidateHeadQualificationFailure::UnsupportedCandidateEntry {
                root: CandidateRoot::First,
                scope: CandidateTreeScope::Semantic,
                path: PathBuf::from("src/alias.rs"),
                kind: UnsupportedCandidateEntryKind::Symlink,
            }
        )
    );
}

