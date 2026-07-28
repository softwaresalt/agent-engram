---
title: "find_function_id returns FIRST match — same-file same-name shadowing binds a wrong call edge"
description: "The direct-edge resolver's find_function_id returns the first definition by name, so a file with multiple top-level defs of the same name (legal in Python; last def shadows) binds a bare call to the earlier/shadowed target instead of the effective one"
problem_type: "logic_error"
category: "bugs"
component: "src/services/code_graph.rs"
root_cause: "find_function_id (src/services/code_graph.rs:2037) uses .find(|(n, _)| n == name) and returns the FIRST id whose name matches; it does not fail closed or apply source-order/last-wins semantics on duplicate same-file same-name definitions"
resolution_type: "fixed_fail_closed"
severity: "medium"
message: "bare call binds to a shadowed (earlier) definition rather than the effective (last) one — RESOLVED fail-closed in 100-F"
file_path: "src/services/code_graph.rs"
date: "2026-07-20"
feature: "094-F"
shipment: "089-S"
follow_up_stash: "FF7DE872"
resolved_by_feature: "100-F"
resolved_by_shipment: "092-S"
resolved_pr: 291
resolved_in_commit: "8a6c6e32507434ff80e7453b92ecf27d21992bc4"
resolution_ref: "docs/compound/bugs/same-file-duplicate-name-fail-closed-resolution-defect-was-rust-cfg-gated-2026-07-28.md"
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
  - "resolved"
  - "100-F"
  - "FF7DE872"
---

# find_function_id returns FIRST match — same-file same-name shadowing binds a wrong call edge

> **RESOLVED in 100-F** (shipment 092-S, PR #291, merge `8a6c6e32`) — fixed
> **fail-closed** (Decision 014-D, Option A). Full resolution and a correction to
> this diagnosis are in
> [`same-file-duplicate-name-fail-closed-resolution-defect-was-rust-cfg-gated-2026-07-28.md`](./same-file-duplicate-name-fail-closed-resolution-defect-was-rust-cfg-gated-2026-07-28.md).
> **Two corrections to the historical framing below:** (1) the **Python** two-def
> case was already fail-closed at fix time — the 096-F `is_contested` guard
> (`module_binding_counts`, fed only by `increment_python_binding`) is
> **Python-scoped**, so the live wrong edge was **Rust-only**; (2) the
> "same-name in different **inline modules** per file" Rust repro is
> **unreachable** — the extractor's top-level dispatch has no `mod_item` descent,
> so the real Rust vector is `#[cfg(unix)]/#[cfg(windows)]`-gated duplicate
> top-level defs (tree-sitter extracts both branches).

## Problem

When a single file defines the **same top-level function name more than once**
(legal in Python — the last `def` shadows earlier ones; in Rust via
`#[cfg(...)]`-gated duplicate top-level defs, both extracted because tree-sitter
does not evaluate `cfg`), a bare call resolves to the **earlier/shadowed**
definition instead of the effective one. This is a target-precision violation
(013-D no-false-edge / 082-F target-correctness). *(Correction: an earlier draft
listed "different inline modules per file" as the Rust repro — that is
unreachable; the extractor does not descend `mod_item`. See the resolution note
above.)*

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

## Resolution (RESOLVED in 100-F — Option A, fail-closed)

Shipped in **100-F** (shipment 092-S, PR #291) as **Option A: fail closed** on
`>1` same-file same-name candidate — an additive `find_unique_function_id`
consulted only at the two direct-edge minting sites, leaving `find_function_id`
byte-identical for its other consumers, with a `same_file_ambiguous_dropped`
counter and Rust + sync-path regression tests (no recall regression). The
**source-order / last-wins** alternative was rejected for v1 (unsound for Rust);
Python-only last-wins recall recovery is a deferred follow-up (stash `B94772CB`).

### Original analysis (why it was split out — retained for context)

This was an **independent** fix on the **direct-edge path**,
**not** a special case the FE8B3B2D namespace feature subsumes: same-file bare
calls resolve via `find_function_id` and get a direct edge
(`code_graph.rs:896-908`) *before* any canonical/singleton post-processing, and
the canonical index fails closed on duplicate `canonical_path`
(`cozo_queries.rs:1021-1025`) — so the namespace feature (which only
disambiguates cross-file/cross-module *staged* calls) can never apply the
source-order / last-wins semantics this bug needs. Split out of the
namespace-resolution deliberation (FE8B3B2D) so it can be fixed **independently
and sooner**, on its own code path. Candidate fixes:

- **Fail closed** on `>1` same-file same-name candidate (mirror the cross-file
  singleton post-pass ambiguity handling), or
- Implement **source-order / last-wins** semantics.

Either fix MUST add a Rust-path regression test proving no recall regression.

## Prevention

- Shared symbol-lookup helpers that assume name uniqueness are latent
  correctness bugs; when a new language with permissive redefinition semantics
  joins the resolver, audit every `.find(by name)` lookup.
- Origin: 094-F Copilot review thread on `python.rs:269` (PR #277).
