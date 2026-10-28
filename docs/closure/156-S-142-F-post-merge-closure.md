---
title: "156-S post-merge operational closure"
doc_type: closure
shipment_id: "156-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (library stages only; no runtime surface consumes them yet) - PR #443 merged and 156-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/156-S-142-F-close-evidence.json"
pr_number: 443
merge_commit: "f11590bf0fd2609baf4a358a96fee919b506e70f"
head_commit_merged: "c6df8dc48fad7c8d5b40757c4f6bf2169df0a491"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash:
  - "D58542C8"
  - "170800D5"
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/156-S-pre-20261028T1628Z.md"
post_reconcile: ".backlogit/reconcile/156-S-post-20261028T1632Z.md"
---

## Summary

Shipment `156-S` delivered the single task `142.054-T` (F50c), the last slot of the F50 family. `ProductionVerifier` in
`crates/engram-indexer/src/preflight.rs` now implements the four probe stages: `DaemonVerified`
(`wait_for_generation(PREFLIGHT_READ, expected)`, the EXPECTED generation, not any generation), `HealthVerified`
(CLI health read accepted), `CliProbeVerified` (CLI stats read accepted with generation == expected; the data
fingerprint is stored) and `McpProbeVerified` (stdio MCP read accepted with generation == expected and fingerprint equal
to the CLI fingerprint; a missing CLI fingerprint fails closed). Verification is observational only (R48): no
generation-control endpoint is called. Real-chain cases drive `ProductionVerifier` with `env!("CARGO_BIN_EXE_engram")`:
a spaced read-server git fixture reaches `Succeeded`; a managed-mode fixture and a daemon serving an older revision fail
at `DaemonVerified`. PR #443 merged with a merge commit (P-009) at head `c6df8dc4`, merge SHA
`f11590bf0fd2609baf4a358a96fee919b506e70f`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not authorized and not used (`DARK_MODE_MERGE_AUTHORIZED`).
* Claim: predecessors 155-S and 153-S were already `archived_status: shipped`; `pipeline-topology` pre_claim and
  post_claim exited 0 and `backlogit shipment claim 156-S` succeeded natively.
* Harness first (`682faecb`): the real-chain success and manifest-untouched cases failed RED at the unimplemented probe
  stages while all F50a and F50b tests stayed GREEN; the implementation (`67ff2de5`) turned them GREEN. The two
  negative real-chain cases are vacuously RED against a fail-closed stub by design.
* Local gates on the final HEAD: `cargo test --test integration_preflight_gate` (24/24), `cargo test -p engram-indexer
  --all-targets --no-fail-fast`, both pedantic clippy runs, `cargo fmt --all -- --check`, `cargo build -p engram-indexer`,
  `cargo build --all-targets` and `cargo dev-test --no-fail-fast` PASS. An earlier dev-test run hit
  `contract_atomic_policy_snapshot` metrics lifecycle lock timeouts under load; it passed alone and in the full rerun.
  PR `build` and `start-launcher-windows` jobs passed on the merged head.
* Local multi-persona review (Correctness, Rust, Security, Architecture in parallel): READY_WITH_FOLLOWUPS, no P0/P1.
  Fixed in `929ffbd6`: Mismatch mapped to the deadline-expired kind, finite daemon idle timeout and fail-fast readiness in
  tests, error-kind assertions, out-of-order test, doc and state reset. Out of scope (P-021 C1, captured before any
  reply): stash `D58542C8` (probe children inherit ENGRAM_* env; `src/preflight_probe.rs`) and `170800D5`
  (`Preflight::verify` discards verifier reasons; F50a). Deferred-entry discovery was not performed before capturing
  `170800D5`, which duplicates `1AB175A8` from 155-S; per the single-write invariant it was not edited, and Stage
  deduplicates it. Residual risk: no request-log assertion that no control endpoint is called (the manifest-unchanged
  assertion and the structural fact that only the three probe facades are used cover R48).
