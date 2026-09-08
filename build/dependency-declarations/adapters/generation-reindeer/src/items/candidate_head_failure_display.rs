impl fmt::Display for CandidateHeadQualificationFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(
                    formatter,
                    "qualification requires Unix filesystem identity semantics"
                )
            }
            Self::InvalidPath {
                field,
                path,
                reason,
            } => write!(
                formatter,
                "qualification path {field:?} at {} was refused: {reason:?}",
                path.display()
            ),
            Self::InvalidLimit {
                limit,
                value,
                maximum,
            } => write!(
                formatter,
                "qualification limit {limit:?} value {value} is outside 1..={maximum}"
            ),
            Self::MissingCandidateInput { root, path } => write!(
                formatter,
                "{root:?} candidate is missing required input {}",
                path.display()
            ),
            Self::InvalidCandidateInput {
                root,
                path,
                expected_directory,
            } => write!(
                formatter,
                "{root:?} candidate input {} is not a {}",
                path.display(),
                if *expected_directory {
                    "directory"
                } else {
                    "regular file"
                }
            ),
            Self::AncestorCargoConfig { root, path } => write!(
                formatter,
                "{root:?} candidate has an uncontrolled ancestor Cargo configuration at {}",
                path.display()
            ),
            Self::CandidateTreeRead {
                root,
                scope,
                path,
                kind,
            } => write!(
                formatter,
                "{root:?} {scope:?} tree entry {} could not be read: {kind:?}",
                path.display()
            ),
            Self::UnsupportedCandidateEntry {
                root,
                scope,
                path,
                kind,
            } => write!(
                formatter,
                "{root:?} {scope:?} tree entry {} was refused: {kind:?}",
                path.display()
            ),
            Self::CandidateTreeLimitExceeded {
                root,
                scope,
                limit,
                value,
                maximum,
            } => write!(
                formatter,
                "{root:?} {scope:?} tree exceeded {limit:?}: {value} > {maximum}"
            ),
            Self::CandidateTreesDiffer { scope, path } => write!(
                formatter,
                "candidate {scope:?} trees differ at {}",
                path.display()
            ),
            Self::CandidateTreesShareStorage { scope, path } => write!(
                formatter,
                "candidate {scope:?} trees share file storage at {}",
                path.display()
            ),
            Self::CandidateTreeChanged { root, scope } => {
                write!(
                    formatter,
                    "{root:?} candidate {scope:?} tree changed during qualification"
                )
            }
            Self::ToolRead { tool, path, kind } => write!(
                formatter,
                "{tool:?} executable {} could not be inspected: {kind:?}",
                path.display()
            ),
            Self::TargetCreate { run, path, kind } => write!(
                formatter,
                "{run:?} target directory {} could not be created: {kind:?}",
                path.display()
            ),
            Self::ProviderSpawn { run, kind } => {
                write!(
                    formatter,
                    "{run:?} Reindeer execution could not start: {kind:?}"
                )
            }
            Self::ProviderWait { run, kind } => {
                write!(
                    formatter,
                    "{run:?} Reindeer execution could not be observed: {kind:?}"
                )
            }
            Self::ProviderKill { run, kind } => {
                write!(
                    formatter,
                    "{run:?} Reindeer execution could not be killed: {kind:?}"
                )
            }
            Self::ProviderReap { run, kind } => {
                write!(
                    formatter,
                    "{run:?} Reindeer execution could not be reaped: {kind:?}"
                )
            }
            Self::ProviderTimeout { run, limit } => {
                write!(formatter, "{run:?} Reindeer execution exceeded {limit:?}")
            }
            Self::OutputRead { run, stream, kind } => write!(
                formatter,
                "{run:?} Reindeer {stream:?} could not be read: {kind:?}"
            ),
            Self::OutputReaderPanicked { run, stream } => {
                write!(formatter, "{run:?} Reindeer {stream:?} reader panicked")
            }
            Self::OutputDrainTimeout { run, stream } => {
                write!(
                    formatter,
                    "{run:?} Reindeer {stream:?} did not close after process exit"
                )
            }
            Self::OutputLimitExceeded {
                run,
                stream,
                limit,
                observed_at_least,
            } => write!(
                formatter,
                "{run:?} Reindeer {stream:?} exceeded {limit} bytes (observed at least {observed_at_least})"
            ),
            Self::ProviderExit {
                run,
                code,
                stdout_bytes,
                stderr,
            } => write!(
                formatter,
                "{run:?} Reindeer execution failed with code {code:?}, {stdout_bytes} withheld stdout bytes, and {} diagnostic bytes",
                stderr.len()
            ),
            Self::ProviderStderr { run, stderr } => write!(
                formatter,
                "{run:?} Reindeer execution succeeded but emitted {} diagnostic bytes",
                stderr.len()
            ),
            Self::EmptyOutput { run } => {
                write!(
                    formatter,
                    "{run:?} Reindeer execution emitted an empty artifact"
                )
            }
            Self::NondeterministicOutput {
                first_bytes,
                second_bytes,
                first_difference,
            } => write!(
                formatter,
                "Reindeer executions differed at byte {first_difference} ({first_bytes} versus {second_bytes} bytes)"
            ),
            Self::PublishedOutputMismatch {
                root,
                generated_bytes,
                published_bytes,
                first_difference,
            } => write!(
                formatter,
                "{root:?} generated output differs from published third-party/BUCK at byte {first_difference} ({generated_bytes} versus {published_bytes} bytes)"
            ),
        }
    }
}

impl std::error::Error for CandidateHeadQualificationFailure {}
