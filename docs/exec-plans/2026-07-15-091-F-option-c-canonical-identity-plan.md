---
title: "Exec Plan — 091-F Option C: canonical-identity qualified/method call resolution"
doc_type: plan
source: "091-F (Option C); spike 091.001-T GO; deliberation 2026-07-15-option-c-canonical-identity-deliberation"
description: >-
  Reviewed implementation plan decomposing Option C into two release units of <=2h, width-isolated,
  test-first tasks: (A) precision-neutral canonical-identity infrastructure, and (B) gated resolution
  enablement. Encodes the four mandatory safeguards, adversarial fixtures, the identity-based
  precision/recall release gate, the hard 084-S dependency, and the 088-F/081-S/091.002-T reconciliation.
topic: "Decompose Option C under the absolute no-false-edge invariant"
depth: "plan"
decision_status: "planned — queued for Ship (two shipments)"
author: stage
date: 2026-07-15
linked_artifacts:
  - "091-F"
  - "091.001-T"
  - "091.002-T"
  - "088-F"
  - "089-F (084-S)"
  - "docs/decisions/2026-07-15-091-001-canonical-identity-spike.md"
---

# Exec Plan — 091-F Option C (canonical-identity qualified/method call resolution)

- **Date:** 2026-07-15 · **Stage owner:** stage agent · **Base:** origin/main `df77584`.
- **Spike:** `docs/decisions/2026-07-15-091-001-canonical-identity-spike.md` — **GO (Rust-first)**.
- **Invariant (kept, absolute):** recall recovery must never create a false or mis-resolved
  `calls_resolved_singleton`/`calls_resolved_canonical` edge (013-D).
- **Boundary:** Stage planning only. No source/test authored; Ship executes. Feature `feat/*` branches
  and PRs are Ship's, not Stage's.

## 1. Objective

Recover the deferred `module::helper` / `crate::free` / bare `Type::method` / `Self::method` /
cross-type associated-call recall by resolving each qualified/method call to a **canonical workspace
identity** and emitting an edge **iff** it resolves to **exactly one** definition — everything uncertain
is dropped (fail-closed). Canonical identity is stored **additively** (`function_meta.canonical_path`),
leaving all existing name-based subsystems untouched (blast-radius isolation).

## 2. Release-unit strategy (blast-radius isolation)

Two coherent, independently reviewable release units:

- **Unit A — Canonical Identity Infrastructure (precision-neutral).** Builds the module tree, use-graph,
  fail-closed resolver, canonical impl-method identity, generic normalisation, unforgeable `Self`
  marker, and the index-format fingerprint/re-index. **Creates no new qualified/method edges** (calls are
  still dropped at `code_graph.rs` L487), so it cannot regress precision. Ships first; builds on
  origin/main; **no external hard dependency**.
- **Unit B — Resolution Enablement (gated flip-on).** Stages qualified/method calls with raw provenance,
  turns on canonical singleton resolution in the post-pass, and adds the adversarial fixtures + the
  identity-based precision/recall release gate. **Depends on Unit A AND 084-S.**

The DoD ("all four safeguards land before any qualified/method resolution ships") is satisfied
structurally: edges flip on **only** in Unit B, which is hard-blocked on Unit A (safeguards #1–#4
infrastructure) and its own gate task.

## 3. The four mandatory safeguards → tasks

| # | Safeguard | Tasks |
|---|---|---|
| 1 | Canonical declaration **and** call-target identity (use-graph + module-path + re-export tracing) | A1, A2, A3, A4, A6 (defs) · B2 (targets) |
| 2 | Unforgeable internal `Self` marker | A7 |
| 3 | Consistent generic-parameter normalisation | A5 |
| 4 | Index-format fingerprint / upgrade-triggered re-index | A8 |
| + | Absolute no-false-edge invariant (fail-closed + identity gate) | §6 rules · B3 · B4 |

## 4. Task decomposition (all <=2h, width-isolated, test-first)

Working names A1–A8 / B1–B4 map to harvested child IDs of 091-F (091.003-T onward). Each task lands its
own tests **first** (red), then implementation (green).

### Unit A — infrastructure
- **A1 · Module-path derivation** (width: module-tree builder + additive storage). Canonical module path
  per Rust file from crate root (`lib.rs`/`main.rs`) + fs layout (`foo.rs` / `foo/mod.rs`) + inline `mod`.
  Tests: mapping fixtures incl. nested/inline mods + multi-crate workspace. *No external dep.*
