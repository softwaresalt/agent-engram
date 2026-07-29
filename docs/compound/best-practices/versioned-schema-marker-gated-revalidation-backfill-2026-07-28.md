---
title: "Retro-fitting persisted-artifact correctness: versioned schema_meta marker + opt-in gated revalidation backfill"
description: "When an extraction/resolution logic change corrects freshly-indexed output but leaves already-persisted artifacts stale (hash-skip never revisits them), gate the migration behind a durable schema_meta generation marker and an OPT-IN --revalidate flag; advance the marker only on a fully-clean pass (fail-closed), and retract stale raw edges in EVERY teardown path before deleting their keying metadata"
problem_type: "logic_error"
category: "best-practices"
component: "src/services/code_graph.rs"
root_cause: "A correctness fix to the extractor/resolver (e.g. 100-F same-file fail-closed guard) only applies to files that are re-extracted; content-hash skip means unchanged files keep the pre-fix WRONG persisted artifacts indefinitely, and a routine sync silently never repairs them"
resolution_type: "code_fix"
severity: "high"
message: "code_graph_extraction_generation_gated_revalidation_backfill"
file_path: "src/services/code_graph.rs"
date: "2026-07-28"
feature: "101-F"
shipment: "094-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/293"
  - "src/services/code_graph.rs (CODE_GRAPH_EXTRACTION_GENERATION const; run_codegraph_revalidation gating; fail-closed marker advance on index + sync paths; handle_deleted_file centralized direct-edge retraction)"
  - "src/db/cozo_queries.rs (retract_direct_calls_edges_for_file; code_graph_extraction_generation read/set)"
  - "tests/integration/codegraph_revalidation_acceptance_test.rs (upgrade round-trip; forced-index; deletion path; partial-failure retry; opt-in no-op)"
  - "docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md (the problem this pattern solves)"
tags:
  - "code-graph"
  - "schema-meta"
  - "versioned-marker"
  - "gated-backfill"
  - "fail-closed"
  - "migration"
  - "hash-skip"
  - "096-F"
  - "101-F"
---

# Versioned schema_meta marker + opt-in gated revalidation backfill

## Problem

A correctness fix to an extractor/resolver only takes effect for files that are
actually re-extracted. engram's incremental sync **hash-skips** unchanged files,
so any WRONG artifact (e.g. a same-file `direct` calls edge minted before the
100-F fail-closed guard) that was persisted *before* the fix stays in the graph
indefinitely. A routine `engram sync` never revisits those files, so the fix
silently fails to reach existing workspaces — but blanket-forcing a full
re-extraction on every sync (churn) is unacceptable, and doing it automatically
risks re-introducing regressions on partial failure.

## Pattern (proven twice: 096-F `python_extraction_version`, 101-F `code_graph_extraction_generation`)

1. **Durable version marker in `schema_meta`.** Store a small generation string
   (e.g. `code_graph_extraction_generation = "1"`) recording the logic version
   the persisted artifacts were last materialized under. Bump the constant when
   the extraction/resolution logic changes.

2. **Opt-in gate, never automatic.** A stale marker on a routine sync is a
   **no-op deferral**, not a silent re-extraction (`run_revalidation =
   !generation_current && caller_passed_--revalidate-flag`). Operators opt in
   with `--revalidate-code-graph` (incremental, generation-gated) or the
   forced-index route; routine sync stays churn-free.

3. **Fail-closed marker advance — and reconcile prior-indexed paths before
   advancing.** Advance the marker **only** when EVERY
   indexed file was freshly (re-)extracted this pass with no errors and no
   bypass (`force || !any_hash_skipped`, and no zero-byte/oversized bypass of an
   indexed file). If any file was skipped/failed/bypassed, keep the OLD marker so
   the next run retries — a half-migrated graph must never be certified as
   current (plan invariants A3/C7-3/H2). **The durable pattern also requires
   reconciling files indexed under the OLD marker that are no longer discovered
   this pass** (renamed/deleted/newly-excluded) before advancing; otherwise
   `force` alone advances the marker while their stale artifacts survive.
   **KNOWN GAP (as shipped in 101-F):** the forced-index route
   (`index_workspace_impl`) advances the marker on `force` alone
   (`code_graph.rs` ~1899-1900) but discovers only currently-present files
   (`code_graph.rs` ~1229), so it does NOT yet reconcile
   previously-indexed-now-excluded paths — for that route the "EVERY indexed
   file" invariant is scoped to files visited this pass, and full prior-path
   reconciliation awaits follow-up `92EE75BB`. The incremental
   `sync --revalidate-code-graph` route is likewise scoped to discovered files.

4. **Retract stale raw edges in EVERY teardown path, before deleting keying
   metadata.** `delete_functions_by_file` removes `function_meta` but NOT the raw
   `calls_edge` rows, so a same-file `direct` edge would survive as a **dangling
   row** keyed on a retired id. Retract it in the modified-file re-index teardown,
   the forced-index teardown, AND the shared `handle_deleted_file` (deletion +
   oversized eviction) — centralize in the shared helper so no call site is
   missed. Direct edges are same-file by construction, so the caller-filtered
   retraction can never remove a cross-file edge (preserves 094-F invariants).

## Gotcha — user-facing invariant vs raw-row hygiene

The retraction query joins `calls_edge.from` to `function_meta.id` to attribute
the file, so it only reaches edges whose caller metadata is still live. That is
sufficient for the **user-facing** guarantee ("no wrong query answers"): graph
queries (`map_code`/`impact_analysis`) join BOTH endpoints to `function_meta`, so
a *legacy already-orphaned* raw row (caller re-minted by a pre-fix ordinary sync)
is **non-traversable** and cannot surface a wrong target. Purging orphaned raw
rows is a separate one-time GC (`calls_edge` rows whose `from`/`to` lacks
`function_meta`), correctly triaged as a scoped follow-up rather than a release
blocker. When a late review flags "the marker advances while a stale row
survives," first ask: *is that row traversable?* If not, the correctness gate
still holds.

## CLI routing (get the docs right)

- `engram sync --revalidate-code-graph` → **incremental, generation-gated**
  (a no-op once the marker matches; idempotent and churn-free).
- `engram index --revalidate-code-graph` and `engram sync --full
  --revalidate-code-graph` → **forced full reparse** (always re-extracts every
  currently-discovered file, even when the marker matches — a hammer, NOT
  churn-free). Caveat: it re-extracts only files present at index time and still
  advances the marker on `force`, so previously-indexed-now-excluded paths are
  NOT reconciled by this route (forced-index reconciliation gap, follow-up
  `92EE75BB`).
- Plain `engram sync --full` does **not** imply `--force`; it still hash-skips.

## Why it matters

This lets a shipped correctness fix reach existing workspaces on the operator's
schedule, without churning routine syncs and without auto-running a risky
migration. The fail-closed marker keeps a graph that is only partially migrated
*over the files a pass actually visited* from being mistaken for a fully-migrated
one, so a later `--revalidate` run finishes the job for those files. It does
**not** yet cover previously-indexed paths that a later exclusion removes from
discovery: the forced-index route can advance the marker without revisiting
them, so whole-workspace reconciliation awaits follow-up `92EE75BB`.
