---
title: "Ship session — rec1 call-edges remediation PR"
type: session-memory
date: 2026-07-11
agent: Ship
shipment: rec1-remediation (follow-up to 078-S / feature 082-F)
branch: fix/rec1-remediation
related: [082-F, 078-S]
---

# rec1 call-edges remediation

Follow-up corrective PR off `main` (post-merge of 078-S PR #241 + closure PR #242)
closing open Copilot review findings from merged PRs #239/#240. The Orchestrator
independently verified the shipped code and confirmed dispositions; this session
executed the two genuine gaps plus documentation and backlog corrections.

## Findings already correct on main (no action — Orchestrator resolved)

- **082.011 export projection** — `edges_from_table` calls_edge branch already
  projects `resolution` (cozo_queries.rs). Verified, no change.
- **082.008 retract** — integration test `reresolution_retracts_now_ambiguous_singleton`
  already exists. Verified, no change.
- **082.013 lock** — migrate.rs already acquires `DaemonLock` and refuses when
  held (exit 2). Verified; only a missing explicit test was added.

## Work done

### CODE FIX 1 — 082.003 durable rollback marker (real shipped bug)

`run_scripts` (schema.rs) invokes the shape-detected `migrate_calls_edge_resolution`
on every `connect_db`, so after `migrate-down` dropped the `resolution` column the
next daemon start re-added it → rollback was non-durable, contradicting 082.013's
fresh-open end-state.

Fix: added a durable `schema_meta { key => value }` marker relation, created in the
bootstrap `scripts` array BEFORE the migrate call. `migrate_calls_edge_resolution`
early-returns when the `calls_resolution_rolled_back` marker is set;
`rollback_calls_edge_resolution` sets the marker (up-front, for crash-safety) so the
drop survives a reopen bootstrap. Idempotent; forward re-enable is out of scope.
Helpers `schema_meta_flag_set` / `set_schema_meta_flag` mirror the graceful
missing-relation handling of `calls_edge_has_resolution`.

- Test (schema.rs unit): `rollback_survives_reopen_bootstrap` — bootstrap →
  rollback → re-run `run_scripts` (reopen) → assert `resolution` column STILL absent.
  Written red-first (confirmed it failed before the marker), then green.
- Ripple: the CLI integration scenario 2 in `cli_calls_resolution_rollback_test.rs`
  formerly relied on the reopen re-adding the column; under durable rollback it now
  asserts the resolution-agnostic edge count is exactly 1 (direct preserved,
  singletons retracted) AND that `count_calls_edges_by_resolution` errors after
  reopen (proving durability through the real CLI path).

### CODE FIX 2 — 082.013 active-daemon refusal test (missing test only)

The lock/refusal already ships. Added integration test
`migrate_down_refuses_while_daemon_active`: the test process holds the workspace
`DaemonLock`, the migrate-down subprocess sees the live-PID lock, returns exit 2
BEFORE `connect_db`, and the pre-seeded singleton edge survives untouched. The
shared-data-dir exclusivity gap stays deferred as stash `5C1EDA41` (not expanded).

### DOC corrections

- `docs/decisions/2026-07-08-...deliberation.md` and
  `docs/exec-plans/2026-07-10-...plan.md`: added/clarified the TARGET-CORRECTNESS
  GATE — every `calls_resolved_singleton` edge must match the fixture manifest
  expected target; `false_edge_rate` (`count_dangling_calls_edges`) only detects
  dangling targets so it is a lower-bound signal. Referenced captured follow-up
  stash `49561F22` (formerly `D07F0919`).
- plan.md §5 and §9: corrected the peer-language follow-on note — only `rust.rs`
  emits `ExtractedEdge::Calls`; `python.rs` / `typescript.rs` / `go_lang.rs` emit
  none, so 082.005/006/007-T need baseline call extraction + caller attribution +
  language-specific member/selector/call node handling + tests, and must be
  re-harvested (likely split), not scoped as a single method-arm.

### BACKLOG corrections

- `.backlogit/queue/082.005-T.md` (Python), `082.006-T.md` (TypeScript),
  `082.007-T.md` (Go): re-scoped to state the true starting point and moved
  `queued → blocked` pending Stage re-harvest. `backlogit sync` rehydrated the
  index (638 artifacts).

## Quality gates

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --no-default-features --features cozo-backend,embeddings
  --all-targets -- -D warnings -D clippy::pedantic` — clean.
- Tests: new schema unit test + 3 CLI integration tests green. Full-suite failures
  (`contract_evaluation::c017_03`, `report_reads_latest_persisted_run`,
  `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`,
  `s072_workspace_status_reports_code_graph_counts`) are all pre-existing:
  the first three pass on serial re-run (parallel CozoDB-lock / timing flakes);
  s072 fails identically on clean `main` (schema.rs stashed) — a local
  environment tree-sitter/embeddings issue, not a regression.
- `cargo audit` — pre-existing transitive-dependency advisories only (no
  Cargo.toml/lock change this session; CI build job has no audit gate).

## Commits

- `1f2e6fa` fix: durable rollback marker (schema.rs) + `rollback_survives_reopen_bootstrap`
  + CLI scenario 2 update + `migrate_down_refuses_while_daemon_active`.
- `35bcabe` docs: target-correctness gate + peer-language follow-on correction.
- `ae52e31` chore: 082.005/006/007-T → blocked, re-scoped.

## Hygiene

Explicit per-file `git add` only; never staged the protected deletions
(`auto-mergeinstall.agent.md`, `auto-tune.agent.md`) or the benign
`.backlogit/stash.jsonl` CRLF touch (empty diff). Branch started from clean main
at `d0a62cc`.
