---
title: "Same-file duplicate function-name shadowing — fail-closed direct-edge target (implementation plan)"
type: plan
date: 2026-07-27
source: docs/decisions/2026-07-27-ff7de872-same-file-shadowing-fail-closed-deliberation.md
deliberation_id: "014-D"
stash_id: FF7DE872
status: reviewed
requires_plan_hardening: true
governing_invariant: "013-D no-false-edge / 082-F target-correctness"
independent_of: ["FE8B3B2D", "096-F"]
tags:
  - code-graph
  - python
  - rust
  - call-graph
  - fail-closed
  - target-correctness
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

engram mints a **direct** `calls` edge whenever both caller and callee resolve
within the file being indexed. Resolution goes through `find_function_id`
(`src/services/code_graph.rs:2988`), which returns the **first** name match. When
a file declares two top-level defs of the same name (Python last-def-wins;
Rust same-name in different inline `mod` blocks per file), the bare call binds to
the **shadowed/earlier** def — a **wrong-target edge** (013-D / 082-F).

The two direct-edge minting sites are symmetric:

* **index path** — `code_graph.rs:~1644-1645`
* **incremental-sync path** — `code_graph.rs:~2522-2523`

The Python `is_contested` guard (`code_graph.rs:332`) already routes
import/rebind-shadowed callees to staging, but it does **not** detect two
same-name **defs** in the file's `function_ids`. Same-file bare calls are
resolved **before** any canonical/singleton post-pass, so 096-F's canonical
resolver cannot repair this (it fails closed on duplicate `canonical_path`, never
last-wins). See the deliberation for the full option analysis; **Option A
(fail-closed, language-agnostic)** is chosen.

## Normative Anchors

* **A1 (governing invariant):** zero false edges (013-D) and target-correctness
  (082-F) take precedence over recall. Recall on same-file duplicate-name calls
  is a **documented v1 non-goal**, not a rollback trigger.
* **A2 (language-agnostic fail-closed):** the fix must hold for **both** Rust and
  Python. No last-wins (Option B) — it is unsound for Rust module-per-file.
* **A3 (additive, minimal blast radius):** `find_function_id` stays
  byte-identical for its other 6 call sites (caller attribution @1617/@2497,
  the resolve path @2805). Introduce a new ambiguity-aware helper used **only**
  at the two direct-edge callee-resolution sites.
* **A4 (symmetry):** index (@1644) and sync (@2522) paths must behave
  identically — an ambiguous callee is staged/dropped in both.
* **A5 (mirror existing precedent):** the cross-file singleton post-pass already
  "skips ambiguous / unmatched names to bound false edges"
  (`code_graph.rs:1764-1766`); route the same-file ambiguous callee into that
  same fail-closed machinery via staging.
* **A6 (recall proof):** a Rust-path regression test must prove legitimate
  unique-name same-file calls still mint their direct edge (no recall
  regression).

## Design

1. Add `find_unique_function_id(ids, name) -> Result<Option<String>, Ambiguous>`
   (or an `Option<String>` returning `None` on ambiguity with a debug log +
   counter). Returns the id only when **exactly one** candidate matches; returns
   the ambiguous signal when `>1` matches; `None` when `0` match.
2. At the index minting site (@1644) and sync minting site (@2522), replace the
   **callee** `find_function_id(callee)` with the ambiguity-aware resolver. On
   ambiguity, **do not** mint a direct edge — route the call into the existing
   staging path (`put_staged_call_with_provenance`) so the cross-file post-pass
   handles it under its own fail-closed rules (A5), or drop fail-closed if
   staging is not applicable.
3. Apply the same ambiguity guard to the **caller** resolution (open question 1)
   so a same-file duplicate enclosing-function name does not mint an edge with an
   uncertain origin. Keep this within the two direct-edge sites only.
4. Observability: increment `cross_file_edges_dropped` (or a dedicated
   `same_file_ambiguous_dropped` counter) when a same-file ambiguous call is not
   minted, mirroring post-pass observability (open question 3).
5. Leave all Rust canonical / Python canonical resolution paths untouched.

## Units (2-hour rule; width-isolated; TDD)

### U1 — Failing regression harness (RED)  [domain: tests / code-graph]
Add failing unit tests that reproduce the wrong-edge bug:
* **Python** — a file with two top-level `def parse(...)` defs and a bare
  `parse()` call currently mints an edge to the **first (shadowed)** def; assert
  the corrected behavior: **no wrong-target edge** (fail-closed / staged).
* **Rust** — a file with `mod a { fn f() {} }` and `mod b { fn f() {} }` plus a
  call: assert the ambiguous same-file same-name callee does **not** mint a
  first-match direct edge.
* **Recall guard (RED-then-GREEN scaffold)** — a legitimate **unique**-name
  same-file call still resolves to its direct edge.
