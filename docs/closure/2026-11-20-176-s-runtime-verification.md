---
title: "176-S runtime verification and pre-merge operational readiness"
date: 2026-11-20
shipment: 176-S
task: 142.089-T
feature: 142-F
mode: pre-merge
surface: auto
verification_status: PASS_WITH_FOLLOW_UP
closure_status: READY_WITH_CONDITIONS
compaction_status: pending
owner: softwaresalt
last_code_affecting_head: "8b4f96c5"
follow_up_stash:
  - "ACC1FDDD"
  - "8C21E5DA"
  - "B84562A2"
  - "2B051681"
conditions:
  - condition: "GitHub Actions CI is green on the PR's final head"
    satisfied: false
    evidence: "PR has not yet been created"
    owner: softwaresalt
    due_utc: "2026-11-20T23:59:59Z"
    attestation_evidence: "All required checks pass on the final head"
    escalation_path: "Hold merge and invoke fix-ci within scope; hand off when the circuit breaker trips"
    backlog_item_id: "N/A"
  - condition: "Copilot review is satisfied for the final head"
    satisfied: false
    evidence: "PR and Copilot review do not yet exist"
    owner: softwaresalt
    due_utc: "2026-11-20T23:59:59Z"
    attestation_evidence: "gsd gate copilot-review status PASS with zero unresolved threads and zero undispositioned body findings"
    escalation_path: "Hold merge until every thread and body finding is dispositioned or explicitly operator-overridden"
    backlog_item_id: "N/A"
  - condition: "Operator grants explicit merge approval"
    satisfied: false
    evidence: "P-014 approval is a separate external merge gate"
    owner: softwaresalt
    due_utc: "2026-11-20T23:59:59Z"
    attestation_evidence: "Explicit operator message approving merge of the final PR head"
    escalation_path: "Stop and preserve the PR; never infer approval from green gates"
    backlog_item_id: "N/A"
  - condition: "Observe the next main CI result after merge"
    satisfied: false
    evidence: "Future main CI is not yet available"
    owner: softwaresalt
    due_utc: "2026-11-21T23:59:59Z"
    attestation_evidence: "Next main CI run is green"
    escalation_path: "Prepare a merge-commit revert if generation-backed parity regresses"
    backlog_item_id: "N/A"
---

## Scope and immutable evidence

This record is pre-merge runtime-verification and operational-readiness evidence
for Shipment `176-S` (`142-F` Slot-20c, Task `142.089-T`). It is not merge
approval, a deployment record, or post-merge canonical closure. The last code
affecting revision is `8b4f96c5`.

The code change is confined to
`tests/contract/read_server_cli_mcp_parity_test.rs`. It replaces the manual
`probe_row` generation database with the PRE-1/PRE-2 candidate generation
pipeline, removes managed `sync_workspace` seeding, resolves the indexed root
ID, and proves generation-backed direct IPC, CLI, and MCP parity inputs.

| Evidence | Result | Provenance |
|---|---|---|
| Harness commit (`HC`) | `951112fd` | RED witness, 98 additions |
| Implementation commit (`IC`) | `06ecddcf` | 51 additions, 66 deletions |
| Review-fix commit | `8b4f96c5` | 49 additions, 20 deletions |
| `cargo check --tests` | PASS | RED harness compiles |
| RED witness | PASS as RED | `Worker: 142.089-T` failed because `function_meta` was absent from the `probe_row` fixture |
| Contract parity, default | PASS (8/8) | Real isolated ReadServer daemon |
| Contract parity, CI features | PASS (8/8) | `cozo-backend,embeddings` |
| `cargo fmt --all -- --check` | PASS | Zero formatting findings |
| Clippy, default and CI features | PASS | `--all-targets -D warnings -D clippy::pedantic` |
| MCP initialize contract | PASS (19/19) | `contract_shim_stdio_initialize` |
| MCP tool catalog contract | PASS (6/6) | `contract_tools_catalog` |
| Line-ending gate | PASS | Zero CRLF, BOM, lone-CR, mixed-newline, or EOF findings |
| Documentation audit | `AUDIT_FOLLOWUPS` | Stage-owned evidence corrections captured in `B84562A2` |

