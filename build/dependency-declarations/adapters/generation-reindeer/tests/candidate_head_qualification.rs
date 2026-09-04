use dependency_declarations_generation::GenerationPort;
use dependency_declarations_generation_reindeer::{
    CandidateHeadQualificationFailure, CandidateHeadQualificationRequest, PINNED_REINDEER_COMMIT,
    PINNED_REINDEER_TREE, ProtectedCandidateInput, QualificationPath, QualificationRun,
    ReindeerCandidateHeadQualifier, RevisionField,
};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const PROVIDER_SOURCE: &str = include_str!("fixtures/reindeer_fixture.rs");
const CANDIDATE_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CANDIDATE_TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const TOOLCHAIN_FILE: &[u8] = b"[toolchain]\nchannel = \"1.98.0\"\ncomponents = [\"rustfmt\", \"clippy\"]\nprofile = \"minimal\"\n";
static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    candidate_root: PathBuf,
    provider: PathBuf,
    cargo: PathBuf,
    rustc: PathBuf,
    cargo_home: PathBuf,
    first_target: PathBuf,
    second_target: PathBuf,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "reindeer-candidate-head-qualification-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("fixture root must be created");

        let candidate_root = root.join("candidate");
        let third_party = candidate_root.join("third-party");
        let cargo_home = third_party.join(".cargo");
        fs::create_dir_all(&cargo_home).expect("provider Cargo home must be created");
        fs::write(candidate_root.join("reindeer.toml"), b"fixture = true\n")
            .expect("candidate config must be written");
        fs::write(candidate_root.join("Cargo.lock"), b"candidate lock\n")
            .expect("candidate lock must be written");
        fs::write(third_party.join("BUCK"), b"candidate Buck\n")
            .expect("candidate Buck file must be written");
        fs::write(candidate_root.join("rust-toolchain.toml"), TOOLCHAIN_FILE)
            .expect("candidate toolchain must be written");

        let tools = root.join("tools");
        fs::create_dir(&tools).expect("tool directory must be created");
        let cargo = tools.join(executable_name("candidate-cargo"));
        let rustc = tools.join(executable_name("candidate-rustc"));
        write_executable_marker(&cargo);
        write_executable_marker(&rustc);

        let provider = tools.join(executable_name(mode));
        materialize_provider(&provider);

        let first_target = root.join("run-one");
        let second_target = root.join("run-two");
        fs::create_dir(&first_target).expect("first target must be created");
        fs::create_dir(&second_target).expect("second target must be created");

        Self {
            root,
            candidate_root,
            provider,
            cargo,
            rustc,
            cargo_home,
            first_target,
            second_target,
        }
    }

    fn request(&self) -> CandidateHeadQualificationRequest {
        CandidateHeadQualificationRequest {
            provider_executable: self.provider.clone(),
            provider_commit: PINNED_REINDEER_COMMIT.to_owned(),
            provider_tree: PINNED_REINDEER_TREE.to_owned(),
            candidate_root: self.candidate_root.clone(),
            candidate_commit: CANDIDATE_COMMIT.to_owned(),
            candidate_tree: CANDIDATE_TREE.to_owned(),
            cargo_executable: self.cargo.clone(),
            rustc_executable: self.rustc.clone(),
            cargo_home: self.cargo_home.clone(),
            first_target_dir: self.first_target.clone(),
            second_target_dir: self.second_target.clone(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn executable_name(name: &str) -> OsString {
    let mut executable = OsString::from(name);
    executable.push(std::env::consts::EXE_SUFFIX);
    executable
}

fn write_executable_marker(path: &Path) {
    fs::write(path, b"fixture executable marker\n").expect("tool marker must be written");
    make_executable(path);
}

fn materialize_provider(destination: &Path) {
    if let Some(source) = std::env::var_os("REINDEER_QUALIFICATION_FIXTURE_PROVIDER") {
        fs::copy(source, destination).expect("Buck fixture provider must be copied");
        make_executable(destination);
        return;
    }

    let source = destination.with_extension("rs");
    fs::write(&source, PROVIDER_SOURCE).expect("fixture provider source must be written");
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let output = Command::new(rustc)
        .arg("--edition=2024")
        .arg("-Dwarnings")
        .arg(&source)
        .arg("-o")
        .arg(destination)
        .output()
        .expect("fixture provider compiler must start");
    assert!(
        output.status.success(),
        "fixture provider must compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .expect("fixture executable metadata must be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("fixture executable mode must be set");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

fn qualify(
    request: &CandidateHeadQualificationRequest,
) -> Result<
    dependency_declarations_generation_reindeer::CandidateHeadQualificationArtifact,
    CandidateHeadQualificationFailure,
> {
    ReindeerCandidateHeadQualifier.generate(request)
}

#[test]
fn exact_arguments_and_cleared_environment_produce_a_bound_artifact() {
    let fixture = Fixture::new("exact-contract");
    let artifact = qualify(&fixture.request()).expect("exact fixture contract must qualify");
    assert_eq!(artifact.repository(), "github.com/oyatie/oyatie");
    assert_eq!(artifact.candidate_commit(), CANDIDATE_COMMIT);
    assert_eq!(artifact.candidate_tree(), CANDIDATE_TREE);
    assert_eq!(artifact.provider_commit(), PINNED_REINDEER_COMMIT);
    assert_eq!(artifact.provider_tree(), PINNED_REINDEER_TREE);
    assert_eq!(artifact.provider_build_toolchain(), "nightly-2026-05-22");
    assert_eq!(artifact.candidate_toolchain(), "1.98.0");
    assert_eq!(artifact.generated_buck(), b"generated\n");
}

#[test]
fn nondeterministic_provider_output_is_refused() {
    let fixture = Fixture::new("nondeterministic");
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::NondeterministicOutput {
            first_bytes: "run-one".len(),
            second_bytes: "run-two".len(),
            first_difference: 4,
        })
    );
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

#[test]
fn protected_candidate_mutations_are_refused_after_the_first_run() {
    for (mode, input) in [
        ("mutate-cargo-lock", ProtectedCandidateInput::CargoLock),
        (
            "mutate-third-party-buck",
            ProtectedCandidateInput::ThirdPartyBuck,
        ),
    ] {
        let fixture = Fixture::new(mode);
        assert_eq!(
            qualify(&fixture.request()),
            Err(CandidateHeadQualificationFailure::ProtectedInputChanged {
                input,
                run: QualificationRun::First,
            }),
            "{mode}"
        );
    }
}

#[test]
fn every_required_input_is_checked_before_execution() {
    for (label, field) in [
        ("provider", QualificationPath::ProviderExecutable),
        ("candidate", QualificationPath::CandidateRoot),
        ("config", QualificationPath::CandidateConfig),
        ("lock", QualificationPath::CandidateCargoLock),
        ("buck", QualificationPath::CandidateThirdPartyBuck),
        ("toolchain", QualificationPath::CandidateToolchain),
        ("cargo", QualificationPath::CargoExecutable),
        ("rustc", QualificationPath::RustcExecutable),
        ("cargo-home", QualificationPath::CargoHome),
        ("first-target", QualificationPath::FirstTargetDirectory),
        ("second-target", QualificationPath::SecondTargetDirectory),
    ] {
        let fixture = Fixture::new("missing-input");
        match label {
            "provider" => fs::remove_file(&fixture.provider).unwrap(),
            "candidate" => fs::remove_dir_all(&fixture.candidate_root).unwrap(),
            "config" => fs::remove_file(fixture.candidate_root.join("reindeer.toml")).unwrap(),
            "lock" => fs::remove_file(fixture.candidate_root.join("Cargo.lock")).unwrap(),
            "buck" => fs::remove_file(fixture.candidate_root.join("third-party/BUCK")).unwrap(),
            "toolchain" => {
                fs::remove_file(fixture.candidate_root.join("rust-toolchain.toml")).unwrap();
            }
            "cargo" => fs::remove_file(&fixture.cargo).unwrap(),
            "rustc" => fs::remove_file(&fixture.rustc).unwrap(),
            "cargo-home" => fs::remove_dir(&fixture.cargo_home).unwrap(),
            "first-target" => fs::remove_dir(&fixture.first_target).unwrap(),
            "second-target" => fs::remove_dir(&fixture.second_target).unwrap(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                qualify(&fixture.request()),
                Err(CandidateHeadQualificationFailure::InvalidPath {
                    field: actual,
                    ..
                }) if actual == field
            ),
            "{label}"
        );
    }
}

#[test]
fn identities_toolchain_and_execution_directories_are_bound() {
    let fixture = Fixture::new("identity-refusal");
    let mut request = fixture.request();
    request.provider_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    assert_eq!(
        qualify(&request),
        Err(
            CandidateHeadQualificationFailure::ProviderIdentityMismatch {
                field: RevisionField::ProviderCommit,
            }
        )
    );

    let fixture = Fixture::new("revision-refusal");
    let mut request = fixture.request();
    request.candidate_tree = "not-an-object-id".to_owned();
    assert_eq!(
        qualify(&request),
        Err(CandidateHeadQualificationFailure::InvalidRevision {
            field: RevisionField::CandidateTree,
        })
    );

    let fixture = Fixture::new("toolchain-refusal");
    fs::write(
        fixture.candidate_root.join("rust-toolchain.toml"),
        b"[toolchain]\nchannel = \"nightly-2026-05-22\"\n",
    )
    .unwrap();
    assert_eq!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::CandidateToolchainMismatch)
    );

    let fixture = Fixture::new("same-target-refusal");
    let mut request = fixture.request();
    request.second_target_dir = request.first_target_dir.clone();
    assert!(matches!(
        qualify(&request),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            field: QualificationPath::SecondTargetDirectory,
            ..
        })
    ));

    let fixture = Fixture::new("inside-target-refusal");
    let inside = fixture.candidate_root.join("target");
    fs::create_dir(&inside).unwrap();
    let mut request = fixture.request();
    request.first_target_dir = inside;
    assert!(matches!(
        qualify(&request),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            field: QualificationPath::FirstTargetDirectory,
            ..
        })
    ));

    let fixture = Fixture::new("cargo-home-refusal");
    let other_home = fixture.root.join("other-cargo-home");
    fs::create_dir(&other_home).unwrap();
    let mut request = fixture.request();
    request.cargo_home = other_home;
    assert!(matches!(
        qualify(&request),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            field: QualificationPath::CargoHome,
            ..
        })
    ));
}

