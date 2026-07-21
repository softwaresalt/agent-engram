---
title: "Python bare-call Calls-edge extraction must be body-scoped and fail-closed"
description: "Adding Python function-call graph edges: walk only the function body field, stop at nested callable/class boundaries, and drop attribute/method + subscript/chained calls fail-closed to honor the 013-D no-false-edge invariant"
problem_type: "logic_error"
category: "best-practices"
component: "src/services/parsing/python.rs"
root_cause: "Before 094-F only Rust emitted Calls edges; a naive Python call walker that recurses whole subtrees (including parameter defaults, annotations, decorators, and nested inner-function bodies) or promotes attribute/method calls produces false or mis-attributed edges"
resolution_type: "code_fix"
severity: "high"
message: "map_code / impact_analysis / query_graph return empty call graphs for .py files"
file_path: "src/services/parsing/python.rs"
date: "2026-07-20"
feature: "094-F"
shipment: "089-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/277"
  - "src/services/parsing/python.rs (extract_calls_from_body ~L233-284, resolve_call_name ~L286-320)"
  - "docs/exec-plans/2026-07-20-python-call-edge-extraction-plan.md"
  - "docs/decisions/2026-07-20-python-call-edge-extraction-spike.md"
tags:
  - "tree-sitter"
  - "python"
  - "call-graph"
  - "code-graph"
  - "no-false-edge"
  - "013-D"
  - "082-F"
  - "094-F"
---

# Python bare-call Calls-edge extraction must be body-scoped and fail-closed

## Problem

Baseline before feature 094-F: **only Rust emitted `Calls` edges**. Python
files were parsed for symbols but produced no call graph, so `map_code`,
`impact_analysis`, and `query_graph` returned empty call relationships for
`.py` files. The pilot goal was to emit `Calls` edges for bare Python calls
(`foo()`) as the first per-language rollout (TS/Go/C#/Node to follow).

The dangerous failure mode is not the missing edges — it is adding a walker
that emits **wrong** edges and violates the 013-D no-false-edge invariant.

## Root Cause / Pitfalls Found In Review

A naive "walk the whole function subtree and emit an edge for every `call`
node" implementation is wrong in four distinct ways. Each was caught in the
094-F review and encoded as a rule:

1. **Walk only the `body` field — not the whole node.** Calls that appear in
   parameter default values, parameter/return annotations, and decorators
   (`def f(x=build_default()): ...`) execute at *definition* time in the
   *enclosing* scope, not when the function runs. Attributing them to the
   function is a false edge. `extract_calls_from_body` seeds its DFS with the
   children of `node.child_by_field_name("body")` **only**.

2. **Stop at nested callable/class boundaries.** The DFS must not descend into
   `function_definition`, `lambda`, or `class_definition` nodes — calls inside
   an inner `def` belong to that inner scope, not the owning top-level
   function. Consequence (accepted v1 scope limit): **calls made by nested /
   inner functions are omitted entirely** from the graph, rather than being
   mis-attributed to the outer function.

3. **Attribute / method calls are dropped fail-closed.** `obj.foo()` /
   `self.bar()` classify as `is_method:true` with an **empty** `raw_qualifier`,
   so `should_stage_provenance_call(true, false, "")` returns `false` and the
   consumer drops them. Emitting a bare `foo` edge for `obj.foo()` would leak a
   false edge to any unrelated top-level `foo`. Instance-method dispatch needs
   type inference and is a documented non-goal.

4. **Unknown call shapes are skipped, not guessed.** `subscript` (`d[k]()`) and
   chained (`a().b()`) callees return `None` in v1 — forward-compatible and
   panic-free. A function with no `body` field yields zero edges (fails closed).

A builtin/idiomatic-callee blocklist (`print`, etc.) further suppresses
navigational noise.

## Resolution

```rust
// src/services/parsing/python.rs
// DFS over a TOP-LEVEL function's BODY only; stop at nested scopes.
fn extract_calls_from_body(node, source, caller_name, edges) {
    let Some(body) = node.child_by_field_name("body") else { return }; // fail closed
    // seed stack with body children only (skips params/annotations/decorators)
    while let Some(current) = stack.pop() {
        if matches!(current.kind(),
            "function_definition" | "lambda" | "class_definition") { continue; } // don't descend
        if current.kind() == "call" {
            if let Some(call) = resolve_call_name(current, source) { edges.push(Calls{..}); }
        }
        // push children …
    }
}

// identifier foo()  -> is_method:false (promoted)
// attribute obj.f() -> is_method:true, raw_qualifier "" (consumer drops: fail closed)
// subscript/chained -> None (skipped in v1)
```

## Prevention

- When adding call-edge extraction for a **new language**, treat the
  013-D no-false-edge invariant as the acceptance gate: prefer *dropping*
  ambiguous calls over emitting a plausible-but-wrong edge.
- Always scope the call walk to the executable body; exclude definition-time
  expressions (defaults, annotations, decorators).
- Write count-based negative assertions (e.g. "zero `Calls` for `save` given
  `obj.save()`") — presence-only tests miss false-edge regressions.
- Attribute/method dispatch and nested-function attribution are separate,
  larger features (type inference); ship the bare-call pilot without them and
  document the omissions.
