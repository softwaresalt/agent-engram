---
title: "Unit tests calling daemon_key_for_workspace must create .git/HEAD in test workspace"
date: 2026-04-23
tags: [test-failures, daemon, workspace, unit-tests]
confidence: high
evidence: tests/unit/socket_permissions_test.rs — fixed in PR #21 (009-S), commit 7e8a4ea
---

# Problem

Unit tests that construct a temporary workspace path and pass it to
`daemon_key_for_workspace` (or any code that calls
`load_or_create_workspace_id`) fail with an error about missing git root,
not a missing `.engram/` dir.

`load_or_create_workspace_id` calls `canonical.join(".git").is_dir()` to
confirm the path is a git repository before proceeding. In unit tests that
only create a bare temp dir (no `.git`), this check returns `false` and the
call errors out — the test fixture is invalid.

# Solution

In unit tests that need a workspace path accepted by the daemon key function,
create a `.git/HEAD` file (or at minimum a `.git/` directory) inside the temp dir:

```rust
let ws = tempdir().unwrap();
std::fs::create_dir(ws.path().join(".git")).unwrap();
std::fs::write(ws.path().join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
```

# When This Applies

Any test that calls:
- `daemon_key_for_workspace(path)`
- `load_or_create_workspace_id(path)`
- anything that traces through workspace validation to the `.git` dir check

# Notes

The `.git` dir check is intentional workspace isolation: the daemon should
only operate on git repositories. Tests must reflect this invariant.
