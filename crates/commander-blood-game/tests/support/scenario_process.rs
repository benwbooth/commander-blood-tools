use std::fs::File;
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

pub struct ProcessOutcome {
    pub status: ExitStatus,
    pub timed_out: bool,
}

struct ChildGuard {
    child: Option<Child>,
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn run(
    command: &mut Command,
    artifact_dir: &Path,
    timeout: Duration,
) -> io::Result<ProcessOutcome> {
    if timeout.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "process timeout must be nonzero",
        ));
    }
    let stdout = File::create(artifact_dir.join("stdout.log"))?;
    let stderr = File::create(artifact_dir.join("stderr.log"))?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let mut guard = ChildGuard {
        child: Some(command.spawn()?),
    };
    let started = Instant::now();
    let poll = Duration::from_millis(5);
    loop {
        if let Some(status) = guard
            .child
            .as_mut()
            .expect("child guard is armed")
            .try_wait()?
        {
            guard.child.take();
            return Ok(ProcessOutcome {
                status,
                timed_out: false,
            });
        }
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            let child = guard.child.as_mut().expect("child guard is armed");
            child.kill()?;
            let status = child.wait()?;
            guard.child.take();
            return Ok(ProcessOutcome {
                status,
                timed_out: true,
            });
        }
        std::thread::sleep(poll.min(remaining));
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn artifact_dir() -> std::path::PathBuf {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "commander-blood-scenario-process-{}-{}",
            std::process::id(),
            id
        ));
        fs::create_dir(&path).unwrap();
        path
    }
    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }
    #[test]
    fn success_captures_stdout_and_stderr() {
        let dir = artifact_dir();
        let result = run(
            &mut shell("printf out; printf err >&2"),
            &dir,
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(result.status.success());
        assert!(!result.timed_out);
        assert_eq!(fs::read_to_string(dir.join("stdout.log")).unwrap(), "out");
        assert_eq!(fs::read_to_string(dir.join("stderr.log")).unwrap(), "err");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn preserves_nonzero_exit_status() {
        let dir = artifact_dir();
        let result = run(&mut shell("exit 7"), &dir, Duration::from_secs(1)).unwrap();
        assert_eq!(result.status.code(), Some(7));
        assert!(!result.timed_out);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn kills_on_timeout() {
        let dir = artifact_dir();
        let result = run(
            &mut shell("printf '%s' \"$$\"; while :; do :; done"),
            &dir,
            Duration::from_millis(40),
        )
        .unwrap();
        assert!(result.timed_out);
        assert!(!result.status.success());
        #[cfg(target_os = "linux")]
        {
            let pid = fs::read_to_string(dir.join("stdout.log")).unwrap();
            assert!(!pid.is_empty());
            assert!(
                !Path::new("/proc").join(pid).exists(),
                "timed-out child was not reaped"
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn zero_timeout_is_rejected_before_spawning() {
        let dir = artifact_dir();
        let error = run(&mut shell("exit 0"), &dir, Duration::ZERO)
            .err()
            .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(!dir.join("stdout.log").exists());
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn reaps_successful_child() {
        let dir = artifact_dir();
        let result = run(
            &mut shell("printf '%s' \"$$\""),
            &dir,
            Duration::from_secs(1),
        )
        .unwrap();
        assert!(result.status.success());
        let pid = fs::read_to_string(dir.join("stdout.log")).unwrap();
        let process_path = std::path::Path::new("/proc").join(&pid);
        #[cfg(target_os = "linux")]
        assert!(
            !process_path.exists(),
            "child process {} was not reaped",
            pid
        );
        #[cfg(not(target_os = "linux"))]
        assert!(
            !Command::new("sh")
                .arg("-c")
                .arg(format!("kill -0 {}", pid))
                .status()
                .unwrap()
                .success()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
