---
title: "Post-Merge Closure — 034-F SQL File Indexing via tree-sitter-sequel"
date: 2026-04-26
mode: post-merge
feature: 034-F
shipment: 013-S
feature_pr_merge_commit: 305b28f
stage_pr_merge_commit: aedc3e0
stage_branch: stage/034-F-sql-parser
pr_35: 35
pr_34: 34
owner: ship-agent
---

## Merge Summary

PR #35 (`feature/034-F-sql-parser` → `stage/034-F-sql-parser`) merged as commit `305b28f`.
PR #34 (`stage/034-F-sql-parser` → `main`) merged as commit `aedc3e0`.
All 6 shipment items archived (013-S + 034-F + 034.001-T through 034.005-T).

Pre- and post-mode shipment reconciliation passed cleanly (no orphans, no missing items,
no archive deletions). Copilot review comments on PR #34 (stage PR) addressed in commits
`cd78274`, `edbaa06`.

---

## Shipment Closure

| Item | Archive Status |
| --- | --- |
| 013-S | archived (commit `305b28f`) |
| 034-F | archived |
| 034.001-T | archived |
| 034.002-T | archived |
| 034.003-T | archived |
| 034.004-T | archived |
| 034.005-T | archived |

Reconcile reports: `.backlogit/reconcile/013-S-pre-20260426T201500.md` (PROCEED),
`.backlogit/reconcile/013-S-post-20260426T202000.md` (PROCEED).

---

## Knowledge Graduation

### architecture.md

Updated `docs/architecture.md`:

- `Language` enum entry updated in the Parsing service table to include `Sql` and `sql.rs`
- Multi-Language Parsing section updated with SQL grammar facts:
  - `tree-sitter-sequel 0.3` (ABI 15, compatible with tree-sitter 0.25 runtime)
  - Symbol mapping: `CREATE TABLE`/`CREATE VIEW` → `Class`, `CREATE FUNCTION` → `Function`
  - New `ExtractedEdge::References { source, target }` variant documented
  - Graceful degradation note for `CREATE PROCEDURE` (produces `ERROR` nodes in grammar 0.3)

### docs/research/

No new design decisions requiring graduation. The SQL parser decision and spike are
fully captured in:
- `docs/decisions/2026-04-24-sql-grammar-spike.md` (spike findings)
- `docs/decisions/2026-04-26-sql-parser-deliberation.md` (decision record)

---

## Source Artifact Cleanup

| Item | Source Stash ID | Deliberation Ref | Action |
| --- | --- | --- | --- |
| 034-F | `8AC6828D` | `docs/decisions/2026-04-26-sql-parser-deliberation.md` | Stash entry `8AC6828D` marked `harvested` in `.backlogit/stash.jsonl`; deliberation ref recorded in closure |

---

## Healthy Signals (post-merge)

- `.sql` files appear in `list_symbols` output after `sync_workspace`
- `CREATE TABLE t` produces one `Class` symbol named `t`
- `CREATE FUNCTION f` produces one `Function` symbol named `f`
- `SELECT ... FROM t` produces a `References` edge
- No daemon panics on SQL files with unsupported syntax

## Failure Signals

- Daemon panics or returns error on `.sql` file ingestion
- `list_symbols` returns no SQL results after indexing a `.sql` file
- Existing non-SQL language tests regress
- Integration test `t034_005_sql_create_table_indexed_via_ipc` fails

---

## Rollback Trigger

If the daemon panics or returns `EngramError` for any `.sql` file:

```bash
git revert 305b28f
```

The `language_from_path()` change is the only wire-up that routes `.sql` files into
the parsing pipeline. Reverting disables SQL indexing with zero impact on other languages.

---

## Validation Window

**72 hours** after next binary rebuild. Owner: operator.

---

## Follow-Up Items (stashed)

These were captured in `.backlogit/stash.jsonl` (IDs 19D78639, F15C561F, 8232DE58) during
pre-merge closure and remain actionable:

1. **CREATE PROCEDURE grammar support** — grammar 0.3 produces `ERROR` nodes; track upstream
   for a future version that correctly parses `CREATE PROCEDURE`
2. **SELECT reference resolution** — `FROM t` emits `References` edge but target is a raw
   string; future work resolves it to a known `Class` node when the table is indexed in the
   same workspace
3. **Multi-schema SQL** — `schema.table` dotted references not yet parsed; only simple
   identifier references are extracted

---

## Post-Merge Closure Decision

**CLOSED** — shipment archived, knowledge graduated, follow-ups stashed, `docs/architecture.md`
updated. PR #34 (`stage/034-F-sql-parser` → `main`) merged as `aedc3e0`.
Post-merge closure PR #36 (`post-merge/034-F-sql-parser` → `main`) awaiting operator approval.
