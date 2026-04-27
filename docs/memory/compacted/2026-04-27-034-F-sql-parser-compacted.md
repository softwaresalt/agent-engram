---
type: compacted-memory
date: 2026-04-27
release_unit: 013-S / 034-F
slug: 034-F-sql-parser
sources:
  - docs/memory/2026-04-26/sql-parser-stage-lifecycle-memory.md
  - docs/memory/2026-04-26/sql-parser-ship-execution-memory.md
  - docs/memory/2026-04-26/sql-parser-ci-fix-memory.md
  - docs/memory/2026-04-26/sql-parser-post-merge-closure-memory.md
  - docs/memory/2026-04-26/autoharness-tune-pr-memory.md
  - docs/memory/2026-04-26/012-S-post-merge-closure-memory.md
compacted_at: 2026-04-27
---

## Release Unit: 013-S — SQL File Indexing via tree-sitter-sequel

**Feature**: 034-F | **Stage**: completed | **Ship**: completed | **Main merge**: `aedc3e0`

---

### What Was Built

SQL file indexing support via `tree-sitter-sequel 0.3`. Parses `.sql` files and emits:
- `CREATE TABLE` / `CREATE VIEW` → `ExtractedSymbol::Class`
- `CREATE FUNCTION` → `ExtractedSymbol::Function`
- `SELECT ... FROM` / `INSERT INTO` → `ExtractedEdge::References { source, target }`

New files: `src/services/parsing/sql.rs` (~210 lines), 10 unit tests, 1 IPC integration test.

---

### Key Technical Decisions

| Decision | Rationale |
| --- | --- |
| tree-sitter-sequel 0.3 (ABI 15) | Only SQL grammar compatible with tree-sitter 0.25 runtime in crates.io |
| `ExtractedEdge::References { source, target }` new variant | Required for SQL FROM/INSERT edges; not a pre-existing variant |
| Node kind discovery via debug tree walk | Grammar docs absent; empirical walk required to find correct kind names |
| `CREATE PROCEDURE` → graceful degradation (0 symbols) | Grammar 0.3 produces ERROR nodes; forward-compat arm retained with doc note |
| `extract_sql_name` reads `object_reference > identifier` | Not via field-name API; specific to sequel grammar structure |
| `language_from_path` extension: `"sql"` | Wire-up only change that routes `.sql` into parsing pipeline |

---

### CI Failure Encountered

**`clippy::items_after_statements`** — inner `fn dump` helpers in two debug tests were
declared after `let` statements. Fixed in `d243dd2` by hoisting `fn dump` before all
`let` bindings (matching existing C++ debug test pattern).

---

### Stage Lifecycle

- Stash `8AC6828D` → deliberation → impl-plan → plan-review (FAIL P1×2 → revise → PASS) → harvest → shipment 013-S
- P1 revisions: added `ExtractedEdge::References` to Unit 1; split Unit 3 into 3a + 3b
- Plan review passed on second pass with 0 remaining P0/P1 findings

---

### Files Changed (Production)

| File | Change |
| --- | --- |
| `Cargo.toml` | `tree-sitter-sequel = "0.3"` added |
| `src/services/parsing.rs` | `Language::Sql`, `ExtractedEdge::References`, `pub parse_sql_source` |
| `src/services/parsing/sql.rs` | NEW — full parser |
| `src/services/code_graph.rs` | References match arms + `"sql"` in `language_from_path` |
| `tests/unit/parsing_test.rs` | 10 new SQL tests |
| `tests/integration/lang_ipc_indexing_test.rs` | SQL IPC integration test |
| `docs/ARCHITECTURE.md` | `Language::Sql` in enum table; Multi-Language Parsing section |

---

### Post-Merge Closure

- Pre-mode reconcile: PROCEED (all 6 items `done`, 0 orphans)
- Archival: 013-S + 034-F + 034.001-T through 034.005-T moved to `.backlogit/archive/`
- Post-mode reconcile: PROCEED (all 7 archive files present, 0 deletions)
- PR #34 (`stage → main`) merged as `aedc3e0`
- PR #36 (`post-merge/034-F-sql-parser → main`) opened; awaiting merge approval
- 3 follow-up stash entries: 19D78639 (CREATE PROCEDURE), F15C561F (ref resolution), 8232DE58 (multi-schema)

---

### Compound Learnings Captured

- `docs/compound/build-errors/tree-sitter-sequel-node-kind-debugging-2026-04-27.md` — NEW
- `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` — updated (sequel row added)

---

### PR #34 Copilot Review Comments (all resolved)

Round 1 (`cd78274`): 5 comments — compact table separators, Tavily API key redaction, `removed_at` timestamp correction.
Round 2 (`edbaa06`): 3 comments — H1 removed from memory file, `sql.rs` CREATE PROCEDURE doc updated, spacing fix in `code_graph.rs`.
All 8 threads resolved via GraphQL.
