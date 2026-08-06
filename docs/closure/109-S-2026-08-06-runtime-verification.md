---
title: "Shipment 109-S final JSON cold CLI runtime verification"
doc_type: runtime-verification
date: 2026-08-06
shipment_id: "109-S"
feature_id: "113-F"
surface: cli
adapter: command
verdict: PASS
classification: CORRELATED-COMPLETION
source_revision: "27d3d3a482cc4beae94ce200893d3d11ceaf908d"
compaction_status: pending
---

# Shipment 109-S Final JSON Cold CLI Runtime Verification

## Validator Contract

- Surface: Windows cold real CLI and auto-spawned named-pipe daemon.
- Adapter: the existing ignored
  `windows_live::windows_cold_cli_request_frame_correlation` command test.
- Live allowance: exactly one new-unit attempt, `1/1`; no retry.
- Required chain: one client disposition, one correlated `index_workspace`
  usage record, and one terminal `response_frame_result` with exact ID
  equality.
- Required cleanup: exact owned PID dead, exact named pipe unreachable, and
  exact owned temp workspace absent after harness return.
- Bounds: one-second request timeout, 300,000 ms aggregate limit including a
  60,000 ms cleanup reserve, and a 20,000 ms idle fallback.

Shipment `108-S` remains archived and exhausted at `2/2`. This run is the sole
attempt of the distinct `109-S` release unit.

## Environment Prechecks

| Evidence | Observation |
|---|---|
| Branch | `feat/109-s-final-json-validation` |
| Revision | `27d3d3a482cc4beae94ce200893d3d11ceaf908d` |
| Binary | `C:\Source\GitHub\engram\target\debug\engram.exe` |
| Binary SHA-256 | `81034B210CB2EE833CFC726658E9920B9CB7EC4D9F77BBDBBB66059C9C557B14` |
| Binary last write | `2026-08-06T18:40:30.2915498Z` |
| Platform | Microsoft Windows `10.0.26200`, x64 |
| Repository daemon | PID `2968`, workspace `ad76ab793358e22b431e1c6519f4e68df70e59ab90e15ee06344fd4778b6ead7` |
| Repository daemon treatment | Healthy and observation-only before and after the attempt |
| Before temp inventory | No `tmp/cold-cli-correlation-*` directory existed |

The harness independently required a fresh owned workspace with no PID state
and an unreachable derived named pipe before launching the CLI.

## U1 Deterministic Preflight

The ignored live scenario remained unexecuted during preflight.

| Command or check | Result |
|---|---|
| `cargo test --test cold_cli_request_frame_correlation_test` | PASS — 3 passed, 0 failed, 1 live test ignored |
| Exact cardinality and ID contract | PASS — rejects missing, duplicate, and adjacency-only frame evidence |
| Completed JSON-line parser behavior | PASS — defers a partial UTF-8 tail and rejects completed malformed JSON |
| Owned cleanup contract | PASS — requires exact PID death and endpoint closure |
| `cargo test --lib response_frame_capture_exercises_production_event_and_preserves_id_types` | PASS — 1 passed |
| Request ID | `62046B37-cold-1` |
| Correlation ID | `62046B37` |
| Frozen corpus SHA-256 | `58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25` |
| Capture switch | `ENGRAM_TEST_CAPTURE_AUTOSPAWN_TRACE` |
| Typed frame outcomes in the production branch | `flushed`, `serialize_error`, `write_error`, `flush_error` |

No source or retained-test file changed.

## Sole Live Attempt — New Unit `1/1`

Command:

```text
cargo test --test cold_cli_request_frame_correlation_test windows_live::windows_cold_cli_request_frame_correlation -- --ignored --exact --nocapture
```

| Evidence | Observation |
|---|---|
| Start | `2026-08-06T18:40:36.964Z` |
| Finish | `2026-08-06T18:40:45.169Z` |
| CLI elapsed | 8,205 ms |
| Aggregate elapsed | 8,354 ms of 300,000 ms |
| Client response ID | `62046B37-cold-1` |
| Client disposition | `completion`; exit code `0` |
| Usage record | `index_workspace`, correlation `62046B37`, outcome `success` |
| Terminal frame | response ID `62046B37-cold-1`, outcome `flushed` |
| Corpus SHA-256 | `58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25` |
| Owned PID | `18248` |
| Owned named pipe | `\\.\pipe\engram-bfdee6ac-bd89-4cd4-a96d-e9d433dace83` |
| Harness cleanup | Exact PID dead; exact pipe unreachable |
| Force kill | `false` |
| Test result | PASS — 1 passed, 0 failed |

## Raw Bounded Evidence

The following line is the complete bounded evidence packet emitted by the
existing harness:

