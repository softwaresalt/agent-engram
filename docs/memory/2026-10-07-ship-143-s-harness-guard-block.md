---
title: "143-S halted at unchanged-code archive verifier harness gate"
date: 2026-10-07
shipment: "143-S"
feature: "142-F"
task: "142.060-T"
branch: "feat/143-s-142-f-slot-01-142-060-t-archive-verifier"
status: "blocked-harness-guard"
---

## Scope and disposition

Resumed 143-S on its existing feature branch. Shipment 143-S and its sole task,
142.060-T, were already `active`; Ship did not claim or transition either again.
The operator explicitly authorized harness generation for 142.060-T, and the
Orchestrator clarified that this authorization covers the scenario-1 RED body
and both missing guard tests, but no production changes during Step 2.

The harness-only attempt is **blocked**. The required complete target ran once
after compilation. One of the required guards failed against unchanged
production code. Per the operator's instruction, do not weaken that guard,
rerun the target, commit the harness, or begin implementation without a new
disposition.

## Environment and baseline

- Starting HEAD: `1f096e7d86ffc8c1c9d0a3fe4c97755b4880036b`.
- Branch: `feat/143-s-142-f-slot-01-142-060-t-archive-verifier`.
- Prior commits on the branch:
  - `de1e9d118c90e08dcf58cb3ac2be9603d280528b`
    `chore(settings): ignore Copilot data and backlogit locks` (operator-approved).
  - `e2171786` `chore(backlog): record 143-S claim and intake`.
  - `1f096e7d` `docs(memory): record 143-S harness gate stop`.
- Pre-harness Git blob SHA-1:
  - `scripts/verify-release-archive.py`:
    `d2152acdd5256864d8ba38d7374d6ccb4da06d9c`.
  - `tests/integration/release_archive_smoke_workflow_test.rs`:
    `4f3acf0d2dce94a66f128cc99b44acba19a6e7a1`.
- Engram daemon was healthy and bound to this workspace, but its code graph
  returned no symbols for the Python verifier, so exact-path inspection was
  used. No agent-intercom instruction file is installed.
- Backlog index sync succeeded (`1465` artifacts); hook poll returned no
  events; checkpoint enumeration had no quarantine anomalies or active
  Ship-owned checkpoint.
- P-001 found no active chore or unrelated top-level feature. 142-F is the
  covering feature for this shipment.

## Harness evidence

The only tracked path changed by harness generation is
`tests/integration/release_archive_smoke_workflow_test.rs`: 248 insertions,
below the 400-line SB-1 Step 2 cap. The verifier production script was not
modified. The task's existing sole `harness-ready` label and active status are
unchanged. The harness is not committed.

`cargo check --all-targets` after the harness edit: **PASS**, exit 0 (21.38s).

The first exact target invocation in the default `target/` exited 101 before
test execution because Windows denied removal of the running
`target/debug/engram.exe`. To avoid interfering with the live binaries, the
complete target was then run once using the existing ignored
`CARGO_TARGET_DIR=target-142051`:

```text
cargo test --test integration_release_archive_smoke_workflow
```

Result: exit 101; 16 passed, 3 failed, 0 ignored. No filters were used.

- `archive_verifier_bounds_an_unresponsive_mcp_server`: **PASS**.
- `archive_verifier_keeps_exit_and_stderr_checks_after_reading_first`:
  **FAIL**. It received `MCP stdio process hung after stdin closed` before
  reaching the required exit/stderr assertions. This guard failure on unchanged
  production code is the blocking condition; do not weaken it.
- `archive_verifier_reads_mcp_responses_before_closing_stdin`: **FAIL** with
  marker `RED: F-ARCHIVE-U1 read-before-close`, but the underlying failure was
  `MCP stdio process hung after stdin closed`, not the required
  `non-JSON stdout from MCP stdio`. Its RED evidence is therefore invalid.
- Existing `archive_verifier_runs_the_unpacked_native_binary`: **FAIL** as
  expected with
  `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`.
- All remaining target tests passed.

No full suite, format, lint, local review, PR, CI, or Copilot review was run.
No harness commit, production implementation, PR, or merge exists.

## Resume boundary

Task 142.060-T and shipment 143-S remain active on the existing feature branch.
The official task comment records the authorization and exact harness result.
The current uncommitted harness must be preserved for review; do not transition
the task or shipment, commit, change the route label, or begin production work.
Resume only after operator/Stage disposition of the failed guard and invalid
scenario-1 diagnostic. Any authorized new harness attempt must satisfy both
guards and both required RED outcomes on unchanged production code before the
separate harness commit and before implementation.

The 2.25-hour slot effort limit and third Step-4 build-fix-loop threshold were
not reached. This halt is the explicit unchanged-code guard failure, not an
SB-1 budget stop.
