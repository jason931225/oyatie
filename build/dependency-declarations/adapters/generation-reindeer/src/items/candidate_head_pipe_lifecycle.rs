struct PipeReader {
    receiver: mpsc::Receiver<Result<PipeCapture, io::ErrorKind>>,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Drop for PipeReader {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(unix)]
trait QualificationPipe: Read + std::os::fd::AsFd {}
#[cfg(unix)]
impl<T: Read + std::os::fd::AsFd> QualificationPipe for T {}

#[cfg(not(unix))]
trait QualificationPipe: Read {}
#[cfg(not(unix))]
impl<T: Read> QualificationPipe for T {}

#[cfg(unix)]
fn nonblocking_pipe(pipe: &impl QualificationPipe) -> io::Result<()> {
    use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
    let flags = fcntl_getfl(pipe)?;
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK).map_err(io::Error::from)
}

#[cfg(not(unix))]
fn nonblocking_pipe(_pipe: &impl QualificationPipe) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}
