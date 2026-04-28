---
title: "Pre-Merge Closure — 034-F SQL File Indexing via tree-sitter-sequel"
date: 2026-04-26
mode: pre-merge
feature: 034-F
shipment: 013-S
branch: feature/034-F-sql-parser
pr: 35
ci_status: green
review_status: no open findings
readiness: READY
owner: ship-agent
---

## Change Summary

Adds SQL file indexing support to the agent-engram code graph service via the
`tree-sitter-sequel 0.3` grammar. The engram daemon now:

- Recognizes `.sql` files via `language_from_path()`
- Parses SQL source using a new `src/services/parsing/sql.rs` module
- Extracts `Class` symbols for `CREATE TABLE` and `CREATE VIEW` definitions
- Extracts `Function` symbols for `CREATE FUNCTION` definitions
- Emits `References` edges for `FROM` (SELECT queries) and `INSERT INTO` targets
- Gracefully degrades on unsupported syntax (e.g., `CREATE PROCEDURE`) — emits 0 symbols rather than panicking

### Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` | `tree-sitter-sequel = "0.3"` added |
| `src/services/parsing.rs` | `Language::Sql`, `ExtractedEdge::References`, `pub parse_sql_source` wrapper |
| `src/services/parsing/sql.rs` | **NEW** — full SQL parser (~210 lines) |
| `src/services/code_graph.rs` | `"sql"` in `language_from_path()`, `Defines \| References` no-op arms |
| `tests/unit/parsing_test.rs` | 10 new SQL unit tests |
| `tests/integration/lang_ipc_indexing_test.rs` | 1 new IPC integration test |

---

## CI Status

| Check | Status | Duration |
|-------|--------|----------|
| `CI/build (cozo-backend)` | ✅ pass | 1m 18s |
| `CI/build (surreal-backend)` | ✅ pass | 8m 14s |

CI remediation required one fix commit (`d243dd2`): `clippy::items_after_statements` on two
debug test helpers that defined `fn dump` after `let` bindings. Fix: hoisted `fn dump` before
all statements. Both backends now green.

**Pre-existing audit advisories** (not introduced by this PR): `webpki 0.101.7` and
`rustls-webpki 0.103.9` have 6 RUSTSEC-2026-* advisories; `rcgen 0.10.0` has 1 advisory.
These are transitive SurrealDB dependencies. The `surreal-backend` audit step shows 10 allowed
warnings (pre-configured in the workflow). These advisories existed on the base branch before
this feature and are tracked separately.

---

## Invariants to Preserve

1. **Graceful degradation** — SQL files containing unsupported grammar nodes (e.g., `ERROR`
   nodes from `CREATE PROCEDURE`) must produce 0 symbols rather than panicking or returning
   an error to the caller.
2. **Existing language parsers unaffected** — `parse_rust_source`, `parse_python_source`,
   `parse_javascript_source`, `parse_typescript_source`, `parse_c_source`, `parse_cpp_source`,
   `parse_swift_source` must behave identically before and after this change.
3. **Empty SQL files** — must return `(vec![], vec![])` without error.
4. **Symbol type mapping** — `CREATE TABLE` and `CREATE VIEW` → `Class`; `CREATE FUNCTION` → `Function`. This contract is tested and must not regress.
5. **No `unwrap()` or `expect()` in production paths** — the parser uses `?` propagation throughout; no panicking fallbacks exist in the implementation path.

---

## Pre-Deploy Checks

This is a local-first daemon binary — no cloud deployment. The "deployment" is the next
binary release or a local rebuild.

- [x] `cargo fmt --all -- --check` — passes
- [x] `cargo clippy -- -D warnings -D clippy::pedantic` — passes
- [x] `cargo test` — all 46+ unit tests pass, integration test registered
- [x] CI green on both backend variants
- [x] No new `unwrap()` or `expect()` in production code paths
- [x] `tree-sitter-sequel` ABI compatibility verified: runtime 0.25.x accepts ABI 13–15; sequel 0.3.11 is compatible
- [ ] Post-merge: confirm `.sql` files are indexed on a real workspace (manual smoke)

---

## Deployment / Rollout Path

