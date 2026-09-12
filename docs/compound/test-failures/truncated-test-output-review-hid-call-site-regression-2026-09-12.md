---
title: "cargo dev-test passing does not prove a control-flow fix is safe for all call sites"
description: "A ReadServer-mode set_workspace refusal fix passed cargo check, clippy, fmt, and a cargo dev-test run, but broke daemon startup in production. The regression was in a test that IS part of cargo dev-test's target set (dev-test = \"test --all-targets\", verified by direct reproduction) -- the escape was in how the prior run's output was reviewed, not in which tests were selected."
problem_type: "regression-missed-during-output-review"
category: "test-failures"
component: "src/tools/lifecycle.rs, src/daemon/startup_activation.rs"
root_cause: "set_workspace_with_probe is called from two production entry points: the trusted startup bind in run_startup_driver (src/daemon/startup_activation.rs) and the public MCP handler reached via dispatch() (src/tools/mod.rs). A fix that changes control flow for the 'no dispatch context admitted yet' case is correct for the public-handler call site but fatal for the startup call site, which is guaranteed to hit that exact case as its first bind. CORRECTION (post-Copilot-review, verified independently): the original version of this document claimed cargo dev-test's default target set excluded the end-to-end read_server_mode_survives_auto_spawn_and_bounded_restart test. That claim is FALSE and has been retracted -- .cargo/config.toml defines `dev-test = \"test --all-targets\"`, the test is registered as a normal [[test]] target in Cargo.toml with no #[ignore], and `cargo test --all-targets --no-run` confirms its binary is built and would be run by cargo dev-test. Direct reproduction on the reverted-fix commit shows this test genuinely FAILS there and PASSES on either side of it, so cargo dev-test as actually invoked in this session's earlier verification pass either did include a failure that was not seen, or the specific invocation used did not match the full alias. Given `cargo test --all-targets` interleaves output from many parallel test binaries and only the individual `test result: FAILED` markers (not a single trailing summary) indicate failure, the most defensible explanation is a review-process gap: only a truncated tail of a very long multi-binary run's output was inspected (e.g. `Select-Object -Last 80` in PowerShell), and this binary's failure lines were not within that tail -- not that the test was excluded from the run."
resolution_type: "workaround"
severity: "high"
message: "EngramError::Activation(ActivationError::GenerationNotYetActivated)"
file_path: "src/tools/lifecycle.rs"
citations:
  - "PR #393 (shipment 139-S, task 142.036-T)"
  - "commit f54ed718 (the incorrect fix)"
  - "commit 8bfb790e (an earlier commit in the same PR that had deliberately made this exact case fall through, for the same startup reason)"
  - "commit 3086969b (the revert)"
  - "stash F0A2A478 (the real underlying gap, deferred per P-021 C1)"
tags:
  - "dark-mode-execution"
  - "copilot-review"
  - "regression-verification"
  - "read-server-mode"
  - "p-021"
---

## Problem

A GitHub Copilot hosted code review on PR #393 flagged that `set_workspace_with_probe` in
`src/tools/lifecycle.rs` fell through to the full write-capable workspace-bind path whenever
`DaemonMode::ReadServer` had no dispatch context admitted yet
(`state.snapshot_dispatch_context().await` returned `None`). The suggested fix — refuse the
`None` case — was implemented, and passed every standard quality gate: `cargo check
--all-targets`, `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, `cargo fmt
--all -- --check`, and the full `cargo dev-test` suite (702 passed, 1 known-flaky failure
unrelated). A new regression test covering the refusal was added and passed.

Despite all gates passing, this fix was a critical regression: it made every
`read_server`-mode daemon fail to start immediately.

## Root Cause

`set_workspace_with_probe` has two distinct production callers:

1. `run_startup_driver` (`src/daemon/startup_activation.rs`) — the trusted daemon-startup
   entry point, used for every daemon mode including `ReadServer`. It calls this function as
   the very *first* bind, at a point where nothing has yet populated `AppState`'s active
   workspace — so `snapshot_dispatch_context()` is **guaranteed** to return `None` here.
2. The public MCP `set_workspace` handler, reached via `dispatch()` in `src/tools/mod.rs` —
   where a `None` case genuinely should be refused, because a caller invoking this mid-flight
   during the startup race window should not be able to trigger write-capable workspace setup.

Refusing unconditionally on `None` satisfies caller (2) but breaks caller (1), because the
function cannot currently tell which caller it is being invoked from — no trust signal is
threaded through either call site. An earlier commit in the very same PR (`8bfb790e`,
message: "allow initial read-server startup bind ... skip same-workspace-only no-op logic
when no workspace is yet active ... restore read-server startup and restart readiness
coverage") had already hit this exact problem and deliberately chosen the fall-through
behavior for this reason. The round-3 Copilot suggestion silently reintroduced the bug
`8bfb790e` had fixed.

**Correction to the original diagnosis (verified during Copilot round-4 review of this very
document, PR #393)**: this document originally claimed the regression was invisible because
`cargo dev-test`'s default target set excluded
`read_server_mode_survives_auto_spawn_and_bounded_restart`
(`tests/integration/read_server_restart_test.rs`). That claim was checked and is **false**:

* `.cargo/config.toml` defines `dev-test = "test --all-targets"` — no target exclusion.
* `tests/integration/read_server_restart_test.rs` is registered as a normal test target in
  `Cargo.toml` and the function carries no `#[ignore]`.
