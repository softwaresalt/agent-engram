---
title: "013-S SQL Parser Enhancements — Full Session Compacted"
shipment_id: "013-S"
feature_id: "033-F"
sessions: ["2026-04-28", "2026-04-29"]
status: "shipped"
merge_sha: "fdc10b9"
pr_number: 44
closure: "docs/closure/2026-04-29-013-S-sql-parser-closure.md"
sources:
  - "docs/archive/memory/2026-04-28/013-S-deliberation-and-lifecycle-memory.md"
  - "docs/archive/memory/2026-04-28/stash-triage-and-shipment-grouping-memory.md"
  - "docs/archive/memory/2026-04-29/013-S-post-merge-closure-memory.md"
---

## Compacted Session Summary — 013-S

### Outcome

013-S shipped. PR #44 merged to main as `fdc10b9`. All 3 tasks done, archived.

### Items

| Item | Outcome |
|---|---|
| 033.004-T: references DB schema + edge ops | ✅ archived |
| 033.002-T: schema-qualified name parsing | ✅ archived |
| 033.001-T: code graph wiring + reresolve | ✅ archived |
| 033-F: SQL Parser Enhancements feature | ✅ archived |
| 013-S: shipment | ✅ shipped |
| 033.003-T: CREATE PROCEDURE (upstream blocked) | ⏳ deferred as 033.005-T (renamed to avoid ID collision with archived mcp-json task) |

### Key Decisions

1. **SCHEMAFULL not TYPE RELATION** for `references` table — prevents silent edge drops when target record doesn't exist in SurrealDB v2.6
2. **SELECT * banned** in all SurrealDB queries — `id:Thing` serde_json deserialization fails on non-empty tables
3. **JOIN table extraction** — `join.relation` child nodes of `from`, not direct `from.relation` children (tree-sitter-sequel 0.3 grammar)
4. **Schema-qualified names** — join all `identifier` children of `object_reference` with `.`
5. **`reresolve_references_edges` post-pass** — global scope (re-resolves all self-loops), acceptable for correctness-first at workspace scale
6. **False dependency removed**: 033.002-T (parser) is independent of 033.001-T (graph wiring) — deliberation finding applied
7. **Qualified-name fallback**: 033.001-T resolution tries `public.users`, then falls back to `users` — deliberation finding applied
8. **033.005-T deferred**: renamed from 033.003-T, then moved `blocked` → `queued` to unblock `backlogit_ship_shipment` (ship cmd validates all parent_id children)

### Stage Lifecycle Summary

- Stash triage: 3 stash entries (F15C561F, 8232DE58, 19D78639) → 033-F, 033.001-T, 033.002-T, 033.003-T
- impl-plan: `docs/exec-plans/2026-04-28-033-F-sql-parser-enhancements-plan.md`
- deliberation: `docs/decisions/2026-04-28-033-F-sql-parser-enhancements-deliberation.md`
- plan-review: PASS (4 P2 advisories; P2-3 `references` reserved word most critical — fixed)
- harvest: 033.004-T added (Unit 1 DB schema)
- 013-S assembled with 4 items

### Ship Lifecycle Summary

- Harness generated for all 3 tasks (harness-ready labels)
- 033.004-T → 033.002-T → 033.001-T build order (sequential)
- CI issues fixed:
  - Rust 1.95 `manual_contains` (5 occurrences in parsing_test.rs)
  - `doc_markdown` lints (SurrealDB, code_file backticks)
  - `unnecessary_map_or` (map_or → is_some_and)
  - JOIN extraction bug (`public.orders` not captured)
- 14 Copilot review threads addressed and resolved
- CI green on both cozo-backend and surreal-backend
- Merge: `gh pr merge 44 --merge --admin`

### P-007 Notes

`backlogit_ship_shipment` deleted `033.003-T.md` from archive (naming collision with prior mcp-json task). Restored with `git restore .backlogit/archive/`. Queue stale files for 033.001-T and 033.002-T manually removed after ship.

### Known Gotchas for Future Sessions

- backlogit `shipment ship` validates ALL children of feature by `parent_id`, not just manifest items
- Historical naming collision during ship: archived `033.003-T` (012-S mcp-json task) conflicted with the then-queued CREATE PROCEDURE task, which was renamed to `033.005-T`
- Rust 1.95 vs 1.85 clippy drift: run `cargo clippy --all-targets` locally

### Follow-Up Stash

| ID | Summary |
|---|---|
| 8C651D9F | Batch-UPDATE for reresolve_references_edges |
| E145945C | INDEX on target in references schema |
| DA9D4948 | DRY refactor index/sync resolution logic |
| B0903A71 | Full Class node resolution for SQL references |

### Compound Learnings Captured

- `docs/compound/database-issues/surrealdb-select-star-serde-json-2026-04-29.md`
- `docs/compound/build-errors/tree-sitter-sequel-join-grammar-2026-04-29.md`
- `docs/compound/workflow-issues/rust-1-95-clippy-lint-ci-mismatch-2026-04-29.md`
