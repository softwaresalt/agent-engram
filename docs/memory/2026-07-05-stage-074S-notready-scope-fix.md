---
date: 2026-07-05
agent: Stage
mode: bug-triage + impl-plan + plan-review + harvest + shipment-assembly
branch: 074S-notready-scope-fix
base: main @ 9b54694
source_pr: 207
tasks: [072.001-T]
features: [072-F]
reviews: [072.001-R]
shipments: [074-S]
plans:
  - docs/exec-plans/2026-07-05-072-001-notready-scope-fix-plan.md
reviews_docs:
  - docs/reviews/2026-07-05-072-001-notready-scope-fix-plan-review.md
status: reviewed-backlog-ready (074-S queued for Ship to claim)
---

# Stage session memory — 2026-07-05 — 074-S NotReady `--direct` hint scope fix

## Task

Operator-directed Stage run against `agent-engram` (main @ `9b54694`, cache freshly
rebuilt — no reflexive `backlogit sync`). Plan a **bug-fix shipment** for a defect a
post-merge Copilot review on **PR #207** caught in the change 073-S shipped:
`DaemonError::NotReady`'s `#[error(...)]` was augmented with a `--direct` escape-hatch
hint, but `NotReady` is a **shared** variant. Its second call site
(`wait_for_daemon_exit`, `src/shim/lifecycle.rs:392-395`, reached via `respawn_daemon`)
returns `NotReady` when the OLD daemon fails to **shut down** during respawn — where the
`--direct` hint is misleading and impossible (the stuck daemon still holds the lock, so
`engram index --direct` fails at `src/cli/direct.rs:73-84`).

Stage delivered **reviewed structure only** (feature + task + plan + plan-review + queued
shipment + this memory). **No Ship/code/PR work.**

## Tool status

- backlogit MCP surface: `TOOL_DEGRADED` — MCP `get_version` returned "Transport closed".
  Registry (`.autoharness/backlog-registry.yaml`) declares CLI fallback → operated via
  `C:\Tools\backlogit.exe` (v1.3.0) for all mutations. `DEGRADED_MODE: backlogit-mcp`.
- Index: cache freshly rebuilt per operator; skipped reflexive sync. End-of-session sync
  run via CLI.

## What I built

| Artifact | ID | Notes |
|---|---|---|
| Bug feature | **072-F** | priority low; labels bug,daemon,errors,shutdown-wait,073-S-followup. (feature counter → 072-F; shipment counter → 074-S) |
| Task | **072.001-T** | single-width; add `ShutdownTimeout` variant; retarget shutdown-wait branch only |
| Plan-review (ACCEPTED) | **072.001-R** | persona: skeptical staff engineer (CLI/errors/lifecycle); 1 cycle |
| Shipment (QUEUED) | **074-S** | items `072-F`, `072.001-T`; Step 5.5 scope guard embedded |
| Impl-plan | docs/exec-plans/2026-07-05-072-001-notready-scope-fix-plan.md | |
| Plan-review doc | docs/reviews/2026-07-05-072-001-notready-scope-fix-plan-review.md | |

## Recommended design (for Ship)

- New `DaemonError::ShutdownTimeout { timeout_ms: u64 }` returned by
  `wait_for_daemon_exit` (`lifecycle.rs:392-395`); message has **no** `--direct`
  ("Daemon failed to shut down within {timeout_ms}ms during respawn; … stop the running
  engram daemon process, then retry"). brace-safe.
- New wire code `DAEMON_SHUTDOWN_TIMEOUT = 8010` (next free 8xxx; 8009 = WATCHER_INIT),
  name `"DaemonShutdownTimeout"`, mapped in `to_response` (exhaustive `Daemon` match →
  compiler forces the arm).
- `NotReady` (startup path via `poll_until_ready` `:457`) unchanged — keeps `--direct`,
  keeps wire contract `8006`/`DaemonNotReady`/`{timeout_ms}`.
- Confirmed: **no** caller matches on `NotReady` outside `to_response`; `respawn_daemon`
  + `ensure_daemon_running_inner` propagate via `?`, so the new variant flows up cleanly.
- Test-first: shutdown message omits `--direct` + brace-free; startup message retains
  `--direct`; wire-contract tests for BOTH variants. Optional lifecycle-level test
  deferred (harness cost).
- Blast radius LOW/additive (≤3 files: errors/mod.rs, errors/codes.rs, shim/lifecycle.rs);
  `plan-harden` NOT warranted.

## Open questions (carried to Ship / operator)

- **Q1 (defer):** thread the concrete stuck `pid` into the `ShutdownTimeout` message?
  `Option<u32>` renders awkwardly in a thiserror string → prefer `{timeout_ms}`-only.
- **Q2 (include):** pin literal `8006` in `not_ready_wire_contract_unchanged` (closure
  advisory F1) while editing the wire tests.
- **Q3 (confirm):** code number `8010` — free; reassign only if a reserved block is desired.

## Scope guard (Step 5.5, on 074-S)

- IN: split/scope the NotReady message so the shutdown path doesn't surface `--direct`;
  add distinct variant + wire code; preserve NotReady contract; test-first both paths.
- OUT: broader error-taxonomy refactor; `IpcError::Timeout` wording; `bin/engram.rs` help.

## Git

- Branch `074S-notready-scope-fix` off `main` @ `9b54694`.
- Committed exactly the 6 Stage artifacts + this memory (explicit paths). Did **NOT**
  touch the pre-existing ` M .gitignore` drift or the untracked
  `.github/workflows/detect-direct-push.yml` (operator/harness drift).
- Pushed. **No PR** (Orchestrator lands it).

## Next steps for Ship

1. Claim `074-S`; branch off `main`; implement 072.001-T test-first per the plan.
2. Honor binding conditions C1-C5 in 072.001-R (retarget only the shutdown branch; freeze
   the NotReady arm; brace-free/`--direct`-free shutdown message; fresh `8010`; ≤3 files).
3. Gates: `cargo fmt --all -- --check`; `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`; `cargo test --all-targets`; `cargo audit`.
