---
title: "Same-file duplicate-name wrong edge fixed fail-closed — the live defect was Rust cfg-gated, not the planned Python two-def case"
description: "100-F fixes the find_function_id first-match wrong-target edge by failing closed on >1 same-file same-name candidate (additive find_unique_function_id at the two direct-edge minting sites). The RED harness revealed the planning premise was wrong: at ship HEAD the Python two-def case was ALREADY fail-closed (the 096-F module-binding contested guard is Python-scoped), so the only live wrong-edge vector was Rust #[cfg(...)]-gated duplicate defs (tree-sitter extracts both branches); the plan's inline-`mod` repro is unreachable because the Rust extractor never descends mod_item."
problem_type: "logic_error"
category: "bugs"
component: "src/services/code_graph.rs"
root_cause: "find_function_id returns the FIRST name match; the pre-existing ambiguity guard that failed the Python case closed (is_contested over module_binding_counts) is fed only by increment_python_binding, so it is language-scoped to Python and left the SHARED direct-edge consumer exposed for Rust. tree-sitter does not evaluate cfg attributes, so #[cfg(unix)]/#[cfg(windows)] duplicate top-level defs are both extracted -> two same-file same-name function_item nodes -> first-match binds the shadowed one."
resolution_type: "fixed_fail_closed"
severity: "medium"
message: "a same-file duplicate-name bare call minted a wrong-target direct edge to the shadowed def; now fails closed (no edge) on >1 same-file same-name candidate"
file_path: "src/services/code_graph.rs"
date: "2026-07-28"
feature: "100-F"
shipment: "092-S"
resolves: "FF7DE872"
supersedes_status_of: "docs/compound/bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md"
citations:
  - "https://github.com/softwaresalt/agent-engram/pull/291"
  - "merge commit 8a6c6e32507434ff80e7453b92ecf27d21992bc4 (merge-commit only, P-009)"
  - "src/services/code_graph.rs (UniqueFunctionId + find_unique_function_id; guarded minting sites index + sync; same_file_ambiguous_dropped counter)"
  - "src/services/code_graph.rs (is_contested over module_binding_counts; increment_python_binding — Python-only feed ~L384/401/419; consulted ~L1698/2616)"
  - "src/services/parsing/rust.rs (top-level dispatch: function_item/struct_item only — no mod_item descent)"
  - "tests/integration/same_file_shadowing_acceptance_test.rs (4 acceptance + sync-path regression tests)"
  - "docs/decisions/2026-07-27-ff7de872-same-file-shadowing-fail-closed-deliberation.md (014-D Option A)"
  - "docs/compound/bugs/same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md (094-F known-issue this resolves)"
tags:
  - "code-graph"
  - "call-graph"
  - "target-correctness"
  - "shadowing"
  - "fail-closed"
  - "no-false-edge"
  - "013-D"
  - "082-F"
  - "014-D"
  - "cfg-gate"
  - "tree-sitter"
  - "language-scoped-guard"
  - "FF7DE872"
  - "100-F"
---

# Same-file duplicate-name wrong edge fixed fail-closed — the live defect was Rust cfg-gated

This resolves the 094-F known-issue
[`same-file-same-name-shadowing-first-match-wrong-edge`](./same-file-same-name-shadowing-first-match-wrong-edge-2026-07-20.md)
(stash FF7DE872). It captures both the fix and a **planning-vs-reality
discrepancy** the TDD RED harness surfaced that is worth remembering.

## What shipped (Option A — fail-closed, language-agnostic; 014-D)

`find_function_id` (first-match by name) is left **byte-identical** for its
other consumers. An **additive** resolver decides target identity only at the
two direct-edge callee minting sites:

- `UniqueFunctionId` enum + `find_unique_function_id(ids, name)` — returns the
  id only when exactly one same-file candidate matches; on `>1` it reports
  ambiguity and the caller **mints no edge** (fail closed), mirroring the
  cross-file singleton post-pass ambiguity handling.
- Both minting sites — the full-index path and the incremental-sync path — were
  rewritten to consult it and drop the direct edge on ambiguity.
- An observability counter `same_file_ambiguous_dropped` was added to
  `IndexResult` / `SyncResult` (`#[serde(default)]`).

