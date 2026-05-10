# Gate OOM-Risk Startup Operations Behind Env Var

**Date**: 2026-05-09
**Evidence**: 034-S / PR #127 — `src/tools/lifecycle.rs`

## Problem

Auto-reindex at daemon startup scanned all source files in the workspace. With 1,382
files the operation consumed 14GB+ RAM and caused OOM crashes on developer machines.
There was no way to disable it without code changes.

## Solution

Gate the auto-reindex behind `ENGRAM_AUTO_REINDEX=true`. Default is `false`. Operators
who want re-indexing on every startup opt in explicitly.

```rust
let auto_reindex = std::env::var("ENGRAM_AUTO_REINDEX")
    .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
    .unwrap_or(false);

if auto_reindex {
    // ... spawn re-index task ...
}
```

## General Pattern

When a startup operation has high resource cost (RAM, CPU, disk) and is not required
for basic tool functionality, gate it behind an opt-in env var with a safe default.
This prevents OOM on developer machines while keeping the capability available for
production or dedicated indexing workflows.

## Related

- `ENGRAM_READY_TIMEOUT_MS` for adjusting the shim health-check timeout
- `early-hydration-ready-before-heavy-io-2026-05-09.md`
