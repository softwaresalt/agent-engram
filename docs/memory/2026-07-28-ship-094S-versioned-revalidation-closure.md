---
title: "Ship 094-S — versioned code-graph revalidation / stale-direct-edge backfill (101-F / 8DD29746) merge closure"
date: "2026-07-28"
type: "ship-closure-memory"
feature: "101-F"
shipment: "094-S"
pr: 293
merge_commit: "bb968963a522ee5ef438dd7ed73f1f52016d165a"
status: "shipped"
---

# Ship 094-S — merge closure memory

## Outcome

Feature 101-F shipped via **PR #293** (merge commit
`bb968963a522ee5ef438dd7ed73f1f52016d165a`, merge-commit strategy per P-009 —
2 parents `c9d19487` ∪ `13f07f91`). Adds a durable
`code_graph_extraction_generation` marker (a `schema_meta` record) plus an
**opt-in `--revalidate-code-graph` gated backfill** so the 100-F same-file
fail-closed guard can reach WRONG same-file `direct` edges that were persisted
*before* 100-F shipped (100-F only guards freshly-indexed files; hash-skip
leaves already-persisted edges stale until a forced revalidation). Mirrors the
proven 096-F T7 `python_extraction_version` versioned-marker pattern.

## Git base decision

`main` was protected/unpushable, so the feature branch
`101-versioned-codegraph-revalidation-backfill` was based on the local Stage
cycle-2 planning commit `11a69318` (planning artifacts for BOTH 101-F and
102-F appeared in the PR base — expected; the merge therefore carried the
102-F planning onto `origin/main`, keeping 093-S's future PR base clean).
`start.ps1`'s unrelated uncommitted modification was never touched or committed.

## Per-task outcomes (TDD honored)

- **101.001-T (U1 RED)** `00de40bf` — failing generation-gated revalidation
  harness proving hash-skip leaves already-persisted WRONG direct edges stale
  after the 100-F fix (RED, then GREEN under U2).
- **101.002-T (U2 GREEN)** `18cb5155` — `CODE_GRAPH_EXTRACTION_GENERATION`
  const + marker read/advance, `run_codegraph_revalidation` gating, force
  re-extraction over unchanged bytes on a generation bump, fail-closed marker
  advance (only on a fully-clean pass). U1 → GREEN.
- **101.003-T (U3)** `34b6cc4f` — upgrade/backfill acceptance suite (mirrors
  096-F T7 acceptance shape) + docs (architecture.md, compound landmine
  update).

## Copilot review remediation (3 cycles — circuit-breaker limit reached)

The review surfaced real defects across three cycles; all fixes stayed additive
and fail-closed. Every actionable thread was replied-to (referencing the fixing
SHA) and resolved via `resolveReviewThread` (9/9 resolved at merge).

- **Cycle 1 (8 actionable):** 7 fixed + 1 declined (Stage-owned base-diff
  provenance). Commits `8e075d21`, `86fc66ee`, `7a7492a0`. Highlights:
  dangling-direct-edge retraction on SYNC teardown; zero-byte-bypass
  fail-closed marker; daemon queued-sync `revalidate` flag preservation; test
  `<dangling:...>` sentinel (`assert_no_dangling_edges`) so a stale raw row is
  DETECTED not masked as `("","")`.
- **Cycle 2 (0 new actionable; 4 of 6 suppressed were real):** forced full-index
  H4 direct-edge retraction (`8580d77d`); pending-sync publish-order race +
  `backfill_python` coalescing preservation (`77d5c757`); doc wording
  (`3b47e38e`).
- **Cycle 3 (1 new actionable + re-raised suppressed):** centralized
  same-file direct-edge retraction inside `handle_deleted_file` so the
  deletion/oversized-eviction teardown no longer leaves a dangling row
  (`e2e7802a`, + `deleted_file_retracts_stale_direct_edge_no_dangling_row`
  scenario); churn-free doc scoping (`13f07f91`). The remaining findings
  (orphan-row sweep, forced-index deletion reconciliation, daemon pending-sync
  drain hardening) were accepted as backlog items at the 3-cycle limit and are
  stashed.

## Key finding (hard-won)

The direct-edge retraction query
(`retract_direct_calls_edges_for_file`) joins `calls_edge.from` to
`function_meta.id` to attribute the file, so it can only retract edges whose
caller metadata is still live. This is CORRECT for H4's user-facing guarantee:
every teardown path retracts direct edges **before** `delete_functions_by_file`
runs, and graph queries (`map_code`/`impact_analysis`) join BOTH endpoints to
`function_meta` — so an orphaned raw row (caller metadata already re-minted by a
pre-101-F ordinary sync) is **non-traversable** and can never surface a wrong
same-file target. H4 = "no wrong *query answers*" holds; purging legacy
orphaned raw rows is a separate one-time GC (stashed `685FAA80`). This
"traversable-edge vs raw-row" distinction is the crux of the remediation and is
graduated to a best-practices compound entry.

## Gates + review + runtime

- fmt PASS; clippy `-D warnings -D clippy::pedantic` (CI feature set
  `cozo-backend,embeddings`) PASS; tests PASS — revalidation 3/3, acceptance
  5/5 (incl. forced-index + deletion scenarios), code_graph 33/33,
  sync-deletion 1/1, same-file-shadowing 4/4 (100-F invariants preserved),
  resilience 5/5, write 7/7; `cargo audit` = 10 pre-existing transitive
  advisories only (Cargo.lock unchanged; 093-S scope).
- CI `build` PASSED on final HEAD `13f07f91` (4m48s). An earlier run failed on
  `backlog_index_100_items_under_5_seconds` — a wall-clock timing flake in an
  untouched subsystem (11.9s under CI load); re-run/next-push green. `build` is
  not a required check.
- Copilot merge gate FULLY GREEN, re-verified immediately before merge:
  latest review `commit_id == 13f07f91 == HEAD`, Copilot off
  `requested_reviewers`, 0 unresolved threads (9/9), `mergeStateStatus ==
  CLEAN`. No fresh review landed past HEAD (circuit-breaker guard clear).
- Runtime: `index`/`sync --help` expose `--revalidate-code-graph` +
  `--backfill-python-canonical` with correct incremental-vs-forced routing;
  end-to-end index/sync/revalidation exercised against real CozoDB by the
  5-scenario acceptance suite. `--direct` mode avoids the pre-existing daemon
  cross-file-singleton hang (stash `5765BAAB`).

## Closure actions

- Shipment 094-S → **shipped**; archived scope: 101.001-T, 101.002-T,
  101.003-T, 101-F, 094-S. Merge SHA recorded.
- Reconcile post-snapshot written (`094-S-post-20260728-143556`,
  recommendation PROCEED; P-007 clean).
- 4 follow-up items stashed: `685FAA80` (orphan direct-edge sweep, task/med),
  `92EE75BB` (forced-index deletion reconciliation, task/low), `BE366218`
  (daemon pending-sync drain hardening, bug/med), `D2416925` (Stage
  harvest-provenance reconciliation, task/low).
- Best-practices compound entry graduated:
  `versioned-schema-marker-gated-revalidation-backfill-2026-07-28`.

## Process learnings for next ship

- The **versioned-marker gated backfill** pattern is now proven twice (096-F
  `python_extraction_version`, 101-F `code_graph_extraction_generation`) — it is
  a reusable recipe (durable `schema_meta` marker + opt-in gate + fail-closed
  advance). Graduated to best-practices.
- When retro-fixing persisted-edge correctness, retract stale edges in **every**
  teardown path (modified re-index, forced index, file deletion, oversized
  eviction) and always **before** deleting the keying metadata — centralize the
  retraction in the shared `handle_deleted_file` helper so no call site is
  missed.
- Distinguish the *user-facing* invariant (no wrong query answers) from
  *raw-row* hygiene when triaging late review findings; the former is the
  release gate, the latter can be a scoped follow-up. This kept cycle 3 inside
  the circuit-breaker budget.
- Closure docs land via a dedicated closure PR (established convention);
  implementation PR stays implementation-only.
