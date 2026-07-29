# Impl plan — Forced-index / revalidate certify-path completeness

- **Date:** 2026-07-29
- **Cycle:** Stage cycle 3 (post-094-S/101-F follow-ups, PR #293 review)
- **Feature:** 103-F
- **Source stash:** 685FAA80 (orphan direct-edge sweep, med), 92EE75BB
  (forced-index deletion/exclusion reconciliation, low)
- **Width:** code-graph reconciliation (`src/services/code_graph.rs` +
  `src/db/cozo_queries.rs`). Single width — no CLI/schema/template mixing.
- **Status:** reviewed + hardened (gate PASS below)

## Problem frame

101-F added a durable `code_graph_extraction_generation` marker
(`CODE_GRAPH_EXTRACTION_GENERATION = "1"`, `code_graph.rs:51`) that is advanced
on the forced-index route (`index_workspace`, marker set at `code_graph.rs:1900`)
and on the `--revalidate-code-graph` sync route (marker set at `~2923`). The
advance is gated fail-closed on "every discovered file was freshly extracted"
(`force || !any_hash_skipped` and `errors.is_empty()`).

Two completeness gaps let the marker certify a generation while stale raw state
survives — both discovered in PR #293 review:

- **G1 (685FAA80 — orphan `calls_edge` rows):** legacy same-file duplicate-name
  `direct` edges that were already orphaned (their caller/callee `function_meta`
  no longer exists) — e.g. re-minted by a pre-101-F ordinary sync — are **not**
  swept when the marker advances. They are non-traversable (queries join through
  `function_meta`, so no wrong answer is returned — `count_dangling_calls_edges`,
  `cozo_queries.rs:3576, 3602` proves the join semantics), but they violate literal
  raw-row hygiene (H4 of the 101-F hardening) and inflate `calls_edge` cardinality.
  There is **no** existing global GC for orphaned `calls_edge` rows:
  `rm_orphan_edges` (`cozo_queries.rs:5973`) is lineage-only
  (`lineage_edge`/`dataset_node`); `count_dangling_calls_edges` only counts.
- **G2 (92EE75BB — forced-index file-set reconciliation):** `index_workspace`
  walks only currently-**discovered** files (`code_graph.rs:1244`). The
  incremental sync path reconciles the prior indexed set against the current set
  in its Phase 1 deletion sweep (`code_graph.rs:2156–2185`, `handle_deleted_file`),
  but the forced-index route has **no** equivalent. A file that was previously
  indexed and is now **excluded** (still on disk, so it will never re-appear in a
  future incremental-sync deletion phase — that phase only fires for on-disk
  deletions) keeps its stale same-file `direct` edge while the generation marker
  advances and falsely certifies it. (Pure on-disk *deletion* self-heals via the
  next incremental sync's deletion phase; the *exclusion* case does not.)

Governing invariant (unchanged): 013-D no-false-edge / 082-F target-correctness.
This plan tightens raw-row hygiene and certify-path honesty; it must not create
or resurrect any traversable wrong edge, and must not regress recall.

## Normative anchors

- **A1** — The generation marker MUST NOT advance on a run that leaves either
  (a) an orphaned `calls_edge` row (no live `function_meta` on `from` **or** `to`)
  or (b) a previously-indexed-but-no-longer-discovered file's stale edges,
  unreconciled. Fail-closed: if reconciliation cannot complete, keep the prior
  marker (mirror the existing `revalidation_incomplete` behaviour, `~2926`).
- **A2** — Orphan sweep is **retraction-only** of non-traversable rows. It MUST
  NOT delete any `calls_edge` whose `from` and `to` both resolve to a live
  `function_meta` (no recall loss). Scope the sweep to the code-graph
  reconciliation pass only; do not touch lineage/concerns edges.
- **A3** — File-set reconciliation reuses the sync path's proven eviction
  primitive (`handle_deleted_file` / `retract_direct_calls_edges_for_file`,
  `cozo_queries.rs:2110`); no new deletion semantics are invented.
- **A4** — Language-agnostic; no Python-vs-Rust divergence in the sweep or the
  reconciliation (consistent with 101-F Option-B posture).
- **A5** — Idempotent: a second immediate revalidate run sweeps/reconciles zero
  rows and re-advances the marker to the same value.
- **A6** — Observability: emit counts (`dangling_edges_swept`,
  `files_reconciled`) via `tracing`/`IndexResult`/`SyncResult` so the CLI/API
  response does not silently under- or over-report (mirror 082.008-T).

## Design

### U1 — Orphan `calls_edge` sweep primitive + certify-path wiring (685FAA80)

- Add `retract_dangling_calls_edges(&self) -> Result<u64, EngramError>` to
  `CodeGraphQueries` (`cozo_queries.rs`, next to `count_dangling_calls_edges`):
  a single CozoScript `:rm calls_edge { from, to }` over the set where
  `not has_def[from]` **or** `not has_def[to]`, with
  `has_def[id] := *function_meta { id }`. Guard on
  `calls_edge_has_resolution` presence like the sibling retract helpers
  (`retract_all_calls_edges_with_resolution`, `~1688`) is **not** needed (this is
  keyed on `function_meta` liveness, not `resolution`), but keep the
  `run_script_busy_retry_mutable` busy-retry wrapper for SQLITE_BUSY parity.
- Wire the call into BOTH marker-advance certify blocks, **before**
  `set_code_graph_extraction_generation`:
  - forced-index route `index_workspace` (`code_graph.rs:~1900`);
  - `--revalidate-code-graph` sync route (`code_graph.rs:~2923`, inside
    `if run_codegraph_revalidation`).
  Return the swept count into `IndexResult`/`SyncResult` for A6.
- Fail-closed (A1): if the sweep errors, propagate `?` so the marker is not set
  (the surrounding `errors.is_empty()` / `revalidation_incomplete` gates already
  hold the prior marker on error).

### U2 — Forced-index file-set reconciliation (92EE75BB)

- Before the marker advance in `index_workspace` (`~1900`), reconcile the prior
  indexed file set against the just-discovered set:
  - read the persisted indexed file map (same source the sync path's Phase 1
    uses to build `deleted_paths`, `code_graph.rs:2156`);
  - compute `indexed − discovered` (files previously indexed but not discovered
    this run — covers both on-disk deletion **and** newly-excluded-still-on-disk);
  - evict each via `handle_deleted_file(&queries, &rel_path, &stale_file_id)`
    (A3), accumulating `files_reconciled` + orphaned-edge counts into
    `IndexResult`.
- This runs only on the **forced-index/revalidate** route (where the marker
  advances), not on ordinary hash-skip re-index (which must not evict on a
  partial visit). Gate identically to the marker-advance condition
  (`force || !any_hash_skipped`) so a partial index never evicts.
- Ordering: run U2 reconciliation **before** the U1 orphan sweep so that edges
  orphaned by U2's evictions are then swept by U1 in the same pass (A5
  idempotence holds either way, but this keeps a single-pass clean exit).

## Units of work (tasks)

| Task | Unit | Scope | Prio | ≤2h |
|---|---|---|---|---|
| 103.001-T | U1 | `retract_dangling_calls_edges` primitive + wire into both certify blocks + unit/integration test proving orphan rows swept and live edges preserved (A2) | med | yes |
| 103.002-T | U2 | forced-index file-set reconciliation (evict indexed-but-not-discovered before marker advance) + test proving an excluded-still-on-disk file's stale edge is evicted and the marker only certifies post-reconcile | low | yes |

Dependency: **103.002-T depends on 103.001-T** — both edit the same certify
region (`~1900` / `~2923`); serialize to avoid same-file merge conflict, and so
U2's evictions feed U1's sweep in the intended single-pass order.

## Plan hardening (risk-triggered — DB retraction + certify gate)

- **H1** — Retraction blast radius: the `not has_def[from] or not has_def[to]`
  predicate must be verified to match ONLY orphans. Add a test that seeds a live
  edge + an orphan edge and asserts exactly one row removed (A2). Covered by
  103.001-T acceptance.
- **H2** — Marker honesty under partial failure: assert that a forced run whose
  sweep or reconciliation errors keeps the prior marker (A1). Add to 103.001-T.
- **H3** — Recall non-regression: run the existing recall suite (18/18) as a gate
  in each task's DoD; a drop is a hard fail (Ship-executed).
- **H4** — Idempotence: second immediate revalidate sweeps/reconciles 0 and
  re-certifies the same marker value (A5). Add to 103.002-T.
- **H5** — Exclusion vs deletion distinction: the reconciliation must treat a
  newly-excluded on-disk file the same as a deleted file for eviction purposes,
  but MUST NOT evict a file merely hash-skipped as unchanged (which is still
  indexed and valid). Test both branches in 103.002-T.
- **H6** — Cardinality observability: `IndexResult`/`SyncResult` expose the swept
  and reconciled counts; CLI/API response reports them (A6).

## Plan review — GATE: PASS

- Scope is single-width (code-graph reconciliation), each task ≤2h, TDD-ordered,
  fail-closed, recall-guarded. Reuses proven primitives (`handle_deleted_file`,
  `count_dangling_calls_edges` semantics). No CLI/schema/template mixing.
- Risk residue: the global orphan sweep is a full-relation scan; acceptable — it
  runs only on the forced/revalidate route, not on hot incremental syncs. Noted
  for Ship to confirm no perf cliff on large graphs (H1 test uses a small seed;
  Ship should sanity-check on a realistic corpus).
- No unresolved blocking questions. **Cleared for harvest.**

## Definition of done (feature)

- Orphan `calls_edge` rows (no live `function_meta` on either endpoint) are swept
  on every forced-index and `--revalidate-code-graph` run before the generation
  marker advances; zero orphans remain post-run (verified by
  `count_dangling_calls_edges == 0`).
- A previously-indexed, now-excluded (still-on-disk) file's stale same-file direct
  edge is evicted before the marker certifies its generation.
- Marker stays fail-closed on sweep/reconcile failure (prior value retained).
- Recall suite green (no regression); ordered gates (fmt/clippy-pedantic/dev-test/
  audit) green — Ship-executed.
