---
title: "Operational Closure — 094-F Python bare-call code-graph edges + language-scoped resolution"
date: "2026-07-20"
mode: "post-merge"
feature: "094-F"
shipment: "089-S"
pr: 277
merge_commit: "5f18b796853cb82f977494672fa280046bcbe5b8"
readiness: "READY"
surface: "local daemon indexing (no production/service deployment surface)"
---

# Operational Closure — 094-F (Python bare-call code-graph edges)

**Mode:** post-merge · **Readiness:** **READY** (merged via PR #277, merge
commit `5f18b79`).

This record closes the operational loop for feature 094-F. It is deliberately
**non-inflated**: engram is a locally-installed developer/agent indexing daemon,
not a hosted service. There is **no production deployment surface, no live
traffic, and no remote monitoring**. This disposition is consistent with the
adjudicated release-observability outcome on PR #277 (observability tooling
waived as justified **N/A** for a local indexing feature). The concrete
"monitoring plane" here is the acceptance-test suite (in CI) plus the first
local reindex of a real Python workspace.

## Change Summary

Emit bare Python `Calls` edges (identifier call sites within a function `body`)
so `map_code` / `impact_analysis` / `query_graph` light up for `.py` files, and
harden the shared cross-file singleton resolver to be **language-scoped**
(filter candidates by the caller's language before the exactly-one check) so
Python bare calls cannot mis-bind cross-language. Fail-closed on attribute /
method / subscript / chained calls and on ambiguous names (013-D no-false-edge
invariant; 082-F target-correctness gate).

Affected surfaces: `src/services/parsing/python.rs`,
`src/db/cozo_queries.rs`, `src/services/code_graph.rs`, `docs/architecture.md`.
No new DB schema: the `function_meta.canonical_path` column already existed from
feature 091-F (Rust canonical resolution) — 094-F only populates/consumes it in
a language-scoped way.

## Invariants to Preserve

- **013-D no-false-edge:** no bare call may produce an edge to a wrong or
  ambiguous target; unresolved/ambiguous calls are dropped, not guessed.
- **082-F target-correctness:** a resolved `Calls` edge points at the actual
  callee definition.
- **Cross-language isolation (094-F U3/U4):** a Python bare call never binds to
  a same-named Rust (or other-language) definition, and Rust singleton
  resolution is unchanged under the added language scope.

## Runtime Verification Outcome — PASS

Verified by the calls acceptance + correctness suite (all green on the merged
tree):

- `tests/integration/calls_recall_acceptance_test.rs`
  - `post_change_recall_exceeds_pre_change` — recall SLI: Python call edges are
    recovered after the change vs the pre-change baseline.
  - `false_edge_rate_within_threshold` — precision SLI: false-edge rate stays
    within the allowed threshold.
  - `singleton_edges_match_expected_manifest` — resolved singleton edges match a
    pinned expected manifest.
  - `ambiguous_name_contributes_no_edge` — fail-closed: an ambiguous name yields
    **no** edge.
  - `rust_singleton_resolution_unchanged_under_language_scope` — cross-language
    guard: Rust resolution is unaffected by the language-scoping change.
- Supporting: `calls_target_correctness_test.rs`,
  `calls_postpass_resolution_test.rs`, `calls_staging_lifecycle_test.rs`,
  `calls_edge_resolution_storage_test.rs`,
  `code_graph_test.rs::python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def`
  (the discriminating mixed-language ordering proof).

**Verdict: PASS.**

## SLIs / Healthy vs Failure Signals

| Signal | Healthy | Failure |
|---|---|---|
| **Calls-edge recall** | `post_change_recall_exceeds_pre_change` green; `.py` files surface call edges in `map_code`/`impact_analysis`/`query_graph` | Python call graphs empty after a forced reindex; recall test red |
| **Calls-edge precision** | `false_edge_rate_within_threshold` green; `ambiguous_name_contributes_no_edge` green | Any false/wrong edge; false-edge rate over threshold; ambiguous names producing edges |
| **Cross-language isolation** | `rust_singleton_resolution_unchanged_under_language_scope` green; Python calls never bind to same-named Rust defs | Python call binds cross-language; Rust resolution changes |

## Pre-Deploy Audits

- **Forced reindex required for existing workspaces.** New extraction logic does
  **not** backfill onto already-indexed `.py` files via a plain `engram sync`,
  `engram index`, or `engram sync --full` (all pass `force=false` and hash-skip
  unchanged files). Operators must run **`engram sync --force`** (or
  `engram index --force`) once after upgrading. See
  `docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`.
- **No schema migration to apply.** `canonical_path` pre-existed from 091-F.

## Deployment / Rollout Path

Merge-only (already merged). Distribution is the normal engram binary upgrade;
each operator's first `engram sync --force` materializes the new edges locally.
No canary, no phased rollout, no maintenance window — there is no shared service.

## Post-Deploy Checks (first local reindex)

1. Upgrade the binary; run `engram sync --force` on a Python workspace.
2. `map_code` / `impact_analysis` on a known caller show the expected Python
   call edges.
3. Spot-check a same-named cross-language pair: the Python call resolves to the
   Python target, not a same-named Rust def.

## Rollback

- **Trigger:** any precision regression — a false/wrong `Calls` edge surfacing
  (i.e., `false_edge_rate_within_threshold` or `ambiguous_name_contributes_no_edge`
  going red), or cross-language mis-binding
  (`rust_singleton_resolution_unchanged_under_language_scope` red).
- **Procedure:** `git revert` the 094-F parsing/resolution change (Python
  bare-call extraction in `python.rs` + the language-scoping in
  `cozo_queries.rs`/`code_graph.rs`), rebuild, then **`engram sync --force`** to
  re-parse and drop the reverted edges. **No schema migration to unwind** — the
  `canonical_path` column pre-existed from 091-F and stays; Python rows simply
  return to unpopulated. Rollback/downgrade safety is pinned by
  `tests/integration/cli_calls_resolution_rollback_test.rs` and
  `tests/integration/calls_resolution_downmigration_test.rs`
  (rehydrate path: `calls_resolution_rehydrate_test.rs`).

## Validation Window & Owner

- **Window:** the first local forced reindex of a Python workspace after upgrade
  (there is no continuous production window to watch). If no false edges surface
  on that first reindex and acceptance tests remain green in CI, the change is
  considered absorbed.
- **Owner:** repository maintainer (local daemon; no on-call/service owner).

## Readiness

**READY.** Already merged; runtime verification PASS; rollback trigger and
procedure explicit; no production surface to monitor. Follow-ups tracked
independently and **not** part of this closure: FF7DE872 (same-file same-name
shadowing precision bug), FE8B3B2D (namespace-qualified resolution feature),
07BFA98E (Spark lineage spike).
