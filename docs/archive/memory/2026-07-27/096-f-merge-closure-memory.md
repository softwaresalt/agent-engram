---
type: session-memory
date: 2026-07-27
feature: 096-F
shipment: 091-S
branch: feat/py-namespace-canonical-resolution
pr: 288
merge_commit: c1f34ae31866ef99b2dedd47b39d97307055b081
status: merged
---

# 096-F / 091-S — Merge & Post-Merge Closure

## Outcome

PR #288 (feature 096-F, shipment 091-S: Python module-namespace-qualified call
resolution) **merged to `main` as a merge commit** `c1f34ae3` (P-009 satisfied;
squash/rebase disabled at repo level). Feature 096-F and shipment 091-S moved to
`done` (auto-archived queue→archive); merge commit tracked against both.

## Review cycles (circuit breaker: 3 max)

- **Cycle 1** — first Copilot review (3 comments): C1 competing same-name imports
  = real P0 false-edge, FIXED `aeb97697`; C2/C3 deferred to backlog (099.007-T,
  099.002-T) with rationale.
- **Cycle 2** — second Copilot review (4 comments A/B/C/D): A/B/C = real
  fail-closed false-edge gaps, FIXED with TDD `d3742fa6`; D (backfill intent)
  = liveness, deferred to 099.007-T (MCP `sync_workspace` path; the CLI path
  was already fixed in 82488eae).
- **Cycle 3** — Copilot review at HEAD `d3742fa6` (COMMENTED, non-blocking):
  **no new actionable threads** → did NOT spiral; 4-point merge gate clean → merged.

## The three cycle-2 fixes (all defend the NON-NEGOTIABLE zero-false-edge invariant)

- **A — callee-module export rebind**: `bar.py: def parse(); parse = None`; caller
  `import bar; bar.parse()`. Module-arm resolver holds the CALLER's shadow, not
  bar.py's, so it returned `Ok("bar.parse")`. Fix: at bar.py's identity-mint site,
  `PythonShadowIndex::module_export_rebound` (order-aware) suppresses canonical
  identity to `""` → excluded from `canonical_index` → module-arm Ok path misses →
  `_ => continue` (no name-only fallback on the Ok path) → no edge.
- **B — nested-function `global` rebind**: `def outer(): def mutate(): global bar;
  bar = factory()`. `scan_scope` stopped at nested defs. Fix:
  `collect_nested_dynamic_rebinds` recurses nested fn bodies → `dynamic_rebinds` →
  `Err(CompetingBindings)` (no fallback).
- **C — class-body `global` rebind**: same recursive pass walks class bodies.

## Files modified this closure session

- `src/services/code_graph.rs` — `python_rebound` param (173,180),
  `module_export_rebound` (355), both mint sites (1462-1472, 2330-2340).
- `src/services/parsing/python_canonical/bindings.rs` —
  `collect_nested_dynamic_rebinds` (234,248-267).
- `tests/integration/calls_recall_acceptance_test.rs` — 3 RED→GREEN A/B/C tests.
- `.backlogit/` — 096-F, 091-S → done (queue→archive); merge commit tracked.

## Gates

fmt / clippy pedantic / `cargo dev-test` (459 lib + all bins) GREEN; 13 python
acceptance tests + 18 unit_python_canonical GREEN. CI `build` green at HEAD.

## Deferred (backlog follow-ups, NOT false edges)

- **099.007-T** (high) — MCP `sync_workspace` backfill-intent liveness
  (write.rs pending_sync coalescing): a backfill request queued during active
  indexing drains as an ungated sync, so the canonical re-extraction never runs.
  The CLI path (indexing.rs:56/93, `sync --full` / `index`) was already fixed in
  82488eae and is NOT deferred.
- **099.002-T** — extraction-version marker advance semantics.

## Memories stored

1. Duplicate competing same-name imports fail closed (DuplicateSameNameImport)
   even with one external dep — uniqueness backstop alone misses it (`aeb97697`).
2. Callee-side module export rebinds + nested-fn/class-body `global` rebinds fail
   closed via `module_export_rebound` + `collect_nested_dynamic_rebinds` (`d3742fa6`).

## Next steps

- Push closure branch (`.backlogit` archive moves + this memory doc) → closure PR.
- `start.ps1` unrelated local edit MUST stay uncommitted (kept out of all commits).
- 099-F follow-ups (099.001-007-T) remain queued for a future shipment.
