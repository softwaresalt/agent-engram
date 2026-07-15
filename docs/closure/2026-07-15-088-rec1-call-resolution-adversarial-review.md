---
title: "Adversarial multi-model review — 088-F rec1 qualified-call resolution (shipment 081-S)"
type: closure
date: 2026-07-15
slug: 088-rec1-call-resolution-adversarial-review
subject_commit: 4b68c3ffaa26554d9ad3769f3dffaf17633e1715
subject_base: a6b09258f0fbe8736e37dc712dd10604df41d58e
subject_branch: feat/088-rec1-call-resolution
scope: src tests Cargo.toml
reviewers: 4
review_models:
  - reviewer-a: claude-opus-4.8 (Tier 3 frontier)
  - reviewer-b: gpt-5.6-sol (Tier 3 frontier)
  - reviewer-c: gemini-3.1-pro-preview (Tier 2)
  - reviewer-d: claude-sonnet-4.6 (Tier 2)
verdict: BLOCK
gate_blocking: true
---

# Adversarial Review — 088-F rec1 Qualified-Call Resolution

- **Date:** 2026-07-15
- **Branch:** `feat/088-rec1-call-resolution`
- **Diff scope:** `git diff a6b0925..HEAD -- src tests Cargo.toml`
- **Commits:** 574f434 (test harness), e446211 (module resolver), 927aa4b (Type resolver), 2dab773 (eval gate)
- **Shipment / feature:** 081-S / 088-F, deliberation 013-D Option A
- **Gate:** operator-mandated pre-PR gate; **precision invariant is the primary, non-negotiable concern**
- **Mode:** report-only (no files modified)

## Verdict: **BLOCK**

