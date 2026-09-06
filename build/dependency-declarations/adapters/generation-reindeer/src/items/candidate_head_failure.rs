/// Typed, fail-closed refusal from candidate qualification.
#[derive(Debug, Eq, PartialEq)]
pub enum CandidateHeadQualificationFailure {
    UnsupportedPlatform,
    InvalidPath {
        field: QualificationPath,
        path: PathBuf,
        reason: PathRefusal,
    },
    InvalidLimit {
        limit: QualificationLimit,
        value: u128,
        maximum: u128,
    },
    MissingCandidateInput {
        root: CandidateRoot,
        path: PathBuf,
    },
    InvalidCandidateInput {
        root: CandidateRoot,
        path: PathBuf,
        expected_directory: bool,
    },
    AncestorCargoConfig {
        root: CandidateRoot,
        path: PathBuf,
    },
    CandidateTreeRead {
        root: CandidateRoot,
        scope: CandidateTreeScope,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    UnsupportedCandidateEntry {
        root: CandidateRoot,
        scope: CandidateTreeScope,
        path: PathBuf,
        kind: UnsupportedCandidateEntryKind,
    },
    CandidateTreeLimitExceeded {
        root: CandidateRoot,
        scope: CandidateTreeScope,
        limit: QualificationLimit,
        value: u64,
        maximum: u64,
    },
    CandidateTreesDiffer {
        scope: CandidateTreeScope,
        path: PathBuf,
    },
    CandidateTreesShareStorage {
        scope: CandidateTreeScope,
        path: PathBuf,
    },
    CandidateTreeChanged {
        root: CandidateRoot,
        scope: CandidateTreeScope,
    },
    ToolRead {
        tool: QualificationTool,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    TargetCreate {
        run: QualificationRun,
        path: PathBuf,
        kind: io::ErrorKind,
    },
    ProviderSpawn {
        run: QualificationRun,
        kind: io::ErrorKind,
    },
    ProviderWait {
        run: QualificationRun,
        kind: io::ErrorKind,
    },
    ProviderKill {
        run: QualificationRun,
        kind: io::ErrorKind,
    },
    ProviderReap {
        run: QualificationRun,
        kind: io::ErrorKind,
    },
    ProviderTimeout {
        run: QualificationRun,
        limit: Duration,
    },
    OutputRead {
        run: QualificationRun,
        stream: QualificationStream,
        kind: io::ErrorKind,
    },
    OutputReaderPanicked {
        run: QualificationRun,
        stream: QualificationStream,
    },
    OutputDrainTimeout {
        run: QualificationRun,
        stream: QualificationStream,
    },
    OutputLimitExceeded {
        run: QualificationRun,
        stream: QualificationStream,
        limit: usize,
        observed_at_least: usize,
    },
    ProviderExit {
        run: QualificationRun,
        code: Option<i32>,
        stdout_bytes: usize,
        stderr: Vec<u8>,
    },
    ProviderStderr {
        run: QualificationRun,
        stderr: Vec<u8>,
    },
    EmptyOutput {
        run: QualificationRun,
    },
    NondeterministicOutput {
        first_bytes: usize,
        second_bytes: usize,
        first_difference: usize,
    },
    PublishedOutputMismatch {
        root: CandidateRoot,
        generated_bytes: usize,
        published_bytes: usize,
        first_difference: usize,
    },
}