#[cfg(unix)]
#[test]
fn non_executable_provider_is_refused() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new("non-executable");
    let mut permissions = fs::metadata(&fixture.provider).unwrap().permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&fixture.provider, permissions).unwrap();
    assert!(matches!(
        qualify(&fixture.request()),
        Err(CandidateHeadQualificationFailure::InvalidPath {
            field: QualificationPath::ProviderExecutable,
            ..
        })
    ));
}

#[test]
#[ignore = "Foundation presubmit selects this hermetic fixture contract explicitly"]
fn foundation_fixture_candidate_head_qualification() {
    let fixture = Fixture::new("foundation-fixture");
    let artifact = qualify(&fixture.request()).expect("Foundation fixture must qualify");
    assert_eq!(artifact.generated_buck(), b"generated\n");
}

#[test]
#[ignore = "requires the exact pinned Reindeer executable"]
fn foundation_real_pinned_provider_fixture_qualification() {
    let fixture = RealProviderFixture::new();
    let before_lock = fs::read(fixture.candidate_root.join("Cargo.lock")).unwrap();
    let before_buck = fs::read(fixture.candidate_root.join("third-party/BUCK")).unwrap();

    let artifact = qualify(&fixture.request()).expect("path-only fixture must qualify twice");
    assert!(!artifact.generated_buck().is_empty());
    assert_eq!(
        fs::read(fixture.candidate_root.join("Cargo.lock")).unwrap(),
        before_lock
    );
    assert_eq!(
        fs::read(fixture.candidate_root.join("third-party/BUCK")).unwrap(),
        before_buck
    );

    let fixup_dir = fixture
        .candidate_root
        .join("third-party/fixups/qualification-leaf");
    fs::create_dir_all(&fixup_dir).unwrap();
    fs::write(
        fixup_dir.join("fixups.toml"),
        b"[buildscript]\nrun = true\n",
    )
    .unwrap();
    let failure = qualify(&fixture.request()).expect_err("unused buildscript fixup must fail");
    match failure {
        CandidateHeadQualificationFailure::ProviderExit {
            run: QualificationRun::First,
            code: Some(code),
            stdout_bytes: 0,
            stderr,
        } => {
            assert_ne!(code, 0);
            assert!(
                String::from_utf8_lossy(&stderr).contains("Unused buildscript fixups"),
                "typed refusal must retain the provider diagnostic: {}",
                String::from_utf8_lossy(&stderr)
            );
        }
        other => panic!("unexpected unused-fixup refusal: {other:?}"),
    }
    assert_eq!(
        fs::read(fixture.candidate_root.join("Cargo.lock")).unwrap(),
        before_lock
    );
    assert_eq!(
        fs::read(fixture.candidate_root.join("third-party/BUCK")).unwrap(),
        before_buck
    );
}

