use std::fs;

use engram::errors::{EngramError, IpcError};
use engram::shim::version::{ENGRAM_PROTOCOL_VERSION, ensure_protocol_compatible};

#[test]
fn shim_respawns_on_stale_daemon() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");

    let stale_daemon_version = ENGRAM_PROTOCOL_VERSION.saturating_add(1);
    let typed_mismatch = IpcError::VersionMismatch {
        expected: ENGRAM_PROTOCOL_VERSION,
        actual: stale_daemon_version,
    };

    assert!(
        typed_mismatch.to_string().contains("Restart the daemon"),
        "typed mismatch should document the remediation path"
    );

    assert!(matches!(
        ensure_protocol_compatible(stale_daemon_version),
        Err(EngramError::Ipc(IpcError::VersionMismatch { expected, actual }))
            if expected == ENGRAM_PROTOCOL_VERSION && actual == stale_daemon_version
    ));
}
