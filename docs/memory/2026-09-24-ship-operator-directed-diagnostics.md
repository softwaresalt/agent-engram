---
title: "Ship Operator-Directed Diagnostic Tests"
date: 2026-09-24
scope: "isolated diagnostic tests; no shipment/task-loop resumption"
status: "complete"
---

# Ship Operator-Directed Diagnostic Tests

## Scope and Safety

* Ran only the three operator-requested test cases; no source or test files were edited.
* Did not select, restore, prune, or resolve the operator-identified Ship checkpoint.
* Did not claim a shipment, transition a task, edit the 142-S manifest, change branches, commit, push, or create a PR.
* The initial working tree already contained extensive unrelated dirty and untracked files. Preserve them.
* The backlog index was synchronized before backlog lookups; 1,402 artifacts were indexed.

## Test Evidence

* The initial benchmark invocation used the wrong Cargo target name (`backlog_hydration_test`) and exited 101 with `no test target named`; no test ran.
* Corrected command: `cargo test --features cozo-backend --test integration_backlog_hydration backlog_index_100_items_under_5_seconds -- --exact --nocapture`
  * Exit code: 0.
  * Test result: 1 passed, 0 failed; test runner elapsed 4.04 seconds.
  * The test still has its existing five-second assertion. On success it does not print the measured ingestion duration, so 4.04 seconds is the test-runner duration, **not** the ingestion benchmark measurement. Do not report it as the measured ingestion time.
* Command: `cargo test --test integration_daemon_startup_order run_with_shutdown_v2_exits_cleanly_on_ttl_expiry -- --exact --nocapture`
  * Exit code: 0.
  * Test passed; test runner elapsed 10.91 seconds.
* Command: `cargo test --test integration_release_archive_smoke_workflow archive_verifier_runs_the_unpacked_native_binary -- --exact --nocapture`
  * Exit code: 101.
  * Test failed; test runner elapsed 5.12 seconds.
  * Reported failure: `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio:` followed by a JSON-RPC response beginning `{"jsonrpc":"2.0","id":2,"result":{"tools":[...]}}`. No remediation was attempted.

## Scope Disposition and Residual Risk

* Changing the hydration test from a five-second pass/fail assertion to a non-gating benchmark is outside `142.054-T` / `142-F` / `142-S` scope under P-021 C1; F50's authorized contract is the typed preflight state machine and its preflight harness.
* Captured the request for Stage deliberation as stash entry `41DB6AD0` (task / provisional medium). Candidate `1346BC60` concerned the same test and limit, but did not positively confirm the requested benchmark semantics; it was not reused. Archived-stash enumeration was unavailable through registered backlogit query surfaces, so the capture records both discovery fail-safe statuses.
* Added the stash ID and carry-forward instruction to the task-level comment on `142.054-T`. Cite `41DB6AD0` in any future PR/closure residual-risk record; no PR or closure record exists in this diagnostic session.
* This entry is a run-level record only. Do not convert the deferred request into a source change or mutate the active shipment manifest without Stage disposition.
