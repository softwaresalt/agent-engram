---
title: "175-S post-merge closure"
date: 2026-11-14
shipment_id: "175-S"
feature_id: "142-F"
task_id: "142.088-T"
merge_commit: "f35438efed8a8b0b65271ca4477b681e510de81c"
closure_status: READY
releasability: READY_WITH_CONDITIONS
compaction_status: done
close_path: safe_close
close_evidence: "docs/closure/evidence/175-S-142-F-close-evidence.json"
follow_up_stash: ["4F67A4D7", "6F21E640", "AD1350C2", "B916A619", "2D026946"]
---

## 175-S

Delivery: PR #484 (feature), commit f35438efed8a8b0b65271ca4477b681e510de81c (merge commit, P-009).

Feature merge was confirmed at 2026-11-14T05:50:10Z and verified in
`origin/main` history via `git merge-base --is-ancestor`.
Final reviewed feature HEAD: `da8e1eabd449e4a0f828f1345f426c852d0fdec2`.
HC `9f1322012c54baf7272ab100f6e0baf75a27ad55` and IC `ef3201b5e01ccf42d035e75c25530c1c3bf45c3e`
(plus P0 review-fix IC `cd745e8b4a64ae6fc11a8a2d5519926a0f1089fb` and CI-fix commit
`a8272cf0b48458332c81d2ee353262cb3e975919`) are distinct.
The shipped scope is task `142.088-T` (F54b, Slot-20b), enforcing `ReadServer`
request-entry refusal gating in `src/daemon/request_entry.rs` and Slot-20b
parity/refusal contracts in `tests/contract/read_server_cli_mcp_parity_test.rs`.
Shared covering feature `142-F` remains `active` across Slots 20c-21.

This final canonical `READY` artifact is the immutable pre-transaction
readiness input for `reconcile-shipped`. Shipment `175-S` is still `active`
at attestation time; no shipment archival, `reconcile-shipped` transaction,
INV-12 disposition, post-mode reconciliation, or closure-PR merge outcome is
claimed here. Subsequent transaction and reconciliation outcomes are recorded
in `.backlogit/reconcile/175-S-post-*.md`.

## Verification and release readiness

| Evidence | Recorded outcome |
|---|---|
| Harness and implementation | Distinct HC `9f132201` and IC `ef3201b5`/`cd745e8b`/`a8272cf0` within two-file budget |
| Format, pedantic clippy, and all-targets build | PASS on `a8272cf0` (`cargo fmt`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, `cargo build --all-targets`) |
| Coverage completeness | PASS (281 targets, 14 modules, 0 unmapped) on `cd745e8b` |
| Targeted parity and activation suites | `contract_read_server_cli_mcp_parity` 7/7 PASS (default and `--no-default-features --features cozo-backend,embeddings` on `a8272cf0`); `integration_request_entry_activation` 12/12 PASS, `integration_lifecycle_read_generation_pin` 3/3 PASS, `contract_read_server_dispatch_refusal` 6/6 PASS (`cd745e8b`, unchanged `src/`) |
| Full suite (`cargo dev-test --no-fail-fast`) | 2,618 passed across 280/283 binaries (20 ignored) on dirty tree with uncommitted `start.ps1` edit; `integration_daemon_startup_order` passed 1/1 isolated (`773841E3`); 10 `start_launcher*` failures solely from local `start.ps1` edit |
| Local Adversarial Review (`mode: report-only`, `reviewers: 3`) | `READY_WITH_FOLLOWUPS` (`P0=0, P1=0, P2=0, P3=1` advisory `stop_daemon` 3 s timeout) on `cd745e8b`; current-HEAD delta verified at `da8e1eabd` |
| Hosted GitHub CI on PR #484 | `build` PASS and `start-launcher-windows` PASS on `da8e1eabd` |
| Copilot review `PRR_kwDORJEduc8AAAABX8a_QA` | Clean on `da8e1eabd`, zero unresolved threads, P-018 `SATISFIED: PASS` |
| Last-mile authorization | Unconditional P-018 PASS, unchanged `headRefOid`, P-009 merge commit `f35438ef` |

## Structured validator evidence and invariants

Runtime verdict: `PASS_WITH_FOLLOW_UP`.
Frozen runtime verification report: `docs/closure/2026-11-14-175-s-runtime-verification.md`
(SHA-256 `328E88754B371C5FC2ABECBB405506B1495D75DF317EF8112C8F40015D228BB2`).

