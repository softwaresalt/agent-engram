# Ship 142-S — F50 Concrete Verifier API Block

- **Date:** 2026-09-26
- **Shipment / feature:** `142-S` / `142-F`
- **Target subtasks:** `142.054.002-ST`, `142.054.003-ST`
- **Status:** both remain **ACTIVE**; F50 is not complete.
- **Branch / HEAD:** `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs` / `4995d6818b7e9caa69b8c0912eeef8b786dedc38`
- **Last branch commit:** `4995d6818b7e9caa69b8c0912eeef8b786dedc38` — `test(142.055-T): align prewarm fixture timeout`
- **Ship commits this session:** none.

## Intake and preserved state

- `backlogit` registry/CLI were available; `backlogit sync` succeeded before semantic reads.
- Ship hook poll returned no events or derived signals; no acknowledgement was needed.
- Unfiltered checkpoint listing returned 34 valid summaries, with no anomalies and no active Ship-owned checkpoint. No restore/resolution was needed.
- `autoharness gate pipeline-topology --mode agent --shipment 142-S --phase post_claim --json` passed. The shipment was already active, so no pre-claim check, re-claim, branch creation, or worktree creation was performed.
- The active top-level release-unit check found only `142-F`; no other active feature/chore was found.
- `142-S`, `142.054.002-ST`, and `142.054.003-ST` remain active. No backlog status, manifest, planning field, or label was changed.
- The worktree already contained numerous unrelated dirty and untracked files. This session did not modify source or test files and preserved those entries. One final Ship memory file was added for continuity.

## Harness and API investigation

- The existing F50 harness is `tests/integration/preflight_gate_test.rs`. It tests the typestate chain through a `RecordingVerifier` mock, not concrete Build/Seal/Publish or daemon/health/CLI/MCP probes.
- A delegated F50 harness-preparation attempt stopped before edits because it could not produce a compiling, real-facade RED test within the existing boundary. Its baseline command `cargo test --test integration_preflight_gate -- --nocapture` passed 2/2; this is not evidence that the required concrete behavior exists.
- Public Build/Seal/Publish components do exist individually: `GenerationStore::seal_candidate`, `engram_indexer::run_for_target_with_options`, `SealedInventory`, and `publish_generation_manifest`. However, the inventory type accepts caller-supplied digests; there is no public candidate inventory/digest builder. The digest helpers found in `engram` are private, and `engram-indexer` does not directly depend on `sha2`.
- `GenerationActivator` and the request-entry reconciliation seam exist. They do not provide a composed F50 probe verifier. The public CLI runner invokes a live IPC endpoint; the public MCP `run_shim` binds process stdin/stdout and has no in-process injectable stdio transport. Building the requested real CLI and MCP provenance probes against a controlled in-process workspace would require a new transport/API seam outside the current F50-owned files or a process/CLI boundary.
- The Stage-created launcher-callable preflight command remains a separate deferred scope expansion (`03AA00A8`) with an unreviewed plan. No part of that command/relay contract was implemented or added to F50.
- Given the explicit stop condition for missing facade/API support, no production implementation or partial facade test was introduced. Do not mark F50 complete or proceed to downstream launcher production work until the missing API/ownership boundary is resolved through the appropriate Stage/operator workflow.

## Verification evidence

- `cargo check --all-targets` — **PASS** (root package check).
- `cargo fmt --all -- --check` — **PASS**.
- `cargo test --test integration_preflight_gate -- --nocapture` — **PASS**, 2/2 current mock-based tests.
- `cargo test -p engram-indexer --all-targets` — **PASS**, including the supervisor-boundary integration test.
- `cargo clippy -p engram-indexer --all-targets -- -D warnings -D clippy::pedantic` — **FAIL** on six existing scaffold lints in `crates/engram-indexer/src/preflight.rs`: missing `# Errors` docs on two `Result` methods, `Result<(), ()>`, two missing `#[must_use]` suggestions, and missing error documentation for `Preflight::verify`. No lint fixes were applied.
- `cargo dev-test --no-fail-fast` — **exit 101**, but completed all **269/269** test targets/results. Exactly 12 tests failed; these matched the previously recorded later-task RED map (F51–F54 and 142.060). There were no compiler errors, runner errors, or independent compiler/Cargo warning lines.
- The two literal `WARNING:` lines appear inside the panic payload for `typed_preflight_failure_is_reported_without_starting_copilot`, with marker `RED F51-FAIL-CLOSED` (`tests/contract/start_launcher_test.rs:547`). They are captured legacy launcher stdout/stderr, not rustc/Cargo warnings. No fixture, output, or assertion was edited or suppressed. The other recorded F54 `error: unrecognized subcommand` text is likewise failure content within its mapped RED, not a compiler error.
- F50 cannot receive a `PASS` or `EXPECTED_PENDING_RED` completion verdict: the current targeted harness does not exercise the required concrete behavior, and the indexer clippy gate is not green.

## Closure and next step

- No source/test edit, task completion transition, commit, push, PR operation, merge, or daemon/process stop occurred.
- Preserve the active shipment and tasks. Route the exact inventory-builder and in-process CLI/MCP transport gaps to Stage/operator for an in-scope facade/ownership disposition. Resume F50 test-first only when the owned surface can express the real facade behaviors without implementing the deferred launcher command.
- No merge approval was requested or inferred.
