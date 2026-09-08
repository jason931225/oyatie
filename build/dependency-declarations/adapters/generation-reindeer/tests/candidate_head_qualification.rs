use dependency_declarations_generation::GenerationPort;
use dependency_declarations_generation_reindeer::{
    CandidateHeadQualificationArtifact, CandidateHeadQualificationFailure,
    CandidateHeadQualificationRequest, CandidateRoot, CandidateTreeScope, PathRefusal,
    QualificationLimit, QualificationLimits, QualificationPath, QualificationRun,
    QualificationStream, ReindeerCandidateHeadQualifier, UnsupportedCandidateEntryKind,
};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const TOOLCHAIN_FILE: &[u8] = b"[toolchain]\nchannel = \"1.98.0\"\ncomponents = [\"rustfmt\", \"clippy\"]\nprofile = \"minimal\"\n";
static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    root: PathBuf,
    first_root: PathBuf,
    second_root: PathBuf,
    provider: PathBuf,
    cargo: PathBuf,
    rustc: PathBuf,
    first_target: PathBuf,
    second_target: PathBuf,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let raw_root = std::env::temp_dir().join(format!(
            "reindeer-candidate-qualification-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&raw_root).expect("fixture root must be created");
        let root = fs::canonicalize(raw_root).expect("fixture root must be canonical");
        let first_root = root.join("candidate-one");
        let second_root = root.join("candidate-two");
        write_candidate(&first_root);
        write_candidate(&second_root);

        let tools = root.join("tools");
        fs::create_dir(&tools).expect("tool directory must be created");
        let cargo = tools.join(executable_name("candidate-cargo"));
        let rustc = tools.join(executable_name("candidate-rustc"));
        write_executable_marker(&cargo);
        write_executable_marker(&rustc);
        let provider = tools.join(executable_name(mode));
        materialize_provider(&provider);

        let targets = root.join("targets");
        fs::create_dir(&targets).expect("target parent must be created");
        Self {
            root,
            first_root,
            second_root,
            provider,
            cargo,
            rustc,
            first_target: targets.join("run-one"),
            second_target: targets.join("run-two"),
        }
    }

    fn request(&self) -> CandidateHeadQualificationRequest {
        CandidateHeadQualificationRequest {
            provider_executable: self.provider.clone(),
            cargo_executable: self.cargo.clone(),
            rustc_executable: self.rustc.clone(),
            first_candidate_root: self.first_root.clone(),
            second_candidate_root: self.second_root.clone(),
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

fn write_candidate(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("source directory must be created");
    fs::create_dir_all(root.join("third-party/.cargo"))
        .expect("Cargo seed directory must be created");
    fs::write(
        root.join("Cargo.toml"),
        b"[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )
    .expect("manifest must be written");
    fs::write(root.join("Cargo.lock"), b"fixture lock\n").expect("lock must be written");
    fs::write(root.join("reindeer.toml"), b"fixture = true\n").expect("config must be written");
    fs::write(root.join("rust-toolchain.toml"), TOOLCHAIN_FILE)
        .expect("toolchain file must be written");
    fs::write(root.join("third-party/BUCK"), b"generated\n")
        .expect("published Buck output must be written");
    fs::write(root.join("third-party/.cargo/seed"), b"identical seed\n")
        .expect("Cargo seed must be written");
    fs::write(root.join("src/lib.rs"), b"pub fn fixture() {}\n").expect("source must be written");
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
    let source = std::env::var_os("REINDEER_QUALIFICATION_FIXTURE_PROVIDER")
        .or_else(|| {
            option_env!("CARGO_BIN_EXE_reindeer_qualification_fixture_provider").map(OsString::from)
        })
        .expect("fixture provider path must be supplied by Cargo or Buck");
    fs::copy(source, destination).expect("fixture provider must be copied");
    make_executable(destination);
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
) -> Result<CandidateHeadQualificationArtifact, CandidateHeadQualificationFailure> {
    ReindeerCandidateHeadQualifier::default().generate(request)
}

fn qualify_with(
    request: &CandidateHeadQualificationRequest,
    limits: QualificationLimits,
) -> Result<CandidateHeadQualificationArtifact, CandidateHeadQualificationFailure> {
    ReindeerCandidateHeadQualifier::with_limits(limits).generate(request)
}

#[test]
fn foundation_fixture_candidate_head_qualification() {
    let fixture = Fixture::new("exact-contract");
    let artifact = qualify(&fixture.request()).expect("exact fixture contract must qualify");
    assert_eq!(artifact.generated_buck(), b"generated\n");
    assert!(fixture.first_target.join("provider-ran").is_file());
    assert!(fixture.second_target.join("provider-ran").is_file());
}

include!("candidate_head_qualification/provider_outcomes.rs");
include!("candidate_head_qualification/candidate_inputs.rs");
include!("candidate_head_qualification/resource_bounds.rs");
include!("candidate_head_qualification/cli.rs");