| Surface and adapter | Evidence and limit |
|---|---|
| CLI / command / version | Frozen report probe PASS: `engram 0.3.0-rc.1+gcd745e8b-dirty` local validation artifact, not deployed binary |
| CLI / persistent workspace daemon | Host persistent daemon unavailable (`daemon unavailable: Daemon failed to reach Ready state within 30000ms`); live `ReadServer` behavior validated via isolated `ReadServerFixture` probes |
| API / live `ReadServer` IPC, CLI, Stdio MCP | Fresh isolated `ReadServer` daemons accepted `_health` (direct IPC) and `get_retrieval_eval_report`, and refused Write, Control (`_shutdown`), `set_workspace` (`16_003`), mutation workflows, and unknown IPC (`16_001`) with zero side effects (7/7 PASS) |
| API / request-entry activation and dispatch refusal | `integration_request_entry_activation` (12/12 PASS), `integration_lifecycle_read_generation_pin` (3/3 PASS), `contract_read_server_dispatch_refusal` (6/6 PASS) |
| Background-job / sync scope | `ReadServer` refuses background mutation/sync workflows (`16_001`); no live `engram sync` run on host workspace |

Invariants preserved:

* `ReadServer` refuses `_shutdown`, non-Read / non-`read_server_available`
  fall-through methods, and unknown IPC methods at request entry with
  `16_001` (`WriteControlRefused`) and `false` before `tools::dispatch` or
  generation activation.
* `ReadServer` refuses unbound `set_workspace` at request entry with `16_003`
  (`WorkspaceRetargetRefused`) and preserves bound-retarget refusal (`16_003`)
  and same-workspace no-op semantics via `lifecycle::set_workspace`.
* Workspace isolation (`#![forbid(unsafe_code)]`, zero `.unwrap()` /
  `.expect()`, pedantic clippy clean) and `D74F9442` lock-advisory baseline
  remain preserved.
* Protected `142-F.md` SHA-256 verified unchanged:
  `42FCA67C1407641C8F3CF4752B3431D978290B1342614A831DF463B572181770`.

## Safe-close readiness and protected evidence

Ship holds the single-writer lock on `.backlogit/queue/175-S.md` through
post-mode reconciliation.
Exact `pre_report_path`: `.backlogit/reconcile/175-S-pre-20261114T055210Z.md`.
`autoharness shipment cascade-close --classify-only` exited 3, selecting
`SAFE_CLOSE` on `VERIFIED` engine `backlogit 1.12.0` (commit `718e710`, `cli`).
Step 0(d) agreement check compared the pre-mode report's `SAFE_CLOSE` verdict
and empty `qualifying_feature_ids` against the pre-close evidence record and
returned `agreed`.

Command-written pre-close evidence:
`docs/closure/evidence/175-S-142-F-close-evidence.json` (120 observation-set
entries, empty linked-deliberation disposition snapshot). It is never
hand-edited.
Manifest task `142.088-T` (`status: done` at baseline) was explicitly archived
to `.backlogit/archive/142.088-T.md` (`status: archived`, `archived_status: done`),
and all 1,318 non-closure-scope backlog Markdown records (digest
`a35cca34216accbcff1478acfd30b7cf0c5c4b616bca3942344b0de7610983b1`) remained
byte-identical (`DriftCount=0`).
Generic `backlogit move 175-S --status shipped` exited 9 as expected,
routing closure through the governed `reconcile-shipped` sequence.

## Source artifact cleanup and documentation gardening

Registered item reads for `142.088-T` and shared active `142-F` declare no
`source_stash_id` or `source_deliberation_id` fields.

| Scope | source_stash_id | Outcome | source_deliberation_id | Outcome and reason_code |
|---|---|---|---|---|
| Shipped task `142.088-T` | absent | none | absent | none; reason_code absent |
| Shared active feature `142-F` | absent | none | absent | none; reason_code absent |

Because `175-S` is a partial-feature shipment and `142-F` remains active across
Slots 20c-21, no shared covering feature stash or deliberation is archived.
No `docs/ARCHITECTURE.md`, `AGENTS.md`, `docs/research/`, or `docs/compound/`
entry was invalidated or requires update for Slot-20b.
Pre-existing stash entries `4F67A4D7` and `6F21E640` plus Slot-20b P-021 C2
deferred entries `AD1350C2`, `B916A619`, and `2D026946` remain active for Stage
triage without duplicate capture.

