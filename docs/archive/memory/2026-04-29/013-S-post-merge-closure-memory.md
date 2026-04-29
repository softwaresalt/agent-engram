---
session_date: "2026-04-29"
shipment_id: "013-S"
branch: "post-merge/013-S-sql-parser-enhancements"
phase: "post-merge-closure"
status: "complete"
---

## Session Memory — 013-S Post-Merge Closure

### Items Completed

| Item | Title | Status |
|---|---|---|
| 033.004-T | Add references relation table and DB edge operations | ✅ archived |
| 033.002-T | Parse multi-schema table references (schema.table syntax) | ✅ archived |
| 033.001-T | Wire References edges into code graph with Class node resolution | ✅ archived |
| 033-F | SQL Parser Enhancements — Reference Resolution and Grammar Coverage | ✅ archived |
| 013-S | Shipment E: SQL Parser Enhancements | ✅ shipped |
| PR #44 | feat: SQL parser enhancements (013-S) | ✅ merged (fdc10b9) |

### Deferred Items

| Item | Title | Status |
|---|---|---|
| 033.005-T | CREATE PROCEDURE support (upstream grammar) | queued (deferred — blocked upstream) |

### Branch State

- **Feature branch**: `ship/013-S-sql-parser-enhancements` — merged to main (fdc10b9)
- **Post-merge branch**: `post-merge/013-S-sql-parser-enhancements` — in progress
- **Commits on post-merge branch**: `4ccfb43` (backlog archival)

### Decisions and Rationale

1. **SCHEMAFULL not TYPE RELATION for `references` table**: SurrealDB v2.6 silently drops RELATE edges when OUT record doesn't exist. Using SCHEMAFULL with string source/target avoids this.

2. **`reresolve_references_edges` is global scope**: Re-resolves ALL self-loops after each indexing pass. Acceptable at workspace scale; deferred N+1 optimization to stash.

3. **`SELECT *` banned in SurrealDB queries**: `id:Thing` causes serde_json deserialization failure on non-empty tables. All queries must use explicit field selection.

4. **033.005-T deferred (renamed from 033.003-T)**: Renamed to avoid ID collision with archived 012-S mcp-json task; changed from `blocked` to `queued` to unblock `backlogit_ship_shipment`. Task is genuinely deferred pending upstream tree-sitter-sequel grammar update.

5. **P-007 archive integrity**: `backlogit_ship_shipment` deleted `033.003-T.md` from archive (naming collision with prior task). Restored with `git restore .backlogit/archive/`. Queue stale files for 033.001-T and 033.002-T manually removed.

### Known Gotchas

- **Historical naming collision (resolved)**: `033.003-T` in the queue (CREATE PROCEDURE task) conflicted with `033.003-T` in the archive (mcp-json task from 012-S). Resolved by renaming the CREATE PROCEDURE task to `033.005-T` in this closure PR.
- **`backlogit shipment ship` validates ALL children of feature by parent_id**, not just manifest items. Any blocked child of the feature blocks the shipment.
- **Rust 1.95 vs 1.85 clippy drift**: CI has `manual_contains`, `doc_markdown`, `unnecessary_map_or` lints not present locally. Run `cargo clippy --all-targets` locally to catch them.

### Follow-Up Items Stashed

| Stash ID | Summary |
|---|---|
| 8C651D9F | Batch-UPDATE optimization for `reresolve_references_edges` |
| E145945C | Add INDEX on target in references schema |
| DA9D4948 | DRY refactor of resolution logic in index/sync |
| B0903A71 | Full Class node resolution for SQL references |

### Files Modified This Session

- `.backlogit/queue/033-F.md` → archived
- `.backlogit/queue/033.001-T.md` → archived
- `.backlogit/queue/033.002-T.md` → archived
- `.backlogit/queue/033.004-T.md` → archived
- `.backlogit/queue/013-S.md` → archived
- `.backlogit/queue/033.003-T.md` → status changed to queued
- `.backlogit/archive/033-F.md` → new
- `.backlogit/archive/033.004-T.md` → new
- `.backlogit/archive/013-S.md` → updated
- `.backlogit/archive/033.001-T.md` → updated
- `.backlogit/archive/033.002-T.md` → updated
- `.backlogit/reconcile/013-S-pre-20260429.md` → new
- `docs/closure/2026-04-29-013-S-sql-parser-closure.md` → new
- `docs/compound/database-issues/surrealdb-select-star-serde-json-2026-04-29.md` → new
- `docs/compound/build-errors/tree-sitter-sequel-join-grammar-2026-04-29.md` → new
- `docs/compound/workflow-issues/rust-1-95-clippy-lint-ci-mismatch-2026-04-29.md` → new
- `docs/memory/2026-04-29/013-S-post-merge-closure-memory.md` → this file

### Next Steps

1. Commit all post-merge closure artifacts
2. Push post-merge branch and open closure PR
3. Await operator approval on closure PR
4. Stage can pick up stashed follow-ups (8C651D9F, E145945C, DA9D4948, B0903A71) and 033.003-T when upstream grammar updates

### Architecture Docs Update Assessment

- `docs/architecture.md` — the References edges are a new graph edge type; should document the new `references` table and its resolution semantics
- No agent/skill changes — no `AGENTS.md` update needed
- `docs/research/` — no new design decisions that haven't been captured in compound/closure
