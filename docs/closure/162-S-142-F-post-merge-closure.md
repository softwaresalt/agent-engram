---
title: "162-S post-merge operational closure"
doc_type: closure
shipment_id: "162-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-29
author: ship
verdict: "READY (preflight documented as local-only CLI; parity allowlist updated) - PR #455 merged and 162-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/162-S-142-F-close-evidence.json"
pr_number: 455
merge_commit: "d2f32f8499481bbd67bff1652bcdf9f11c56ed4f"
head_commit_merged: "938525ec4b14abdda8be2ae77cec8af66f144349"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: ["9B294631"]
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/162-S-pre-20261029T0056Z.md"
post_reconcile: ".backlogit/reconcile/162-S-post-20261029T0105Z.md"
---

## Summary

Shipment `162-S` delivered the single docs-route task `142.074-T` (NEW-6). It adds the row
`engram preflight <WORKSPACE> --timeout-ms <MS>` (column 3 `local`, notes starting "Local dev launcher only") under
`## Canonical mapping` and a matching bullet under `## CLI commands without MCP tools` in `docs/cli-mcp-parity.md`.
It also adds `"preflight"` to `LOCAL_CLI_ALLOWLIST` in `tests/contract/lint_dax_cli_parity_test.rs`. Flags and argument
names (`<WORKSPACE>`, `--timeout-ms`) were checked against `engram preflight --help`. PR #455 merged with a merge
commit (P-009), merge SHA `d2f32f8499481bbd67bff1652bcdf9f11c56ed4f`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used.
* Scope expansion: the first attempt halted because the plan-mandated row failed `contract_lint_dax_cli_parity` and the
  allowlist edit lay outside the task (P-021 C1). Ship captured stash `9B294631` (`DEFERRED SCOPE EXPANSION`). The
  operator then authorized Option A, widening `142.074-T` to the allowlist edit and the `local` column wording; the
  authorization is recorded as a comment on `142.074-T` and in the PR description.
* RED then GREEN: before the allowlist edit `contract_lint_dax_cli_parity` failed 2 of 11
  (`local_cli_only_rows_are_explicitly_allowlisted_with_rationale`, `local_cli_allowlist_matches_documented_local_rows`);
  after it, 11/11 pass. `contract_cli_tool_catalog_parity` passes 6/6.
* Local gates, all PASS: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`,
  `cargo build --all-targets`, `cargo dev-test`. One `hcl_indexing_test` failure on the first `cargo dev-test` run
  passed on rerun and in the next full run; it is attributed to low disk space and was not root-caused.
* The disk filled during verification. Ship removed only the regenerable `target\debug\incremental` cache and built with
  `CARGO_INCREMENTAL=0`. The stale ignored `target-142051` and `target-145` directories were not touched.
* Local multi-persona review (Correctness, Rust, Scope Boundary, Architecture, in parallel, informed of Option A):
  aggregate READY_WITH_FOLLOWUPS, no P0/P1/P2. The single P3 (blank line after the mapping table) was fixed in scope.
* Copilot review on `938525ec`: "Approval recommended", no open issues, zero threads. `build` and
  `start-launcher-windows` passed. `autoharness gate copilot-review 455` returned `SATISFIED: PASS`, re-run before merge.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/162-S-pre-20261029T0056Z.md` was written while
  `.backlogit/queue/162-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 162-S --feature 142-F --sha
  d2f32f8499481bbd67bff1652bcdf9f11c56ed4f --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `VERIFIED`.
  Evidence `docs/closure/evidence/162-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.074-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.074-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 162-S` moved the record to `.backlogit/archive/162-S.md`; then
  `backlogit shipment reconcile-shipped 162-S` sets `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* This closure document was reviewed (Correctness and Scope Boundary personas) and finalized before the
  `reconcile-shipped` call, which attests it as committed with that call; it is not edited afterwards.
* Post-reconcile report: `.backlogit/reconcile/162-S-post-20261029T0105Z.md`.

## 162-S

Delivery: PR #455 (feature), commit d2f32f8499481bbd67bff1652bcdf9f11c56ed4f (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` before `backlogit archive`. This
close instead followed the sequence the operator prescribed for this dark-mode run (the same as 154-S to 161-S):
`backlogit archive 162-S` (`archived_status: active`) and then the official `backlogit shipment reconcile-shipped` with
`--closure-evidence` and `--confirm`, which records a `shipment_reconciled_shipped` event bound to merge SHA
`d2f32f8499481bbd67bff1652bcdf9f11c56ed4f` and ends at `archived_status: shipped`. Approval source: operator
activation contract.

## Runtime verification

Documentation and contract-test change only; no runtime surface changed. The check was documentation against the
binary: `target\debug\engram.exe preflight --help` shows `Usage: engram.exe preflight [OPTIONS] --timeout-ms
<TIMEOUT_MS> <WORKSPACE>`, which agrees with the documented `<WORKSPACE>` and `--timeout-ms <MS>`.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own.

| Evidence | Value |
|---|---|
| SLI | `docs/cli-mcp-parity.md` lists `engram preflight` as a `local` row and the contract guard passes. |
| Observation query | `cargo test --test contract_lint_dax_cli_parity` and `cargo test --test contract_cli_tool_catalog_parity`. |
| Baseline | 11/11 and 6/6 tests pass. |
| Alert threshold | Either test failing on `main`, or the documented `preflight` flags differing from `engram preflight --help`. |
| Owner | Release owner of the next tagged release; Stage owns follow-up triage. |
| Observation window | The first release containing the documented row, plus one CI cycle. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert only the changes from PR #455 (merge `d2f32f8499481bbd67bff1652bcdf9f11c56ed4f`) in `docs/cli-mcp-parity.md` (the table row and the bullet) and `tests/contract/lint_dax_cli_parity_test.rs` (the `"preflight"` allowlist entry); revert both together because the guard requires the allowlist to equal the documented local rows. Do not revert the whole merge: that would also undo the `142.074-T` archival and conflict with the closed `162-S` shipment record. Preserve backlog state and closure evidence. |

Conditions for `READY_WITH_CONDITIONS`: the observation window above completes and Stage triages stash `9B294631`.

## Source artifact cleanup

Shipped item `142.074-T` carried no `source_stash_id` or `source_deliberation_id` custom field (its description cites
lineage stash `03AA00A8`, which is not the manifest field). No stash or deliberation was archived; no
`linked_deliberation_disposition` entry applies. Stash `9B294631`, captured by Ship during the halt, is now satisfied by
the operator-authorized change. Ship may not archive it discretionarily, so Stage should triage and close it.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-29-162-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-29-ship-162-s-dark-factory-halt-memory.md`. Only the just-closed unit was a candidate.
Compaction status: `done`.

## Follow-ups

* No new stash entries. Stash `9B294631` remains open and non-blocking; Stage should triage and close it.
* Stage proceeds with the next 142-F slot.
