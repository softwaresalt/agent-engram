//! Current-process resident-memory measurement.
//!
//! Shared by `get_daemon_status`, `get_health_report`, and the legacy HTTP
//! health handler so every status surface reports the engram process's own
//! memory rather than system-wide RAM. `sysinfo::System::used_memory()` returns
//! whole-machine usage; on a busy host that overstates engram's footprint by
//! more than an order of magnitude and has misled operators into thinking the
//! daemon is leaking memory.

use sysinfo::System;

/// Return the current process's resident set size in bytes.
///
/// Returns `None` when the current process id cannot be resolved or `sysinfo`
/// has no entry for it. Callers typically substitute `0` for a missing value.
#[must_use]
pub fn current_process_memory_bytes() -> Option<u64> {
    let pid = sysinfo::get_current_pid().ok()?;
    let mut sys = System::new();
    sys.refresh_process(pid);
    sys.process(pid).map(sysinfo::Process::memory)
}

#[cfg(test)]
mod tests {
    use sysinfo::System;

    use super::current_process_memory_bytes;

    /// The helper must report a positive value that is a subset of system-wide
    /// usage — a per-process figure, never the whole machine's used memory.
    #[test]
    fn returns_positive_and_bounded_by_system() {
        let bytes = current_process_memory_bytes().expect("process memory available");
        assert!(bytes > 0, "process memory must be positive");

        let mut sys = System::new();
        sys.refresh_memory();
        let system_used = sys.used_memory();
        assert!(
            bytes <= system_used,
            "process memory ({bytes}) must not exceed system used memory ({system_used})"
        );
    }
}
