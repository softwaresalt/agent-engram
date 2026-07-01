---
title: "Ship session — 052-S engram verify CLI (Phase 1a)"
type: session-memory
role: ship
date: 2026-07-01
feature: 064-F
shipment: 052-S
pr: 185
branch: 064-engram-verify-cli
---

## What shipped (pending operator merge)

`engram verify <path>` — a local, no-daemon structural-conformance linter CLI
(Phase 1a of 064-F). Exit contract `0` pass / `1` non-conformant / `2` error.
Modeled on `engram manifest`. Three tasks delivered test-first (RED→GREEN):
core service (`src/services/verify.rs`), CLI subcommand
(`src/cli/commands/verify.rs` + `src/bin/engram.rs`), cross-platform path
normalization + subprocess integration test.

## Hard-won lessons (compound knowledge)

### 1. backlogit ID reuse causes add/add merge conflicts on archive relocation
The IDs `064-F` / `064.001-T` … `064.004-T` were **reused** — they already exist
on `main` as *archived powerbi-tmdl tasks* (shipped via PR #169, closure
`2026-06-14-064-S-tmdl-parser-closure.md`). Moving my new verify tasks
`queue → archive` (the normal "done" transition) created
`.backlogit/archive/064.00X-T.md` paths that collide with `main`'s existing
powerbi archives — an add/add conflict that makes the PR non-mergeable.
**Resolution:** keep the verify task specs in `.backlogit/queue/` with
`status: done` in place (no archive relocation). This preserved both my specs
and `main`'s powerbi archives and produced a conflict-free backlog diff.
**Detection tool:** `git merge-tree --write-tree --name-only origin/main HEAD`
enumerates conflicts in-memory without touching the working tree — use it before
pushing when the base is behind `main`.

### 2. Isolating one shipment from a 29-file dirty tree
Committed ONLY 052-S/064-F artifacts (verify src + tests + plan + design +
memory + queue specs). Left all drift (`.cursor/mcp.json`, `.claude/`,
`start.ps1`, `copilot-instructions.md`, `.backlogit/*.jsonl`, `memories.json`)
and sibling-shipment artifacts (`010-D`, `053-S`, `065-*`, `064.005/006`,
daemonless plan, stage-0E042A84 memory) untracked/modified for their owners.
Explicit-path `git add` (never `git add .` / `git add -A`) is essential here.

### 3. Merging `main` into a feature branch past a dirty tree
The uncommitted drift blocked `git merge origin/main`: local mods to
`.backlogit/archive/stash.jsonl` and untracked `064.005/006-T.md` (which `main`
tracks) triggered "would be overwritten." Parked the untracked ones to `$TEMP`
and `git stash push -- <the one tracked drift file>`, merged, resolved, then
dropped the stash (drift superseded by `main`). Only real conflict was
`Cargo.toml`'s `[[test]]` tail (both sides appended) — kept both blocks.
`src/services/mod.rs` auto-merged.

### 4. Distinguish real failures from environmental flakes on Windows local runs
- `unit_notebook_extract` "could not compile" during parallel `--all-targets` —
  **transient incremental-linker contention**; passed on isolated recompile.
- `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` — **self-documented racy**
  daemon TTL test (background hydration holds fd-lock past TTL under load);
  passes in isolation.
- `t030_001_{c,swift}_function_indexed_via_ipc` failed on CI (ubuntu) with a
  **panic inside `cozo-0.7.6` `sqlite.rs:49:45`** under concurrent IPC indexing
  — pre-existing cozo instability; `main`'s own CI fails ~2/6 recent runs on this
  class. Verified via `gh run list --branch main`. Correct action: CI re-run,
  not code change.
- Always confirm a suspected flake with an **isolation run**
  (`cargo test --test <target> -- --test-threads=1`) before treating it as real.

### 5. Cross-shell exit-code verification gotcha (cmd.exe)
`cmd /c "app ... & echo %errorlevel%"` expands `%errorlevel%` at **parse time**
(before the app runs) -> always shows the pre-command value. Use
`cmd /v:on /c "app ... & echo !errorlevel!"` (delayed expansion). PowerShell
`$LASTEXITCODE` is correct; and never name a PowerShell function parameter
`$args` (collides with the automatic variable — splat silently sends nothing).

### 6. Review adjudication against the authoritative plan
A reviewer flagged two P1s. Both were adjudicated **against the pinned plan**
(not the review prompt): non-markdown->0 is explicitly pinned (Q6 RESOLVED), and
relative-path containment is outside the Phase-1a usage model + freeze-scope.
Lesson: ground severity calls in the plan's pinned contract/scope, and defer
hardening that would need a new RED test rather than expanding a frozen scope.

## Gate/CI status at checkpoint

fmt OK, clippy OK (local + CI), 12 verify tests OK, runtime contract OK
cross-shell, audit advisory-only. CI test step hit the cozo IPC flake; re-run
triggered (fix-ci cycle 1). PR #185 MERGEABLE, base `main`, merge-commit-only
repo policy verified. **STOPPED before merge — operator-gated.**

## Files

New: `src/services/verify.rs`, `src/cli/commands/verify.rs`,
`tests/{unit/verify_core_test.rs,contract/verify_test.rs,integration/cli_verify_test.rs}`,
`tests/fixtures/verify/{conformant,malformed}.md`.
Modified: `src/services/mod.rs`, `src/cli/commands/mod.rs`, `src/bin/engram.rs`,
`Cargo.toml` (3 `[[test]]` entries).
Closure: `docs/closure/2026-07-01-052-S-engram-verify-cli-closure.md`.
