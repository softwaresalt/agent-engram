//! Unit tests for hardened Unix `/tmp` fallback socket permissions (029.009-T).
//!
//! Verifies that when the workspace path is long enough to exceed the
//! `sockaddr_un` limit, `ipc_endpoint` creates a private subdirectory under
//! `/tmp` with `0o700` permissions and returns a socket path inside it rather
//! than directly in `/tmp`.
//!
//! These tests are Unix-only (`#[cfg(unix)]`) and are compiled as no-ops on
//! Windows.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use engram::daemon::ipc_server::ipc_endpoint;

/// Build a workspace path long enough to exceed the `sockaddr_un` limit.
///
/// The primary socket path is `{workspace}/.engram/run/engram.sock` (23-char suffix).
/// Linux allows 107 chars, macOS 103. Using 90 nesting chars exceeds both.
///
/// A minimal `.git/HEAD` is created so that `daemon_key_for_workspace` treats
/// the directory as a valid git root.
fn long_workspace(base: &std::path::Path) -> std::path::PathBuf {
    // 90-char subdirectory name pushes the full socket path well past 107 chars
    let long_name = "w".repeat(90);
    let ws = base.join(&long_name);
    fs::create_dir_all(&ws).expect("create long workspace dir");
    let git_dir = ws.join(".git");
    fs::create_dir_all(&git_dir).expect("create .git dir");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    ws
}

/// `ipc_endpoint` on a long-path workspace must return a path inside a private
/// `/tmp/engram-{key}/` directory, not directly in `/tmp`.
#[test]
fn fallback_endpoint_is_inside_private_directory() {
    let base = tempfile::tempdir().expect("base tempdir");
    let workspace = long_workspace(base.path());

    let endpoint = ipc_endpoint(&workspace).expect("ipc_endpoint should succeed");

    // Path must end with /engram.sock and have a parent component
    assert!(
        endpoint.ends_with("/engram.sock"),
        "fallback endpoint must end with /engram.sock, got: {endpoint}"
    );

    // Parent must be a directory under /tmp, not /tmp itself
    let path = std::path::Path::new(&endpoint);
    let parent = path.parent().expect("endpoint has parent");
    assert!(
        parent != std::path::Path::new("/tmp"),
        "fallback socket must not be directly in /tmp; parent was: {}",
        parent.display()
    );
    assert!(
        parent
            .to_str()
            .is_some_and(|s| s.starts_with("/tmp/engram-")),
        "fallback parent must be /tmp/engram-{{key}}/, got: {}",
        parent.display()
    );
}

/// The private directory created for the fallback socket must have `0o700`
/// permissions — no group or world read/write/execute bits.
#[test]
fn fallback_private_directory_has_0700_permissions() {
    let base = tempfile::tempdir().expect("base tempdir");
    let workspace = long_workspace(base.path());

    let endpoint = ipc_endpoint(&workspace).expect("ipc_endpoint should succeed");

    let path = std::path::Path::new(&endpoint);
    let private_dir = path.parent().expect("endpoint has parent");

    assert!(
        private_dir.is_dir(),
        "private socket directory must exist after ipc_endpoint: {}",
        private_dir.display()
    );

    let meta = fs::metadata(private_dir).expect("read metadata");
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(
        mode, 0o700,
        "private socket directory must have 0o700 permissions, got: {mode:#o}"
    );
}

/// A short-path workspace must still use the standard socket path, not fallback.
#[test]
fn short_path_workspace_uses_standard_socket_not_fallback() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");

    let endpoint = ipc_endpoint(workspace.path()).expect("ipc_endpoint should succeed");

    // Standard path ends with .engram/run/engram.sock, not in /tmp
    assert!(
        !endpoint.starts_with("/tmp/engram-"),
        "short-path workspace should not use /tmp fallback, got: {endpoint}"
    );
    assert!(
        endpoint.contains(".engram"),
        "standard endpoint should contain .engram, got: {endpoint}"
    );
}

/// Calling `ipc_endpoint` twice on the same long-path workspace must not fail
/// (the directory already exists — `recursive(true)` handles this).
#[test]
fn fallback_directory_creation_is_idempotent() {
    let base = tempfile::tempdir().expect("base tempdir");
    let workspace = long_workspace(base.path());

    let first = ipc_endpoint(&workspace).expect("first call should succeed");
    let second = ipc_endpoint(&workspace).expect("second call should succeed");

    assert_eq!(first, second, "endpoint must be stable across calls");
}
