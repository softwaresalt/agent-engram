---
title: "find_function_id returns FIRST match — same-file same-name shadowing binds a wrong call edge"
description: "The direct-edge resolver's find_function_id returns the first definition by name, so a file with multiple top-level defs of the same name (legal in Python; last def shadows) binds a bare call to the earlier/shadowed target instead of the effective one"
problem_type: "logic_error"
category: "bugs"
component: "src/services/code_graph.rs"
root_cause: "find_function_id (src/services/code_graph.rs:2037) uses .find(|(n, _)| n == name) and returns the FIRST id whose name matches; it does not fail closed or apply source-order/last-wins semantics on duplicate same-file same-name definitions"
resolution_type: "known_issue"
severity: "medium"
message: "bare call binds to a shadowed (earlier) definition rather than the effective (last) one"
file_path: "src/services/code_graph.rs"
date: "2026-07-20"
feature: "094-F"
shipment: "089-S"
follow_up_stash: "FF7DE872"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/277"
  - "src/services/code_graph.rs (find_function_id ~L2037-2041)"
  - "backlogit stash FF7DE872 (bug, medium — split from FE8B3B2D)"
tags:
  - "code-graph"
  - "call-graph"
  - "target-correctness"
  - "shadowing"
  - "no-false-edge"
  - "013-D"
  - "082-F"
  - "known-issue"
  - "FF7DE872"
---

# find_function_id returns FIRST match — same-file same-name shadowing binds a wrong call edge

## Problem

When a single file defines the **same top-level function name more than once**
(legal in Python — the last `def` shadows earlier ones; also reachable in Rust
via same-name-in-different-modules-per-file), a bare call resolves to the
**earlier/shadowed** definition instead of the effective one. This is a
target-precision violation (013-D no-false-edge / 082-F target-correctness).

This was **exposed, not introduced,** by Python 094-F: `find_function_id` is a
pre-existing shared consumer. Python simply makes duplicate same-file names
common enough to surface it.

## Root Cause

```rust
// src/services/code_graph.rs:2037
fn find_function_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter()
        .find(|(n, _)| n == name)   // FIRST match wins — no shadowing awareness
        .map(|(_, id)| id.clone())
}
```

`.find` returns the first `(name, id)` pair, so on `>1` same-file same-name
candidates it silently binds to whichever was indexed first.

## Scope / Blast Radius

This is a **same-file target-precision gap only**. It does **not** create
cross-file or cross-language false edges — the 094-F U3 (language-scoped
singleton) and U4 invariants still hold. The wrong edge points at a real,
same-name, same-file definition; it is simply the *shadowed* one.

## Resolution (deferred — tracked as bug FF7DE872)

Not fixed in 094-F. Split out of the namespace-resolution deliberation
(FE8B3B2D) so it can be fixed **independently and sooner**, NOT gated behind
that larger feature (which would subsume it as a special case). Candidate fixes:

- **Fail closed** on `>1` same-file same-name candidate (mirror the cross-file
  singleton post-pass ambiguity handling), or
- Implement **source-order / last-wins** semantics.

Either fix MUST add a Rust-path regression test proving no recall regression.

## Prevention

- Shared symbol-lookup helpers that assume name uniqueness are latent
  correctness bugs; when a new language with permissive redefinition semantics
  joins the resolver, audit every `.find(by name)` lookup.
- Origin: 094-F Copilot review thread on `python.rs:269` (PR #277).
