---
date: 2026-07-03
agent: Ship (Orchestrator-resumed)
mode: resume + build-verify + PR
shipment: 067-S
feature: 067-F
pr: 190
branch: 067-usage-telemetry-emit
status: green-awaiting-user-approved-merge
---

# Ship — 067-S Engram usage-telemetry EMIT (resumed after restart)

## Context

Prior session was interrupted by a system restart mid-Ship-pipeline. All 6
tasks (067.001-T..067.006-T) + 067-F were already implemented and committed
(9 commits ahead of origin/main). Working tree held uncommitted backlog
closure (tasks moved queue→archive as `done`, 067-S flipped `active`) and the
branch was un-pushed with no PR. This session resumed from the pre-PR steps.

## Actions taken

1. **Quality gates (CI feature set `--no-default-features --features
   cozo-backend,embeddings`)**: fmt PASS, clippy (pedantic) PASS.
   - Note: `--all-features` breaks on pre-existing `otlp-export`/
     `src/server/observability.rs` (opentelemetry 0.26 API mismatch). Out of
     067 scope; CI does not enable that feature.
2. **Committed backlog closure** `7f5c628` — 7 queue→archive renames + 067-S
   `active`. Preserved drift (`.cursor/mcp.json`, `.backlogit/telemetry.jsonl`,
   `.claude/`, `docs/design-docs/.gitkeep`) intentionally excluded.
3. **Pushed** branch, **opened PR #190** → main.
4. **CI run 1 FAILED** on `t067_003_rotation_triggers_at_cap_preserves_lines`
   (`left: 3, right: 4` — a line lost during rotation).

## Key decision / bug fixed

- Initially mis-diagnosed the rotation failure + a daemon-startup failure as
  Windows-only SQLite flakes (both passed in isolation on Windows). **CI on
  Ubuntu proved the rotation failure was a REAL, reproducible bug.**
- **Root cause**: `append_usage_line` (`src/services/metrics.rs`) wrote via
  `tokio::fs::File::write_all` and returned **without `flush().await`**.
  `write_all` does not guarantee bytes reach the OS; the next append's
  rotation `rename()` could run before the prior write landed → dropped line.
- **Fix** `35a5f01`: added `file.flush().await?` after `write_all`.
- **CI run 2 PASSED** (build, 3m23s).
- The `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` daemon test IS a
  genuine pre-existing Windows-only SQLite `database is locked` flake (code
  byte-identical on main; 067 touched no `src/daemon`/`src/db`); green on
  Ubuntu CI. Left as-is.

## Files modified this session

- `src/services/metrics.rs` (flush fix)
- `.backlogit/queue/067-S.md` (→ active), `.backlogit/queue/067-F.md` +
  `067.001-T..067.006-T.md` → `.backlogit/archive/` (renames, status done)

## State / next steps

- PR #190: CI green, `MERGEABLE` but `BLOCKED` pending review approval
  (the user-approved-merge gate). Repo merge strategy verified P-009 compliant
  (merge_commit only; squash/rebase disabled).
- Copilot review could NOT be auto-requested via `gh` CLI (`copilot` reviewer
  slug unresolved; no MCP tool available) — degraded, non-blocking.
- **Next**: operator approves + merges PR #190 (merge commit). Then Ship
  post-merge closure: shipment-reconcile (067-S → archived), compound-refresh,
  compact-context.
- Compound-learning candidate: "tokio::fs::File append needs explicit
  flush().await before any rename/rotation or data can be lost — reproduces on
  Linux, not always on Windows."