* `cargo test --all-targets --no-run` confirms the `integration_read_server_restart` binary is
  built, and running it in isolation reproduces the regression cleanly: it fails at the fix
  commit (`f54ed718`) and passes at the commits on either side of it (`679500ce` before,
  `3086969b` after the revert).

So the test genuinely is part of `cargo dev-test`'s scope, and would have failed had its
output been observed. The actual, more defensible explanation: `cargo test --all-targets`
compiles and runs dozens of test binaries with interleaved/parallel output, and failures are
marked per-binary (`test result: FAILED. ... — see error: test failed, to rerun pass
--test <name>`), not collected into one trailing summary. During this session's original
verification of the fix, only a truncated tail of that output was inspected (a
`Select-Object -Last 80`-style view), and this expensive, ~70+-second end-to-end test's
`FAILED` marker was very likely not within the reviewed tail. The escape was in **how the
prior run's output was reviewed**, not in **which tests were selected to run**.

## Resolution

1. An independent scoped `code-review` agent run against the diff introducing the fix (not
   just the file-level unit tests) caught the regression by reasoning about all call sites of
   the changed function, then independently ran the specific end-to-end test to confirm:
   failed at the fix commit, passed at the pre-fix commit.
2. Reverted the fix and its accompanying (incorrectly-passing) unit test via `git revert`.
3. Re-verified `cargo check`, `clippy --pedantic`, `fmt --check`, the file-level test suite,
   and — critically — re-ran the specific end-to-end daemon-restart test to confirm the
   revert restored passing behavior.
4. Captured the real underlying gap (production `ReadServer` mode has no trust-boundary
   separation between a trusted startup bind and the public `set_workspace` handler) as a
   distinct P-021 deferred-scope stash entry, since a correct fix requires threading a trust
   signal from un-owned files (`src/daemon/startup_activation.rs` and/or
   `src/tools/mod.rs::dispatch()`) into the owned file — out of scope for a shipment whose
   owned-files set is limited to `src/tools/{read,lifecycle,eval,lint,doctor}.rs`.

## Prevention

* When a fix changes control flow in a function called from more than one production entry
  point, **grep for all call sites of the changed function** before considering the fix
  verified — do not rely solely on the file's own harness tests.
* **Never review a truncated tail of a large multi-binary test run's output as proof of a
  full pass.** `cargo test --all-targets` / `cargo dev-test` interleaves parallel output from
  dozens of binaries; a `FAILED` marker for an expensive, slow-running binary (e.g. a
  real-daemon-spawn end-to-end test) can appear well before the end of the stream. Either
  capture the complete output to a file and grep it for `FAILED`/`test result:` lines, or rely
  on the process exit code (non-zero means at least one failure occurred somewhere in the
  run) rather than eyeballing a tail view.
* Do not treat "the standard quality-gate sequence (check/clippy/fmt/dev-test) passed" as
  sufficient proof of safety for a control-flow change in shared code unless the complete
  output was actually inspected (or the exit code checked) for every binary in the run,
  including expensive end-to-end tests.
* A hosted/automated review suggestion (e.g., GitHub Copilot) can be locally correct about a
  narrow code path while being wrong about the fix's safety across all callers of that path —
  treat suggested fixes with the same call-site verification rigor as self-authored fixes,
  especially in shared/dispatch-adjacent code.
* When a fix reverts behavior introduced by an earlier commit in the same branch/PR, always
  read that earlier commit's message and diff first — it likely encodes a lesson already
  learned once.
* Institutional-knowledge documents (like this one) are themselves subject to review and can
  contain incorrect root-cause claims — verify a compound doc's factual claims (e.g. "test X
  is excluded from Y") independently before trusting them, exactly as Copilot's round-4 review
  did here.
