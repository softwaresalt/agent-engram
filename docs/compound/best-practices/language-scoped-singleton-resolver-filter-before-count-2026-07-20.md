---
title: "Cross-file singleton call resolver must filter by language BEFORE the unambiguity check"
description: "The shared cross-file singleton Calls resolver must scope candidate definitions to the caller's language first, then apply the exactly-one rule — otherwise a same-named definition in another language causes a mis-binding or a false ambiguity drop"
problem_type: "logic_error"
category: "best-practices"
component: "src/db/cozo_queries.rs"
root_cause: "The cross-file singleton resolver counts definitions of a callee name workspace-globally; once Python bare calls join the shared resolver, a same-named function in a different language (e.g. Rust) either mis-binds the edge cross-language or makes a truly-unique same-language target look ambiguous"
resolution_type: "code_fix"
severity: "high"
message: "python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def"
file_path: "src/db/cozo_queries.rs"
date: "2026-07-20"
feature: "094-F"
shipment: "089-S"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/277"
  - "src/db/cozo_queries.rs (language-scoped singleton resolver ~L2151-2280; same_language filter ~L2262-2270)"
  - "src/services/code_graph.rs (canonical_path is Rust-only, fail-closed empty for other languages ~L41-67; cross-file singleton post-pass ~L985-1002)"
  - "tests/integration/code_graph_test.rs (python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def)"
tags:
  - "code-graph"
  - "call-graph"
  - "cross-file-resolution"
  - "singleton"
  - "language-scoping"
  - "no-false-edge"
  - "013-D"
  - "082-F"
  - "094-F"
---

# Cross-file singleton call resolver must filter by language BEFORE the unambiguity check

## Problem

engram resolves cross-file bare calls with a **singleton** rule: a staged bare
call `foo()` becomes a real `calls_resolved_singleton` edge only when the
workspace contains **exactly one** definition named `foo`; zero or 2+ matches
are dropped to bound false edges. This resolver is a **shared consumer** across
languages.

When Python bare calls joined this shared resolver (094-F), a new hazard
appeared: a callee name may be defined in **two different languages**
(`helper` in `b.py` and in `r.rs`). A language-blind resolver either:

- **mis-binds cross-language** — a Python caller's `helper()` resolves to the
  Rust `helper` (violates 013-D no-false-edge and the 082-F target-correctness
  gate), or
- **falsely deems it ambiguous** — sees two `helper` definitions, calls it
  ambiguous, and drops a real edge that *is* unique within Python.

## Root Cause

The unambiguity count was computed over a workspace-global name index without a
language predicate. Ordering matters: counting first and filtering later loses
the information needed to make the correct decision.

Note engram's canonical-path resolver (built for Rust) does **not** help Python
here: `canonical_path` is populated Rust-only and is fail-closed empty for other
languages (`src/services/code_graph.rs`), so Python leans entirely on the
name-based singleton pass.

## Resolution

Filter candidate definitions to the **caller's language first**, then apply the
exactly-one rule to the filtered set (U3):

```rust
// src/db/cozo_queries.rs — LANGUAGE-SCOPED candidate index: name -> [(id, language)]
let caller_language = file_language.get(&call.source_file);
let same_language: Vec<&String> = name_index
    .get(&call.callee).into_iter().flatten()
    .filter(|(_, language)| Some(language) == caller_language) // FILTER BEFORE COUNT
    .map(|(id, _)| id)
    .collect();
if same_language.len() == 1 {
    // resolve to same_language[0] as calls_resolved_singleton
} else {
    // 0 or 2+ same-language matches -> retract/skip (fail closed)
}
```

## Prevention / Test That Pins the Ordering

A same-language-only *rejection* test is insufficient — it passes under both
orderings. Add a **discriminating positive** where the callee exists in the
caller's language AND another language:

`tests/integration/code_graph_test.rs::python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def`
— `helper` defined in both `b.py` and `r.rs`; a Python caller must resolve to
the **Python** target. This case fails under a global-singleton-first ordering
(it would see two candidates, deem them ambiguous, and create no edge), so it
uniquely pins "filter-by-language before the singleton check." Keep both the
adversarial rejection test and this positive test to fully bound U3.

- Any future per-language rollout (TS/Go/C#/Node) inherits this resolver;
  re-run the mixed-language ordering proof when a new language is added.
