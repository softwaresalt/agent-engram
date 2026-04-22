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

        let start_time_matches = self.start_time_unix <= UNKNOWN_START_TIME_UNIX
            || process.start_time() == self.start_time_unix;

        Ok(start_time_matches)
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
    use std::fs;

    use tempfile::TempDir;

    use super::PidFile;

    fn create_pid_workspace() -> TempDir {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        fs::create_dir_all(workspace.path().join(".engram").join("run"))
            .expect("create pid run directory");
        workspace
    }

    #[test]
    fn verify_alive_returns_false_when_pid_reused() {
        let _workspace = create_pid_workspace();
        let pid_file = PidFile {
            pid: u32::MAX,
            start_time_unix: 1,
        };

        let _ = pid_file.verify_alive();
    }

    #[test]
    fn verify_alive_returns_true_for_live_process() {
        let _workspace = create_pid_workspace();
        let pid_file = PidFile {
            pid: std::process::id(),
            start_time_unix: 1,
        };

        let _ = pid_file.verify_alive();
    }

    #[test]
    fn atomic_write_persists_in_pid_dir_not_temp_dir() {
        let workspace = create_pid_workspace();
        let pid_dir = workspace.path().join(".engram").join("run");
        let pid_file = PidFile {
            pid: std::process::id(),
            start_time_unix: 1,
        };

        let _ = pid_file.atomic_write(&pid_dir);
    }
}
