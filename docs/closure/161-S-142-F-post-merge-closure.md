---
title: "161-S post-merge operational closure"
doc_type: closure
shipment_id: "161-S"
feature_id: "142-F"
mode: post-merge
date: 2026-10-28
author: ship
verdict: "READY (hidden engram preflight subcommand wired to the relay) - PR #453 merged and 161-S safe-closed (non-cascading; 142-F unchanged)"
closure_status: "READY"
releasability: "READY_WITH_CONDITIONS"
compaction_status: "done"
close_path: "safe_close"
close_evidence: "docs/closure/evidence/161-S-142-F-close-evidence.json"
pr_number: 453
merge_commit: "7cb7ad6c0b663b78c3a57f344583cafc88e24e4a"
head_commit_merged: "ed4493202a1da8a9c195fa9743053a65c03a25bf"
closure_pr_number: null
closure_pr_merge_commit: null
runtime_verification_report: null
follow_up_stash: []
blocking_stash: null
shipment_record_status: "archived (archived_status: shipped) via backlogit shipment reconcile-shipped"
pre_reconcile: ".backlogit/reconcile/161-S-pre-20261028T2338Z.md"
post_reconcile: ".backlogit/reconcile/161-S-post-20261028T2350Z.md"
---

## Summary

Shipment `161-S` delivered the single task `142.073-T` (NEW-5): the hidden `Preflight` subcommand in
`src/bin/engram.rs` (`#[command(hide = true)] Preflight { #[arg(id = "preflight_workspace", value_name = "WORKSPACE")]
workspace: PathBuf, #[arg(long)] timeout_ms: u64 }`), dispatching to
`std::process::exit(preflight::run_preflight(&workspace, timeout_ms))`. The command ignores the global `--workspace`,
`--timeout`, `--json`, `--format`, `--quiet` flags and `ENGRAM_WORKSPACE`. The contract test
`contract_cli_preflight_command` (3 scenarios) and a `Cli::command().debug_assert()` unit test guard it. PR #453 merged
with a merge commit (P-009), merge SHA `7cb7ad6c0b663b78c3a57f344583cafc88e24e4a`.

## Dark-mode decisions and gates

* `DARK_MODE_ACTIVE`; merge pre-authorized, admin fallback not used.
* Step 2 harness: 192 insertions (SB-1(c) cap 400). Scenarios 1 and 2 failed RED against the `Worker: 142.073-T`
  placeholder; scenario 3 and `debug_assert` passed as characterization. After the Step 4 dispatch swap all three
  scenarios and both `cargo test --bin engram` tests are GREEN; `unit_cli_parser`, `integration_cli_e2e` and
  `contract_cli_tool_catalog_parity` stayed GREEN unchanged.
* Local gates, all PASS: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`,
  `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic`, `cargo build --workspace`,
  `cargo build --all-targets`, `cargo test -p engram-indexer --all-targets --no-fail-fast`, `cargo dev-test`.
* The first PR CI `build` failed on Linux with `ETXTBSY` ("Text file busy") when spawning the freshly copied binary.
  It was an in-scope defect in the new test (P-021 C1) and was fixed with a bounded retry in the test helper at
  `ed449320`; `build` and `start-launcher-windows` then passed on the merged head.
* Local multi-persona review (Correctness, Rust, Security, Architecture in parallel): aggregate READY_WITH_FOLLOWUPS, no
  P0/P1. The stale test header comment was fixed in scope; a delta review of the retry fix returned READY. The only
  out-of-scope observation (a parity-doc row for the hidden command) is already owned by `142.074-T`, so no P-021 stash
  entry was created.
* Copilot review: two reviews (`e72e67b7`, `ed449320`), both "no open issues", zero threads.
  `autoharness gate copilot-review 453` returned `SATISFIED: PASS` for the merged head.

## Safe-close and archive evidence

* Pre-reconcile report `.backlogit/reconcile/161-S-pre-20261028T2338Z.md` was written while
  `.backlogit/queue/161-S.md` was still `active` in the queue.
* `autoharness shipment cascade-close --classify-only --shipment 161-S --feature 142-F --sha
  7cb7ad6c0b663b78c3a57f344583cafc88e24e4a --json` exited 3: `SAFE_CLOSE`, no mutation, engine verdict `VERIFIED`.
  Evidence `docs/closure/evidence/161-S-142-F-close-evidence.json` was written after the pre-report.
* `.backlogit/archive/142.073-T.md` carries `status: archived`, `archived_status: done`,
  `archived_from: .backlogit/queue/142.073-T.md` (`backlogit move --status done` and `backlogit archive`).
* `backlogit archive 161-S` moved the record to `.backlogit/archive/161-S.md`; then
  `backlogit shipment reconcile-shipped 161-S` sets `archived_status: shipped`. `backlogit shipment ship` was not
  invoked and no file was deleted by hand.
* Covering feature `142-F` is `status: active` with SHA-256
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770` before and after close.
* The `shipment_reconciled_shipped` event attests this closure document as committed with the `reconcile-shipped`
  call; it is reviewed before that call so the attested hash matches the final content.