* Copilot review `5685456527` raised one thread (coverage for mismatched or missing MCP fingerprints), in scope for
  this task's own stage and fixed in `c6df8dc4` with a compiled fake-engram fixture; replied, resolved, and
  `Copilot-Review-Body-Disposition: 5685456527` posted. Re-review returned no open issues.
  `autoharness gate copilot-review 443` returned `SATISFIED: PASS` before and at merge.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/156-S-pre-20261028T1628Z.md` (generated 16:28:10Z) was written while
  `.backlogit/queue/156-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 156-S --feature 142-F --sha
  f11590bf0fd2609baf4a358a96fee919b506e70f --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `UNVERIFIED`.
  Evidence `docs/closure/evidence/156-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.054-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.054-T.md` (`backlogit move --status done` and `backlogit archive`). Subtasks
  `142.054.001-ST` to `.003-ST` were not touched (Stage closes them per plan 10.1).
* `backlogit archive 156-S` moved the record to `.backlogit/archive/156-S.md`; then
  `backlogit shipment reconcile-shipped 156-S` set `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* Post-reconcile report: `.backlogit/reconcile/156-S-post-20261028T1632Z.md`.

## 156-S

Delivery: PR #443 (feature), commit f11590bf0fd2609baf4a358a96fee919b506e70f (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` and verifies live `status: shipped` before `backlogit archive`. This close instead followed the sequence the operator prescribed in the 156-S dark-mode activation contract (the same sequence approved and merged for 154-S and 155-S): `backlogit archive 156-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with `--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA `f11590bf0fd2609baf4a358a96fee919b506e70f` and ends at `archived_status: shipped`. Approval source: operator activation contract (`merge_approval_pre_authorized: true`, explicit closure procedure). Aligning the skill text with the `reconcile-shipped` path is a Stage concern.

## Runtime verification

Not applicable. The change adds library stages that no binary or launcher consumes yet, so no runtime surface changed.
The real-chain integration cases run against a real daemon and the real `engram` executable in CI and locally.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The handoff applies to the first release whose
launcher preflight consumes `ProductionVerifier` (142.069-T).

| Evidence | Value |
|---|---|
| SLI | Fraction of preflight runs that reach `Succeeded` only when the daemon serves the expected generation and CLI and MCP reads agree, and that otherwise fail typed within the shared deadline at the correct stage. |
| Observation query | Preflight verdict output of the launcher once wired (142.069-T), and `cargo test --test integration_preflight_gate` in CI. |
| Baseline | 24/24 tests pass; a managed-mode daemon and a daemon serving an older revision never verify; MCP fingerprint mismatch or absence fails at `McpProbeVerified`. Known gaps: stashes `D58542C8` (inherited ENGRAM_* env) and `170800D5`/`1AB175A8` (verifier reasons dropped by the machine). |
| Alert threshold | Any run reporting `Succeeded` while the daemon serves a generation other than the expected one, any `integration_preflight_gate` failure on `main`, or sustained `DaemonVerified` deadline failures on read-server workspaces. |
| Owner | Release owner of the next tagged release; Stage owns triage of the stash entries above. |
| Observation window | The first release that consumes the preflight, plus one daemon restart. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Today the verifier is unconsumed, so reverting merge commit `f11590bf0fd2609baf4a358a96fee919b506e70f` removes only the probe stages and their tests. Once a launcher consumes the complete `ProductionVerifier`, roll back the whole consuming release (or revert the consumer together with this merge) and rebuild. |

## Source artifact cleanup

Shipped item `142.054-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or deliberation was
archived; no `linked_deliberation_disposition` entry applies to this close.

## Compaction (P-020)

`compact-context` was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-28-156-s-compacted.md` and the original placed in `docs/archive/memory/`. Only the
just-closed unit was a candidate. Compaction status: `done`.

## Follow-ups

* Stash `D58542C8` (provisional medium): probe children in `src/preflight_probe.rs` should remove ENGRAM_DIRECT,
  ENGRAM_DATA_DIR and ENGRAM_WORKSPACE from the child environment.
* Stash `170800D5` (provisional low, requires deliberation): `Preflight::verify` discards `VerificationError` reasons;
  duplicates stash `1AB175A8`.
* Stage: close subtasks `142.054.001-ST` to `.003-ST` (plan 10.1); F50 is complete.
