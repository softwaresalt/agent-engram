---
title: "Call-graph cross-file & method-call resolution — design deliberation"
type: deliberation
date: 2026-07-08
status: decided
signed_off_by: operator
signed_off_on: 2026-07-10
stash_id: 3AF329FF
harvested_to: 082-F
plan: docs/exec-plans/2026-07-10-callgraph-cross-file-resolution-plan.md
related: [079-F, engram-graph-quality-assessment, 081-F, 082-F]
---

# Call-graph cross-file & method-call resolution

**Status: DECIDED — operator signed off on 2026-07-10.** Option B (deferred post-pass +
unambiguous-name guard) accepted. Operator answers to the four open questions:
(1) precision policy = **unambiguous-name-only** (skip ambiguous, precision-first);
(2) edge provenance = **tag post-pass-resolved calls distinctly** (`calls_resolved_singleton`);
(3) performance = **gate the global post-pass behind full/`--force` index only** (incremental
sync skips it); (4) scope = **Rust-first as slice 1**, fan out to peer tree-sitter extractors
as follow-on tasks. Acceptance is gated by the retrieval-eval subsystem (081-F slices S1+S3):
resolution recall must rise and false-edge rate stay within the operator-chosen threshold.

> **TARGET-CORRECTNESS GATE (acceptance clarification).** Recall/false-edge thresholds are
> necessary but **not sufficient**. Every `calls_resolved_singleton` edge MUST match the
> fixture manifest's expected target, checked by exact target identity — not merely that the
> target exists. `false_edge_rate` (via `count_dangling_calls_edges`) only detects **dangling**
> targets (a `to` with no `function_meta` row), so it is a **lower-bound** signal: it cannot
> catch a call resolved to a wrong-but-existing function (mis-resolution to a real symbol).
> Acceptance therefore requires manifest target-correctness assertions in addition to the
> aggregate rates. Mis-resolution detection (distinguishing correct from wrong-but-existing
> targets) is captured as follow-up stash `49561F22`.

Harvested into feature **082-F**; plan at
`docs/exec-plans/2026-07-10-callgraph-cross-file-resolution-plan.md`. This is the highest-leverage of the
three graph/search quality recommendations from the 2026-07-08 assessment, but
it reshapes graph semantics for every indexed workspace with real precision/recall
tradeoffs, so it should not be shipped autonomously.

## Problem (evidence)

`get_workspace_status`/`map_code`/`impact_analysis` return **zero outgoing call
edges** for many idiomatic Rust functions (e.g. `get_daemon_status`,
`get_workspace_status`), making blast-radius analysis unreliable. On-disk edge
data for `agent-engram` itself: `defines: 2333, calls: 1368, inherits_from: 2`.
Functions in unchanged files (`connect_db`, `hydrate_code_graph`) have call edges;
functions that call cross-file or via method receivers do not.

## Root causes (two compounding limitations)

1. **Resolution is file-local.** `src/services/code_graph.rs` (index path ~466,
   sync path ~1070) resolves `ExtractedEdge::Calls { caller, callee }` with
   `find_function_id(&function_ids, callee)` where `function_ids` contains only
   symbols **defined in the current file** (comment: "Resolve names to IDs within
   this file's symbols"). Every cross-file call is silently dropped. `Imports`
   edges are already counted as `cross_file_edges_dropped` — cross-file was a
   known deferred limitation, not an accident.
2. **Method calls are dropped at extraction.** `src/services/parsing/rust.rs`
   `resolve_call_name` handles `identifier` (bare `foo()`) and `scoped_identifier`
   (`a::b::foo()` → last segment) but `_ => None` drops `field_expression`
   (method/receiver calls `x.foo()`, `self.foo()`), which dominate idiomatic Rust.

## Design options

### A. Full workspace-global name resolution
Build a workspace-wide `name → [symbol_id]` index; resolve every call against it.
- Pro: maximal recall.
- Con: name collisions produce **ambiguous/false edges** (e.g. `x.count()` →
  every `count` in the workspace). Method calls have no receiver-type resolution,
  so precision collapses. Edge counts could grow 3–5×, much of it noise.

### B. Deferred post-pass + unambiguous-name guard (recommended)
Mirror the **existing** `reresolve_references_edges` pattern (code_graph.rs:539,
1149) that already re-resolves `References` edges cross-file in a post-pass:
1. Extraction: add a `field_expression` arm to `resolve_call_name` to capture
   method names (keep the `CALL_BLOCKLIST`).
2. During indexing, record unresolved `Calls` edges (callee name only) instead of
   dropping them.
3. Post-pass: resolve each unresolved callee against a workspace-global name
   index, **but only create an edge when exactly one function has that name**
   (unambiguous). Skip ambiguous names (bounds false edges).
- Pro: fixes unique-name cross-file calls (`get_health_report_for_daemon`,
  `current_process_memory_bytes`) with near-zero false-edge risk; reuses an
  established pattern; incremental sync gets the same post-pass.
- Con: method calls to common names (`count`, `get`, `new`) stay unresolved
  (acceptable — better under-recall than false edges). Adds a workspace-global
  query + a post-pass cost on large workspaces.

### C. Receiver-type-aware resolution
Resolve `x.foo()` by inferring the type of `x` (impl/trait method tables).
- Pro: precise method edges.
- Con: requires real type inference — large, out of scope for a heuristic graph.

## Recommendation

Pursue **Option B**. It is the best recall/precision/effort balance and reuses the
`reresolve_references_edges` machinery. Open design questions for the operator:

1. **Precision policy**: is the strict "unambiguous name only" guard acceptable,
   or should ambiguous calls emit edges to all candidates (recall over precision)?
2. **Edge typing**: mark post-pass-resolved calls with a distinct provenance
   (e.g. `calls_unresolved_singleton`) so consumers can weight them?
3. **Performance**: acceptable added latency for the global post-pass on large
   workspaces (10k+ symbols)? Consider gating behind `--force`/full index only.
4. **Scope**: Rust-only first, or apply the same `field_expression`/post-pass
   pattern to the other tree-sitter language extractors?

## Blast radius / why not autonomous

Changes core indexing for **every** workspace; alters edge counts and graph
semantics; false-edge risk cannot be validated on a single workspace; the
precision/recall policy is a product decision. Requires operator sign-off before
implementation. Harvest this deliberation into a feature once the questions above
are answered.
