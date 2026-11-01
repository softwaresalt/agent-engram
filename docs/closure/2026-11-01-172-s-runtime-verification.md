---
title: "172-S runtime verification and pre-merge handoff"
date: 2026-11-01
shipment_id: "172-S"
feature_id: "142-F"
task_id: "142.085-T"
verdict: PASS_WITH_FOLLOW_UP
releasability: READY_WITH_CONDITIONS
follow_up_stash: ["D74F9442", "56CCC9F0", "773841E3"]
---

## Validator contract and environment

Invoked runtime-verification with surface auto, mode auto, and the
validator_manifest and validation_expectations in
`.autoharness/workspace-profile.yaml`. Invoked operational-closure in
pre-merge mode with this evidence and the profile releasability contract.
This slot changes the API read adapter and admission, not deployment or
background indexing. Command adapters and real generation fixtures are used.
No manual checkpoint is declared in the manifest.

The tested artifact is IC 65b155f7f5d716c97260d8a89b7312cd24235ebe,
built with CARGO_TARGET_DIR=target/validation. The existing installed binary
reports engram 0.3.0-rc.1+ge043299. Daemon PID 47916 is reachable in
read_server mode and bound to this workspace on main. Its health is yellow:
no active revision is installed. It is not the newly deployed IC runtime.
No daemon restart, binding, indexing, flush or binary replacement occurred.

## Structured validator evidence

| Surface and adapter | Probe and expected signal | Observed outcome |
|---|---|---|
| API, command fixtures | Pinned query_changes uses opened generation and provenance | Three GREEN runs in both default and git-graph builds, 3/3 each |
| API, command fixtures | Generation differs from unindexed managed state; no stray DB | Real candidate generation scenarios pass under git-graph |
| CLI and admission, command fixtures | Generation wiring preserves readiness and dispatch | Three GREEN runs, 13/13 each |
| Adjacent API, command fixtures | Report pinning and read-path containment | Default 2/2, git-graph 3/3 report tests; 4/4 pinning contract |
| API, full-suite fixtures | MCP initialize and tool contracts remain compatible | Included in completed full suite; no failure in these targets |
| CLI, read-only commands | Version, daemon reachability and workspace binding | Exit 0 for version, daemon-status and workspace-status |
| Background, fixture coverage | Existing synchronization and generation lifecycle contracts | Full-suite fixture evidence, not a new live sync/indexing invocation |

Required invariants remain: owned generation data and provenance, validation
before generation DB access, unchanged managed path, MCP schemas and errors,
workspace containment, tool-managed .engram state and safe Rust.
No missing fixture prerequisite or new handshake/index corruption failure
was observed. Live deployment observation remains a release condition.

## Quality evidence and residuals

Initial cargo check, format, default all-targets pedantic clippy,
Stage-scoped git-graph clippy, cargo build --all-targets and final cargo build
passed. Completeness oracle passed: 281 targets, zero unmapped targets or
modules. All Cargo commands used target/validation.

Exactly one `cargo dev-test --no-fail-fast` ran with default parallelism:
2619 passed, one failed, 20 ignored, exit 101. The unchanged
`t046_s050_daemon_exits_after_idle_timeout_and_restarts` passed unchanged in
isolation. Operator disposition 773841E3 accepts this as Steps 4.3 and 5
PASS. No second full suite or single-threaded suite ran.

Cargo audit returned eight vulnerabilities and 16 allowed warnings against
unchanged Cargo.lock. D74F9442 accepts that baseline risk; audit is not
represented as a clean pass. 56CCC9F0 tracks the unchanged feature-build
report-test lint excluded by Stage amendment 85e0ffa0. No out-of-scope fix
was made and no new deferred entry was created.

## Monitoring, rollback and ownership

Owner: softwaresalt. Rollout is merge-only; no tag, publish or deployment is
required for this partial-feature slot. Use the log observation guidance in
`docs/log-observation-guide.md` and the next main CI run.
Validation window: 30 minutes after a separately authorized deployment.

Healthy signals are correct provenance and results, no stray generation
sidecars, stable admission and successful MCP handshake. A reproducible
provenance mismatch, managed-store substitution, sidecar creation, handshake
regression, restart loop or unrecoverable sync error triggers intervention.
Halt rollout and prepare a reviewed source/test-only revert of this slot;
obtain explicit approval before rollback. Deployment-level mitigation may
reinstall the prior release asset and regenerate state only with separate
approval. Never flush or restart the existing daemon to manufacture PASS.

Releasability READY_WITH_CONDITIONS records baseline audit risk and pending
separately authorized deployment observation. Merge-only fixture validation
and the monitoring/rollback/owner handoff are satisfied. Completed live
deployment observation is not claimed.

## Safety

ProposedAction: bounded context-aware read dispatch implementation.
ActionRisk: moderate. Scope is exactly the three declared production files.
ActionResult: applied and verified. Tests, DB implementation, shared parent
and unrelated runtime ownership are untouched.