Compiling, asserting-the-target-identity, initially failing. ~3 scenarios.

### U2 — Ambiguity-aware resolver + guarded minting sites (GREEN)  [domain: code-graph]
Implement `find_unique_function_id`; wire it into the callee (and caller, per
open question 1) resolution at the index (@1644) and sync (@2522) minting sites;
route ambiguous same-file calls to staging / fail-closed; leave
`find_function_id` byte-identical elsewhere. Add the observability counter
increment. Make U1 green. ~4 functions/edits, symmetric across both sites.

### U3 — Acceptance + Rust no-recall-regression + docs  [domain: tests + docs]
Add an integration/acceptance test asserting exact target-identity on an
adversarial same-file duplicate-name corpus (Python + Rust) yields **zero**
false edges, and that the unique-name control still resolves (recall preserved,
A6). Document the v1 limitation (same-file duplicate-name effective call is
fail-closed, not last-wins) and the deferred Python-only last-wins follow-up in
the code-graph capability notes. ~3 scenarios.

Dependency order: **U1 → U2 → U3**.

## Plan Hardening (elevated blast radius)

`find_function_id` is a **shared cross-language consumer at 7 call sites**. This
elevates blast radius, so the plan is hardened as follows:

* **H1 — Isolation:** all behavior change is confined to an **additive** helper
  and the **two** direct-edge minting sites. `find_function_id` semantics are
  unchanged for caller attribution (@1617/@2497) and the resolve path (@2805).
  A regression assertion pins Rust singleton/canonical resolution as unchanged.
* **H2 — No language-specific unsoundness:** Option B (last-wins) is explicitly
  rejected because it is unsound for Rust module-per-file. The fix is a pure
  fail-closed narrowing — it can only **remove** a wrong edge or stage a call,
  never mint a new target.
* **H3 — Recall floor:** U3 certifies (target-identity gate) that every
  legitimate unique-name same-file call still resolves; recall regression on the
  unique-name control is a **release blocker**. Recall on the duplicate-name case
  is a documented non-goal.
* **H4 — Rollback trigger:** a confirmed wrong-target same-file edge after
  indexing, OR a recall regression on the unique-name control corpus → revert the
  guarded minting-site change (additive helper makes revert a one-site change).
* **H5 — Monitoring:** the `same_file_ambiguous_dropped` / `cross_file_edges_dropped`
  counter surfaces how often the guard fires; a nonzero-but-bounded value on the
  adversarial corpus is the positive signal; an unexpected spike on real repos is
  the investigation trigger.

## Plan Review (gate record)

Self-review + persona lenses (fail-closed / target-correctness, Rust-safety,
architecture-cohesion). Findings and resolutions:

* **[P1 — fail-closed] Caller-side ambiguity.** The first plan draft guarded only
  the callee. A same-file duplicate *enclosing-function* name also yields an
  uncertain edge **origin**. **Resolved:** U2 extends the guard to caller
  resolution at the same two sites (open question 1 → decided YES).
* **[P1 — architecture] Do not mutate the shared helper.** Changing
  `find_function_id` globally would ripple to caller attribution and the resolve
  path (@2805). **Resolved:** A3/H1 mandate an **additive** helper; the shared
  function is byte-identical elsewhere, with a Rust regression assertion.
* **[P2 — target-correctness] Reject last-wins for v1.** Option B is unsound for
  Rust module-per-file. **Resolved:** A2/H2; last-wins is a documented Python-only
  follow-up, not v1.
* **[P2 — observability] No counter for the new drop class.** **Resolved:** U2
  adds a counter increment mirroring the post-pass (open question 3 → decided).
* **[P3 — nit] Stash line refs drifted.** **Resolved:** all anchors re-verified
  against current code (`find_function_id` @2988, sites @1644 / @2522).

**Gate verdict:** PASS (0 open P0/P1). Ready for harvest.

## Definition of Done

* All 3 tasks done; ordered quality gates pass (`cargo fmt --all -- --check`;
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`;
  `cargo dev-test`; `cargo audit`).
* Acceptance: a same-file duplicate-name bare call (Python **and** Rust) mints
  **no wrong-target edge** (target-identity gate, zero false edges on the
  adversarial corpus).
* Recall parity: every legitimate unique-name same-file call still resolves to
  its direct edge (Rust + Python control corpus) — no recall regression.
* `find_function_id` unchanged for its other 6 consumers (regression assertion on
  Rust singleton/canonical resolution).
* Docs updated: v1 fail-closed limitation + deferred Python-only last-wins
  follow-up.
* Monitoring/rollback: same-file ambiguous drop counter present; rollback trigger
  = confirmed wrong-target same-file edge or unique-name recall regression.
