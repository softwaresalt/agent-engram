---
title: "176-S pre-PR Ship summary"
date: 2026-11-20
shipment_id: 176-S
task_id: 142.089-T
feature_id: 142-F
agent: ship
branch: feat/176-s-142-f-slot-20c-142-089-t-f54c
phase: pre-pr
status: ready-with-conditions
last_code_affecting_head: "8b4f96c5"
reviewed_subject_sha: "8b4f96c5"
---

## Outcome

Ship completed the PA-7 Slot-20c implementation and moved `142.089-T` to
`done`. The implementation is confined to
`tests/contract/read_server_cli_mcp_parity_test.rs`.

## Commits

| Purpose | Commit |
|---|---|
| Harness commit (`HC`) | `951112fd` |
| Harness route label | `5dd5054a` |
| Implementation commit (`IC`) | `06ecddcf` |
| Review fix | `8b4f96c5` |
| P-021 follow-up capture | `8e92d44a` |

## Verified evidence

* Harness resolver accepted `HARNESS_READY ALL_SURFACES_PRESENT 0`
* RED witness compiled and failed on the missing `function_meta` table with
  marker `Worker: 142.089-T`
* Default parity suite passed 8/8
* `cozo-backend,embeddings` parity suite passed 8/8
* Clippy passed for default and CI feature sets
* MCP initialize and tool-catalog validators passed 19/19 and 6/6
* Local review of immutable subject `8b4f96c5` finished with `P0=0, P1=0`,
  followed by `READY_WITH_FOLLOWUPS`; this is not a readiness claim for a
  future PR head
* Pattern retrieval found concrete matching classes; the historical taxonomy
  filename itself was unavailable

## Follow-ups

* `ACC1FDDD`: Windows `contract_verify` relative-target failures
* `8C21E5DA`: current dependency audit remediation
* `B84562A2`: Stage-owned backlog evidence reconciliation
* `2B051681`: downstream full CLI/MCP equivalence coverage

## Conditions and next step

PR CI and Copilot review remain open. The next operation is to push the branch,
create the PR with immutable local-review evidence, wait for CI and Copilot, and
stop at P-014 operator merge approval. No merge was attempted in this run.