#[test]
#[ignore = "requires exact provider binary and immutable scratch candidate paths"]
fn foundation_exact_candidate_head_qualification() {
    let candidate_commit = required_env("REINDEER_QUALIFICATION_CANDIDATE_COMMIT")
        .into_string()
        .expect("candidate commit must be UTF-8");
    let candidate_tree = required_env("REINDEER_QUALIFICATION_CANDIDATE_TREE")
        .into_string()
        .expect("candidate tree must be UTF-8");
    let request = CandidateHeadQualificationRequest {
        provider_executable: required_env("REINDEER_QUALIFICATION_EXECUTABLE").into(),
        provider_commit: PINNED_REINDEER_COMMIT.to_owned(),
        provider_tree: PINNED_REINDEER_TREE.to_owned(),
        candidate_root: required_env("REINDEER_QUALIFICATION_CANDIDATE_ROOT").into(),
        candidate_commit,
        candidate_tree,
        cargo_executable: required_env("REINDEER_QUALIFICATION_CARGO").into(),
        rustc_executable: required_env("REINDEER_QUALIFICATION_RUSTC").into(),
        cargo_home: required_env("REINDEER_QUALIFICATION_CARGO_HOME").into(),
        first_target_dir: required_env("REINDEER_QUALIFICATION_FIRST_TARGET_DIR").into(),
        second_target_dir: required_env("REINDEER_QUALIFICATION_SECOND_TARGET_DIR").into(),
    };
    let artifact = qualify(&request).expect("exact candidate must qualify");
    assert_eq!(artifact.candidate_commit(), request.candidate_commit);
    assert_eq!(artifact.candidate_tree(), request.candidate_tree);
}