```text
COLD_CLI_CORRELATION_RESULT={"aggregate_elapsed_ms":8354,"aggregate_limit_ms":300000,"cleanup":{"endpoint_unreachable":true,"exact_pid_dead":true,"force_kill_used":false,"graceful_shutdown_requested":true,"idle_fallback_ms":"20000"},"cleanup_reserve_ms":60000,"cli_elapsed_ms":8205,"cli_exit_code":0,"cli_finished_at":"2026-08-06T18:40:45.169Z","cli_started_at":"2026-08-06T18:40:36.964Z","cli_stderr":"","cli_stdout":"{\"id\":\"62046B37-cold-1\",\"jsonrpc\":\"2.0\",\"result\":{\"classes_indexed\":0,\"cross_file_edges_dropped\":0,\"dangling_edges_swept\":0,\"duration_ms\":281,\"edges_created\":1,\"embeddings_generated\":1,\"errors\":[],\"files_parsed\":1,\"files_reconciled\":0,\"files_skipped\":0,\"functions_indexed\":1,\"interfaces_indexed\":0,\"oversized_files_skipped\":0,\"same_file_ambiguous_dropped\":0,\"tier1_count\":1,\"tier2_count\":0}}\n","command":"\"C:\\Source\\GitHub\\engram\\target\\debug\\engram.exe\" --workspace \"\\\\?\\C:\\Source\\GitHub\\engram\\tmp\\cold-cli-correlation-ZJxzha\" --json --id 62046B37-cold-1 --correlation-id 62046B37 --timeout 1 index --force","corpus_sha256":"58275c855655b513a682d3e3954d3c55d60d6634300e6e8f17541893aaa00a25","correlation_id":"62046B37","correlation_result":"Ok(CorrelationEvidence { request_id: \"62046B37-cold-1\", correlation_id: \"62046B37\", client_disposition: \"completion\", dispatch_outcome: \"success\", frame_outcome: \"flushed\" })","named_pipe":"\\\\.\\pipe\\engram-bfdee6ac-bd89-4cd4-a96d-e9d433dace83","owned_pid":18248,"request_id":"62046B37-cold-1","run_label":"bounded-cold-cli-characterization"}
```

## Post-Return Cleanup Proof

After the harness returned:

- `Get-Process -Id 18248` found no process: exact owned PID dead.
- A 200 ms connection probe to
  `\\.\pipe\engram-bfdee6ac-bd89-4cd4-a96d-e9d433dace83` failed: exact owned
  pipe unreachable.
- `Test-Path` returned false for
  `\\?\C:\Source\GitHub\engram\tmp\cold-cli-correlation-ZJxzha`: exact owned
  temp path absent.
- The after inventory contained no `tmp/cold-cli-correlation-*` directory.
- Repository daemon PID `2968` remained healthy and bound to the repository.
- No force-kill or destructive cleanup was used.

## Verdict and Prior-Blocker Decision

The exact client request ID, usage correlation ID, and terminal frame response
ID form one complete chain. The client completed, dispatch succeeded, the
terminal frame was `flushed`, and PID/pipe/temp cleanup is complete.

**Verdict: PASS. Classification: `CORRELATED-COMPLETION`.**

This runtime result closes the prior `BLOCKED` classification that existed only
because shipment `108-S` could not capture a JSON-decodable terminal frame.
It does not change or reinterpret `108-S`; that shipment remains archived and
exhausted at `2/2`.

## Operational Closure Handoff

### Invariants and Releasability

- Exact request ID `62046B37-cold-1`, correlation ID `62046B37`, and frozen
  corpus hash remain unchanged.
- Release behavior, timeout semantics, daemon lifecycle, IPC framing, and
  persistence are unchanged.
- No source, schema, configuration, S072, audit, or retained-test work entered
  the shipment.
- Releasability status: **READY** for a documentation/backlog-only merge.

### Monitoring and Validation Window

Ship owns the validation window through PR readiness and operator-approved
merge. Healthy signals are unchanged current-HEAD evidence, successful or
non-required checks, zero unresolved review threads, no requested reviewers,
and `CLEAN` merge state. Any branch advance invalidates exact-HEAD review
readiness and requires re-review.

### Rollback and Failure Signals

There is no runtime rollback because the shipment changes no production or
test behavior. If the record is found inaccurate, correct only the new
documentation and backlog evidence through a reviewed commit. Do not rerun the
live scenario: the new-unit allowance is exhausted at `1/1`.

## Risky Action Record

- ProposedAction: one bounded cold-Windows real-CLI validation.
- ActionRisk: moderate.
- Approval path: reviewed plan authorized attempt `1/1`.
- ActionResult: applied successfully.
- Destructive fallback actions: abandoned; no force termination or preserved
  workspace deletion was needed.