- **A2 · Use-graph extraction** (width: `parsing/rust.rs` use path + storage). Full `use` tree: single,
  groups `{a,b}`, glob `*` (marker), `as` aliases, `self`/`super`/`crate` roots, `pub use` (re-export
  flag). Tests: use-decl → alias map + glob/pub-use flags. *No external dep.*
- **A3 · Canonical resolver — in-crate roots** (width: new resolver module, pure fn). Resolve
  in-module names, `self/super/crate`, direct aliases; fail-closed on external/glob/macro/ambiguous.
  Tests: exhaustive resolve + fail-closed. Dep: A1, A2.
- **A4 · Re-export (`pub use`) transitive closure** (width: resolver closure). Bounded transitive
  tracing + cycle/depth detection; fail-closed on cycle/limit. Tests: chains, cycles, depth-limit. Dep: A3.
- **A5 · Generic-parameter normalisation** (width: normalisation util). `Type<T>::m`, turbofish,
  lifetimes, multi-arg → one canonical form; applied at index + resolve. Tests: variants converge. Dep: A3.
- **A6 · Canonical impl-method identity at indexing** (width: `code_graph.rs` def indexing + additive
  `function_meta.canonical_path`). Canonicalise `impl <Type>` → populate `canonical_path`; leave `name`
  intact. Tests: `impl Widget` vs `impl crate::a::Widget` vs same-named cross-module types → distinct
  `canonical_path` (**RMeJ0 regression**). Dep: A1, A3, A5.
- **A7 · Unforgeable `Self` marker + scope-aware walk** (width: `parsing/rust.rs` Self + body walk).
  Typed `Qualifier::SelfType` sentinel set only from the `Self` keyword node; `extract_calls_from_body`
  stops at nested `impl_item`/`function_item` (**closes F2**). Tests: forge `Self::Assoc::method`;
  nested-impl scope. Dep: A3, A6.
- **A8 · Index-format fingerprint + one-time forced re-index** (width: migration + fingerprint).
  `schema_meta["code_index_format_version"]`; bump → invalidate code `file_hash` → forced re-index;
  additive `function_meta` `:replace` via `run_script_retrying` (086.001-T) w/ rollback. Tests: bump
  forces reindex; same-version unchanged content skipped. Precedent: 087.001-T. Dep: A6.

### Unit B — enablement (gated)
- **B1 · Stage qualified/method calls with raw provenance** (width: `code_graph.rs` staging +
  additive `staged_call` columns). Replace the `continue`-drop (L487/L1138) with staging that carries the
  **raw qualifier**, **kind** (`module`/`type`/`self`/`method`), and **enclosing canonical type**;
  additive + legacy-tolerant (089-F F10). Tests: staged rows carry raw provenance; legacy rows tolerated;
  bare-name staging unchanged. Dep: A7, **089-F (084-S)**.
- **B2 · Canonical singleton resolution in post-pass** (width: `reresolve_calls_edges` wiring).
  Canonicalise each staged qualified/method callee (A3/A4/A5/A7) → match `function_meta.canonical_path`;
  exactly one → `calls_resolved_canonical` edge; else no edge (fail-closed). Bare-name path unchanged.
  Tests: single→edge; ambiguous/none→no edge. Dep: A6, B1.
- **B3 · Adversarial precision fixtures (non-vacuous negatives)** (width: integration fixtures). F1
  (`mem::swap` + unique free `swap` → 0 edges), F6 (`use ext::Widget as Alias; Alias::build()` → 0),
  re-export collision, glob collision, generic convergence, `Self`-forge. Red until B2 correct. Dep: B2.
- **B4 · Identity-based eval gate + release gate** (width: `retrieval_eval` + eval-gate test). Replace
  the dangling-only precision signal (**fixes F4**) with an identity assertion (resolved edge → correct
  canonical def or none) on an adversarial collision fixture; wire **recall-up + target-correctness=1.0**
  release gate to the 081-F harness. Tests: gate fails on a seeded mis-resolution-to-real-target. Dep: B2, B3.

### Reconciliation
- **091.002-T** (existing) · reconcile 088.005-T archived-done vs blocked full-resolver acceptance.
  State-reconciliation only; adjudicable **after** B4. **Must not** modify 088.005-T or the 081-S
  manifest until a formal resumption. Dep: **B4** (added).

