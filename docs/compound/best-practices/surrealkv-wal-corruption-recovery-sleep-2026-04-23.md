---
title: "SurrealKV WAL corruption recovery requires 500ms sleep before retry"
date: 2026-04-23
tags: [best-practices, surrealkv, database, crash-recovery, daemon]
confidence: high
evidence: src/tools/lifecycle.rs — discovered and fixed in PR #21 (009-S)
superseded_by: "017-S — surreal-backend removal (2026-05-01)"
status: "stale"
stale_reason: "The surreal-backend and SurrealKV were fully removed in Shipment 017-S. This workaround no longer applies to the codebase."
---

# Problem

When SurrealKV detects a WAL corruption condition (via a verification read
on open), the recovery path wipes the data directory and retries the open.
Without a delay before the retry, the open panics with:

```
receiving from an empty and closed channel
```

This happens because the background teardown of the previous (corrupted)
SurrealKV instance is still in-flight on another thread when the retry
attempts to open the same path.

# Solution

After wiping the corrupted data directory and before retrying, sleep for
**at least 500ms** to allow the background thread to complete teardown:

```rust
// Wipe corrupted WAL
std::fs::remove_dir_all(&data_dir)?;

// Wait for SurrealKV background teardown to complete
tokio::time::sleep(std::time::Duration::from_millis(500)).await;

// Now safe to retry
let db = connect_db(&data_dir).await?;
```

# When This Applies

Any code path that detects SurrealKV state corruption, wipes the data
directory, and attempts an immediate re-open on the same path.

# Notes

- 500ms is empirically sufficient; 100–200ms may also work but is less
  reliable under heavy system load.
- The corruption detection typically occurs during the verification read
  that `background_db_hydration` performs immediately after open.
- Without this guard, intermittent panics in CI are hard to diagnose
  because the panic message does not reference SurrealKV directly.