fn required_env(name: &str) -> OsString {
    std::env::var_os(name).unwrap_or_else(|| panic!("{name} must be set"))
}

struct RealProviderFixture {
    root: PathBuf,
    candidate_root: PathBuf,
    provider: PathBuf,
    cargo: PathBuf,
    rustc: PathBuf,
    cargo_home: PathBuf,
    first_target: PathBuf,
    second_target: PathBuf,
}

impl RealProviderFixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "reindeer-real-provider-qualification-{}-{sequence}",
            std::process::id()
        ));
        let candidate_root = root.join("candidate");
        let cargo_home = candidate_root.join("third-party/.cargo");
        fs::create_dir_all(candidate_root.join("src")).unwrap();
        fs::create_dir_all(candidate_root.join("third-party/qualification-leaf/src")).unwrap();
        fs::create_dir_all(&cargo_home).unwrap();
        fs::write(
            candidate_root.join("Cargo.toml"),
            concat!(
                "[package]\n",
                "name = \"qualification-root\"\n",
                "version = \"0.1.0\"\n",
                "edition = \"2024\"\n",
                "\n[dependencies]\n",
                "qualification-leaf = { path = \"third-party/qualification-leaf\" }\n",
            ),
        )
        .unwrap();
        fs::write(candidate_root.join("src/lib.rs"), b"pub fn root() {}\n").unwrap();
        fs::write(
            candidate_root.join("third-party/qualification-leaf/Cargo.toml"),
            concat!(
                "[package]\n",
                "name = \"qualification-leaf\"\n",
                "version = \"0.1.0\"\n",
                "edition = \"2024\"\n",
            ),
        )
        .unwrap();
        fs::write(
            candidate_root.join("third-party/qualification-leaf/src/lib.rs"),
            b"pub fn leaf() {}\n",
        )
        .unwrap();
        fs::write(
            candidate_root.join("Cargo.lock"),
            concat!(
                "# This file is automatically @generated by Cargo.\n",
                "# It is not intended for manual editing.\n",
                "version = 4\n",
                "\n[[package]]\n",
                "name = \"qualification-leaf\"\n",
                "version = \"0.1.0\"\n",
                "\n[[package]]\n",
                "name = \"qualification-root\"\n",
                "version = \"0.1.0\"\n",
                "dependencies = [\n",
                " \"qualification-leaf\",\n",
                "]\n",
            ),
        )
        .unwrap();
        fs::write(
            candidate_root.join("reindeer.toml"),
            concat!(
                "manifest_path = \"Cargo.toml\"\n",
                "third_party_dir = \"third-party\"\n",
                "unresolved_fixup_error = true\n",
            ),
        )
        .unwrap();
        fs::write(
            candidate_root.join("third-party/BUCK"),
            b"protected output\n",
        )
        .unwrap();
        fs::write(candidate_root.join("rust-toolchain.toml"), TOOLCHAIN_FILE).unwrap();
        let first_target = root.join("run-one");
        let second_target = root.join("run-two");
        fs::create_dir(&first_target).unwrap();
        fs::create_dir(&second_target).unwrap();

        Self {
            root,
            candidate_root,
            provider: required_env("REINDEER_QUALIFICATION_EXECUTABLE").into(),
            cargo: required_env("REINDEER_QUALIFICATION_CARGO").into(),
            rustc: required_env("REINDEER_QUALIFICATION_RUSTC").into(),
            cargo_home,
            first_target,
            second_target,
        }
    }

    fn request(&self) -> CandidateHeadQualificationRequest {
        CandidateHeadQualificationRequest {
            provider_executable: self.provider.clone(),
            provider_commit: PINNED_REINDEER_COMMIT.to_owned(),
            provider_tree: PINNED_REINDEER_TREE.to_owned(),
            candidate_root: self.candidate_root.clone(),
            candidate_commit: CANDIDATE_COMMIT.to_owned(),
            candidate_tree: CANDIDATE_TREE.to_owned(),
            cargo_executable: self.cargo.clone(),
            rustc_executable: self.rustc.clone(),
            cargo_home: self.cargo_home.clone(),
            first_target_dir: self.first_target.clone(),
            second_target_dir: self.second_target.clone(),
        }
    }
}

impl Drop for RealProviderFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