R9.9 evidence was preserved: the drift ledger
`git log --oneline f6f3171f..ce148405 -- src/tools/capabilities.rs` contains
`a13c4245` and `b8721d67`; parked T0 commit `0f2cca20` resolves the historical
B0 record; daemon and CLI/MCP fixture launches strip inherited
`ENGRAM_DATA_DIR`, `ENGRAM_WORKSPACE`, and `ENGRAM_DIRECT`; the task-owned HC and
IC adaptation is below R-A6's 400-line bound.

## Environment prechecks and validator evidence

| Surface and adapter | Manifest mapping | Expected and observed evidence | Outcome |
|---|---|---|---|
| CLI command | `cli-version` | `target\debug\engram.exe --version` printed the current `0.3.0-rc.1` version line and exited 0 | PASS |
| CLI daemon status | `cli-daemon-status` | Persistent host daemon did not reach `Ready` within 30 seconds; isolated fixture daemons completed successfully | BLOCKED prerequisite for host-daemon probe; isolated runtime path PASS |
| API command | `mcp-initialize-handshake` | Stdio initialize contract passed 19/19 | PASS |
| API command | `mcp-tool-invocation` | Tool catalog and full parity contracts passed 6/6 and 8/8 | PASS |
| API command | indexed generation | Real generation `generation-142` published at revision 1; statistics reported one file and two functions; symbols and `query_graph` resolved the indexed root | PASS |
| Background job | `workspace-sync` | No managed indexing is created; the test asserts `.engram/cozo` remains absent, so mutation/sync is intentionally unavailable for this read-server fixture | N/A for test-only read-server scope |

## Invariants and pre-deploy audit

* The published generation is revision 1 and carries `GenerationBackedRead`
  provenance for every asserted read.
* The managed `.engram/cozo` tree remains absent, preventing false proof from
  managed state.
* `indexed_symbol_id` requires exactly one indexed symbol row.
* `map_code`, `query_graph`, direct IPC, CLI, and Stdio MCP arguments use the
  indexed generation's real root identity.
* No `src/` file or `tests/helpers/activation_settle.rs` changed, and no
  suppression attribute was added.
* The local `cargo dev-test` run is not green on this Windows host because
  pre-existing `contract_verify` cases cannot resolve relative fixture files
  and exit 2; this is captured in `ACC1FDDD`.
* `cargo audit` is installed and reports 19 dependency vulnerabilities; the
  dependency upgrade scope is captured in `8C21E5DA`.
* `cargo build --all-targets` could not complete because the host had 232 MB
  free and the linker reported `LNK1180` insufficient disk space. No
  production source changed, and focused build, test, and Clippy gates passed.

## Monitoring and rollback handoff

| Requirement | Plan and threshold | Owner |
|---|---|---|
| Healthy signal | PR CI and the next `main` CI pass; isolated generation-backed parity remains 8/8 on both feature sets | softwaresalt |
| Failure signal | Any generation parity, MCP handshake, catalog, provenance, or activation regression | softwaresalt |
| Monitoring plan | Observe GitHub Actions PR checks, the next `main` CI run, and the first authorized release smoke check | softwaresalt |
| Rollback trigger | A confirmed false generation-backed read, managed-state contamination, or indexed root-resolution regression | softwaresalt |
| Rollback procedure | Prepare a reviewed merge-commit revert of `06ecddcf` and `8b4f96c5`, preserving HC `951112fd` and predecessor slots | softwaresalt |
| Validation window | PR CI, next `main` CI, and 30 minutes after any separately authorized deployment | softwaresalt |

## Risky action record

| Proposed action | ActionRisk | Approval and rollback boundary | ActionResult |
|---|---|---|---|
| Publish and activate a real indexed fixture generation in isolated temporary workspaces | moderate | Authorized by `142.089-T`; contained to the contract test process and temp fixture | applied and verified |
| Merge the feature branch | high | Explicit P-014 approval required after CI and Copilot gates pass; merge commit only | blocked pending operator approval |
| Deploy or release a new binary | high | Separate operator approval; no deployment performed | not started |

## Releasability

Required PR CI, Copilot review, and operator approval remain open conditions;
the first two are automated gates, while P-014 approval is recorded in the
frontmatter as a separate external merge gate.
The test-only runtime contract is `PASS_WITH_FOLLOW_UP`; pre-merge
releasability is `READY_WITH_CONDITIONS`, with the follow-up IDs
`ACC1FDDD`, `8C21E5DA`, `B84562A2`, and `2B051681`.
