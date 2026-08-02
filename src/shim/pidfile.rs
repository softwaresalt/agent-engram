//! PID file helpers for daemon reliability harnesses.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sysinfo::{Pid, System};

use crate::errors::{EngramError, SystemError};

const PID_FILE_NAME: &str = "engram.pid";
const RUN_DIR_NAME: &str = "run";
const UNKNOWN_START_TIME_UNIX: u64 = 1;

/// PID file metadata stored in `.engram/run/engram.pid`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PidFile {
    /// Process identifier recorded in `.engram/run/engram.pid`.
    pub pid: u32,
    /// Recorded process start time expressed as Unix seconds.
    #[serde(default)]
    pub start_time_unix: u64,
}

impl PidFile {
    /// Build PID metadata for the current process.
    #[must_use]
    pub fn current() -> Self {
        let pid = std::process::id();
        let process_id = Pid::from_u32(pid);
        let mut system = System::new();
        let start_time_unix = if system.refresh_process(process_id) {
            system
                .process(process_id)
                .map_or(UNKNOWN_START_TIME_UNIX, sysinfo::Process::start_time)
        } else {
            UNKNOWN_START_TIME_UNIX
        };

        Self {
            pid,
            start_time_unix,
        }
    }

    /// Return the runtime PID path for `workspace`.
    #[must_use]
    pub fn path(workspace: &Path) -> PathBuf {
        workspace
            .join(".engram")
            .join(RUN_DIR_NAME)
            .join(PID_FILE_NAME)
    }

    /// Read PID metadata from disk, accepting both JSON and legacy numeric PID files.
    #[must_use]
    pub fn read(workspace: &Path) -> Option<Self> {
        let raw = std::fs::read_to_string(Self::path(workspace)).ok()?;
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            return None;
        }

        serde_json::from_str(trimmed).ok().or_else(|| {
            trimmed.parse::<u32>().ok().map(|pid| Self {
                pid,
                start_time_unix: UNKNOWN_START_TIME_UNIX,
            })
        })
    }

    /// Verify whether the recorded PID still refers to the same live process.
    ///
    /// # Errors
    ///
    /// This probe is currently infallible in practice and returns `Ok(false)`
    /// whenever the process cannot be verified as live.
    pub fn verify_alive(&self) -> Result<bool, EngramError> {
        if self.pid == 0 {
            return Ok(false);
        }

        let mut system = System::new();
        let process_id = Pid::from_u32(self.pid);

        if !system.refresh_process(process_id) {
            return Ok(false);
        }

        let Some(process) = system.process(process_id) else {
            return Ok(false);
        };

        if self.start_time_unix > UNKNOWN_START_TIME_UNIX
            && process.start_time() != self.start_time_unix
        {
            return Ok(false);
        }

        if !system.refresh_process(process_id) {
            return Ok(false);
        }

        let Some(process) = system.process(process_id) else {
            return Ok(false);
        };

        Ok(self.start_time_unix <= UNKNOWN_START_TIME_UNIX
            || process.start_time() == self.start_time_unix)
    }

    /// Persist the PID file via a same-directory temp file and atomic rename.
    ///
    /// # Errors
    ///
    /// Returns [`EngramError::System`] when directory creation, serialization,
    /// or persistence fails.
    pub fn atomic_write(&self, pid_dir: &Path) -> Result<PathBuf, EngramError> {
        std::fs::create_dir_all(pid_dir).map_err(|_| flush_err(pid_dir))?;

        let target_path = pid_dir.join(PID_FILE_NAME);
        let serialized = serde_json::to_vec(self).map_err(|_| flush_err(&target_path))?;

        let mut temp_file =
            tempfile::NamedTempFile::new_in(pid_dir).map_err(|_| flush_err(&target_path))?;

        temp_file
            .write_all(&serialized)
            .map_err(|_| flush_err(&target_path))?;
        temp_file.flush().map_err(|_| flush_err(&target_path))?;
        temp_file
            .as_file()
            .sync_all()
            .map_err(|_| flush_err(&target_path))?;
        temp_file
            .persist(&target_path)
            .map_err(|_| flush_err(&target_path))?;

        Ok(target_path)
    }
}

