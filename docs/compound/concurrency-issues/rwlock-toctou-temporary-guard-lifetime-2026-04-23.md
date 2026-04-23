---
title: "RwLock TOCTOU from dropped temporary guard in Rust"
description: "Acquiring two RwLock read guards sequentially leaves a TOCTOU window — the first guard is dropped when .clone()? completes before the second acquire"
problem_type: "concurrency race condition"
category: "concurrency-issues"
component: "src/server/state.rs — snapshot_dispatch_context"
root_cause: "Rust drops a temporary read guard as soon as the expression using it completes (e.g., guard.clone()?). Acquiring a second lock after that drop creates an observable window between the two reads."
resolution_type: "code_fix"
severity: "high"
message: "workspace and config snapshot taken at different logical times — concurrent set_workspace or set_workspace_config can produce a mismatched pair"
file_path: "src/server/state.rs"
citations:
  - "docs/closure/2026-04-23-001-S-toctou-fix-closure.md"
  - "tests/contract/atomic_policy_snapshot_test.rs"
  - "PR #22 — Copilot review comment 1"
tags:
  - "rwlock"
  - "toctou"
  - "rust"
  - "concurrency"
  - "temporary-lifetime"
  - "dispatch"
---

## Problem

`snapshot_dispatch_context()` originally acquired two read guards sequentially:

```rust
// WRONG — TOCTOU window between these two reads
let workspace = self.active_workspace.read().await.clone()?;
// guard is dropped here because .clone()? consumed the temporary
let config_guard = self.workspace_config.read().await;
let config = config_guard.clone();
```

A concurrent `set_workspace` or `set_workspace_config` call between the first
`.clone()?` completing and the second `read().await` could produce a snapshot
where `workspace` is from time T1 and `config` is from time T2. The policy
engine would evaluate the tool call against a mismatched context.

This is a Rust-specific footgun: `temporary_guard.clone()?` materializes the
clone and drops the guard immediately — Rust does not extend the guard's
lifetime past the expression boundary.

## Root Cause

Rust temporary values are dropped at the end of the statement they appear in.
A `read()` guard created in `let value = guard.read().clone()` is a temporary
that lives only for that statement. The lock is released before the next
statement begins, creating the observable window.

This is easy to miss because the two lock acquisitions look "close together" in
source code, but there is a real scheduler yield point between them.

## Resolution

Hold both read guards in scope simultaneously before cloning either value:

```rust
// CORRECT — both guards held simultaneously
let ws_guard = self.active_workspace.read().await;
let cfg_guard = self.workspace_config.read().await;
// both locks are held here; no TOCTOU window
let workspace = ws_guard.clone()?;
let config = cfg_guard.clone();
// both guards drop here together when the function returns
```

This ensures `workspace` and `config` are snapshotted at the same logical
point in time, regardless of concurrent writes.

**Lock ordering**: always acquire `active_workspace` before `workspace_config`
in any code path that acquires both. This ordering must be consistent across
all callers to avoid potential deadlock with any future write-side code.

## Prevention

- Never use `lock().await.clone()` on two separate locks when the resulting
  values must be consistent with each other.
- When a struct method returns a composite snapshot of multiple `RwLock`-
  protected fields, bind all guards to named variables first, then clone.
- The pattern `let snap_a = guard_a.clone(); let snap_b = guard_b.clone()`
  (where guards are temporaries) is a TOCTOU bug even though both lines look
  like independent operations.
- Write contract tests that hold a write lock on one field between two
  sequential reads to detect future regressions (see C018-05 in
  `tests/contract/atomic_policy_snapshot_test.rs`).
