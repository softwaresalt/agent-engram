---
title: "Ship session — 105-S Windows PID identity stale recovery"
description: >-
  Implementation, validation, review, merge, runtime verification, and closure
  memory for prerequisite shipment 105-S.
date: 2026-08-02
author: ship
shipment: 105-S
feature: 110-F
status: shipped
---

## Completed Work

- Planning PR #309 merged as
  `6076aa033fbcac6f6442581b74726119180887b5`.
- Implementation PR #310 merged as
  `846f3b74bf7292a07634e4fd6e44a388be411666`.
- `110.001-T` added four deterministic PID identity contracts and captured the
  reaped-child RED.
- `110.002-T` replaced the fixed crash delay with exact-child kill/wait and
  captured the stale-recovery `ShutdownTimeout` RED.
- `110.003-T` added the same-System active-state refresh and retained complete
  identity through lifecycle shutdown waiting.
- Review remediation changed the recovery assertion from numeric PID inequality
  to complete PID/start identity inequality.
- Backlogit shipped and archived 105-S, 110-F, and all three tasks with the
  implementation merge SHA.

## Important Commits

| Commit | Purpose |
|---|---|
| `d6bde8de` | Unit 1 PID identity RED contracts |
| `bc1240ff` | Unit 2 exact-child stale-recovery RED |
| `c1b7c1ca` | Unit 3 production GREEN |
| `900a815d` | Full-identity review remediation |
| `846f3b74` | PR #310 merge commit |

## Validation and Review

Targeted and adjacent suites passed. Final ordered gates passed for formatting,
serialized strict clippy, 504 library tests, and the approved per-test-process
hermetic all-target suite. Audit remained at the accepted transitive
`lz4_flex 0.10.0` advisory with no dependency diff.

Standard review passed. Rust specialist review raised one P2 about possible
Windows PID reuse; the test now compares the full identity and rereview passed.
GitHub CI and exact-head Copilot review passed with no unresolved threads.

## Runtime Evidence

The disposable Windows runtime progressed through structured PID 7528, exact
kill/wait recovery to PID 39488, legacy numeric management and upgrade to PID
3472, and malformed metadata without replacement. PID/workspace/pipe health
was green and duplicate-daemon telemetry stayed zero. The final disposable
daemon was stopped.

## Failed or Adjusted Approaches

- `npx` was permanently removed from the validation path after its external
  npm-cache diagnostic write. Repository-local markdownlint was unavailable.
- An offline initial build could not link ONNX Runtime; the normal locked build
  populated only the repository target output and succeeded.
- Setting `TEMP` inside the repository caused the existing no-git fixture to
  inherit the parent repository. The approved hermetic runner therefore
  isolates `ENGRAM_DATA_DIR` only and leaves disposable OS temp fixtures at
  their normal location.
- Optional `--all-features` clippy exposed pre-existing OpenTelemetry API drift;
  no out-of-scope production file was changed.

## Next Step

Merge the 105-S closure PR, clean only
`logs/test-data-hermetic/105-S`, run approved `cargo clean`, verify one clean
core worktree and disk/process gates, then unblock and resume 103-S/108-F by
merging updated `origin/main` into
`feat/108-ordinary-index-fail-closed` without rebasing.
