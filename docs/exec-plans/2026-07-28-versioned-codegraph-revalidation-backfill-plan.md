---
title: "Versioned code-graph revalidation / stale-direct-edge backfill (implementation plan)"
type: plan
date: 2026-07-28
source: stash 8DD29746 (Copilot review thread PRRT_kwDORJEduc6UUYBN on PR #291)
stash_id: 8DD29746
status: reviewed
requires_plan_hardening: true
mirrors: "096-F T7 (096.010-T rollout + 096.013-T schema_meta version-marker seam)"
layers_on: "100-F"   # same-file fail-closed extractor upgrade (merged)
relates_to: ["5765BAAB"]
governing_invariant: "013-D no-false-edge / 082-F target-correctness"
tags:
  - code-graph
  - freshness
  - reindex
  - hash-skip
  - schema-meta
  - upgrade-backfill
---

## Problem Frame

engram's content-hash skip keys re-parsing on **file content**, not on the
**extractor generation** (compound landmine
`docs/compound/workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`).
After the **100-F** same-file fail-closed extractor upgrade, an unchanged source
file that already has a **wrong-target direct edge** persisted (minted before
100-F) is **hash-skipped** and keeps that stale wrong edge until a manual
`engram index --force`. 100-F's fail-closed guard only applies to **freshly
re-extracted** files.

The fix (Copilot review thread `PRRT_kwDORJEduc6UUYBN`): add a **versioned
extractor-generation marker** that, on a generation bump, triggers **targeted
re-extraction / re-resolution** of affected `calls` edges so stale wrong edges
are corrected — mirroring **096-F T7** (`schema_meta`
`python_canonical_extraction_version` + gated `run_python_backfill`, advanced
only on full success).

## Normative Anchors

* **A1 (correctness):** a persisted **wrong** same-file direct edge must be
  corrected/dropped after a generation bump (013-D / 082-F). This is the primary
  goal; recall recovery is secondary.
* **A2 (no silent stale state):** a stale generation must be **detectable** and
  actionable without the operator having to know the internal landmine (the
  compound doc's "Prevention" recommendation).
* **A3 (fail-closed marker advance):** the generation marker advances **only on a
  fully-successful pass**; any affected-file error keeps the OLD marker so the
  migration retries on the next run (**C7-3**, mirror T7).
* **A4 (content_hash contract preserved):** the generation marker is a
  **separate `schema_meta` record**, never folded into `file_node.content_hash`
  (mirror the 096.013-T T7-seam).
* **A5 (no steady-state churn):** a **matching** generation is a strict **no-op**
  — no re-extraction, no canonical-edge churn (**C12-5**).
* **A6 (zero canonical-schema migration):** reuse the existing `schema_meta`
  get/set machinery (`python_extraction_version` precedent); additive
  non-migrating DB change only.

## Design

1. **Generation marker.** Add a durable `schema_meta` key
   `code_graph_extraction_generation` with a `CODE_GRAPH_EXTRACTION_GENERATION`
   const, bumped whenever direct-edge extraction semantics change (the 100-F
   fail-closed guard bumps it to gen ≥ 1). Get/set helpers in
   `cozo_queries.rs`, mirroring `python_extraction_version` /
   `set_python_extraction_version`.
2. **Generation-gated revalidation backfill.** On sync/index, compare stored vs
   current generation. When stale, run a gated backfill (opt-in flag, mirroring
   `--backfill-python-canonical`, e.g. `--revalidate-code-graph`): **force
   re-extract affected files** so the 100-F fail-closed guard re-runs and the
   stale wrong direct edges are dropped/corrected; then run the cross-file
   post-pass to re-materialize singletons; **advance the marker only on full
   success** (A3).
3. **Stale-generation detection (A2).** When the stored generation is behind,
   surface a one-line hint (and/or a `doctor` finding) telling the operator to
   run the revalidation — so the correctness gap is never silent.
4. **No-op fast path (A5).** A matching generation short-circuits with zero
   re-extraction.

> **Design fork (locked in review below):** *opt-in gated backfill + automatic
> stale-generation detection* (mirrors T7) — NOT automatic heavy re-extraction
> on every start (churn / C12-5 risk). The immediate-retract-all alternative
> (C8-1 style) was considered and rejected for v1 to avoid a recall cliff; the
> wrong-edge exposure is bounded (same-file duplicate defs are rare) and the hint
> makes it actionable.

## Units (2-hour rule; width-isolated; TDD)

### U1 — Failing revalidation harness (RED)  [domain: tests / code-graph]
* Persist a **wrong** same-file direct edge under an OLD generation; assert that
  a generation-gated revalidation run corrects/drops it (target-identity).
* Assert a **matching** generation run is a strict no-op (no re-extraction, A5).
* Assert `file_node.content_hash` staleness detection is unchanged (A4).
Compiling, initially failing. ~3 scenarios.

### U2 — Generation marker + gated backfill (GREEN)  [domain: code-graph / db-seam]
* Add `code_graph_extraction_generation` `schema_meta` key + get/set helpers
  (mirror `python_extraction_version` and the 096.013-T seam).
* Wire the generation-gated revalidation into the sync path: force re-extract
  affected files → 100-F guard drops stale wrong edges → post-pass
  re-materializes cross-file singletons → advance the marker only when no
  affected-file errors (A3). Add the opt-in flag + stale-generation hint (A2).
* Make U1 green. ~4 functions/edits.

### U3 — Upgrade acceptance + docs  [domain: tests + docs]
* Integration test for the full upgrade scenario: old-generation persisted wrong
  edge → after revalidation the edge is corrected/absent, cross-file singletons
  re-materialized, **no recall regression on unaffected edges**, marker advanced;
  a **partial failure keeps the old marker** and retries (C7-3).
* Docs: operational guidance that **supersedes** the manual-`--force` guidance in
  the 2026-07-20 compound landmine; capability notes for the generation marker.
~3 scenarios.

Dependency order: **U1 → U2 → U3**.

## Plan Hardening (elevated blast radius: DB marker + index/sync freshness contract)

* **H1 — Additive, non-migrating DB change.** The generation marker is a new
  `schema_meta` key reusing existing get/set; **no** `function_meta` / `calls_edge`
  / `content_hash` contract change (A4/A6).
* **H2 — Fail-closed marker semantics.** Marker advances only on a fully-clean
  pass; partial failure retries (A3, C7-3). A crash mid-backfill leaves the OLD
  marker → safe re-run.
* **H3 — No churn on steady state.** Matching generation is a strict no-op (A5,
  C12-5) — no canonical-edge churn on routine sync.
* **H4 — Correctness floor.** After revalidation, zero stale wrong same-file
  edges remain (target-identity gate on the upgrade corpus). Recall on unaffected
  edges must not regress (release blocker).
* **H5 — Rollback.** The backfill is opt-in/gated and additive; rollback = leave
  the marker unset → falls back to today's manual-`--force` behavior. No data
  migration to reverse.
* **H6 — Monitoring.** Report revalidated / dropped / re-materialized edge counts;
  the stale-generation hint surfaces the need to run it.

## Plan Review (gate record)

Personas: freshness/correctness, DB-schema-safety, architecture-cohesion.

* **[P1 — churn] Auto vs opt-in re-extraction.** Automatic heavy re-extraction on
  every daemon start risks churn (C12-5). **Resolved:** opt-in gated backfill
  (mirror T7) + automatic stale-generation **detection/hint**; the immediate
  retract-all alternative rejected for v1 (recall cliff); exposure is bounded and
  the hint makes it actionable (H3/A2).
* **[P1 — marker granularity] Global vs per-file.** **Resolved:** global
  `schema_meta` generation marker mirrors T7; per-file stamping is a heavier
  future option, out of v1 scope.
* **[P2 — DB contract] Do not fold generation into content_hash.** **Resolved:**
  separate `schema_meta` record (A4, 096.013-T precedent).
* **[P2 — retry] Partial-failure marker advance.** **Resolved:** advance only when
  no affected-file errors (A3, C7-3).
* **[P2 — scope] Distinct from 5765BAAB.** This fixes the *sync-path / hash-skip*
  freshness gap; it does **not** address the daemon fresh-workspace non-persist or
  the IPC hang (5765BAAB spike, 015-D). **Resolved:** documented as separate width.

**Gate verdict:** PASS (0 open P0/P1). Ready for harvest.

## Definition of Done

* All 3 tasks done; ordered gates pass (`cargo fmt --all -- --check`;
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`;
  `cargo dev-test`; `cargo audit`).
* Generation marker + gated revalidation backfill + opt-in flag +
  stale-generation hint implemented.
* Upgrade acceptance: an old-generation persisted **wrong** same-file edge is
  corrected/dropped after revalidation; cross-file singletons re-materialized;
  **no recall regression** on unaffected edges; marker advanced.
* No-op on matching generation (A5); `content_hash` staleness preserved (A4);
  partial-failure keeps old marker (A3).
* Docs updated (supersede the 2026-07-20 manual-`--force` operational guidance).
