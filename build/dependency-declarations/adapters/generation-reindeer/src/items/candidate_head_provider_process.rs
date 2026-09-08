const PIPE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PIPE_DRAIN_GRACE: Duration = Duration::from_secs(2);
struct ProviderOutput {
    run: QualificationRun,
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct PipeCapture {
    bytes: Vec<u8>,
    total: usize,
}

#[derive(Clone, Copy)]
enum TerminationReason {
    Timeout,
    OutputLimit(QualificationStream),
}

fn execute_provider(
    run: QualificationRun,
    request: &ValidatedRequest,
    candidate_root: &Path,
    cargo_home: &Path,
    target_dir: &Path,
    limits: QualificationLimits,
) -> Result<ProviderOutput, CandidateHeadQualificationFailure> {
    let mut command = Command::new(&request.provider);
    command
        .arg("-c")
        .arg(candidate_root.join("reindeer.toml"))
        .arg("--cargo-path")
        .arg(&request.cargo)
        .arg("--rustc-path")
        .arg(&request.rustc)
        .arg("--cargo-options=--locked")
        .arg("buckify")
        .arg("--stdout")
        .current_dir(candidate_root)
        .env_clear()
        .env("CARGO_HOME", cargo_home)
        .env("CARGO_NET_OFFLINE", "true")
        .env("CARGO_TARGET_DIR", target_dir)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_provider_group(&mut command);
    let mut child =
        command
            .spawn()
            .map_err(|error| CandidateHeadQualificationFailure::ProviderSpawn {
                run,
                kind: error.kind(),
            })?;
    let stdout = child.stdout.take().expect("piped stdout must exist");
    let stderr = child.stderr.take().expect("piped stderr must exist");
    let stdout_observed = Arc::new(AtomicUsize::new(0));
    let stderr_observed = Arc::new(AtomicUsize::new(0));
    let stdout_receiver =
        spawn_pipe_reader(stdout, limits.stdout_bytes, Arc::clone(&stdout_observed));
    let stderr_receiver =
        spawn_pipe_reader(stderr, limits.stderr_bytes, Arc::clone(&stderr_observed));

    let start = Instant::now();
    let mut termination = None;
    let mut wait_failure = None;
    loop {
        if stdout_observed.load(Ordering::Relaxed) > limits.stdout_bytes {
            termination = Some(TerminationReason::OutputLimit(QualificationStream::Stdout));
            break;
        }
        if stderr_observed.load(Ordering::Relaxed) > limits.stderr_bytes {
            termination = Some(TerminationReason::OutputLimit(QualificationStream::Stderr));
            break;
        }
        match provider_exited(&child) {
            Ok(true) => break,
            Ok(false) if start.elapsed() >= limits.runtime => {
                termination = Some(TerminationReason::Timeout);
                break;
            }
            Ok(false) => thread::sleep(PIPE_POLL_INTERVAL),
            Err(error) => {
                wait_failure = Some(CandidateHeadQualificationFailure::ProviderWait {
                    run,
                    kind: error.kind(),
                });
                break;
            }
        }
    }
    let status = terminate_child(run, &mut child);

    let drain_deadline = Instant::now() + PIPE_DRAIN_GRACE;
    let stdout_capture = receive_pipe(
        run,
        QualificationStream::Stdout,
        stdout_receiver,
        drain_deadline,
    );
    let stderr_capture = receive_pipe(
        run,
        QualificationStream::Stderr,
        stderr_receiver,
        drain_deadline,
    );
    let status = status?;
    let stdout_capture = stdout_capture?;
    let stderr_capture = stderr_capture?;
    if let Some(failure) = wait_failure {
        return Err(failure);
    }
    if let Some(reason) = termination {
        return match reason {
            TerminationReason::Timeout => Err(CandidateHeadQualificationFailure::ProviderTimeout {
                run,
                limit: limits.runtime,
            }),
            TerminationReason::OutputLimit(stream) => {
                let (limit, observed) = match stream {
                    QualificationStream::Stdout => {
                        (limits.stdout_bytes, stdout_observed.load(Ordering::Relaxed))
                    }
                    QualificationStream::Stderr => {
                        (limits.stderr_bytes, stderr_observed.load(Ordering::Relaxed))
                    }
                };
                Err(CandidateHeadQualificationFailure::OutputLimitExceeded {
                    run,
                    stream,
                    limit,
                    observed_at_least: observed,
                })
            }
        };
    }

    if stdout_capture.total > limits.stdout_bytes {
        return Err(CandidateHeadQualificationFailure::OutputLimitExceeded {
            run,
            stream: QualificationStream::Stdout,
            limit: limits.stdout_bytes,
            observed_at_least: stdout_capture.total,
        });
    }
    if stderr_capture.total > limits.stderr_bytes {
        return Err(CandidateHeadQualificationFailure::OutputLimitExceeded {
            run,
            stream: QualificationStream::Stderr,
            limit: limits.stderr_bytes,
            observed_at_least: stderr_capture.total,
        });
    }
    Ok(ProviderOutput {
        run,
        status,
        stdout: stdout_capture.bytes,
        stderr: stderr_capture.bytes,
    })
}

fn terminate_child(
    run: QualificationRun,
    child: &mut Child,
) -> Result<ExitStatus, CandidateHeadQualificationFailure> {
    let group_result = terminate_provider_group(run, child.id());
    if group_result.is_err() {
        let _ = child.kill();
    }
    let status = child
        .wait()
        .map_err(|error| CandidateHeadQualificationFailure::ProviderReap {
            run,
            kind: error.kind(),
        });
    group_result?;
    status
}

fn spawn_pipe_reader<R: QualificationPipe + Send + 'static>(
    mut reader: R,
    limit: usize,
    observed: Arc<AtomicUsize>,
) -> PipeReader {
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let worker = thread::spawn(move || {
        if let Err(error) = nonblocking_pipe(&reader) {
            let _ = sender.send(Err(error.kind()));
            return;
        }
        let mut bytes = Vec::with_capacity(cmp::min(limit, FILE_BUFFER_BYTES));
        let mut total = 0_usize;
        let mut buffer = [0_u8; 8 * 1024];
        let result = loop {
            if worker_cancelled.load(Ordering::Relaxed) {
                break Ok(PipeCapture { bytes, total });
            }
            match reader.read(&mut buffer) {
                Ok(0) => break Ok(PipeCapture { bytes, total }),
                Ok(count) => {
                    total = total.saturating_add(count);
                    observed.store(total, Ordering::Relaxed);
                    if bytes.len() < limit {
                        let keep = cmp::min(count, limit - bytes.len());
                        bytes.extend_from_slice(&buffer[..keep]);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(PIPE_POLL_INTERVAL);
                }
                Err(error) => break Err(error.kind()),
            }
        };
        let _ = sender.send(result);
    });
    PipeReader {
        receiver,
        cancelled,
        worker: Some(worker),
    }
}

fn receive_pipe(
    run: QualificationRun,
    stream: QualificationStream,
    reader: PipeReader,
    deadline: Instant,
) -> Result<PipeCapture, CandidateHeadQualificationFailure> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    match reader.receiver.recv_timeout(remaining) {
        Ok(Ok(capture)) => Ok(capture),
        Ok(Err(kind)) => Err(CandidateHeadQualificationFailure::OutputRead { run, stream, kind }),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(CandidateHeadQualificationFailure::OutputDrainTimeout { run, stream })
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(CandidateHeadQualificationFailure::OutputReaderPanicked { run, stream })
        }
    }
}

fn accept_provider_output(
    output: ProviderOutput,
) -> Result<ProviderOutput, CandidateHeadQualificationFailure> {
    if !output.status.success() {
        return Err(CandidateHeadQualificationFailure::ProviderExit {
            run: output.run,
            code: output.status.code(),
            stdout_bytes: output.stdout.len(),
            stderr: output.stderr,
        });
    }
    if !output.stderr.is_empty() {
        return Err(CandidateHeadQualificationFailure::ProviderStderr {
            run: output.run,
            stderr: output.stderr,
        });
    }
    if output.stdout.is_empty() {
        return Err(CandidateHeadQualificationFailure::EmptyOutput { run: output.run });
    }
    Ok(output)
}
