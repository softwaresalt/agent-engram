# Session Memory: 007-S Code Graph Tier-2 Completion

**Date**: 2026-04-24  
**Session**: 8c321d50-649b-48ce-9441-1f48e80309ce  
**Shipment**: 007-S (shipped at commit 659c29c)

---

## Tasks Completed

| ID | Title | Commit |
|---|---|---|
| 030.001-C | IPC e2e verification for Swift/C/C++ | Prior session commit |
| 030.002-C | C++ inline member extraction | Prior session commit |
| 030.003-C | Markdown parser (pulldown-cmark 0.10) | dec7a9f |
| 030.004-C | SQL dialects spike | 3e399d8 |
| style fix | fmt + clippy pedantic on markdown.rs | 570324a |
| 007-S closure | Archive + reconcile reports | 659c29c |

---

## Files Created

- `src/services/parsing/markdown.rs` — Markdown parser using pulldown-cmark 0.10
  - Headings → `ExtractedClass`, fenced code blocks → `ExtractedFunction`, links → `ExtractedEdge::Imports`
  - `#[allow(clippy::unnecessary_wraps)]` on `parse_markdown_source` (dispatcher contract requires Result)
  - `#[allow(clippy::naive_bytecount)]` on `byte_offset_to_line` (bytecount crate not a dep)
- `tests/integration/markdown_indexing_test.rs` — IPC e2e for Markdown indexing
- `docs/decisions/2026-04-24-sql-grammar-spike.md` — recommends `tree-sitter-sequel 0.3.11`
- `.backlogit/reconcile/007-S-pre-2026-04-24T0622.md` — pre-mode reconcile report
- `.backlogit/reconcile/007-S-post-2026-04-24T0625.md` — post-mode reconcile report

## Files Modified

- `src/services/parsing.rs` — `Language::Markdown` variant, dispatch arm
- `src/services/code_graph.rs` — `"md" => "markdown"` in `language_from_path()`
- `tests/unit/parsing_test.rs` — 7 Markdown unit tests added

---

## Key Technical Decisions

### pulldown-cmark chosen over tree-sitter-md
`tree-sitter-md` had ABI compatibility uncertainty. `pulldown-cmark 0.10` was already a dep, 
is stable, and the event-based `into_offset_iter()` API maps cleanly to our extraction model.

### SQL spike: tree-sitter-sequel 0.3.11 recommended
- Requires `~0.25.0` → ABI 15, compatible with our 0.25 runtime
- Broadest dialect coverage (ANSI, PostgreSQL, MySQL, SQLite, T-SQL, BigQuery)
- Build-verified (cargo check clean)
- Follow-up grammar wire-up task stashed in backlogit

### clippy::unnecessary_wraps: allow attribute pattern
When a function's return type is required by a dispatcher contract but the function 
never errors (infallible), add `#[allow(clippy::unnecessary_wraps)]` with a doc comment 
explaining why. See `kotlin.rs` for prior precedent.

---

## Known Issues / Observations

- `t046_s050_daemon_exits_after_idle_timeout_and_restarts` is a flaky timing test when run 
  concurrently in the full suite; passes reliably in isolation. Pre-existing issue.
- `backlogit_move_item` to `done` physically moves files to `.backlogit/archive/` as untracked 
  files. `backlogit_ship_shipment` subsequently processes and removes these pre-archived files.
  Post-mode reconcile handles this via "confirmed-by-tool" classification.
- Backlogit subtasks missing `priority:` field in frontmatter will fail `move_item` with 
  `missing required fields: priority`. Fix: call `update_item` with `priority: "medium"` first.

---

## Backlog State After Session

- 007-S: **shipped** (archived)
- 030-F + all 030.xxx-C + all 030.xxx.xxx-T: **archived**
- 030.005-C (Kotlin): remains **blocked** in queue (upstream ABI issue)
- SQL follow-up: **stashed** for future implementation

---

## Next Steps

- 008-S is next in queue (check with `backlogit_get_shipment 008-S`)
- SQL parser implementation: create tasks from stash when tree-sitter-sequel is chosen
- Kotlin: watch for tree-sitter-kotlin 0.25-compatible release