Under the stated **absolute** precision invariant ("recall recovery must never come at the cost
of a false/mis-resolved edge"), this change introduces a **new, verified false-edge path** on
everyday standard-library idioms. All four independent reviewers reached **BLOCK** on the same
root cause, and I independently verified it against source. The change is otherwise well-built:
the Type route is precision-safe and well-tested, and the persistence/retraction/rehydration
axis is correct. If the team elects to **downgrade** the invariant to "best-effort precision,
bounded by name collision," this degrades to **SHIP-WITH-FIXES** — but that is a design decision
the operator must make explicitly, not a default.

**Minimum to unblock:** F1 (gate the module route), F3 (add the missing negative precision test),
F4 (make the eval gate able to detect mis-resolution-to-a-real-target). F2 and F5 should ride the
same PR.

## Reviewer panel (genuine multi-model diversity)

| Reviewer | Model | Family | Tier | Verdict |
|---|---|---|---|---|
| A | `claude-opus-4.8` | Anthropic | 3 (frontier) | BLOCK |
| B | `gpt-5.6-sol` | OpenAI | 3 (frontier) | BLOCK |
| C | `gemini-3.1-pro-preview` | Google | 2 | BLOCK |
| D | `claude-sonnet-4.6` | Anthropic | 2 | BLOCK |

Consensus rule: HIGH = flagged by all 4; MEDIUM = flagged by 3; LOW–MED = flagged by 2;
LOW = flagged by 1. Severity conflicts resolved to the most conservative, with the spread noted.
Every finding below was **cross-checked by the aggregator against the actual source** (not taken
on the reviewers' word).

## Consensus findings table

| ID | Sev | Conf | Location | Issue (verified) | Remediation |
|---|---|---|---|---|---|
| **F1** | **P0** | **HIGH (4/4)** | `code_graph.rs:1545-1547, 1560-1568` → realized in `cozo_queries.rs:1734-1748` | Lowercase-first qualifier (external module **or** primitive type) is routed to the **bare** callee, then singleton-resolved against the global name index. `mem::swap()`, `cmp::min()`, `fs::read()`, `u32::from_str_radix()`, `str::parse()`, `tokio::spawn()` collapse to `swap/min/read/from_str_radix/parse/spawn`; if the workspace defines exactly one free fn of that name, a **false `calls_resolved_singleton` edge** is created. Pre-088 these calls were `continue`-dropped, so this is a **new** false edge. Strictly less safe than bare-name resolution: the qualifier was written precisely to escape the local namespace, and the code discards it. | Gate the module route: only collapse to bare name when the path root is `crate`/`super`/`self` (in-crate) or the immediate qualifier is a **known workspace module**; otherwise drop (recall loss, safe). Route primitive-type qualifiers (`str`,`u32`,`char`,`bool`,`usize`,…) to `Type::method` (or drop) so they cannot match a free fn. |
| **F2** | **P1** (spread P0–P2) | **MEDIUM (3/4)** | `parsing/rust.rs:~232-258` | `extract_calls_from_body` is a **flat full-subtree stack walk** that never stops at nested `impl_item`/`function_item` boundaries and carries one outer `enclosing_type`. A nested `impl Inner { fn g(){ Self::h(); } }` inside a method of `impl Outer` rewrites `Self::h()` → `Outer::h` and attributes it to the outer caller; if `Outer::h` exists, a **mis-resolved edge**. New vector: pre-088 `Self::` was dropped. Trigger is narrow (nested impl inside a fn body + same-named outer method). | Make the walk scope-aware: stop at nested `impl_item`/`function_item`, or re-derive `enclosing_type`/caller when entering a nested impl. |
| **F3** | **P1** | **MEDIUM (3/4)** | `tests/integration/calls_qualified_resolution_test.rs:196-222` | Scenario 5 guards only the **uppercase/Type** fallback (`Thing::parse()` must not hit free `parse`). There is **no symmetric negative test** for the lowercase/module/primitive route (`str::parse()` / `mem::swap()` must not hit a unique free `parse`/`swap`). The green suite gives false confidence exactly at the precision boundary; such a test would **fail today**, exposing F1. | Add a scenario: file A calls `mem::swap(...)` (or `str::parse(...)`), file B defines a unique unrelated `pub fn swap`/`parse`; assert `singleton_count == 0` and no edge targets it. |
| **F4** | **P1** | **MEDIUM (2/4, aggregator-confirmed)** | `tests/integration/calls_qualified_eval_gate_test.rs:89-93,170-182` | The release gate's precision signal is `count_dangling_calls_edges` — edges whose target has **no** definition. An F1 false edge targets a **real** (wrong) function, so it is **not dangling** and the gate reports `false_edge_rate 0.00 → 0.00` regardless. The gate certifies **recall recovery** but is **structurally blind** to mis-resolution-to-a-real-target — i.e. blind to the exact P0 class it is named to guard. The `0.50→0.75` recall lift is legitimate; the "precision preserved" claim is not established by this artifact. | Add an **identity-based** precision assertion: a negative fixture where a lowercase-qualified call collides with a unique unrelated free fn, asserting the resulting singleton (if any) targets the **correct** def or none. Do not treat `dangling == 0` as precision evidence. |
| F5 | P3 | MEDIUM (2/4) | `cozo_queries.rs:171`; `schema.rs:601-603` | `StagedCall.callee_name` is documented as "bare name … as it appears at the call site," but now stores the **normalized** name (`Type::method` for type-qualified calls, path-stripped bare for module calls). Doc-only; the stored value is functionally correct. | Update the doc to "resolved workspace index name used by exact-name re-resolution and retraction." |
| F6 | P2 (spread P0–P3) | LOW–MED (2/4) | `code_graph.rs:1561-1563` | Type route is name-only: `use ext::Widget as Alias; Alias::build()` (or an uppercase module `mod Util`) matches any workspace `Alias::build` / `Util::process` impl method by name, ignoring imports/aliases. Same root cause as F1 but on the Type route; **narrower** (needs a same-named local type + method). Reviewer A argued the uppercase directions are mostly recall-safe; B/D flagged the alias case as a real false edge. | Document the limitation; longer-term, resolve imports/aliases or maintain a workspace `type_names` set and gate `qualifier_is_type` on membership rather than case alone. |
| F7 | P3 | LOW (1/4) | `retrieval_eval.rs:~318-340` (`count_call_sites`) | Denominator dedups by `(caller, resolved-target-name)`; `helper()` and `ext::helper()` collapse to one relation. This keeps recall ≤ 1.0 and commensurable with the `(from,to)` numerator (**no recall>1.0 bug** — aggregator checked), but it **masks** a mis-resolution inside the recall number (reinforces F4). | No math change required; treat as corroborating evidence for F4's identity-based gate. |
| F8 | P3 | LOW (1–2/4) | `code_graph.rs:1546`; `parsing/rust.rs:380` | `char::is_uppercase` on the first scalar classifies non-ASCII identifiers by Unicode case; `is_some_and` safely handles an empty qualifier and `scoped_call_qualifier` uses `unwrap_or(path)` — **no panic path** (verified). Minor determinism nit. | Optional: use `is_ascii_uppercase`; make `scoped_call_qualifier` return `None` on a missing `name` field instead of falling back to the whole path node. |
| F9 | P3 | LOW (1/4) | `tests/integration/calls_qualified_resolution_test.rs:105` | `assert_single_edge_to` asserts `edges.len() == 1` (whole-workspace count). Correct for the isolated two-file fixtures but fragile if reused with multi-caller fixtures. | Assert `edges.contains(&(caller_id, target_id))` instead of constraining the total count. |

## Verified-correct (credit where due — counters over-claims)

These were probed and found **sound**; no action needed:

- **Retraction / normalized name consistency.** `retract_singleton_edge_from_caller_by_name`
  (`cozo_queries.rs:1507-1531`) matches on `function_meta.name`. Impl methods are indexed as
  `Type::method` (`rust.rs` extract_impl: `func.name = format!("{ty}::{}", func.name)`), so
  staging the **normalized** `Type::method` name is **required** for retraction-by-name to match
  the stored edge. Storing the bare name here would have *broken* revalidation. The change is
  correct on this axis.
- **Rehydration / rollback / idempotency.** `reresolve_calls_edges` retraction is scoped to
  currently-staged callers (comment `cozo_queries.rs:1696-1700`), so JSONL-restored edges without
  staged rows are preserved. The unchanged post-pass resolves `Type::method` names fine (they exist
  in `function_meta.name`), and a rollback to bare-name staging remains graceful. No corruption.
- **`Self::` outside an impl** → `enclosing_type = None`, qualifier stays `"Self"` → target
  `Self::foo`, which matches no index name → no edge (safe recall loss).
- **Blocklist mitigation.** `CALL_BLOCKLIST` (`new/default/into/clone/from/unwrap/expect/ok/err`)
  neutralizes the most common associated-fn collision names (`T::from`, `T::new`, `T::default`),
  shrinking F1's surface — but `swap/min/max/read/write/parse/replace/take/spawn/copy/from_str*`
  remain exposed.
- **Turbofish / enum variants** (`Vec::<u8>::new`, `Enum::Variant`) route to names with no impl-method
  match (or are blocklisted) → no edge (safe).

## Remediation plan (ordered by confidence × severity)

1. **[F1 · manual · P0/HIGH]** Close the lowercase-qualifier → bare-name false-edge path. Gate the
   module route to in-crate roots / known modules; route primitive-type qualifiers to `Type::method`
   or drop. **Release blocker.**
2. **[F3 · gated_auto · P1/MED]** Add the lowercase/module/primitive **negative precision test**
   (fails today; becomes the regression guard for F1).
3. **[F4 · manual · P1/MED]** Add an **identity-based** precision assertion to the eval gate so it
   can detect mis-resolution-to-a-real-target; stop treating `dangling == 0` as precision evidence.
4. **[F2 · gated_auto · P1/MED]** Make `extract_calls_from_body` scope-aware for nested impls so
   `Self::` cannot be rewritten to the wrong enclosing type.
5. **[F5 · safe_auto · P3/MED]** Fix the `StagedCall.callee_name` doc.
6. **[F6 · manual/advisory · P2/LOW-MED]** Document (or index-gate) the Type-route alias/uppercase-module
   limitation.
7. **[F7/F8/F9 · advisory · P3/LOW]** Metric-masking note, ASCII/`None`-fallback hardening, test-helper
   robustness.

## Backlog work items (P0/P1)

```yaml
- type: bug
  title: "Precision invariant: lowercase-qualified calls collapse to bare name and mis-resolve to a unique free fn"
  description: >
    Lowercase-first qualifiers (external modules like mem/cmp/fs/tokio and primitive types like
    str/u32/char) route to the bare callee in qualified_target_name, then reresolve_calls_edges
    creates a calls_resolved_singleton edge to a same-named unique free function. e.g. mem::swap()
    with a local `fn swap` -> false edge. Pre-088 these calls were dropped, so this is a new false
    edge and a violation of the absolute precision invariant.
  file: src/services/code_graph.rs
  line: 1566
  severity: P0
  confidence: HIGH
  fix: >
    Only collapse module-qualified calls to the bare name for in-crate roots (crate/super/self) or a
    known workspace module; route primitive-type qualifiers to Type::method or drop. Otherwise defer.
  linked_review: docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md

- type: test
  title: "Add negative precision test for lowercase/module/primitive-qualified calls"
  description: >
    calls_qualified_resolution_test.rs Scenario 5 covers only the uppercase/Type fallback. Add a
    symmetric scenario: mem::swap() (or str::parse()) alongside a unique unrelated free fn, asserting
    singleton_count == 0. This test fails until the F1 fix lands.
  file: tests/integration/calls_qualified_resolution_test.rs
  line: 222
  severity: P1
  confidence: MEDIUM
  fix: Add scenario asserting zero singleton edges for lowercase-qualified collisions.
  linked_review: docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md

- type: bug
  title: "Eval gate is structurally blind to mis-resolution-to-a-real-target"
  description: >
    The release gate measures precision via count_dangling_calls_edges (edges to non-existent defs).
    An F1 false edge targets a real (wrong) function, so it is never dangling; false_edge_rate stays
    0.00. The gate certifies recall recovery but cannot detect the P0 class it is named to guard.
  file: tests/integration/calls_qualified_eval_gate_test.rs
  line: 180
  severity: P1
  confidence: MEDIUM
  fix: Add an identity-based precision assertion with an adversarial collision fixture.
  linked_review: docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md

- type: bug
  title: "Self:: rewrite applies the wrong enclosing type across nested impl boundaries"
  description: >
    extract_calls_from_body walks the whole body subtree with one enclosing_type, so Self::h() inside
    a nested `impl Inner` within a method of `impl Outer` is rewritten to Outer::h and can mis-resolve.
    Narrow trigger (nested impl in a fn body + same-named outer method) but a new false-edge vector.
  file: src/services/parsing/rust.rs
  line: 242
  severity: P1
  confidence: MEDIUM
  fix: Make the traversal scope-aware; stop at nested impl_item/function_item or re-derive context.
  linked_review: docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md
```

## Notes on method

- 4 reviewers across 3 model families (Anthropic, OpenAI, Google), tiers 2–3, all high reasoning
  effort, each returning structured JSON findings independently.
- Aggregator (this agent) re-derived every P0/P1 against source: `code_graph.rs:1545-1593`,
  `cozo_queries.rs:1507-1531 / 1701-1758`, `parsing/rust.rs` extract_calls_from_body / extract_impl /
  scoped_call_qualifier, `retrieval_eval.rs` count_call_sites, and all three new test files.
- No file was modified. fmt/clippy/affected-area tests reported green by the operator and not re-run
  (live-daemon hang risk).