**Merge-only** — no service deployment. After merge to `stage/034-F-sql-parser` (PR #35),
and subsequently to `main` when the stage branch is merged:

1. Users rebuild the binary (`cargo build --release`)
2. The daemon auto-detects `.sql` files on next workspace index or file watcher event
3. No configuration changes needed; SQL support is unconditionally enabled

---

## Post-Deploy Checks

After the binary is rebuilt and the daemon restarted:

1. Create a test `.sql` file with a `CREATE TABLE` statement in a watched workspace
2. Trigger indexing (`sync_workspace` MCP tool or file-save event)
3. Call `list_symbols` — verify the table name appears as a `Class` symbol
4. Call `map_code` on the `.sql` file — verify the symbol appears in the call graph
5. Verify no panic or error in daemon logs for a SQL file with `CREATE PROCEDURE` syntax

---

## Risky Action Record

| Action | Risk | Approval | Result |
|--------|------|----------|--------|
| Add `ExtractedEdge::References` variant | moderate — new enum variant; existing match arms must be exhaustive | N/A (non-breaking — new variant, both match blocks updated) | applied — no arms broke |
| `language_from_path()` returns `Sql` for `.sql` extension | low — additive path | N/A | applied |
| `tree-sitter-sequel` C grammar build dependency | moderate — requires C compiler in CI | verified via CI run | applied — both backends compiled successfully |

---

## Healthy Signals

- `.sql` files appear in `list_symbols` output after indexing
- `CREATE TABLE t` produces one `Class` symbol named `t`
- `CREATE FUNCTION f` produces one `Function` symbol named `f`
- `SELECT ... FROM t` produces a `References` edge from the query file to `t`
- No daemon errors or panics in logs for any valid `.sql` file

## Failure Signals

- Daemon panics or returns error on `.sql` file ingestion
- `list_symbols` returns no results for a `.sql` file that should have symbols
- Existing non-SQL language tests regress (e.g., Rust symbol extraction breaks)
- Integration test `t034_005_sql_create_table_indexed_via_ipc` fails in future CI runs

---

## Monitoring Plan

| Signal | Check | Dashboard / Tool |
|--------|-------|-----------------|
| SQL indexing runs | `list_symbols` on `.sql` files after sync | Manual / MCP CLI |
| No regressions | `cargo test` suite | CI / local |
| Grammar ABI stability | `tree-sitter-sequel` dependency audit on next `cargo update` | `cargo audit` |

No persistent metrics or dashboards required — this is a local-first binary with no
server-side telemetry surface.

---

## Rollback Trigger

If the daemon panics or returns `EngramError` for any `.sql` file that was previously
processable (or any Rust/Python/JS file regresses), rollback is warranted.

**Rollback procedure**: Revert the merge commit on the applicable branch. The
`language_from_path()` change is the only wire-up that causes `.sql` files to enter
the parsing pipeline. Reverting it effectively disables SQL indexing with zero impact
on other languages.

```bash
git revert <merge-commit-sha>
```

---

## Validation Window

**72 hours** after first binary rebuild with the new code. Primary validator: any developer
who uses the engram daemon with a SQL-containing workspace.

**Owner**: operator / user

---

## Follow-Up Items

1. **`CREATE PROCEDURE` grammar support** — `tree-sitter-sequel 0.3` produces `ERROR` nodes
   for `CREATE PROCEDURE`. Track `tree-sitter-sequel` for a future version that supports it,
   then update the test from "expect 0 symbols" to "expect 1 Function symbol".
2. **`SELECT` symbol extraction** — currently `FROM t` emits a `References` edge but the
   target is only the raw identifier string. Future work: resolve the reference to a known
   `Class` node in the graph when the table was indexed from the same workspace.
3. **Multi-schema SQL** — `schema.table` references (e.g., `FROM public.users`) are not
   yet parsed; only simple identifier references are extracted.
4. **RUSTSEC audit advisories** in `webpki` / `rcgen` / `rustls-webpki` — pre-existing
   transitive SurrealDB dependencies. Track for upstream fixes.

---

## Readiness Decision

**READY** — CI is green on both backends, all quality gates pass, no open review findings,
no unresolved runtime risk. The change is additive and gracefully degrades on unsupported
syntax. PR #35 is ready for operator merge approval.
