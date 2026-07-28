---
title: "Same-file duplicate function-name shadowing — fail-closed direct-edge target (deliberation)"
type: deliberation
date: 2026-07-27
deliberation_id: "014-D"
stash_id: "FF7DE872"
conclusion: "Option A — fail-closed on same-file same-name ambiguity (language-agnostic)"
confidence: "high"
governing_invariant: "013-D no-false-edge / 082-F target-correctness"
independent_of: ["FE8B3B2D", "096-F"]
related_feature: "096-F"   # Python namespace canonical resolution — SPLIT OUT, does NOT subsume this bug
tags:
  - code-graph
  - python
  - rust
  - call-graph
  - fail-closed
  - target-correctness
  - "013-D"
---

## Problem Frame

> **Execution correction (100-F ship, 2026-07-28).** Two premises below were
> disproven during implementation; the authoritative behavior is recorded in the
> durable code-graph capability notes (`docs/architecture.md`).
> 1. **Rust vector.** Same-name defs in different inline `mod` blocks are *not*
>    reachable — the Rust extractor does not descend `mod_item` bodies. The
>    verified same-file duplicate-name shape is mutually-exclusive
>    `#[cfg(...)]`-gated top-level definitions (tree-sitter does not evaluate
>    `cfg`, so both are extracted). The RED harness and acceptance tests use it.
> 2. **Python was already fail-closed.** Two same-name top-level `def`s were
>    *already* caught by the 096-F module-binding contest check (`is_contested`,
>    `module_binding_counts > 1`), so the live wrong-edge defect was **Rust-only**.
>    The chosen guard stays language-agnostic and still hardens Python as
>    defense-in-depth; the Python RED test is retained as a green regression guard.

`find_function_id` (`src/services/code_graph.rs:2988`) resolves a direct-edge
callee target by **name only** using first-match `.find()`:

```rust
fn find_function_id(ids: &[(String, String)], name: &str) -> Option<String> {
    ids.iter().find(|(n, _)| n == name).map(|(_, id)| id.clone())
}
```

When a single file declares **more than one top-level def of the same name**
(legal in Python, where the last def shadows/wins; also reachable in Rust via
same-name defs in different inline `mod` blocks within one file), the two
direct-edge minting sites bind a bare call to the **first (earlier / shadowed)**
def instead of the effective target:

* index path — `code_graph.rs:~1644-1645`
* incremental-sync path — `code_graph.rs:~2522-2523`

Both sites do `match (find_function_id(caller), find_function_id(callee))` and,
on `(Some, Some)`, mint a **direct** `calls` edge. A duplicate same-name callee
therefore yields a **WRONG target edge** — a **013-D no-false-edge / 082-F
target-correctness** violation.

The existing Python `is_contested` guard (`code_graph.rs:332/334`) catches
*import / rebind* ambiguity via `is_ambiguous`, but does **not** catch two
same-name **top-level defs** in this file's `function_ids` vector.

Same-file bare calls take the **DIRECT-edge path BEFORE** any canonical /
singleton post-pass, so **096-F's canonical resolver cannot fix this** — the
canonical index fails closed on a duplicate `canonical_path` rather than
applying last-wins. This bug was newly **EXPOSED (not introduced)** by Python
094-F; the **shared** `find_function_id` consumer also affects **Rust**
same-name-in-different-inline-modules-per-file. **INDEPENDENT of
FE8B3B2D / 096-F** and not gated behind that namespace-resolution work.

## Options Considered

### Option A — Fail closed on same-file same-name ambiguity (CHOSEN)

When `>1` candidate shares the callee name in this file's `function_ids`, do
**not** mint a first-match direct edge; instead route the call to staging (the
cross-file post-pass already **skips ambiguous / unmatched names to bound false
edges**, `code_graph.rs:1764-1766`) or drop fail-closed.

* Language-agnostic — preserves 013-D across **both** Rust and Python.
* Mirrors the existing cross-file singleton "skip ambiguous" precedent and the
  canonical `duplicate_canonical_path` fail-closed.
* Additive: introduce an ambiguity-aware helper; leave `find_function_id`
  byte-identical for its other 5 consumers.
* **Cost:** recall for the rare legitimate same-file redefinition *effective*
  call is deferred — a **documented v1 limitation**, not a false edge.

### Option B — Source-order / last-wins semantics (REJECTED for v1)

Return the **last** matching def (Python last-def-wins) to recover recall +
correctness for the Python case.

* **Unsound for the shared consumer:** in Rust, two same-name defs in different
  inline `mod` blocks per file are **distinct targets** disambiguated by module
  path, **not** source order. Blanket last-wins would **mint a new WRONG Rust
  edge** — trading one false-edge class for another.
* Requires language-gated logic + per-language correctness proof; higher blast
  radius; violates "keep the shared resolver language-agnostic and fail-closed".

## Decision

**Option A.** Add an additive **ambiguity-aware** resolver
(`find_unique_function_id` returning `None` / a typed reason when `>1` same-file
same-name candidate exists), used **only** at the direct-edge callee-resolution
minting sites. Leave `find_function_id` byte-identical for caller attribution,
the resolve path, and every other call site. Apply the guard **symmetrically**
at the index (`~1644`) and sync (`~2522`) sites so indexing and incremental sync
fail closed identically. Ship a **Rust-path regression test** proving no recall
regression on legitimate unique-name same-file calls **and** correct fail-closed
on an adversarial same-file duplicate-name corpus.

A Python-only **last-wins** recall recovery is a documented, deferrable
follow-up — explicitly **NOT v1**.

## Open Questions (carried into the plan)

1. **Caller side** — should the guard also cover a same-file duplicate
   *enclosing-function* name (uncertain edge origin)? **Recommend YES** —
   symmetric fail-closed to fully honor 013-D.
2. **Source order** — confirm `function_ids` preserves source order (only needed
   IF a future last-wins follow-up is pursued; **not** required for fail-closed
   v1).
3. **Observability** — increment `cross_file_edges_dropped` or add a dedicated
   same-file-ambiguous counter for parity with the post-pass?

## Blast Radius

`find_function_id` is a **shared consumer at 7 call sites**
(`code_graph.rs` 1617 / 1644 / 1645 / 2497 / 2522 / 2523 / 2805) across the
index, sync, and resolve paths, for **both caller and callee**, **Rust +
Python**. Do **not** change its global semantics — add the additive helper.
(Stash line refs `2037` / `896-908` had drifted; verified against current code:
`find_function_id` @ 2988, minting sites @ 1644 / @ 2522.)
