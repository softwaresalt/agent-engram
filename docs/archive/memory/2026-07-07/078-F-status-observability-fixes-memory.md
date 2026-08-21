---
title: "078-F Status Observability Fixes — Session Memory"
type: session-memory
date: 2026-07-07
feature: 078-F
tasks: [078.001-T, 078.002-T]
prs: [228, 229]
merge_sha: 8ae9d57
status: complete
---

## Summary

Routed two observability defects (findings #1 and #2 from a workspace-validation
session) through the full Stage → Ship pipeline: backlog → plan → adversarial
review → TDD harness → implementation → quality gates → adversarial code review →
PR → Copilot review → admin merge → backlog closure → operational closure.

## Tasks completed

- **078.001-T** — `get_daemon_status.memory_bytes` reported `sysinfo::used_memory()`
  (system-wide RAM, ~22 GB) instead of the engram process RSS (~1.2 GB). Legacy
  `legacy-sse` `/health` handler had the same bug plus a wrong `* 1024`.
- **078.002-T** — `get_workspace_status.code_graph` counts were gated behind
  `#[cfg(feature = "git-graph")]`, returning all-zeros in the shipped default build
  even with a fully populated graph. `git-graph` only gates git *history* tooling;
  the code-graph indexer is always active.

## Files modified

- `src/services/process_memory.rs` (new) — `current_process_memory_bytes()` helper.
- `src/services/mod.rs` — register module.
- `src/tools/lifecycle.rs` — `get_daemon_status` uses helper; `get_workspace_status`
  ungated + warns on DB-connect failure; added lib regression test.
- `src/tools/read.rs` — `get_health_report.memory_mb` uses helper.
- `src/server/router.rs` — legacy handler uses helper (dropped `* 1024`).
- `tests/integration/smoke_test.rs` — rewrote `s072` to assert non-zero code_graph.

## Key decisions & rationale

- **Shared helper placement**: `services::process_memory` (services is a leaf dep of
  both `tools` and `server`, avoiding a tools↔server cycle).
- **Removed the false git-graph gate** rather than preserving it; the premise
  ("code-graph indexer inactive without git-graph") was factually wrong, proven by
  `get_workspace_statistics` reading the same counts unconditionally.
- **Rewrote `s072`** (uncfg) to assert non-zero counts under the default feature set,
  replacing a test that asserted the buggy zero on an empty workspace.

## Failed approaches / corrections

- First test-hardening attempt used `try_start_indexing()`/`finish_indexing()` to
  serialize the manual index against `set_workspace`'s background hydration. **Copilot
  review correctly flagged this as insufficient**: `background_db_hydration` proceeds
  through `connect_db`/`hydrate` regardless of the lock result (it uses the lock only
  to decide who calls `finish_indexing`). The race remained.
- **Final fix**: index the workspace *before* `set_workspace`. Background hydration
  then hits `hydrate_code_graph`'s "DB already populated" fast-path (reads only), so
  there is no concurrent CozoDB writer and no scheduler-timing assumption. Verified
  stable across repeated parallel runs and two full `--all-targets` suites.

## Verification

- fmt + clippy clean (default and `legacy-sse`/all-features paths).
- Full CI-equivalent suite green (0 failures, 14k+ log lines).
- PR #228 merged (merge commit `8ae9d57`, 2 parents). Copilot's one finding
  addressed, replied, thread resolved. PR #229 closed the backlog.
- **Operational closure**: rebuilt + reinstalled `C:\Tools\engram.exe`
  (`0.2.0+g14903ba`); restarted the workspace daemon and confirmed live:
  `daemon-status.memory_bytes` = 0.62 GB (process, not 17.69 GB system);
  `workspace-status.code_graph` = 272 files / 2134 fns / 198 classes / 3703 edges,
  matching `stats`.

## Pre-existing issues noted (not addressed — out of scope)

- `--all-features` build fails on `otlp-export` bitrot (opentelemetry API drift in
  `src/server/observability.rs`). CI lanes do not use `--all-features`.
- `graph_vector_rehydration_test` remains `#[ignore]`d on Windows/Linux (CozoDB
  SQLITE_BUSY on daemon restart, stash `100EACD8`).
