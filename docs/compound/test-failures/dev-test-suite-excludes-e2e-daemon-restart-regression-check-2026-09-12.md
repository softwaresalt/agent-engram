---
title: "cargo dev-test passing does not prove a control-flow fix is safe for all call sites"
description: "A ReadServer-mode set_workspace refusal fix passed cargo check, clippy, fmt, and the full cargo dev-test suite, but broke daemon startup in production because the default test target set excludes the end-to-end daemon-restart integration test that exercises the changed function's other call site."
problem_type: "regression-not-caught-by-default-test-suite"
category: "test-failures"
component: "src/tools/lifecycle.rs, src/daemon/startup_activation.rs"
root_cause: "set_workspace_with_probe is called from two production entry points: the trusted startup bind in run_startup_driver (src/daemon/startup_activation.rs) and the public MCP handler reached via dispatch() (src/tools/mod.rs). A fix that changes control flow for the 'no dispatch context admitted yet' case is correct for the public-handler call site but fatal for the startup call site, which is guaranteed to hit that exact case as its first bind. cargo dev-test's default target set did not include the end-to-end read_server_mode_survives_auto_spawn_and_bounded_restart test (a real daemon process spawn), so the regression was invisible to the standard quality-gate sequence (cargo check, clippy --pedantic, fmt --check, cargo dev-test)."
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

The regression was invisible to the full standard quality-gate sequence because
`cargo dev-test`'s default target set did not exercise
`read_server_mode_survives_auto_spawn_and_bounded_restart`
(`tests/integration/read_server_restart_test.rs`), an end-to-end test that spawns a real
daemon process in `read_server` mode and asserts it starts and survives a restart. This test
is expensive (spawns a real process, ~70+ seconds) and is evidently excluded from (or not
covered by) the default `cargo dev-test` invocation used during the build-feature/quality-gate
loop.

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
  verified — do not rely solely on the file's own harness tests or the default full-suite
  target set.
* Do not treat "the standard quality-gate sequence (check/clippy/fmt/dev-test) passed" as
  sufficient proof of safety for a control-flow change in shared code. If the codebase has
  expensive end-to-end tests that are excluded from the default fast test loop (real daemon
  spawns, restart scenarios, etc.), explicitly identify and run the ones covering any
  call site of the changed function.
* A hosted/automated review suggestion (e.g., GitHub Copilot) can be locally correct about a
  narrow code path while being wrong about the fix's safety across all callers of that path —
  treat suggested fixes with the same call-site verification rigor as self-authored fixes,
  especially in shared/dispatch-adjacent code.
* When a fix reverts behavior introduced by an earlier commit in the same branch/PR, always
  read that earlier commit's message and diff first — it likely encodes a lesson already
  learned once.
