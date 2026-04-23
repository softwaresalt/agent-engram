---
title: "Process-global metrics store causes concurrent test interference"
description: "metrics::recent_events() is a process-global ring buffer — concurrent tests sharing the same tool name and outcome can match each other's events"
problem_type: "flaky test"
category: "test-failures"
component: "src/services/metrics.rs — recent_events; tests/contract/atomic_policy_snapshot_test.rs"
root_cause: "The in-process metrics writer stores events in a globally shared structure. Concurrent tests using find() with only tool_name + outcome as the predicate can spuriously match events emitted by sibling tests."
resolution_type: "code_fix"
severity: "medium"
message: "test c018_07_denied_metrics_event_carries_agent_role: assertion failed — expected agent_role present, but matched wrong event"
file_path: "tests/contract/atomic_policy_snapshot_test.rs"
citations:
  - "docs/closure/2026-04-23-001-S-toctou-fix-closure.md"
  - "tests/contract/atomic_policy_snapshot_test.rs — c018_07 (line ~321)"
  - "PR #22 — CI run 24846996785 fix commit a29fe7f"
tags:
  - "metrics"
  - "test-isolation"
  - "concurrent-tests"
  - "global-state"
  - "predicate"
---

## Problem

`c018_07_denied_metrics_event_carries_agent_role` failed intermittently under
the `surreal-backend` CI configuration (which runs all contract tests in
parallel). The test verified that a denied `list_symbols` call recorded a
`UsageEvent` carrying the caller's `agent_role`. The find predicate was:

```rust
events.iter().find(|e| e.tool_name == "list_symbols" && e.outcome == "denied")
```

Other tests in the same process also called `list_symbols` with
`outcome == "denied"` but without an `agent_role`. The predicate matched one
of those events instead of the test's own event, causing the assertion that
`agent_role` is present to fail.

## Root Cause

`metrics::recent_events()` returns events from a process-global ring buffer
shared across all tests running in the same process. When multiple contract
tests run concurrently (the default with `cargo test --test-threads N`) and
multiple tests exercise the same tool with the same outcome, a two-field find
predicate (`tool_name + outcome`) is not unique enough to identify the specific
event a given test injected.

## Resolution

Expand the find predicate to include every distinguishing field the test
controls:

```rust
// Before (ambiguous — matches any test's denied list_symbols event)
events.iter().find(|e| e.tool_name == "list_symbols" && e.outcome == "denied")

// After (unique — matches only the event from this test's agent_role)
events.iter().find(|e| {
    e.tool_name == "list_symbols"
        && e.outcome == "denied"
        && e.agent_role.as_deref() == Some("test-agent-role")
})
```

The three-field predicate (`tool_name + outcome + agent_role`) is unique
across all concurrent test invocations because each test sets a distinct
`agent_role` value.

## Prevention

- Any test that searches `metrics::recent_events()` for a specific event must
  include ALL fields that uniquely identify the test's own call in the predicate.
- Never use only `tool_name + outcome` — those two fields are shared by
  multiple tests in the contract suite.
- Prefer including `agent_role`, `workspace_id`, or another per-call
  discriminator in both the test setup and the find predicate.
- Consider adding a unique `correlation_id` or `request_id` to `UsageEvent`
  in the future to eliminate this class of ambiguity entirely.
- This issue only manifests with parallel test execution. It may pass locally
  with `--test-threads 1` but fail in CI where parallelism is higher.