fn flush_err(path: &Path) -> EngramError {
    EngramError::System(SystemError::FlushFailed {
        path: path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::io;

    #[cfg(windows)]
    use std::ffi::OsStr;
    #[cfg(windows)]
    use std::io::{BufRead as _, BufReader, Write as _};
    #[cfg(windows)]
    use std::process::{Command, Stdio};
    #[cfg(windows)]
    use sysinfo::{Pid, System};

    use super::{PidFile, UNKNOWN_START_TIME_UNIX};

    #[cfg(windows)]
    const CHILD_FIXTURE_ENV: &str = "ENGRAM_PIDFILE_REAPED_CHILD_FIXTURE";
    #[cfg(windows)]
    const CHILD_FIXTURE_VALUE: &str = "110.001-T";
    #[cfg(windows)]
    const CHILD_READY_MARKER: &[u8] = b"ENGRAM_PIDFILE_110_001_T_CHILD_READY\n";
    #[cfg(windows)]
    const REAPED_CHILD_TEST_NAME: &str =
        "shim::pidfile::tests::verify_alive_returns_false_for_reaped_child";

    type TestResult = Result<(), Box<dyn Error>>;

    #[cfg(windows)]
    fn run_reaped_child_fixture() -> TestResult {
        {
            let stdout = io::stdout();
            let mut stdout = stdout.lock();
            stdout.write_all(b"\n")?;
            stdout.write_all(CHILD_READY_MARKER)?;
            stdout.flush()?;
        }

        io::copy(&mut io::stdin().lock(), &mut io::sink())?;
        Ok(())
    }

    #[cfg(windows)]
    fn wait_for_child_ready(stdout: impl std::io::Read) -> io::Result<()> {
        let mut stdout = BufReader::new(stdout);
        let mut line = Vec::new();

        loop {
            line.clear();
            if stdout.read_until(b'\n', &mut line)? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "child exited before emitting the PID identity readiness marker",
                ));
            }
            if line == CHILD_READY_MARKER {
                return Ok(());
            }
        }
    }

    #[cfg(windows)]
    fn process_start_time(pid: u32) -> io::Result<u64> {
        let process_id = Pid::from_u32(pid);
        let mut system = System::new();
        if !system.refresh_process(process_id) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("child process {pid} was not visible to sysinfo after readiness"),
            ));
        }

        let start_time = system
            .process(process_id)
            .map(sysinfo::Process::start_time)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("sysinfo omitted ready child process {pid}"),
                )
            })?;

        if start_time == UNKNOWN_START_TIME_UNIX {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("child process {pid} had the legacy start-time sentinel"),
            ));
        }

        Ok(start_time)
    }

    #[cfg(windows)]
    #[test]
    fn verify_alive_returns_false_for_reaped_child() -> TestResult {
        if std::env::var_os(CHILD_FIXTURE_ENV).as_deref() == Some(OsStr::new(CHILD_FIXTURE_VALUE)) {
            return run_reaped_child_fixture();
        }

        let mut child = Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                REAPED_CHILD_TEST_NAME,
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_FIXTURE_ENV, CHILD_FIXTURE_VALUE)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let child_pid = child.id();
        let child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("child stdout was not piped"))?;

        wait_for_child_ready(child_stdout)?;
        let start_time_unix = process_start_time(child_pid)?;
        assert_ne!(start_time_unix, UNKNOWN_START_TIME_UNIX);

        let pid_file = PidFile {
            pid: child_pid,
            start_time_unix,
        };

        child.kill()?;
        let _exit_status = child.wait()?;
        assert_eq!(child.id(), child_pid);
        assert!(!pid_file.verify_alive()?);
        Ok(())
    }

    #[test]
    fn current_has_non_sentinel_start_fingerprint_and_verifies_alive() -> TestResult {
        let pid_file = PidFile::current();

        assert_eq!(pid_file.pid, std::process::id());
        assert_ne!(pid_file.start_time_unix, UNKNOWN_START_TIME_UNIX);
        assert!(pid_file.verify_alive()?);
        Ok(())
    }

    #[test]
    fn current_pid_with_different_start_fingerprint_does_not_verify() -> TestResult {
        let current = PidFile::current();
        assert_ne!(current.start_time_unix, UNKNOWN_START_TIME_UNIX);

        let different_start_time = current
            .start_time_unix
            .checked_add(1)
            .unwrap_or(UNKNOWN_START_TIME_UNIX + 1);
        assert_ne!(different_start_time, UNKNOWN_START_TIME_UNIX);

        let mismatched = PidFile {
            pid: current.pid,
            start_time_unix: different_start_time,
        };

        assert!(!mismatched.verify_alive()?);
        Ok(())
    }

    #[test]
    fn legacy_numeric_current_pid_remains_manageable() -> TestResult {
        let current = PidFile::current();
        assert_ne!(current.start_time_unix, UNKNOWN_START_TIME_UNIX);

        let workspace = tempfile::tempdir_in(std::env::current_dir()?)?;
        let pid_path = PidFile::path(workspace.path());
        let pid_dir = pid_path
            .parent()
            .ok_or_else(|| io::Error::other("PID path did not have a parent directory"))?;
        fs::create_dir_all(pid_dir)?;
        fs::write(pid_path, current.pid.to_string())?;

        let legacy = PidFile::read(workspace.path())
            .ok_or_else(|| io::Error::other("legacy numeric PID file was not readable"))?;

        assert_eq!(legacy.pid, current.pid);
        assert_eq!(legacy.start_time_unix, UNKNOWN_START_TIME_UNIX);
        assert_ne!(legacy.start_time_unix, current.start_time_unix);
        assert!(legacy.verify_alive()?);
        Ok(())
    }
}
