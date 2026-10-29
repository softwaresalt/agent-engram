//! Shared support for read-server activation-settle coverage (PRE-3F).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, UNIX_EPOCH};

use engram::daemon::protocol::{IpcRequest, IpcResponse};
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};

/// Upper bound for the activation-settle barrier.
pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(75);

const IPC_TIMEOUT: Duration = Duration::from_secs(10);

/// Outcome of the activation-settle barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    /// The server reports no generation activator.
    NoActivator,
    /// The published generation is active and the server is quiescent.
    Active,
}

/// Sends one direct-IPC request with no parameters.
///
/// # Errors
///
/// Returns an error when the request cannot be sent or no response arrives
/// within the IPC timeout.
pub async fn send_ipc(endpoint: &str, method: &str) -> Result<IpcResponse, String> {
    let request = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::from(1_u64)),
        method: method.to_owned(),
        params: None,
    };
    send_request(endpoint, &request, IPC_TIMEOUT)
        .await
        .map_err(|error| error.to_string())
}

/// Returns the path, branch, database path and generation of one
/// `get_workspace_status` response, or `None` when the call fails.
pub async fn binding_fingerprint(endpoint: &str) -> Option<Value> {
    let response = send_ipc(endpoint, "get_workspace_status").await.ok()?;
    let result = response.result?;
    Some(json!({
        "path": result.get("path"), "branch": result.get("branch"),
        "db_path": result.get("db_path"), "generation": result.get("generation"),
    }))
}

/// Snapshots every entry below `root`, keyed by relative path.
///
/// # Errors
///
/// Returns an error when a directory entry cannot be read.
pub fn filesystem_snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut snapshot = BTreeMap::new();
    snapshot_directory(root, root, &mut snapshot)
        .map_err(|error| format!("cannot snapshot fixture side effects: {error}"))?;
    Ok(snapshot)
}

fn snapshot_directory(
    root: &Path,
    directory: &Path,
    snapshot: &mut BTreeMap<String, Vec<u8>>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(std::io::Error::other)?
            .to_string_lossy()
            .into_owned();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            snapshot.insert(relative, vec![b'd']);
            snapshot_directory(root, &path, snapshot)?;
        } else if file_type.is_file() {
            let mut contents = vec![b'f'];
            let metadata = fs::symlink_metadata(&path)?;
            contents.extend(metadata.len().to_le_bytes());
            let modified = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)?;
            contents.extend(modified.as_secs().to_le_bytes());
            contents.extend(modified.subsec_nanos().to_le_bytes());
            let file_name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            let is_database_file = path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("db"))
                || file_name.to_ascii_lowercase().contains(".db-");
            if !is_database_file {
                match fs::read(&path) {
                    Ok(bytes) => contents.extend(bytes),
                    Err(error) if is_file_lock_error(&error) => {}
                    Err(error) => return Err(error),
                }
            }
            snapshot.insert(relative, contents);
        } else if file_type.is_symlink() {
            let target = fs::read_link(&path)?;
            let mut contents = vec![b'l'];
            contents.extend(target.to_string_lossy().as_bytes());
            snapshot.insert(relative, contents);
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                snapshot.insert(
                    relative,
                    vec![if file_type.is_socket() { b's' } else { b'o' }],
                );
            }
            #[cfg(not(unix))]
            snapshot.insert(relative, vec![b'o']);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_file_lock_error(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(32 | 33))
}

#[cfg(not(windows))]
fn is_file_lock_error(_error: &std::io::Error) -> bool {
    false
}

/// Waits until the read server at `endpoint` has settled activation.
///
/// # Errors
///
/// Returns an error when the barrier cannot observe a settled state
/// before [`SETTLE_TIMEOUT`].
pub async fn await_activation_settled(
    endpoint: &str,
    snapshot_root: &Path,
    expected_revision: u64,
) -> Result<Settled, String> {
    tokio::task::yield_now().await;
    Err(format!(
        "activation-settle barrier not implemented (PRE-3F); endpoint={endpoint}, \
         root={}, revision={expected_revision}, timeout={SETTLE_TIMEOUT:?}, variants={:?}",
        snapshot_root.display(),
        [Settled::NoActivator, Settled::Active],
    ))
}
