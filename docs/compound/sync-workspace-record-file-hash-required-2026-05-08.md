---
title: "sync_workspace must call record_file_hash after each file upsert"
description: "sync_workspace() must call record_file_hash after upsert_code_file to keep the hash table current; without it detect_offline_changes reports false positives on the next daemon startup"
problem_type: "missing_update"
category: "best-practices"
component: "src/services/code_graph.rs"
root_cause: "sync_workspace called upsert_code_file but not record_file_hash; index_workspace already had the correct paired pattern"
resolution_type: "code_fix"
date: "2026-05-08"
shipment: "030-S"
---
# sync_workspace must call record_file_hash after each file upsert

## Problem

`sync_workspace()` in `src/services/code_graph.rs` previously called
`upsert_code_file()` for each changed file but did NOT call
`record_file_hash()` afterwards. This caused the file hash table to remain
stale after a sync run, so subsequent `detect_offline_changes()` calls would
report ALL previously synced files as "added" (false positives), triggering
unnecessary re-indexes.

By contrast, `index_workspace()` already had the correct pattern:
call `upsert_code_file()` then `record_file_hash()` in sequence.

## Fix

After every successful `upsert_code_file` call in `sync_workspace()`, also
call `record_file_hash()`:

```rust
upsert_code_file(&ws_path, &rel_path, &content, &queries)?;
// Mirror index_workspace: update the hash table so detect_offline_changes
// sees this file as current on the next run.
if let Err(e) = record_file_hash(&rel_path_str, file_path, &queries) {
    debug!(error = %e, path = %rel_path_str, "record_file_hash failed (non-fatal)");
}
```

The failure is non-fatal and logged at debug level — same pattern as
`index_workspace`.

## When to Apply

Any time a new file-mutation loop is added that writes code file records
via `upsert_code_file`, it MUST also update the hash table via
`record_file_hash` to maintain freshness detection integrity.

## Evidence

- `src/services/code_graph.rs`: sync loop ~line 719 (Gap 1 fix in 030-S)
- `index_workspace()` at ~line 448: reference implementation with correct pattern

## Date

2026-05-08 | Shipment 030-S (045-F CLI-direct mode)
