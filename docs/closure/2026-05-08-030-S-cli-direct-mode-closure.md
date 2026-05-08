---
title: '030-S CLI-Direct Daemonless Mode — Closure'
type: closure
date: 2026-05-08
feature: 045-F
shipment: 030-S
merge_sha: d09bba011bce49cccf3dd9377aa4e0126cdee262
prs:
  - 94
  - 95
  - 96
---

# 030-S CLI-Direct Daemonless Mode — Closure

## Shipment Summary

Shipped feature 045-F: `engram sync --direct` and `engram index --direct` daemonless mode. Enables one-shot CLI workspace indexing from `start.ps1`/`start.sh` scripts without spawning a persistent daemon.

## Merge Confirmation

- **PR #94** (docs/staging): merged `15158d7` at 2026-05-08T18:30:25Z
- **PR #95** (fix/docs review): merged `a33be5b` at 2026-05-08T18:39:06Z
- **PR #96** (feat/implementation): merged `d09bba0` at 2026-05-08T22:50:19Z
- SHA `d09bba0` confirmed in `origin/main` history ✅

## Tasks Completed

| Task | Title | Status |
|------|-------|--------|
| 045.001-T | Implement direct runner module (`src/cli/direct.rs`) | ✅ done |
| 045.002-T | Wire `--direct` flag and `ENGRAM_DIRECT` env var | ✅ done |
| 045.003-T | Integration tests for CLI-direct mode | ✅ done |
| 045.004-T | Index freshness detection (daemon skips re-index) | ✅ done |

## Files Shipped

### New Files
- `src/cli/direct.rs` — daemonless runner: lock acquisition, config parsing, index/sync dispatch, JSON output
- `tests/integration/cli_direct_test.rs` — 5 integration tests for direct mode
- `docs/compound/clap-bool-env-var-boolish-value-parser-2026-05-08.md`
- `docs/compound/sync-workspace-record-file-hash-required-2026-05-08.md`
- `docs/compound/hydrate-code-graph-fast-path-already-indexed-2026-05-08.md`
- `docs/memory/2026-05-08/030-S-ship-session-memory.md`
- `docs/memory/2026-05-08/030-S-staging-session-memory.md`

### Modified Files
- `src/bin/engram.rs` — `--direct`/`ENGRAM_DIRECT` flags on `sync` and `index` subcommands
- `src/cli/commands/indexing.rs` — dispatch through direct runner when flag is set
- `src/cli/mod.rs` — `pub mod direct;`
- `src/services/code_graph.rs` — `content_hash.clone()` + `record_file_hash_precomputed` in sync loop
- `src/services/file_tracker.rs` — `record_file_hash_precomputed()` to avoid double disk I/O
- `src/services/hydration.rs` — fast-path skips JSONL reload when DB already populated
- `Cargo.toml` — `integration_cli_direct` test target

## Review Rounds

Two Copilot review rounds on PR #96 — 13 threads total, all fixed and resolved:

### Round 1 (commit `a2bb5dc`)
- Hardcoded null `id` → `Value::from(1_u64)` (JSON-RPC 2.0 compliance)
- Non-standard tool error code → `-32603`
- Swallowed `parse_config` error now propagated
- Plan doc unit numbering (×2 on PR #95)

### Round 2 (commit `394def3`)
- JSON schema parity: `serde_json::to_value(r)` for full field coverage
- Null id default in direct mode mirroring IPC runner
- Double disk I/O eliminated via `record_file_hash_precomputed()`
- YAML frontmatter added to 3 compound learning docs
- Duplicate H1 removed from session memory doc
- 3 plan doc accuracy fixes

## Healthy Signals

- `cargo build` clean
- `cargo clippy -- -D warnings -D clippy::pedantic` clean
- `cargo test --lib` 122 passed
- `cargo test --test integration_cli_direct` 5 passed
- CI green on all 3 PRs

## Deferred Items (stash candidates)

- Test cases 3 (daemon mutex) and 5 (no orphaned processes) from 045.003-T require `DaemonHarness` subprocess infrastructure not yet available — marked ⏸ deferred in plan doc

## Rollback

```bash
git revert --no-edit -m 1 d09bba011bce49cccf3dd9377aa4e0126cdee262
```

## Compound Learnings

Three new compound docs shipped:
1. `clap-bool-env-var-boolish-value-parser-2026-05-08.md` — use `value_parser!(bool)` not `BoolishValueParser` for `ENGRAM_DIRECT`-style env vars
2. `sync-workspace-record-file-hash-required-2026-05-08.md` — always record file hash after sync to enable offline change detection
3. `hydrate-code-graph-fast-path-already-indexed-2026-05-08.md` — check DB populated state before JSONL reload to skip redundant hydration