## Actual compaction result

P-020 `compact-context` (`target: all`) completed prior to this canonical
finalization (`compaction_status: done`).
One superseded memory file (`docs/memory/2026-11-01/175-s-implementation-blocked.md`,
5,245 bytes, SHA-256 `3FD64D645BE809581257AC1A8FF06F69AD9B30FCA011B3AE2A86C3188258048D`)
was moved byte-identically to `docs/archive/memory/175-s-implementation-blocked.md`.
The latest readiness summary (`docs/memory/2026-11-14/175-s-pre-pr-summary.md`,
5,206 bytes, SHA-256 `D460BAFA1AA71B33DA683BE4B7A1864C4F824D0B9EC9B094ECBEBE8B1A194CB4`)
is preserved unchanged.
Compacted summary written at `docs/memory/compacted/2026-11-14-175-s-compacted.md`
(SHA-256 `ECB7786599D110C51C232872A752FDC9AA03A9A94C508539C95E47AC3B538FFF`).

## Monitoring, rollback, and releasability

Owner: `softwaresalt`. Delivery path: merge-only (no tag, publish, or
deployment performed).

| SLI / signal | Dashboard or query | Baseline | Alert threshold | Owner |
|---|---|---|---|---|
| `main` CI build and Windows launcher pass rate | GitHub Actions `CI` workflow (`build`, `start-launcher-windows`) on `main` | 100% pass on PR #484 (`da8e1eabd`) | Any non-zero exit or test regression on `main` | softwaresalt |
| `ReadServer` request-entry refusal rate and unexpected side effects | `.engram/logs/` IPC error codes (`16_001` / `16_003`) and `engram daemon-status` / `engram workspace-status` | 0 side effects on refused Write/Control/unknown/unbound `set_workspace` calls | Any unexpected `tools::dispatch` fall-through, `ReadServer` state mutation, or `respawn_daemon`/`uninstall` hang | softwaresalt |

Pre-deploy audit: migrations, feature flags, and schema changes are N/A; no
binary deployment occurred; rollback procedure is documented below.
Post-deploy observation window: observe next `main` GitHub Actions CI run to
completion, and observe 30 minutes post-deployment if a binary release is later
separately authorized.

Rollback trigger: confirmed regression in `ReadServer` request admission,
refusal error codes (`16_001` / `16_003`), or unexpected managed-mode fallback.
Rollback procedure: prepare a separately reviewed merge-commit revert
restoring `src/daemon/request_entry.rs` and
`tests/contract/read_server_cli_mcp_parity_test.rs` (commits `9f132201`,
`ef3201b5`, `cd745e8b`, `a8272cf0`) while preserving `142.087-T` and `142-F`
shared state.

| ProposedAction | ActionRisk | Approval and containment | ActionResult |
|---|---|---|---|
| Finalize canonical closure artifact with LF line endings | low | Authorized under Step 6.2 prior to governed `reconcile-shipped` | applied |
| Archive `175-S` and execute `backlogit shipment reconcile-shipped 175-S` | moderate | Governed single-shipment INV-11 transaction under `175-S` lock | planned |
| Deploy new `engram` binary or reconcile `respawn_daemon`/`stop_daemon` | high | Separate operator approval required (`AD1350C2` deferred) | blocked |
| Merge-commit source revert of `142.088-T` if regression confirmed | moderate | Separate operator approval, review, and full validation | planned |

| Required evidence | Disposition |
|---|---|
| Healthy signal | Conditional: PR #484 CI and local suites PASS; next `main` CI and any future deployment window pending |
| Failure signal | Defined above (`16_001`/`16_003` regression, `ReadServer` side effect, `main` CI failure) |
| Monitoring plan | Structured SLI table above (`softwaresalt` manual observation handoff) |
| Rollback trigger and procedure | Defined above |
| Owner and validation window | `softwaresalt`; next `main` CI to completion plus 30-minute post-deploy window |
| Follow-up | Deferred P-021 stash entries `AD1350C2`, `B916A619`, `2D026946`, plus pre-existing `4F67A4D7`, `6F21E640`, `D74F9442`, `773841E3`, and P3 advisory `stop_daemon` 3 s timeout |
