---
type: session-memory
date: 2026-07-16
agent: ship
session: "2c95481b — Option C Unit A ship"
topic: "087-S Option C Unit A merged; post-merge closure"
---

# Session memory — 087-S Option C Unit A shipped

## Outcome

Shipment **087-S** (Option C Unit A canonical-identity infrastructure) merged to `main` as merge
commit **d5ef75b** via **PR #251** (P-009 merge-commit). Precision-neutral (0 new call edges),
fail-closed (013-D).

## Backlog reconciliation (done this session)

- 091.003-T..091.010-T (A1..A8) -> `done`
- 087-S -> `done` -> archived (`.backlogit/archive/087-S.md`); merge-commit comment appended
- 091-F left `queued` (covering feature releases with Unit B / 088-S, not here)
- 088-* untouched (Unit B; still blocked on 084-S)

## Files modified in the shipped diff (commit c306b69, final)

- `src/db/cozo_queries.rs` — `canonical_paths_for_function_name` projects `id` to defeat Cozo
  set-semantics collapse (F1)
- `src/services/parsing/canonical/module_path.rs` — honour `[workspace].exclude`
  (`read_workspace_exclude_dirs` / `is_excluded_dir`) (F3)
- `src/services/parsing/canonical/use_graph.rs` — softened/scoped `extract_use_graph` docstring (F2)
- `tests/integration/code_graph_test.rs` — `duplicate_canonical_path_rows_are_not_collapsed`
- (earlier commits: 45012bb A8 removal, 29cc414 round-1 fixes, 415ee01 backlog reconcile)

## Key decisions

- **A8 forced re-index removed** (DECISIVE): symbol IDs are random UUIDs
  (`format!("function:{}", Uuid::new_v4())`), so re-parsing content-unchanged files disturbs the
  edge set. `canonical_path` ships additive/opportunistic; ID-preserving backfill -> Unit B.
- **A7 body walk reverted** to preserve the original edge set; only `Self` marker + qualifier
  classification staged.
- **Merged over red non-required CI**: the sole failing check is the embeddings model-download flake
  (`backfill_reports_progress_and_populates_embeddings`), proven environmental — passes locally
  (1.31 s, cached model) and on `main`; `main` is unprotected so `build` is not required. Diff cannot
  touch embeddings. 3 consecutive same-check failures = circuit-breaker limit; merging was the sound
  call vs. stranding the 084-S -> 088-S queue.

## Failed approaches

- Re-running the CI `build` job 3x — all failed identically at the ~30 s HF model-load deadline. Not
  a code problem; do not keep re-running past the breaker.

## Deferred to Unit B (088-S)

nested-`use`/`has_error` fail-closed; alias-shadows-crate; external-crate roots; module-graph rigor
(lib+main, `#[path]`, `#[cfg]`); `ReexportMap` wiring; trait-impl identity; generic specialization.
Unit B requires the mandatory multi-model adversarial panel before edges flip on.

## Known flakes

- embeddings backfill (HF model download; no CI cache) — future hardening chore candidate
- `c017_03_agents_have_required_subfields` (parallel-execution telemetry flake; passes in isolation)

## Next steps

1. Commit + push this closure branch (`chore/087-closure`) to `main` (closure doc + memory + backlog).
2. 084-S next in queue, then 088-S (Unit B, adversarial panel), 083-S, 085-S, 086-S.
3. PR #248 (081-S): candidate to close as superseded by Option C.
