---
title: "Preflight fallible post-pass context before mutating last-known-good graph state"
doc_type: learning
source: "103-S / 108-F / PR #312"
description: "A graph post-pass must load every source context that can fail before retracting or replacing prior edges. Otherwise a read race can erase the last-known-good graph and still bypass retry certification."
problem_type: "logic_error"
category: "best-practices"
component: "src/services/code_graph.rs"
root_cause: "Canonical post-pass source contexts were loaded lazily after bare-name resolution and canonical-edge retraction had begun. A staged source that became unreadable could therefore return early after mutating prior graph state and after the canonical workspace snapshot had been cleared."
resolution_type: "code_fix"
severity: "high"
message: "preflight_fallible_postpass_context_before_mutation"
file_path: "src/services/code_graph.rs"
date: "2026-08-02"
feature: "108-F"
shipment: "103-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/312"
  - "commits af6b89d9 and 9714dc14"
  - "tests/integration/code_graph_test.rs"
tags:
  - "indexing"
  - "fail-closed"
  - "canonical-resolution"
  - "retry-state"
  - "error-propagation"
  - "108-F"
  - "103-S"
---

## Failure Shape

The ordinary-index file loop intentionally retains prior staged calls and
edges when a source read fails. The later canonical post-pass previously
loaded Python and Rust source context lazily, after it had begun re-resolving
bare calls and retracting canonical edges. If the retained staged source was
now unreadable, the pass could fail after destroying its caller's
last-known-good edge. Because ordinary indexing clears its topology snapshot
before discovery, the same early return also bypassed snapshot restoration.

## Rule

> Load every fallible input needed by a mutating post-pass before its first
> mutation. Preserve legitimate "unresolvable" results as data, but propagate
> actual I/O or database failures.

For staged calls this means:

1. list the retained staged sources;
2. preflight and cache Python and Rust source contexts;
3. return on any source-read failure before bare-name or canonical-edge
   mutation;
4. restore the prior topology snapshot before propagating the post-pass error;
5. mutate and publish only after preflight succeeds.

## Verification

Deterministic Python and Rust regressions establish a prior snapshot and
canonical edge, replace the staged caller with invalid UTF-8, and prove that
the post-pass error propagates while both snapshot and edge remain intact.
The complete code-graph binary, strict Clippy, 504 library tests, and the
hybrid hermetic all-target suite passed.

## Related Boundary

An authoritative zero-byte read is different from an unreadable source. Zero
bytes authorize exact-path derived-state eviction through the existing shared
teardown primitive. Any read failure remains non-authoritative and must retain
last-known-good state for retry.
