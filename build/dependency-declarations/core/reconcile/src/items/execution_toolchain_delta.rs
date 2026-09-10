/// Direction of an execution-channel transition.
///
/// The semver directions classify stable-to-stable moves only. Nightly dates
/// order among themselves; a stable and a dated nightly do not order at all,
/// and that pair is reported as `ChannelChanged` rather than ranked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionVersionDelta {
    Unchanged,
    ForwardPatch,
    ForwardMinor,
    ForwardMajor,
    Downgrade,
    NightlyDateForward,
    NightlyDateBackward,
    ChannelChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclarationFieldDelta {
    Unchanged,
    Changed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionToolchainDelta {
    execution: ExecutionVersionDelta,
    msrv: DeclarationFieldDelta,
    profile: DeclarationFieldDelta,
    components: DeclarationFieldDelta,
    targets: DeclarationFieldDelta,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionToolchainAnalysis {
    protected: ExecutionToolchainState,
    candidate: ExecutionToolchainState,
    delta: ExecutionToolchainDelta,
}

impl ExecutionToolchainDelta {
    pub fn execution(&self) -> ExecutionVersionDelta {
        self.execution
    }

    pub fn msrv(&self) -> DeclarationFieldDelta {
        self.msrv
    }

    pub fn profile(&self) -> DeclarationFieldDelta {
        self.profile
    }

    pub fn components(&self) -> DeclarationFieldDelta {
        self.components
    }

    pub fn targets(&self) -> DeclarationFieldDelta {
        self.targets
    }
}

impl ExecutionToolchainAnalysis {
    pub fn protected(&self) -> &ExecutionToolchainState {
        &self.protected
    }

    pub fn candidate(&self) -> &ExecutionToolchainState {
        &self.candidate
    }

    pub fn delta(&self) -> &ExecutionToolchainDelta {
        &self.delta
    }
}

pub fn analyze_execution_toolchain_transition(
    protected_toolchain: &str,
    candidate_toolchain: &str,
    protected_workspace: &str,
    candidate_workspace: &str,
) -> Result<ExecutionToolchainAnalysis, ExecutionToolchainAnalysisRefusal> {
    let protected = parse_state(
        ToolchainSide::Protected,
        protected_toolchain,
        protected_workspace,
    )?;
    let candidate = parse_state(
        ToolchainSide::Candidate,
        candidate_toolchain,
        candidate_workspace,
    )?;
    let delta = ExecutionToolchainDelta {
        execution: execution_delta(&protected.execution, &candidate.execution),
        msrv: field_delta(&protected.msrv, &candidate.msrv),
        profile: field_delta(&protected.profile, &candidate.profile),
        components: field_delta(&protected.components, &candidate.components),
        targets: field_delta(&protected.targets, &candidate.targets),
    };
    Ok(ExecutionToolchainAnalysis {
        protected,
        candidate,
        delta,
    })
}

fn parse_state(
    side: ToolchainSide,
    toolchain_source: &str,
    workspace_source: &str,
) -> Result<ExecutionToolchainState, ExecutionToolchainAnalysisRefusal> {
    let toolchain = parse_toolchain(toolchain_source)
        .map_err(|reason| ExecutionToolchainAnalysisRefusal::InvalidToolchain(side, reason))?;
    let msrv = parse_msrv(workspace_source)
        .map_err(|reason| ExecutionToolchainAnalysisRefusal::InvalidMsrv(side, reason))?;
    Ok(ExecutionToolchainState {
        execution: toolchain.execution,
        msrv,
        profile: toolchain.profile,
        components: toolchain.components,
        targets: toolchain.targets,
    })
}

fn execution_delta(
    protected: &ExecutionChannel,
    candidate: &ExecutionChannel,
) -> ExecutionVersionDelta {
    if candidate == protected {
        return ExecutionVersionDelta::Unchanged;
    }
    match (protected, candidate) {
        (ExecutionChannel::Stable(protected), ExecutionChannel::Stable(candidate)) => {
            stable_delta(protected, candidate)
        }
        (ExecutionChannel::DatedNightly(protected), ExecutionChannel::DatedNightly(candidate)) => {
            if candidate > protected {
                ExecutionVersionDelta::NightlyDateForward
            } else {
                ExecutionVersionDelta::NightlyDateBackward
            }
        }
        (ExecutionChannel::Stable(_), ExecutionChannel::DatedNightly(_))
        | (ExecutionChannel::DatedNightly(_), ExecutionChannel::Stable(_)) => {
            ExecutionVersionDelta::ChannelChanged
        }
    }
}

fn stable_delta(protected: &Version, candidate: &Version) -> ExecutionVersionDelta {
    if candidate < protected {
        ExecutionVersionDelta::Downgrade
    } else if candidate.major > protected.major {
        ExecutionVersionDelta::ForwardMajor
    } else if candidate.minor > protected.minor {
        ExecutionVersionDelta::ForwardMinor
    } else {
        ExecutionVersionDelta::ForwardPatch
    }
}

fn field_delta<T: Eq>(protected: &T, candidate: &T) -> DeclarationFieldDelta {
    if protected == candidate {
        DeclarationFieldDelta::Unchanged
    } else {
        DeclarationFieldDelta::Changed
    }
}
