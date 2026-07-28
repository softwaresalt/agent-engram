---
date: 2026-07-03
agent: Ship (Orchestrator-resumed)
mode: resume + build-verify + PR
shipment: 067-S
feature: 067-F
pr: 190
branch: 067-usage-telemetry-emit
status: merged-closed
merge_commit: d7c3168b3a5bb60cfc8a85ca41ca086935b03387
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
- Compound-learning candidate: "tokio::fs::File append needs explicit
  flush().await before any rename/rotation or data can be lost — reproduces on
  Linux, not always on Windows."

## Closure (2026-07-04)

- **Two Copilot review rounds addressed** (11 threads total, all replied +
  resolved via GraphQL): round 1 — 5x `archived_from` provenance, rotation
  stale-generation prune (+ regression test), symlink-doc softening, 1 declined
  false-positive (`json!` use-after-move); round 2 — `start.ps1` `.env.local`
  `Test-Path` guard, untracked `.backlogit/telemetry.jsonl` (+ gitignore), and
  PR-description reconciliation for operator harness/editor config committed in
  `68fe014`/`73d3bac`.
- **PR #190 MERGED** via merge commit `d7c3168b3a5bb60cfc8a85ca41ca086935b03387`
  (operator-approved; `gh pr merge --merge --admin` to satisfy the review
  ruleset). Merged by softwaresalt 2026-07-04T01:32:00Z.
- **Post-merge closure**: local `main` fast-forwarded to `d7c3168`; feature
  branch pruned local + remote; shipment `067-S` archived
  (`.backlogit/archive/067-S.md`, `status: archived`, merge SHA recorded).
- **GI/GR gate PASS**: all manifest items (067-F, 067.001-T..067.006-T) archived
  `done`; 067-S archived; nothing left in `.backlogit/queue/067*`.
- Compound learning already persisted:
  `docs/compound/tokio-fs-file-flush-before-rename-2026-07-03.md`.
