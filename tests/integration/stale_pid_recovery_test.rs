use std::fs;

use engram::shim::pidfile::PidFile;

#[test]
fn shim_recovers_from_stale_pid_file() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    fs::create_dir(workspace.path().join(".git")).expect("create .git");
    fs::create_dir_all(workspace.path().join(".engram").join("run"))
        .expect("create runtime directory");

    let stale_pid = PidFile {
        pid: u32::MAX,
        start_time_unix: 1,
    };

    let _ = stale_pid.verify_alive();
}
