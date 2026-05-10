# yield_now in Async Hydration Loops Prevents Tokio Starvation

**Date**: 2026-05-09
**Evidence**: 034-S / PR #127 — `src/services/hydration.rs`

## Problem

CozoDB's `run_script()` is synchronous. When called inside an `async fn`, it blocks
the tokio thread for the full duration of the JSONL code-graph load. With thousands
of upsert operations, this starved other async tasks (including the IPC server handler)
of executor time, causing the daemon to appear unresponsive.

## Solution

Call `tokio::task::yield_now().await` every 50 **actual upserts** to yield control
back to the executor:

```rust
let mut upsert_count: usize = 0;
for line in reader.lines() {
    let Ok(node) = serde_json::from_str::<CodeNode>(&line?) else {
        continue;   // ← corrupt line: do NOT count, do NOT yield
    };
    db.upsert_node(&node)?;
    upsert_count += 1;
    if upsert_count % 50 == 0 {
        tokio::task::yield_now().await;
    }
}
```

## Critical Detail

The counter MUST be inside the `if let Ok(node)` / successful-parse branch. Counting
corrupt or skipped lines inflates the counter and reduces yield frequency, defeating
the purpose. Copilot review caught this bug in the first attempt.

## Tradeoff

Yield every 50 upserts adds minimal overhead (~microseconds per yield) while ensuring
the IPC server can respond to health-checks and tool calls during hydration.

## Related

- `early-hydration-ready-before-heavy-io-2026-05-09.md`
- Tokio docs: `tokio::task::yield_now`