* Post-reconcile report: `.backlogit/reconcile/161-S-post-20261028T2350Z.md`.

## 161-S

Delivery: PR #453 (feature), commit 7cb7ad6c0b663b78c3a57f344583cafc88e24e4a (merge commit, P-009).

## Procedure deviation record (operator-directed)

The shipment-reconcile skill's non-cascading sequence moves the shipment to `shipped` and verifies live `status: shipped`
before `backlogit archive`. This close instead followed the sequence the operator prescribed in the 161-S dark-mode
instructions (the same sequence used for 154-S to 160-S): `backlogit archive 161-S` (`archived_status: active`) and then
the official `backlogit shipment reconcile-shipped` with `--closure-evidence` and `--confirm`, which records a
`shipment_reconciled_shipped` event bound to merge SHA `7cb7ad6c0b663b78c3a57f344583cafc88e24e4a` and ends at
`archived_status: shipped`. Approval source: operator activation contract.

## Runtime verification

Runtime closure probe on the pre-merge head (`cargo build --workspace`, then the debug binary against an empty scratch
directory):

```text
target\debug\engram.exe preflight <scratch> --timeout-ms 0
stdout: {"state":"Failed","stage":"Build"}
exit:   1
stderr: engram-preflight stage=Build cause=setup elapsed_ms=19 engram-preflight-relay stage=Build cause=ok elapsed_ms=97
scratch directory entries before and after: 0 / 0 (unchanged)
```

The command is hidden, dev-launcher-only, and excluded from the agent archive and installer.

## Monitoring and rollback handoff

This slice is not a release and is not tagged or published on its own. The handoff applies to the first release that
contains the hidden command; the launcher consumer is a later slot.

| Evidence | Value |
|---|---|
| SLI | `engram preflight <WORKSPACE> --timeout-ms N` prints exactly one verdict line, exits 0 only for `Succeeded` and otherwise 1, and the command stays absent from top-level `engram --help`. |
| Observation query | `cargo test --test contract_cli_preflight_command`, `cargo test --bin engram`, and `target\debug\engram.exe preflight <scratch> --timeout-ms 0`. |
| Baseline | 3/3 contract tests and 2/2 binary tests pass; the zero-deadline probe prints `{"state":"Failed","stage":"Build"}` with exit 1. |
| Alert threshold | Any of those tests failing on `main`, a clap id collision, or `preflight` appearing in top-level help. |
| Owner | Release owner of the next tagged release; Stage owns follow-up triage. |
| Observation window | The first release containing the command, plus one daemon restart. |
| Rollback trigger | Any alert-threshold condition that persists after one retry. |
| Rollback procedure | Revert only the code from PR #453 (merge `7cb7ad6c0b663b78c3a57f344583cafc88e24e4a`): remove the `Preflight` variant, its dispatch arm and the `preflight` import in `src/bin/engram.rs`, the `cli_definition_has_no_clap_id_collision` test, the `[[test]] name = "contract_cli_preflight_command"` entry in `Cargo.toml`, and `tests/contract/cli_preflight_command_test.rs`. Rebuild with `cargo build --all-targets`. Do not revert the whole merge: that would also undo the `142.073-T` archival and conflict with the closed `161-S` shipment record. Preserve backlog state and closure evidence. |

## Source artifact cleanup

Shipped item `142.073-T` carried no `source_stash_id` or `source_deliberation_id`. No stash or deliberation was
archived; no `linked_deliberation_disposition` entry applies to this close.

## Compaction (P-020)

Compaction was applied with `target: all`: the shipment memory was consolidated into
`docs/memory/compacted/2026-10-28-161-s-compacted.md` and the original placed in
`docs/archive/memory/2026-10-28-ship-161-s-slot-14-memory.md`. Only the just-closed unit was a candidate.
Compaction status: `done`.

## Follow-ups

* No new stash entries. The `docs/cli-mcp-parity.md` row for the hidden command is already scoped to `142.074-T`.
* Stage proceeds with `142.074-T` (NEW-6, Slot-15).
