---
title: "Compound Refresh — 094-F Python bare-call code-graph edges + language-scoped resolution"
date: "2026-07-20"
scope: "recent"
mode: "apply"
context: "Post-merge closure of shipment 089-S / feature 094-F (PR #277, merge 5f18b79)"
feature: "094-F"
shipment: "089-S"
pr: 277
merge_commit: "5f18b796853cb82f977494672fa280046bcbe5b8"
---

# Compound Refresh — 094-F (Python bare-call code-graph edges)

Post-merge capture of durable learnings from feature 094-F, the first
per-language rollout of function-call graph extraction (Python bare calls),
plus the language-scoping hardening of the shared cross-file singleton
resolver. Evidence gathered from the merged tree on `main` at
`5f18b79`.

> **Operational closure** for this feature is recorded separately in
> [`docs/closure/2026-07-20-094-F-operational-closure.md`](./2026-07-20-094-F-operational-closure.md)
> (runtime-verification outcome, SLIs, rollback trigger/procedure, and
> observation window). This compound-refresh report captures durable *learnings*
> only and is **not** a substitute for that operational-closure record.

## New Entries Created (mode=apply)

| File | Category | Learning |
|---|---|---|
| `best-practices/python-bare-call-edge-extraction-body-scoped-2026-07-20.md` | best-practices | Only-Rust-emitted-Calls baseline; Python bare-call extraction must walk the `body` field only, stop at nested callable/class scopes (inner-fn calls omitted), and drop attribute/method + subscript/chained calls fail-closed (013-D). |
| `best-practices/language-scoped-singleton-resolver-filter-before-count-2026-07-20.md` | best-practices | Cross-file singleton resolver must filter candidates by the caller's language BEFORE the exactly-one check; pinned by a discriminating mixed-language positive test. |
| `workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md` | workflow-issues | Content-hash skip (plain `sync`, `index`, and `sync --full` all pass force=false) + full-index-only post-pass mean existing unchanged `.py` files need a forced reindex — `engram sync --force` (or `engram index --force`) — to acquire new edge types. |
| `bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md` | bugs | `find_function_id` returns FIRST name match → same-file same-name shadowing binds a wrong (shadowed) target; known issue, tracked as bug FF7DE872. |

## Entries Reviewed for Overlap (classification: keep)

| Existing Entry | Classification | Rationale |
|---|---|---|
| `sync-workspace-record-file-hash-required-2026-05-08.md` | keep | Covers hash-table upkeep *within* sync; distinct from the new "extractor-version vs content-hash skip" caveat. Cross-referenced from the new workflow-issues entry. |
| `hydrate-code-graph-fast-path-already-indexed-2026-05-08.md` | keep | Startup fast-path freshness; adjacent but distinct. Cross-referenced. |
| `build-errors/tree-sitter-*` (grammar walkers) | keep | Grammar-walking pitfalls for SQL/TSX; the new Python entry is language-specific and non-overlapping. |

No existing entries were consolidated, replaced, or deleted — the four new
learnings are net-new and non-duplicative.

## Evidence Used

- `src/services/parsing/python.rs` — `extract_calls_from_body` (~L233-284):
  body-only DFS, stops at `function_definition`/`lambda`/`class_definition`;
  `resolve_call_name` (~L286-320): identifier→bare, attribute→fail-closed drop,
  subscript/chained→`None`.
- `src/db/cozo_queries.rs` — language-scoped singleton resolver (~L2151-2280);
  `same_language` filter-before-count (~L2262-2270).
- `src/services/code_graph.rs` — `canonical_path` Rust-only/fail-closed empty
  (~L41-67); cross-file singleton post-pass "Full/--force index only" (~L985-1002);
  `find_function_id` first-match (~L2037-2041).
- `tests/integration/code_graph_test.rs` —
  `python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def`
  (commit 9a76627), the discriminating ordering proof.
- `docs/exec-plans/2026-07-20-python-call-edge-extraction-plan.md`,
  `docs/decisions/2026-07-20-python-call-edge-extraction-spike.md`.

## Follow-Up Items (not part of this refresh)

- **FF7DE872** (bug, medium): fix same-file same-name shadowing target
  precision — independent, sooner; not gated behind namespace resolution.
- **FE8B3B2D** (feature, medium): Python module-namespace-qualified call
  resolution — later staging cycle.
- **07BFA98E** (spike, medium): Spark notebook data-lineage — later.
- Per-language rollout (TS/Go/C#/Node): each new language inherits the
  language-scoped resolver; re-run the mixed-language ordering proof.
