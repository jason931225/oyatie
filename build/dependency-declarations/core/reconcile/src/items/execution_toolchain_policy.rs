const PATCH_ONLY_COMPONENTS: [&str; 2] = ["clippy", "rustfmt"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatchOnlyExecutionToolchainDecision {
    Unchanged(ExecutionChannel),
    ForwardPatch {
        protected: ExecutionChannel,
        candidate: ExecutionChannel,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PatchOnlyExecutionToolchainRefusal {
    ComponentSet(ToolchainSide, BTreeSet<String>),
    NonMinimalProfile(ToolchainSide, ExecutionToolchainProfile),
    TargetsChanged(BTreeSet<String>, BTreeSet<String>),
    MsrvChanged(Version, Version),
    ExecutionBelowMsrv(ExecutionChannel, Version),
    VersionDeltaNotAdmitted(ExecutionVersionDelta, ExecutionChannel, ExecutionChannel),
}

pub fn apply_patch_only_execution_toolchain_policy(
    analysis: &ExecutionToolchainAnalysis,
) -> Result<PatchOnlyExecutionToolchainDecision, PatchOnlyExecutionToolchainRefusal> {
    require_current_components(ToolchainSide::Protected, &analysis.protected.components)?;
    require_current_components(ToolchainSide::Candidate, &analysis.candidate.components)?;
    require_minimal_profile(ToolchainSide::Protected, analysis.protected.profile)?;
    require_minimal_profile(ToolchainSide::Candidate, analysis.candidate.profile)?;
    if analysis.delta.targets == DeclarationFieldDelta::Changed {
        return Err(PatchOnlyExecutionToolchainRefusal::TargetsChanged(
            analysis.protected.targets.clone(),
            analysis.candidate.targets.clone(),
        ));
    }
    if analysis.delta.msrv == DeclarationFieldDelta::Changed {
        return Err(PatchOnlyExecutionToolchainRefusal::MsrvChanged(
            analysis.protected.msrv.clone(),
            analysis.candidate.msrv.clone(),
        ));
    }
    if !analysis
        .candidate
        .execution
        .meets_msrv(&analysis.candidate.msrv)
    {
        return Err(PatchOnlyExecutionToolchainRefusal::ExecutionBelowMsrv(
            analysis.candidate.execution.clone(),
            analysis.candidate.msrv.clone(),
        ));
    }
    match analysis.delta.execution {
        ExecutionVersionDelta::Unchanged => Ok(PatchOnlyExecutionToolchainDecision::Unchanged(
            analysis.candidate.execution.clone(),
        )),
        ExecutionVersionDelta::ForwardPatch => {
            Ok(PatchOnlyExecutionToolchainDecision::ForwardPatch {
                protected: analysis.protected.execution.clone(),
                candidate: analysis.candidate.execution.clone(),
            })
        }
        delta => Err(PatchOnlyExecutionToolchainRefusal::VersionDeltaNotAdmitted(
            delta,
            analysis.protected.execution.clone(),
            analysis.candidate.execution.clone(),
        )),
    }
}

fn require_current_components(
    side: ToolchainSide,
    components: &BTreeSet<String>,
) -> Result<(), PatchOnlyExecutionToolchainRefusal> {
    let current = components.len() == PATCH_ONLY_COMPONENTS.len()
        && PATCH_ONLY_COMPONENTS
            .iter()
            .all(|component| components.contains(*component));
    if current {
        Ok(())
    } else {
        Err(PatchOnlyExecutionToolchainRefusal::ComponentSet(
            side,
            components.clone(),
        ))
    }
}

fn require_minimal_profile(
    side: ToolchainSide,
    profile: ExecutionToolchainProfile,
) -> Result<(), PatchOnlyExecutionToolchainRefusal> {
    if profile == ExecutionToolchainProfile::Minimal {
        Ok(())
    } else {
        Err(PatchOnlyExecutionToolchainRefusal::NonMinimalProfile(
            side, profile,
        ))
    }
}

impl fmt::Display for ExecutionToolchainProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Minimal => "minimal",
            Self::Default => "default",
            Self::Complete => "complete",
        })
    }
}

impl fmt::Display for ToolchainSide {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Protected => "protected",
            Self::Candidate => "candidate",
        })
    }
}

impl fmt::Display for DeclarationRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedToml(error) => write!(formatter, "malformed TOML: {error}"),
            Self::Missing(field) => write!(formatter, "missing `{field}`"),
            Self::Unknown(field) => write!(formatter, "unknown field `{field}`"),
            Self::WrongType(field, expected) => write!(formatter, "`{field}` must be {expected}"),
            Self::Duplicate(field, value) => write!(formatter, "`{field}` duplicates `{value}`"),
            Self::InvalidStableVersion(field, value) => {
                write!(
                    formatter,
                    "`{field}` value `{value}` is not an exact stable version"
                )
            }
            Self::InvalidNightlyDate(field, value) => {
                write!(
                    formatter,
                    "`{field}` value `{value}` is not an exact dated nightly `nightly-YYYY-MM-DD`"
                )
            }
            Self::FloatingNightlyChannel(field, value) => {
                write!(
                    formatter,
                    "`{field}` value `{value}` floats; an exact dated nightly `nightly-YYYY-MM-DD` is required"
                )
            }
            Self::UnsupportedValue(field, value) => {
                write!(formatter, "`{field}` value `{value}` is unsupported")
            }
        }
    }
}

impl fmt::Display for ExecutionToolchainAnalysisRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToolchain(side, reason) => {
                write!(formatter, "{side} rust-toolchain.toml is invalid: {reason}")
            }
            Self::InvalidMsrv(side, reason) => {
                write!(formatter, "{side} workspace MSRV is invalid: {reason}")
            }
        }
    }
}

impl fmt::Display for PatchOnlyExecutionToolchainRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComponentSet(side, components) => write!(
                formatter,
                "{side} components {components:?} do not match the current patch-only set"
            ),
            Self::NonMinimalProfile(side, profile) => {
                write!(formatter, "{side} profile `{profile}` is not minimal")
            }
            Self::TargetsChanged(protected, candidate) => write!(
                formatter,
                "targets changed from {protected:?} to {candidate:?} without qualification"
            ),
            Self::MsrvChanged(protected, candidate) => write!(
                formatter,
                "MSRV changed from {protected} to {candidate} without qualification"
            ),
            Self::ExecutionBelowMsrv(execution, msrv) => {
                write!(
                    formatter,
                    "execution toolchain {execution} is below MSRV {msrv}"
                )
            }
            Self::VersionDeltaNotAdmitted(delta, protected, candidate) => write!(
                formatter,
                "{delta:?} execution transition from {protected} to {candidate} requires qualification"
            ),
        }
    }
}

impl std::error::Error for PatchOnlyExecutionToolchainRefusal {}
impl std::error::Error for ExecutionToolchainAnalysisRefusal {}
