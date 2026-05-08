---
title: "hydrate_code_graph skips JSONL reload when DB is already populated"
description: "hydrate_code_graph() should fast-path when DB already has code files, avoiding redundant re-indexing after a --direct run or prior daemon session"
problem_type: "missing_optimization"
category: "best-practices"
component: "src/services/hydration.rs"
root_cause: "hydrate_code_graph always ran JSONL reload even when DB was fully populated, causing redundant work after engram sync --direct"
resolution_type: "code_fix"
date: "2026-05-08"
shipment: "030-S"
---
# hydrate_code_graph skips JSONL reload when DB is already populated

## Problem

`hydrate_code_graph()` always attempted to load code graph data from legacy
JSONL files (`.engram/code_graph/*.jsonl`) into CozoDB, even when the DB was
already fully populated from a prior `--direct` run or daemon session. This
caused unnecessary work on daemon startup after a `engram sync --direct` had
already built the index.

## Fix

Added a fast-path at the top of `hydrate_code_graph()`:

```rust
// Fast-path: if the DB already has indexed code files, skip JSONL reload.
// This avoids redundant re-indexing when the daemon starts after a --direct run.
match cg_queries.count_code_files() {
    Ok(count) if count > 0 => {
        debug!(count, "code files already in DB; skipping JSONL hydration");
        return Ok(CodeGraphHydrationResult::default());
    }
    Err(e) => {
        warn!(error = %e, "count_code_files failed; falling back to JSONL reload");
    }
    Ok(_) => {} // DB is empty — proceed with JSONL reload
}
```

On DB error, falls back to JSONL reload (safe default). On empty DB, proceeds
normally.

## When to Apply

Any time a hydration or loader function might be called redundantly (e.g.,
daemon startup after CLI pre-load), add a presence-check fast-path before
the expensive load operation.

## Evidence

- `src/services/hydration.rs`: `hydrate_code_graph()` lines ~129-155
- `src/db/cozo_queries.rs`: `count_code_files()` queries `file_node` table

## Date

2026-05-08 | Shipment 030-S (045-F CLI-direct mode)