### 4.1 Adversarial remediations folded in (D1–D6, pre-harvest)

From `docs/closure/2026-07-15-091-F-option-c-canonical-identity-adversarial-review.md`:

- **D1 (A1):** honour `#[path="…"]` mod attributes; `#[cfg]`-gated / non-derivable module mappings →
  fail-closed (no `canonical_path`). Add `#[path]`/cfg fixtures.
- **D2 (spike §6, B1/B2):** method-call resolution is scoped to **known-receiver** only (`self.m()`/
  `Self::m()`); arbitrary `x.foo()` stays dropped. Recall claim narrowed.
- **D3 (A1/A3):** enumerate the **workspace crate-name set**; A3 classifies a path root as workspace vs
  external; external → fail-closed. Fixture: external qualifier never resolves.
- **D4 (A6/B2):** empty `canonical_path` is **never** a match target; B2 filters empties before the
  singleton test. Fixture: empty-vs-empty → 0 edges.
- **D5 (A8):** run the re-index in the normal index path (not a blocking open hook), **single-flight**,
  `run_script_retrying` for SQLITE_BUSY, duration-observable.
- **D6 (A3):** Rust name-resolution precedence (explicit item/`use` > glob); fail-closed on ambiguity;
  external-crate-root shadowing (`mod std`) conservative (D7). Shadowing fixtures in B3.

## 5. Dependency graph (encoded, not prose)

```
A1,A2 ─► A3 ─► A4
          └──► A5
A1,A3,A5 ─► A6 ─► A7
                  A6 ─► A8
Unit A (A1..A8)  ──────────────┐
084-S (089-F durable staging) ─┼─► B1 ─► B2 ─► B3 ─► B4 ─► 091.002-T
                                                       (release gate)
```

Shipment-level: **Unit-B shipment `blocks`-on Unit-A shipment** and **`blocks`-on 084-S**.

## 6. Fail-closed rules (invariant teeth)

Emit an edge **iff** the callee canonicalises to **exactly one** `function_meta.canonical_path`. `None`
(no edge, recall loss) for: external-crate qualifier (kills **F1**), glob-only binding, macro-generated
symbol, `pub use` cycle/depth-limit, ≥2 matches, trait-method/blanket-impl/`<T as Trait>::m`/associated
projection (out of scope), unresolvable `Self::Assoc::…`, primitive-type qualifier.

## 7. Release gate & proof obligations

Wired to 081-F retrieval-eval: resolution-**recall rises**; **every** `calls_resolved_canonical` edge
matches the fixture expected-edge manifest (**target-correctness = 1.0 / zero mis-resolved edges**);
aggregate false-edge rate is a supporting lower bound only. Proof obligations (all green before B flips
on): identity soundness, non-forgeability, generic convergence, external/glob/macro closure,
re-materialisation, identity-based gate (see spike §8).

## 8. Rollout / rollback

- **Rollout:** Unit A (precision-neutral) → Unit B (gated). Monitor re-index duration on the A8 bump
  (release-observability, per 087.001-T).
- **Rollback:** version-marker down-migration retracts `calls_resolved_canonical` edges and reverts to
  bare-name behaviour; additive columns droppable; JSONL additive + legacy-tolerant. No destructive
  rewrite of `name`, existing edges, or the 081-S/088-F manifest. **Rollback order (D10):** retract
  Unit-B `calls_resolved_canonical` edges **before** the Unit-A A8 format/version down-migration, so no
  canonical edges are orphaned.

## 9. Reconciliation with 088-F / 081-S / PR #248

Option C **supersedes** the blocked name/spelling qualified-resolution approach on `feat/088` and builds
on origin/main; it does **not** require PR #248 to merge, and re-derives the `Self::` subset correctly
under the unforgeable marker. The 081-S/088-F blocked manifest is **not mutated**; only informational
`related_to` links exist. If Ship prefers to retire PR #248, that is a separate Ship decision recorded on
088-F — this plan neither requires nor performs it.

## 10. Recommended execution order (relative to queued shipments)

`086-S` (fast concurrency-race closure) → `{083-S, 084-S, Unit-A}` in parallel/any order → `Unit-B`
(after Unit-A **and** 084-S) → `085-S` (last). 083-S/087.001-T lands the fingerprint pattern A8 mirrors
(soft benefit, not a hard dep). Existing 083/084/085/086 manifests are **unchanged** by this plan.
