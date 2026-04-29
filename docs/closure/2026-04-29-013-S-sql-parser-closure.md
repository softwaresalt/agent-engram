---
title: "Post-Merge Closure — 013-S SQL Parser Enhancements"
shipment_id: "013-S"
feature_id: "033-F"
merge_sha: "fdc10b9"
pr_url: "https://github.com/softwaresalt/agent-engram/pull/44"
branch: "ship/013-S-sql-parser-enhancements"
closure_date: "2026-04-29"
status: "READY"
mode: "post-merge"
---

## Change Summary

Shipment 013-S shipped three tasks under 033-F (SQL Parser Enhancements):

| Task | Title | Outcome |
|---|---|---|
| 033.004-T | Add references relation table and DB edge operations | ✅ Done |
| 033.002-T | Parse multi-schema table references (schema.table syntax) | ✅ Done |
| 033.001-T | Wire References edges into code graph with Class node resolution | ✅ Done |
| 033.003-T | CREATE PROCEDURE support (upstream grammar) | ⏳ Deferred (blocked) |

**Files changed**: 14 files, +841 / -52 lines

Key files:
- `src/services/parsing/sql.rs` — JOIN extraction, schema-qualified names
- `src/db/queries.rs` — `create_references_edge`, `reresolve_references_edges`, `delete_edges_from_file`
- `src/db/schema.rs` — SCHEMAFULL `references` table
- `src/services/code_graph.rs` — References edges wired into both index/sync paths
- `tests/contract/references_edge_test.rs` — New contract tests
- `tests/integration/sql_references_integration_test.rs` — New integration tests
- `tests/unit/parsing_test.rs` — 4 new parsing unit tests

## CI Status

- ✅ `cozo-backend` build passed
- ✅ `surreal-backend` build passed
- 14 Copilot review comments addressed and threads resolved

## Invariants to Preserve

1. `select *` from any non-empty SurrealDB table must NOT be used — `id: Thing` deserialization fails with serde_json
2. `references` is a reserved word in SurrealQL — always backtick-escape as `` `references` ``
3. SCHEMAFULL table (not TYPE RELATION) for references avoids silent edge drops
4. `ALLOWED_EDGE_TABLES` allowlist in `delete_edges_from_file` must be maintained to prevent injection
5. `cozo_queries.rs` stubs must mirror every method signature in `queries.rs`

## Pre-Deploy Audit

| Check | Status |
|---|---|
| Feature flags | N/A (local daemon only, no rollout gates) |
| Rollback procedure | `git revert fdc10b9` on `main` |
| Data migration | Schema auto-created at daemon startup via `schema.rs`; backward-compatible |
| Cross-service dependencies | None (local MCP daemon, no external services) |
| Monitoring plan | See below |

## Deployment Path

Merge-only. Local daemon binary — users rebuild from source or await next release build.

## Post-Deploy Checks

1. Start engram daemon against a SQL-containing workspace
2. Call `index_workspace` — verify log shows References edges created
3. Verify `references` table populated: `SELECT source, target, qualified_name FROM \`references\`` in SurrealDB
4. Test JOIN extraction: SQL file with `SELECT * FROM a JOIN b ON a.id = b.a_id` — verify `b` captured as References edge
5. Test schema-qualified: SQL with `FROM public.orders` — verify `public.orders` in `qualified_name`

## Healthy Signals

- Daemon starts without panic on workspaces containing SQL files
- `index_workspace` returns nonzero `edges_created` for SQL-containing workspaces
- `references` table populated with source/target/qualified_name after indexing

## Failure Signals

- Daemon panic on startup → schema migration error (check `schema.rs` DEFINE statements)
- `id: Thing` serde_json error in logs → `SELECT *` leaked into a query path
- Missing `references` edges after index → check `code_graph.rs` References arm, `reresolve_references_edges`

## Monitoring Plan

Local daemon only — no production deployment. Manual smoke test per post-deploy checks above. No dashboards or alerts required.

## Rollback Trigger

If `index_workspace` panics or returns error on any SQL-containing workspace after the update.

## Rollback Procedure

```bash
git revert fdc10b9  # reverts the feature merge commit
cargo build --release
```

## Validation Window

Informal — test at next workspace indexing session. No time-bound window (local tooling only).

## Owner

softwaresalt

## Source Artifact Cleanup

| Item | Source Stash ID | Deliberation Ref | Notes |
|---|---|---|---|
| 033.001-T | F15C561F | — | Stash text: "SQL parser: resolve FROM references to known Class nodes in graph." |
| 033.002-T | 8232DE58 | — | Stash text: "SQL parser: multi-schema reference support" |
| 033.004-T | (none) | docs/exec-plans/2026-04-28-033-F-sql-parser-enhancements-plan.md | Synthesized task from planning |
| 033-F | — | docs/closure/2026-04-26-034-F-sql-parser-closure.md | Feature synthesized from prior closure follow-ups |

No `backlogit_stash_remove` available in registry — stash IDs F15C561F and 8232DE58 recorded here for manual retirement.

## Follow-Up Items Identified

| Item | Summary | Priority |
|---|---|---|
| FU-1 | Batch-UPDATE optimization for `reresolve_references_edges` (currently N+1 round-trips) | low |
| FU-2 | Add INDEX on `target` field in `references` schema | low |
| FU-3 | DRY refactor of inline reference-resolution logic between `index_workspace` and `sync_workspace` | low |
| FU-4 | 033.003-T: CREATE PROCEDURE support when tree-sitter-sequel grammar updates | blocked (upstream) |
| FU-5 | Full Class node resolution for SQL references (currently falls back to raw string) | medium |

## Risky Action Record

| Action | Risk | Approval | Result |
|---|---|---|---|
| Adding new SurrealDB schema table | moderate | operator merge approval | applied |
| Modifying `delete_edges_from_file` allowlist | moderate | code review + CI | applied |
| Global `reresolve_references_edges` post-pass (rewrites all self-loops) | moderate | code review | applied |

## Recommendation

**READY** — all tasks shipped, CI green, invariants documented, follow-ups stashed.
