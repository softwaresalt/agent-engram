---
title: "Use Option<T> for Non-Fatal Error Paths; Avoid Result<Option<T>, E> When All Errors Warn-and-Continue"
description: "When every error path in a function is non-fatal (log a warning, return an empty/absent result), use Option<T> as the return type instead of Result<Option<T>, E>; the Result wrapper implies a meaningful failure the caller must handle, but if callers always ignore errors with ok().flatten(), the type contract is misleading and triggers clippy::unnecessary_wraps"
problem_type: "design_smell"
category: "best-practices"
component: "daemon/mod"
root_cause: "remove_stale_pid_if_dead returned Result<Option<u32>, EngramError> propagating FlushFailed on read errors; FlushFailed is a write-side error code and read failures are non-fatal cleanup operations that should only warn"
resolution_type: "refactor"
severity: "low"
file_path: "src/daemon/mod.rs"
citations:
  - "src/daemon/mod.rs — remove_stale_pid_if_dead"
  - "docs/closure/2026-05-02-025-F-daemon-startup-fix-closure.md"
  - ".backlogit/archive/025.002-T.md"
tags:
  - "error-handling"
  - "option"
  - "result"
  - "clippy"
  - "unnecessary_wraps"
  - "non-fatal"
  - "daemon"
  - "pid-file"
---

## Problem

`remove_stale_pid_if_dead` originally had signature `fn(...) -> Result<Option<u32>, EngramError>`.
Its job is cleanup: attempt to read a PID file, check if the process is alive, remove the file
if the process is dead. All failure modes (file not found, read error, parse error) are
non-fatal — the correct behavior is to log a `warn!` and return `None`.

The `Result<Option<u32>>` wrapper implied there was a meaningful error the caller needed to
handle. But the call site was:

```rust
let _ = remove_stale_pid_if_dead(&run_dir)?;
```

This propagates a `FlushFailed` error (semantically wrong — `FlushFailed` is for write failures)
upward on a read failure, blocking daemon startup unnecessarily.

After Copilot review flagged `FlushFailed` as semantically wrong for a read failure, the return
type was changed to `Option<u32>` and all error paths were converted to `warn!` + `None`.
clippy's `unnecessary_wraps` lint correctly triggered and was resolved by the type change.

## Resolution

```rust
// BEFORE
fn remove_stale_pid_if_dead(run_dir: &Path) -> Result<Option<u32>, EngramError> {
    let contents = fs::read_to_string(&pid_path).map_err(|e| EngramError::FlushFailed(...))?;
    // ...
    Ok(Some(pid))
}

// AFTER
fn remove_stale_pid_if_dead(run_dir: &Path) -> Option<u32> {
    let contents = match fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(e) => {
            warn!("could not read PID file: {e}");
            return None;
        }
    };
    // ...
    Some(pid)
}
```

Call site change: `let _ = remove_stale_pid_if_dead(&run_dir)?;` → `remove_stale_pid_if_dead(&run_dir);`

## Prevention

Use this heuristic when designing error-returning functions:

| All errors non-fatal? | Caller always ignores Err with ok()? | Correct return type |
|---|---|---|
| Yes | Yes | `Option<T>` |
| Yes | Sometimes | `Option<T>` |
| No | No | `Result<T, E>` |

If `clippy::unnecessary_wraps` fires on a `Result<T, E>` function, check whether all error
paths are non-fatal. If yes, convert to `Option<T>` rather than suppressing the lint.

Do NOT use error codes from the write domain (e.g., `FlushFailed`) for read failures. Choosing
the wrong error variant misleads callers and reviewers about the nature of the failure.
