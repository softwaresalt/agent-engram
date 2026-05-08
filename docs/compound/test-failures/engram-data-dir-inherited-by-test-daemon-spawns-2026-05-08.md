---
title: "ENGRAM_DATA_DIR in shell causes all daemon-spawning tests to open production CozoDB"
date: 2026-05-08
tags: [test-isolation, engram-data-dir, daemon, cozo, sqlite]
feature: 043-F
commits: [f43fe0a]
confidence: high
---

## Problem

When `ENGRAM_DATA_DIR` is set in the developer's shell (e.g., pointing at
`D:\Source\GitHub\agent-engram\.engram`), ALL subprocess daemon spawns in
integration tests inherit this environment variable. The spawned daemon then
opens the **production CozoDB** (thousands of indexed files) instead of the
test-scoped temp directory. This causes:

- `background_db_hydration` to scan and hydrate thousands of files
- Daemon startup to take 30+ seconds instead of <2 seconds
- All shim lifecycle tests to time out at their `poll_until_ready` deadline

The failure mode looks like daemon-startup tests hanging or timing out, with
no obvious log message about the production database being used.

## Fix

Add `.env_remove("ENGRAM_DATA_DIR")` to **every subprocess spawn** that
starts a daemon process:

### 1. Test helpers (`tests/helpers/mod.rs`)

All three `Command::new(...)` spawn functions must strip the variable:

```rust
Command::new(env!("CARGO_BIN_EXE_engram"))
    .arg("daemon")
    .env_remove("ENGRAM_DATA_DIR")
    // ... other args
```

There are currently 3 spawn sites: `spawn()`, `spawn_for_workspace()`, and
`spawn_with_idle_timeout_ms()`. All three must include the remove.

### 2. Production shim (`src/shim/lifecycle.rs`)

`spawn_daemon` also must strip the variable so the shim never passes a
developer's global override to daemons it launches:

```rust
Command::new(&binary_path)
    .arg("daemon")
    .env_remove("ENGRAM_DATA_DIR")
    // ... other args
```

This ensures per-workspace isolation regardless of the caller's environment.

## Why the Variable Exists

`resolve_data_dir()` in `src/db/workspace.rs` reads `ENGRAM_DATA_DIR` as the
highest-priority override for where the daemon stores its CozoDB files. This
allows developers to redirect all daemon state to a fixed location for manual
testing. The override is correct for interactive use but wrong for test subprocesses.

## Detection

If you see tests like `t020_s001_s005_daemon_becomes_healthy_within_startup_timeout`
timing out at 30s with no apparent crash, run:

```powershell
echo $env:ENGRAM_DATA_DIR
```

If it prints a path (rather than being empty), this is the root cause.

## Related

- `src/db/workspace.rs` — `resolve_data_dir()` reads `ENGRAM_DATA_DIR`
- `tests/helpers/mod.rs` — all 3 `Command::new(...)` blocks
- `src/shim/lifecycle.rs` — `spawn_daemon` function
