use std::fs;
use std::time::Duration;

use engram::daemon::ipc_server::ipc_endpoint;
use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use engram::shim::lifecycle::{check_health, ensure_daemon_running};
use engram::shim::pidfile::PidFile;
use serde_json::Value;

#[tokio::test]
async fn shim_recovers_from_stale_pid_file() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let pid_dir = workspace.path().join(".engram").join("run");
    fs::create_dir_all(&pid_dir).expect("create runtime directory");

    let stale_pid = PidFile {
        pid: u32::MAX,
        start_time_unix: 1,
    };
    stale_pid
        .atomic_write(&pid_dir)
        .expect("persist stale pid file");

    tokio::time::timeout(
        Duration::from_secs(15),
        ensure_daemon_running(workspace.path()),
    )
    .await
    .expect("daemon recovery should complete before timeout")
    .expect("daemon should recover from stale pid metadata");

    let pid_file = PidFile::read(workspace.path()).expect("pid file should be rewritten");
    assert_ne!(
        pid_file.pid, stale_pid.pid,
        "recovery should replace stale PID metadata with the live daemon"
    );
    assert!(
        pid_file
            .verify_alive()
            .expect("pid verification should succeed"),
        "rewritten pid file should point at a live daemon"
    );
    assert!(
        workspace
            .path()
            .join(".engram")
            .join(".workspace-id")
            .exists(),
        "daemon startup should persist a workspace identity during recovery"
    );

    let endpoint = ipc_endpoint(workspace.path()).expect("endpoint should resolve");
    assert!(
        check_health(&endpoint).await,
        "recovery path should leave the daemon healthy"
    );

    shutdown_daemon(&endpoint).await;
}

async fn shutdown_daemon(endpoint: &str) {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(1))),
        method: "_shutdown".to_owned(),
        params: None,
    };

    let _ = send_request(endpoint, &request, Duration::from_secs(5)).await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while check_health(endpoint).await && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
