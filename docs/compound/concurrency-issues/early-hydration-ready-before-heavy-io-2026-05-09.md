# Early set_hydration_ready Before Heavy I/O Prevents Shim Timeout

**Date**: 2026-05-09
**Evidence**: 034-S / PR #127 — `src/tools/lifecycle.rs`

## Problem

The daemon's `set_hydration_ready()` call was placed only after a 22MB JSONL code-graph
load completed. The shim's health-check loop (`poll_until_ready`) fired within 500ms of
daemon start, long before the JSONL load finished. Result: shim always timed out and
reported the daemon as unreachable.

## Solution

Move `set_hydration_ready()` to immediately after `connect_db()` succeeds. The JSONL
hydration then runs as a background task. The daemon is logically ready to serve
requests as soon as the DB is connected; partial data is acceptable during hydration.

```rust
// In background_db_hydration:
connect_db(&data_dir, &branch_safe).await?;
state.set_hydration_ready();   // ← BEFORE the heavy JSONL load
// ... then spawn JSONL load as background task ...
```

## Key Insight

"Hydration ready" means "the DB is connected and tools can run", not "all historical
data has been loaded". Read tools returning partial results during hydration is
preferable to the daemon appearing dead to the shim.

## Related

- `ENGRAM_AUTO_REINDEX` gate (see `auto-reindex-oom-gate.md`)
- `poll_until_ready` deadline loop (see `poll-until-ready-deadline-loop.md`)
