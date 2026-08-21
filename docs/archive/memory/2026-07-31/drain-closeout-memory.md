# Stage session memory — drain closeout (101-S)

- **Date:** 2026-07-31
- **Agent:** Stage (planning/backlog/decision-artifacts only)
- **Branch:** `101-drain-closeout` (base main `f5c955a2`)
- **Mode:** dark-factory (autonomous); backlogit MCP degraded (Transport
  closed), CLI fallback `C:\Tools\backlogit.exe --no-update-check` used
  throughout; read-only sqlite3 SELECT for verification.

## Deliverables

### Part 1 — deferral deliberations (parked, not built)

Two durable decision artifacts authored in `docs/decisions/`:

- `2026-07-31-090.005-cli-mcp-routing-identity-dispatch-registry-deferral-deliberation.md`
  — 090.005-T. Recommendation: DEFER as a future feature (new dispatch-registry
  subsystem, elevated cross-cutting blast radius).
- `2026-07-31-091-canonical-resolver-tail-reexport-single-snapshot-deferral-deliberation.md`
  — combined for 091.019-T and 091.021-T (shared canonical subsystem).
  Recommendations: 091.019-T DEFER as a future feature (recall recovery,
  adversarial review required); 091.021-T DEFER as a future feature (perf and
  robustness, plan-harden plus benchmarking required).

Each task parked to `blocked` with a `references` link to its deliberation, a
comment recording the reason, and a committed body "Deferral" note. Parents:
090-F (archived), 091-F (done).

### Part 2 — refuted task dropped

- 091.017-T archived (queue -> archive) with status `rejected` and a `wont-fix`
  label. Rationale (comment + committed body): speculative, single-reviewer,
  refuted by a second reviewer (gemini-3.1 duck); fail-closed invariant bounds
  blast radius. Provenance preserved; not deleted.

### Part 3 — doc reconciliation (Stage-owned)

- Reconciled `docs/decisions/2026-07-29-stage-harvest-provenance-convention.md`.
  Chosen approach: SOFTEN the convention prose (lower risk) rather than rewrite
  the historical archive row. The Disposition and Convention step 3 now state
  that `backlogit stash archive` (v1.7.0) has no `--reason` flag (verified), so
  the archive stores generic `reason: archived` and the artifact comment plus the
  doc are the authoritative promotion links. A dated Reconciliation note records
  the correction.
- Stash `A85DC0E3` consumed via harvest into 106.001-T; archived with
  `state: harvested`.

### Part 4 — closeout shipment

- Covering feature `106-F` (queued, docs/backlog-hygiene) created.
- `106.001-T` (queued, low, parent 106-F) harvested from `A85DC0E3` to represent
  the doc chore in the manifest; harvest-provenance comment appended.
- Shipment `101-S` (status `queued`) assembled with items `106-F`, `106.001-T`;
  covering_feature projects to 106-F. Left queued for Ship to claim.

## Provenance table

| Source | Disposition | Target | Deliberation |
|---|---|---|---|
| 090.005-T | parked blocked | future feature | 2026-07-31-090.005 deferral deliberation |
| 091.019-T | parked blocked | future feature | 2026-07-31-091 canonical deferral deliberation |
| 091.021-T | parked blocked | future feature | 2026-07-31-091 canonical deferral deliberation |
| 091.017-T | dropped (rejected, wont-fix) | archive | none (refuted) |
| stash A85DC0E3 | harvested | 106.001-T (shipment 101-S) | none |

## Decisions and rationale

- All three tail tasks recommended DEFER-as-future-feature (not
  build-later-as-a-task): each card self-describes as substantial and needs its
  own plan-harden/adversarial-review or benchmarking, so keeping them as `queued`
  low tasks under done/archived parents misrepresented feature-sized work as
  ready cleanup.
- Soften-not-backfill for Part 3: the CLI cannot carry a descriptive
  stash-archive reason, so matching the prose to reality is safer than editing
  history.
- Covering feature 106-F created so the harvested doc task is not orphaned; no
  existing open in-scope feature was available (087-F active is off-limits;
  041-F/088-F blocked; others done/archived).

## Grounding verified (main f5c955a2)

- `src/tools/mod.rs` (should_record_metrics @63, dispatch @249);
  `tests/contract/lint_dax_cli_parity_test.rs` (allowlist @21, dispatch_tool_names
  @147, gap tests @302/@337, help resolver @388); `src/cli/commands/*.rs` (10).
- `src/services/parsing/canonical/reexport.rs` `ReexportMap` unit-tested with no
  production consumer (declared `pub mod reexport`, not re-exported/applied).
- `src/services/code_graph.rs` (unsafe_module_prepass @~88,
  rust_ctx_for_staged_file @~720, reresolve_calls_edges_with_canonical_context
  @~970, function_ids_by_canonical_path @~1035, force_prepass_cache_miss @~1278).

## Guardrails honored

- No production Rust code written or modified; no build/test run; no PR; no push.
- main unchanged at `f5c955a2`; all work on branch `101-drain-closeout`.
- Untouched: active 100-S; blocked 041-F/088-F; RUSTSEC 05EA3D39/99AFF44B; the
  18 parked follow-up stash entries listed in the operator scope.

## Next steps (future cycles)

- Ship claims and closes shipment 101-S (docs/backlog-only).
- Revisit 090.005-T / 091.019-T / 091.021-T only on their named triggers.
- Doctor still reports 43 pre-existing `archived_from_self_ref` findings on old
  archived items (out of this cycle's frozen scope).
