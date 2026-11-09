//! Bounded reference-worker protocol, not a general process sandbox/quiescence API.
use crate::ReferenceError;
use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};
struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
fn read(
    mut pipe: impl Read,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> Result<Vec<u8>, ReferenceError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let n = pipe.read(&mut buffer).map_err(|_| ReferenceError::Io)?;
        if n == 0 {
            return Ok(bytes);
        }
        if bytes.len().checked_add(n).is_none_or(|n| n > limit) {
            overflow.store(true, Ordering::Relaxed);
            return Err(ReferenceError::OutputLimit);
        }
        bytes.try_reserve(n).map_err(|_| ReferenceError::Capacity)?;
        bytes.extend_from_slice(&buffer[..n]);
    }
}
pub(super) fn execute(
    mut command: Command,
    input: Vec<u8>,
    timeout: Duration,
    output_limit: usize,
) -> Result<Vec<u8>, ReferenceError> {
    let start = Instant::now();
    let mut worker = Worker(
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ReferenceError::ProcessFailed)?,
    );
    let mut stdin = worker.0.stdin.take().ok_or(ReferenceError::ProcessFailed)?;
    let stdout = worker
        .0
        .stdout
        .take()
        .ok_or(ReferenceError::ProcessFailed)?;
    let stderr = worker
        .0
        .stderr
        .take()
        .ok_or(ReferenceError::ProcessFailed)?;
    let overflow = Arc::new(AtomicBool::new(false));
    let flag = overflow.clone();
    let out = std::thread::Builder::new()
        .spawn(move || read(stdout, output_limit, flag))
        .map_err(|_| ReferenceError::ProcessFailed)?;
    let flag = overflow.clone();
    let err = std::thread::Builder::new()
        .spawn(move || read(stderr, 1024, flag))
        .map_err(|_| ReferenceError::ProcessFailed)?;
    let writer = std::thread::Builder::new()
        .spawn(move || stdin.write_all(&input))
        .map_err(|_| ReferenceError::ProcessFailed)?;
    let mut refusal = None;
    let status = loop {
        if let Some(status) = worker
            .0
            .try_wait()
            .map_err(|_| ReferenceError::ProcessFailed)?
        {
            break status;
        }
        if overflow.load(Ordering::Relaxed) {
            refusal = Some(ReferenceError::OutputLimit);
        } else if start.elapsed() >= timeout {
            refusal = Some(ReferenceError::Deadline);
        }
        if refusal.is_some() {
            let _ = worker.0.kill();
            break worker.0.wait().map_err(|_| ReferenceError::ProcessFailed)?;
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let write = writer.join().map_err(|_| ReferenceError::ProcessFailed)?;
    let stdout = out.join().map_err(|_| ReferenceError::ProcessFailed)?;
    let stderr = err.join().map_err(|_| ReferenceError::ProcessFailed)?;
    if let Some(error) = refusal {
        return Err(error);
    }
    let stdout = stdout?;
    let stderr = stderr?;
    if !status.success() || write.is_err() || !stderr.is_empty() {
        return Err(ReferenceError::ProcessFailed);
    }
    Ok(stdout)
}
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn output_limit_and_worker_failure_are_refusals_not_partial_tables() {
        let mut command = Command::new("/usr/bin/printf");
        command.arg("%s").arg("abcdef");
        assert_eq!(
            execute(command, vec![], Duration::from_secs(1), 3),
            Err(ReferenceError::OutputLimit)
        );
        assert_eq!(
            execute(
                Command::new("/usr/bin/false"),
                vec![],
                Duration::from_secs(1),
                64
            ),
            Err(ReferenceError::ProcessFailed)
        );
    }
    #[test]
    fn worker_deadline_stops_and_reaps_the_direct_reference_process() {
        let mut command = Command::new("/bin/sleep");
        command.arg("2");
        assert_eq!(
            execute(command, vec![], Duration::from_millis(10), 64),
            Err(ReferenceError::Deadline)
        );
    }
}