No cross-file or cross-language edge is created (094-F U3/U4 invariants
preserved). Recall for legitimate **unique-name** same-file calls is unchanged.

## The discrepancy the RED test exposed (the durable lesson)

The plan/deliberation framed the live defect as a **Python** two-def case
(last `def` shadows). The RED harness proved that premise **false at ship HEAD**:

1. **Python was already fail-closed.** The two-def Python fixture in the RED
   test *passed* (no wrong edge) before any fix. The reason: an earlier feature
   (096-F namespace/canonical resolution) added an ambiguity guard,
   `ShadowIndex::is_contested`, keyed on `module_binding_counts`. That map is fed
   **only** by `increment_python_binding` — it is **language-scoped to Python**.
   So the Python two-def call was already suppressed at the minting sites
   (`is_contested` consulted at code_graph.rs ~L1698 and ~L2616).

2. **The live wrong edge was Rust-only.** Because the contested guard never sees
   Rust bindings, the **shared** `find_function_id` consumer was still exposed
   for Rust. The real, reachable vector is **`#[cfg(unix)]` / `#[cfg(windows)]`
   duplicate top-level defs**: tree-sitter does **not** evaluate `cfg`
   attributes, so it extracts **both** branches as two `function_item` nodes with
   the same name in one file — and first-match bound the shadowed one.

3. **The plan's inline-`mod` repro is unreachable.** The Rust extractor's
   top-level dispatch (`src/services/parsing/rust.rs`) walks `root.children()`
   and matches `function_item` / `struct_item` / … — there is **no `mod_item`
   case**, so functions declared inside inline `mod { … }` blocks are never
   extracted. You cannot produce a same-file duplicate name that way.

## Lessons

- **A language-scoped guard on a shared consumer is a latent bug for every other
  language.** `is_contested` correctly closed Python but silently left Rust open
  because the *shared* `find_function_id` had no equivalent gate. When a guard
  lives on a per-language feed (here `increment_python_binding`), audit every
  language that flows through the same downstream resolver.
- **Validate the RED harness reproduces via the REAL extraction path before
  implementing.** The RED test — not the plan — is authoritative. Ours revealed
  (a) the Python case was already green and (b) the inline-`mod` repro extracts
  nothing, redirecting the fix to the actual `#[cfg]`-gated Rust vector. A test
  that fails for the reason you *assumed* is the only proof the defect is real.
- **tree-sitter extracts all `cfg` branches.** Any analysis that assumes one
  effective definition per name-per-file must treat `#[cfg(...)]`-gated
  duplicate defs as a first-class same-name-collision source (they are the
  canonical Rust trigger for this whole class of bug).
- **Correct authoritative artifacts when execution diverges.** The deliberation,
  plan, and `docs/architecture.md` each got an "Execution correction" note so the
  durable record reflects Rust-cfg reality, not the Python framing.

## Verification

- 4 acceptance/regression tests in
  `tests/integration/same_file_shadowing_acceptance_test.rs`: zero wrong-target
  edges across resolution classes; unique-name recall preserved; cross-file
  singleton unchanged; **sync-path** fail-closed regression.
- No recall regression: the 18-test `calls_recall_acceptance` suite stays green;
  461 lib tests green; fmt + clippy (`-D warnings -D clippy::pedantic`) clean.
- Runtime (real binary + git workspace + live cozo/JSONL): same-file
  `describe -> plat` (`#[cfg]`-gated dup) mints **no** calls edge (fail-closed);
  same-file `caller_unique -> helper` mints a direct edge (recall preserved).

## Deferred (see stash)

- **Python-only last-wins recall recovery** (feature, low, stash `B94772CB`):
  v1 fails closed for both languages; a future enhancement could mint the edge to
  the effective (last) Python def instead of dropping it.
- **Versioned stale-direct-edge backfill** (task, medium, stash `8DD29746`):
  already-persisted wrong edges stay stale until a forced reindex (hash-skip);
  mirror 096-F's opt-in backfill. Related freshness note:
  `workflow-issues/new-extraction-logic-needs-forced-reindex-2026-07-20.md`.
