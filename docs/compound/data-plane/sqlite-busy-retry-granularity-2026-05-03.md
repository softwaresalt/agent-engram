---
title: "SQLITE_BUSY retry must be at per-statement level, not per-file level"
domain: data-plane
tags: [sqlite, retry, cozo, concurrency, content-hash]
evidence: 038.004-T, PR #74 Copilot review, d727183
confidence: high
date: 2026-05-03
---

## Problem

When retrying `index_workspace` at the top-level (per-file batch), a `SQLITE_BUSY` mid-symbol upsert corrupts the index silently:

1. `upsert_code_file` writes `content_hash` to `file_node` **before** symbol writes complete.
2. A `SQLITE_BUSY` on any subsequent `run_script` call in `upsert_function/class/interface` leaves partial rows.
3. The outer retry re-runs `index_workspace_impl`, but `list_code_files()` sees the already-committed `content_hash` and **skips the file as unchanged**.
4. Partial symbol rows (`function_meta` without `function_code`/`function_embedding`) persist permanently until `force: true` re-index.

## Root Cause

`content_hash` commit ordering: `upsert_code_file` (line ~221) runs before `upsert_function` (line ~271). A file is never re-indexed unless its hash changes, so any top-level retry is unsafe for recovering mid-file failures.

## Fix Pattern

Move `SQLITE_BUSY` retry to the individual `run_script` call level. In `CodeGraphQueries`, add a private `run_script_busy_retry_mutable` helper and apply it to all mutable write calls within `upsert_function`, `upsert_class`, `upsert_interface`. This makes each statement retryable without requiring the entire file to be re-processed.

```rust
async fn run_script_busy_retry_mutable(
    &self, script: &str, params: BTreeMap<String, DataValue>,
) -> Result<cozo::NamedRows, EngramError> {
    const MAX_ATTEMPTS: u32 = 5;
    let mut delay = Duration::from_millis(50);
    for attempt in 0..MAX_ATTEMPTS {
        match self.db.run_script(script, params.clone(), ScriptMutability::Mutable) {
            Ok(r) => return Ok(r),
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if (msg.contains("locked") || msg.contains("busy")) && attempt + 1 < MAX_ATTEMPTS {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_millis(500));
                    continue;
                }
                return Err(map_db_err(e.to_string()));
            }
        }
    }
    unreachable!()
}
```

## Rule

> When a multi-step write operation has a skip-guard based on any intermediate write's side effect (e.g., content_hash), SQLITE_BUSY retry must live at or below the granularity of individual write statements — never at the granularity of the full batch.
