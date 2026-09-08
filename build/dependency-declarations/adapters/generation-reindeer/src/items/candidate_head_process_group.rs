#[cfg(any(target_os = "linux", target_os = "macos"))]
fn provider_exited(child: &Child) -> io::Result<bool> {
    use rustix::process::{Pid, WaitId, WaitIdOptions, waitid};
    let pid = Pid::from_raw(child.id() as i32)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    waitid(
        WaitId::Pid(pid),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )
    .map(|status| status.is_some())
    .map_err(io::Error::from)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn provider_exited(_child: &Child) -> io::Result<bool> {
    Err(io::ErrorKind::Unsupported.into())
}

#[cfg(unix)]
fn configure_provider_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_provider_group(_command: &mut Command) {}

#[cfg(unix)]
fn terminate_provider_group(
    run: QualificationRun,
    id: u32,
) -> Result<(), CandidateHeadQualificationFailure> {
    use rustix::process::{Pid, Signal, kill_process_group};
    let pid = Pid::from_raw(id as i32).ok_or(CandidateHeadQualificationFailure::ProviderKill {
        run,
        kind: io::ErrorKind::InvalidInput,
    })?;
    match kill_process_group(pid, Signal::KILL) {
        Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(error) => Err(CandidateHeadQualificationFailure::ProviderKill {
            run,
            kind: io::Error::from(error).kind(),
        }),
    }
}

#[cfg(not(unix))]
fn terminate_provider_group(
    _run: QualificationRun,
    _id: u32,
) -> Result<(), CandidateHeadQualificationFailure> {
    Err(CandidateHeadQualificationFailure::UnsupportedPlatform)
}
