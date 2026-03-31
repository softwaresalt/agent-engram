---
title: "TempDir Dropped Early When Not Returned From Test Helper"
problem_type: test_failure
category: test_failure
component: mcp_tools
root_cause: type_mismatch
resolution_type: test_fix
severity: medium
message: "TempDir created in setup helper is dropped at function end unless returned alongside AppState, deleting the workspace before the test runs"
file_path: "tests/contract/evaluation_contract_test.rs"
resolved: true
bug_id: ""
tags: [tempdir, tempfile, lifetime, ownership, drop, test-helpers, contract-tests, arc, appstate]
date: 2026-03-30
---

## Problem

When a test helper creates a `tempfile::TempDir` and returns only `Arc<AppState>`, the `TempDir`
is dropped at the end of the helper function. Rust's `Drop` impl for `TempDir` deletes the
underlying directory immediately, so by the time the test body runs, the workspace no longer
exists on disk.

## Symptoms

- Tests pass locally if you hold a reference to the `TempDir` accidentally (e.g., via a `let _`
  binding in an earlier iteration), but fail otherwise
- The workspace-bound `AppState` reports the workspace path correctly, but file operations
  against `.engram/` fail with `No such file or directory`
- `get_evaluation_report` returns a `MetricsError::NotFound` even when events were written in the
  setup helper — because the entire temp directory was deleted before the tool call
- Failures appear non-deterministic if the OS delays filesystem cleanup

## What Did Not Work

- Writing events to the metrics file inside the helper before returning the `Arc<AppState>` —
  the events file is created, but the parent directory is already queued for deletion
- Using `workspace.path()` string conversion and storing the path string instead of the handle —
  the path string outlives the `TempDir`, but the directory itself is gone

## Solution

Return the `TempDir` alongside the `Arc<AppState>` from every test helper that creates a
temporary workspace. The caller holds the `TempDir` binding for the duration of the test body,
keeping the directory alive.

### Before

```rust
async fn setup_workspace_with_events(events: &[UsageEvent]) -> Arc<AppState> {
    let workspace = tempfile::tempdir().expect("tempdir");
    // ... set up state, write events ...
    state  // workspace is dropped here — directory deleted!
}

#[tokio::test]
async fn test_evaluation_report() {
    let state = setup_workspace_with_events(&events).await;
    // workspace directory already deleted before this line
    let result = tools::dispatch(state.clone(), "get_evaluation_report", None).await;
    // fails: MetricsError::NotFound
}
```

### After

```rust
async fn setup_workspace_with_events(
    events: &[UsageEvent],
) -> (Arc<AppState>, tempfile::TempDir) {  // TempDir returned to keep directory alive
    let workspace = tempfile::tempdir().expect("tempdir");
    // ... set up state, write events ...
    (state, workspace)  // ownership transferred to caller
}

#[tokio::test]
async fn test_evaluation_report() {
    let (state, _workspace) = setup_workspace_with_events(&events).await;
    // _workspace keeps the directory alive for the entire test body
    let result = tools::dispatch(state.clone(), "get_evaluation_report", None).await;
    // succeeds: workspace directory exists
}
```

## Why This Works

`tempfile::TempDir` implements `Drop` to recursively delete its directory when the value goes out
of scope. Rust drops locals at the end of the block in which they are declared. When the helper
returns only `Arc<AppState>`, the `TempDir` local is dropped at the `}` of the helper function —
before the caller ever runs. Returning the `TempDir` transfers ownership to the caller's binding,
so the drop (and directory deletion) is deferred until the caller's `_workspace` binding goes out
of scope at the end of the test function.

## Prevention

- **Convention**: every test helper that creates a `TempDir` must include it in the return type.
  Use a named struct or a tuple like `(Arc<AppState>, TempDir)`.
- **Naming**: prefix the caller binding with `_` only if you truly want the directory alive for
  the test body. Using `_workspace` (underscore-prefix, not bare `_`) keeps the value alive while
  signalling it is intentionally unused as a value.
- **Clippy**: `clippy::let_underscore_drop` detects `let _ = tempdir()` (immediate drop). Enable
  it in `#[cfg(test)]` modules to catch accidental immediate drops.
- **Review checklist**: any helper returning `Arc<AppState>` that also constructs a `TempDir`
  internally is suspicious — flag it during PR review.

## Related Solutions

No related solutions found in `.backlog/compound/` at time of writing.
