---
title: "030-S Ship Session Memory — CLI-Direct Daemonless Mode"
type: session-memory
date: 2026-05-08
shipment: 030-S
feature: 045-F
pr: 96
branch: feat/cli-direct-daemonless-mode
status: awaiting-merge
---

## Completed Tasks

| Task | Title | Status |
|------|-------|--------|
| 045.001-T | `src/cli/direct.rs` — `run_direct_sync()` implementation | done |
| 045.002-T | `--direct` / `ENGRAM_DIRECT` flag wired through `engram.rs` + `indexing.rs` | done |
| 045.003-T | Integration tests (`tests/integration/cli_direct_test.rs`, 5 tests) | done |
| 045.004-T | Freshness detection fixes (Gap 1 + Gap 2) | done |

## Files Modified

- `src/cli/direct.rs` *(new)* — core daemonless runner
- `src/cli/commands/indexing.rs` — `--direct` dispatch; tautological tests removed
- `src/cli/mod.rs` — `pub mod direct`
- `src/bin/engram.rs` — `Sync { direct }` + `Index { direct }` with `BoolishValueParser`
- `src/services/code_graph.rs` — `record_file_hash` after each `upsert_code_file` in `sync_workspace`
- `src/services/hydration.rs` — `count_code_files` fast-path in `hydrate_code_graph`
- `tests/integration/cli_direct_test.rs` *(new)* — 5 integration tests
- `Cargo.toml` — `integration_cli_direct` test binary registration
- `.backlogit/queue/030-S.md` — status: active
- `.backlogit/queue/045.001-T.md`, `045.002-T.md`, `045.003-T.md`, `045.004-T.md` — status: done
- `docs/compound/` — 3 new compound learnings

## Key Decisions

1. **`DaemonLock` acquired in `run_direct_sync`** — prevents concurrent DB writes between daemon and --direct mode. Returns exit 2 if already held.
2. **`Box::leak` for lock guard** — intentional: one lock per process lifetime, OS reclaims on exit.
3. **`BoolishValueParser`** — required for `ENGRAM_DIRECT=1` to work (clap 4 default bool parser only accepts "true"/"false").
4. **`record_file_hash` in `sync_workspace`** — Gap 1 fix: sync now correctly updates the hash table so `detect_offline_changes` doesn't report false positives.
5. **`count_code_files` fast-path** — Gap 2 fix: daemon startup after `--direct` run skips redundant JSONL reload.
6. **Tautological unit tests removed** — integration tests provide real subprocess dispatch coverage.

## Failed Approaches

- Initial attempt used clap default bool parser for `ENGRAM_DIRECT=1` — failed because it only accepts "true"/"false". Fixed with `BoolishValueParser::new()`.

## Review Findings (addressed)

- P2: Tautological unit tests in `indexing.rs` → removed, integration tests cover it
- P2: Module doc promised unimplemented tests 4+5 → doc updated, deferred to backlog
- P3: `Result<..., i32>` in `resolve_workspace_params` → documented with comment (intentional pattern)

## PR State

- PR #96: https://github.com/softwaresalt/agent-engram/pull/96
- CI: ✅ green (3m42s)
- Review: 0 P0/P1 findings
- **Awaiting user merge approval** (branch protection)

## Next Steps (post-merge)

1. `gh pr view 96 --json state,mergedAt,mergeCommit` → confirm merge
2. `git fetch origin main && git merge-base --is-ancestor <SHA> origin/main`
3. Update `.backlogit/queue/030-S.md` → `status: done`
4. Write closure artifact to `docs/closure/`
5. Run `backlogit sync` (CLI fallback for index resync)
6. Invoke `compact-context` skill

## Compound Learnings Written

- `docs/compound/clap-bool-env-var-boolish-value-parser-2026-05-08.md`
- `docs/compound/sync-workspace-record-file-hash-required-2026-05-08.md`
- `docs/compound/hydrate-code-graph-fast-path-already-indexed-2026-05-08.md`
