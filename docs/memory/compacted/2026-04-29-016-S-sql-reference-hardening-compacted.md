---
title: "016-S SQL Reference Resolution Hardening — Session Compacted Memory"
shipment: 016-S
feature: 035-F
pr: 49
merge_commit: 0e4e79a
status: shipped
session_checkpoints:
  - docs/memory/2026-04-29/008-S-post-merge-closure-session-memory.md
---

## Outcome

PR #49 merged (merge commit `0e4e79a`). All 4 units shipped. Shipment 016-S archived.

## Tasks Completed

| ID | Title | Status |
|---|---|---|
| 035.001-T | Add `references` target index | done |
| 035.002-T | Batch class lookup in `reresolve_references_edges` | done |
| 035.003-T | Extract `resolve_reference_target` helper (DRY) | done |
| 035.004-T | Quoted/case-insensitive identifier resolution | done |
| 035-F | SQL Reference Resolution Hardening (feature) | done |
| 016-S | Shipment | shipped |

## Files Modified

- `src/db/schema.rs` — `references_target` index
- `src/db/queries.rs` — `ReresolveResult`, `ClassNameIdRow`, `get_class_by_name_ci`, `resolve_reference_target`, `strip_sql_quotes`, refactored `reresolve_references_edges`
- `src/db/cozo_queries.rs` — `resolve_reference_target` + `get_class_by_name_ci` stubs
- `src/services/code_graph.rs` — 2 inline resolution blocks replaced
- `tests/contract/references_edge_test.rs` — 5 new contract tests

## Key Decisions

1. **Candidates-list pattern**: build `[raw, last-seg, stripped, stripped-last]`, dedup in order, try exact then CI
2. **SurrealDB `string::lowercase()` unreliable** in WHERE clauses on embedded KV — Rust-side lowercasing required
3. **cozo-backend API parity**: every new `pub(crate)` method in `queries.rs` needs a stub in `cozo_queries.rs`; unused stubs need `#[allow(dead_code)]`
4. **`cargo clippy --all-targets`** required to catch test-file pedantic violations; CI uses it, local default doesn't

## CI Fixes Required (2 rounds)

1. Added cozo-backend stubs (`resolve_reference_target`, `get_class_by_name_ci`) — CI failed immediately
2. Fixed test-file pedantic violations (`doc_markdown`, `similar_names`) — only caught with `--all-targets`

## Copilot Review (8 comments, 2 rounds)

All 8 threads resolved programmatically via `gh api graphql resolveReviewThread`. Key substantive fixes:
- Refactored to candidates-list (per review comment 2)
- Added all 4 name variants to batch pre-compute (per review comment 4)
- Removed `!contains('.')` fallback guard (per review comment 6)
- Updated 016-S.md `custom_fields.items` (per review comment 5)

## Failed Approaches

- **`string::lowercase()` WHERE clause**: silently returned zero rows — switched to Rust-side post-fetch lowercasing
- **`!contains('.')` per-edge guard**: prevented schema-qualified names like `public."Users"` from resolving

## Compound Learnings Captured

- `workflow-issues/clippy-all-targets-test-file-lints-2026-04-29.md`
- `build-errors/cozo-backend-api-parity-stub-required-2026-04-29.md`
- `database-issues/surrealdb-lowercase-where-clause-broken-2026-04-29.md`
- `best-practices/sql-quoted-identifier-resolution-candidates-list-2026-04-29.md`

## Closure Artifacts

- `docs/closure/2026-04-29-016-S-sql-reference-resolution-closure.md`
- `.backlogit/reconcile/016-S-pre-20260429T194700.md`
- `.backlogit/reconcile/016-S-post-20260429T194900.md`
- `docs/exec-plans/2026-04-29-sql-reference-resolution-hardening-decided-plan.md`
