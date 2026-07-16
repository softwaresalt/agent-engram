---
title: "SPIKE 091.001-T — canonical module/type identity feasibility (GO)"
doc_type: decision
source: "091.001-T (spike) under 091-F (Option C); operator invariant decision 2026-07-15 19:03 -07:00"
description: >-
  Time-boxed feasibility spike proving whether per-file use-graph + module-path resolution +
  re-export tracing can yield canonical module/type identity for BOTH impl-method index names and
  call targets, on Engram's existing tree-sitter parser / CozoDB index / staged-call post-pass,
  without violating the absolute no-false-edge invariant (013-D). Verdict: GO (Rust-first).
topic: "Canonical identity feasibility for Option C qualified/method call resolution"
depth: "spike"
decision_status: "GO — feasible; proceed to full reviewed plan/backlog"
author: stage
date: 2026-07-15
linked_artifacts:
  - "091-F"
  - "091.001-T"
  - "091.002-T"
  - "088-F"
  - "089-F (084-S)"
  - "docs/decisions/2026-07-15-option-c-canonical-identity-deliberation.md"
  - "docs/closure/2026-07-15-088-rec1-call-resolution-adversarial-review.md"
  - "docs/decisions/decision-013 - Cross-File-Call-Edges-Deferred.md"
---

# SPIKE 091.001-T — Canonical Module/Type Identity Feasibility

- **Mode:** DARK_MODE Stage execution, isolated worktree `stage/stash-followups-2026-07-15` @ base `7346f86` (origin/main `df77584`).
- **Boundary:** Stage-only. No production/test code authored, no build, no PR. This is a planning artifact backed by read-only source inspection.
- **Invariant (authoritative, kept):** *Recall recovery must never create a false or mis-resolved `calls_resolved_singleton` edge* (013-D). Operator resolved the prior blocker in favour of **absolute precision**; **no downgrade to best-effort**.

## Verdict: **GO (Rust-first)**

Canonical module/type identity is **feasible** on Engram's existing substrate without unsafe
assumptions, **provided** every edge is created under a strict *fail-closed* discipline (an edge is
emitted only when a call canonicalises to **exactly one** workspace symbol identity; every ambiguity,
external, glob, macro, or unresolved case drops to **no edge / recall loss**). The four mandatory
safeguards are each implementable against named code sites, and the durable-staging substrate
(084-S / 089-F) is the correct layer to build upon. Scope is **Rust-first**; this is *not* a
cross-language parity feature (see §9).

If the fail-closed discipline or the identity-based precision gate (§8) cannot be demonstrated green
on the adversarial fixtures, the feature must **stay blocked** — but the evidence below shows a sound
path exists, so the spike is marked **complete** and Option C proceeds to a full reviewed plan.

---

## 1. Evidence base (read-only inspection, origin/main `df77584`)

Authoritative substrate reconciliation (important): **none of the 088 qualified-resolution work is
on origin/main.** All 15 commits (`574f434 … 4b68c3f`) live on the blocked `feat/088-rec1-call-resolution`
branch (PR #248, open). What ships on `df77584` is only the **rec1 bare-name** subsystem (078-S / 082-F):

| Concern | Site (origin/main) | Current behaviour |
|---|---|---|
| Rust extraction | `src/services/parsing/rust.rs` | `extract_impl` indexes impl methods as **source-spelling** `format!("{ty}::{}", name)` (L197). `use` declarations extracted as raw `Imports{import_path}` text (L77-81, `extract_use_path` L210). `resolve_call_name` reduces `a::b()` → final segment `b`, flags `is_qualified` (L264-272); `x.foo()` → `field`, flags `is_method` (L280-286). |
| Call staging | `src/services/code_graph.rs` L474-504, L1129-1152 | `if *is_method \|\| *is_qualified { continue; }` — qualified/method calls are **dropped before staging**. Bare calls: in-file → `create_calls_edge` (direct); else `put_staged_call`. |
| Post-pass resolver | `src/db/cozo_queries.rs` `reresolve_calls_edges` L1701-1759 | Builds `name → [id]` from `function_meta{id,name}`; a staged callee with **exactly one** name match → `calls_resolved_singleton` edge; 0 or ≥2 → retract. Purely **string-name** matching. |
| Provenance | `staged_call { caller_id, callee_name, source_file => created_at }` (schema.rs L684) | Only bare final-segment `callee_name`; **no raw qualifier, no module context**. Not exported to JSONL (089-F closes this). |
| Change detection | `src/services/ingestion.rs` L316-330 (`file_hash` compare → `unchanged += 1`) | A **content-unchanged** file is skipped, so a **format-only** index change never re-materialises on existing DBs (safeguard #4 rationale). |
| Fingerprint home | `schema_meta { key => value }` (schema.rs L701) | Durable KV already used for migration state; natural home for an index-format version. |
| Migration precedent | `migrate_calls_edge_resolution` (`:replace calls_edge {…, resolution}`), 086.001-T `run_script_retrying` | Proven additive-column `:replace` migrate/rollback pattern. |
| Fingerprint precedent | 083-S / 087.001-T ("DAX index-format-version fingerprint + one-time re-index") | Established pattern for exactly safeguard #4. |

**Root cause the invariant must defeat (from 081-S halt + 088 review):**
- **RMeJ0** — impl methods carry the **impl source spelling**, so `impl crate::a::Widget` vs an unrelated
  `b::Widget` mis-resolve; same spelling of different types collide.
- **F1 (P0/HIGH 4/4)** — a lowercase qualifier (`mem::swap`, `str::parse`) collapses to the **bare**
  callee and singleton-resolves to a unique unrelated free fn → a **new** false edge.
- **F2** — a flat body walk rewrites `Self::h()` across nested-impl boundaries to the wrong enclosing type.
- **F4** — the eval gate measures precision by `count_dangling_calls_edges`; an F1 edge targets a **real**
  (wrong) fn, so it is never dangling → the gate is **structurally blind** to mis-resolution.
- **F6** — the Type route is name-only, so `use ext::Widget as Alias; Alias::build()` matches any local
  `Alias::build` by name.

## 2. Why canonical identity is feasible on this substrate

Three enabling facts make the new capability tractable:

1. **A deferred post-pass already exists** (`reresolve_calls_edges`). Canonical resolution is a *drop-in
   replacement for the string-name match* inside the existing post-pass — no new pipeline stage.
2. **`use` declarations are already extracted** (just dropped per 013-D). The use-graph is an
   *enrichment* of an existing extraction path, not a greenfield parser.
3. **Rust module identity is static and deterministic.** A file's module path is a pure function of the
   crate root + filesystem layout + inline `mod` blocks; `self`/`super`/`crate` roots resolve against it
   with no type inference. Canonical identity needs **name/path resolution, not type checking.**

The one genuinely new capability — the **use-graph + module-path + re-export closure** — is bounded and
memoisable (§7). Nothing here requires borrow/trait solving or macro expansion; those are handled by
**fail-closed** (§6), which is precisely what preserves the invariant.

## 3. Target architecture (canonical identity as an *additive* identity surface)

**Design decision (blast-radius isolation):** canonical identity is stored as a **new, separate field**,
never by overwriting the existing `function_meta.name`. Existing name-based subsystems (search,
`references_edge` resolution via `resolve_reference_target`, bare-name singleton resolution, JSONL
display) are therefore **unchanged**. Only the new canonical resolver reads the new field.

Pipeline (Rust files only; all other languages unaffected):

```
parse ─► (A) module-path derivation ─► (B) per-file use-graph ─► (C) canonical resolver (pure, fail-closed)
                                                                        │
 index defs ─► (D) canonical impl-method identity  ────────────────────┤
 index calls ─► (E) stage qualified/method calls w/ raw provenance ─────┘
                                                                        ▼
 post-pass ─► (F) canonical singleton resolution ─► calls_resolved_canonical edge  (gated by (G) identity eval gate)
 open/index ─► (H) index-format fingerprint ─► one-time forced re-index on version bump
```

- **(A) Module tree** — derive `crate::a::b` per file from crate root (`lib.rs`/`main.rs`) + directory/file
  layout (`foo.rs` / `foo/mod.rs`) + inline `mod` extension. Deterministic; workspace-crate aware.
- **(B) Use-graph** — per file, map `alias → canonical path` from `use` trees: single, groups
  `{a,b}`, `as` aliases, `self`/`super`/`crate` roots. Record `pub use` (re-export) edges and `use *`
  **glob** markers separately.
- **(C) Canonical resolver** — pure function `(module_path, use_graph, path_expr) → Option<CanonicalId>`.
  Resolves in-module names, `self/super/crate` roots, imported aliases, and transitive `pub use`
  re-exports (bounded closure). Returns `None` (fail-closed) for external crates, glob-only bindings,
  macro-generated names, cycles, depth-limit, or any ambiguity.
- **(D) Canonical impl-method identity** — canonicalise the `impl <Type>` type via (A)+(C) → index each
  method's identity as `crate::a::Widget::method` in the **new** `function_meta.canonical_path` field
  (source `name` unchanged). Fixes RMeJ0.
- **(E) Staged provenance** — stop dropping qualified/method calls; **stage** them carrying the **raw
  qualifier**, a **qualifier kind** (`module`/`type`/`self`/`method`), and the **enclosing impl canonical
  type** (for `Self::`). Additive `staged_call` columns, legacy-tolerant (mirrors 089-F F10).
- **(F) Canonical singleton resolution** — in the post-pass, canonicalise each staged qualified/method
  callee via (C)+(§5 generics)+(§4 Self) and match against `function_meta.canonical_path`; **exactly one**
  match → edge tagged `calls_resolved_canonical`; else no edge. Bare-name path is unchanged.
- **(G) Identity eval gate** — adversarial collision/alias/re-export fixtures asserting each resolved
  edge targets the **correct** canonical def or **none** (fixes F4). Precision + recall release gate.
- **(H) Fingerprint** — `schema_meta["code_index_format_version"]`; on bump, invalidate `file_hash` for
  code sources → one-time forced re-index so `canonical_path` populates (fixes safeguard #4).

## 4. Safeguard #2 — unforgeable `Self` representation

`Self` is represented internally as a **sentinel that source text cannot produce** — e.g. an enum
variant `Qualifier::SelfType` (preferred, type-level) or a reserved non-lexable string token. The
sentinel is set **only** when the parser observes the `Self` keyword node at a scoped-path root inside
an impl; the resolver substitutes the **enclosing impl's canonical type** (from (A)+(D)). Because the
marker is a typed value, not a string, a source qualifier such as `Self::Assoc::method()` **cannot forge
it** — the `Assoc` projection resolves through canonical identity or **fails closed**. The body walk that
assigns the enclosing type is made **scope-aware** (stops at nested `impl_item`/`function_item`), closing
F2. `Self::` outside an impl → no enclosing type → `None` (safe, as today).

## 5. Safeguard #3 — generic normalisation

A single normalisation routine strips/placeholders generic arguments so **definition and call spellings
converge**: `Type<T>::method`, `Type::<T>::method` (turbofish), `Type<'a>::method`, `Type<A,B>::method`
all normalise to the same canonical `…::Type::method`. Applied symmetrically at (D) indexing and (F)
resolution so a generic def is never split from its call, and a monomorphised call never mis-binds.

## 6. Ambiguity / fail-closed rules (the invariant's teeth)

An edge is emitted **iff** the callee canonicalises to **exactly one** `function_meta.canonical_path`.
Every one of the following yields **`None` → no edge (recall loss, precision-safe)**:

| Case | Rule |
|---|---|
| External crate qualifier (`std`, deps: `mem::swap`, `tokio::spawn`) | Leading segment is **not in the workspace crate-name set** (enumerated from crate roots / Cargo metadata) → `None`. **Directly kills F1.** |
| Glob-only binding (`use a::*` then bare `X`) | Cannot enumerate exports without full resolution → `None`. |
| Macro-generated symbol/callee | Invisible to tree-sitter → `None`. |
| `pub use` re-export cycle / depth-limit exceeded | Cycle-detected closure aborts → `None`. |
| ≥2 canonical matches (incl. `#[cfg]`-duplicated defs) | Ambiguous → `None`. |
| Trait-method / blanket-impl / `<T as Trait>::m` / associated-type projection | Requires trait solving → **out of scope, `None`** (deferred increment). |
| Unresolvable `Self::Assoc::…` | `None`. |
| Primitive-type qualifier (`str::parse`, `u32::from`) | Not a workspace type → `None`. |
| Non-default module mapping — `#[path="…"]` mod, `#[cfg]`-gated mod (D1) | Honour `#[path]`; otherwise **`None`** rather than guess. |
| `use`/local-item **shadowing** or module shadowing an external crate root (`mod std`) (D6/D7) | Apply Rust precedence (explicit item/`use` > glob); ambiguous or external-root shadow ⇒ `None` unless `::`/`crate::` disambiguates. |
| **Empty** `canonical_path` (non-Rust / legacy / unresolved def) (D4) | Never a candidate match target; filtered before the singleton test. |
| Arbitrary-receiver method call `x.foo()` (unknown receiver type) (D2) | **Out of scope** — no type inference → `None`. |

This makes false edges **structurally impossible**: every edge is backed by a *proven* single canonical
identity. Recall is recovered only where identity is certain; everything uncertain is dropped.

**Method-call scope (D2, post-review):** Option C resolves **path-qualified** calls
(`module::`/`crate::`/`self::`/`super::`/`Type::`/`Self::`). Receiver method calls are resolved **only**
when the receiver type is statically known — i.e. `self.m()` / `Self::m()` inside an impl (enclosing
canonical type known via §4). General `x.foo()` stays deferred/dropped. The recall claim is scoped to
qualified + self-receiver calls, not arbitrary method dispatch.

## 7. Performance bounds

- Module-tree derivation: `O(files)`, one-time per index.
- Use-graph build: `O(use-decls)` during parse (already visiting these nodes).
- Re-export closure: over `pub use` edges only (a small fraction of use-decls), transitive with
  memoisation + cycle detection → effectively `O(pub-use-edges)`; worst realistic workspace = low
  thousands → negligible.
- Post-pass: same `O(staged_calls)` as today plus `O(1)`-ish canonicalisation (hashmap lookups + a
  bounded path walk) per call → **no asymptotic regression**.
- Fingerprint re-index: one full parse pass on version bump only — identical cost to an initial index,
  which the daemon already performs; amortised to zero on steady state. Monitor re-index duration as a
  release-observability signal (same treatment as 087.001-T).

## 8. Evaluation metrics & proof obligations for the invariant

**Proof obligations (must all hold before edges flip on in the enabling increment):**

1. **Identity soundness** — every `calls_resolved_canonical` edge's `(caller → callee)` corresponds to a
   canonical identity that resolves to **exactly one** workspace def (unit + property tests on the
   resolver; construct-only-single-match invariant).
2. **Non-forgeability** — no source string reproduces the `Self` sentinel (adversarial `Self::Assoc::…`).
3. **Generic convergence** — def/call generic-spelling variants map to one identity (paired fixtures).
4. **External/glob/macro closure** — the F1/F6 family (`mem::swap` + unique free `swap`; `Alias::build`
   via `use … as`; glob-imported collision) asserts **zero** edges to the wrong target.
5. **Re-materialisation** — a fingerprint bump forces re-index and populates `canonical_path`; an
   unchanged workspace on the *same* version is skipped (no needless churn).
6. **Identity-based gate (fixes F4)** — the eval gate asserts resolved edges hit the **correct** def or
   none, on an adversarial collision fixture that would pass the old `dangling==0` signal but fail here.

**Release gate (wired to the 081-F retrieval-eval harness):** graph resolution-**recall must rise**;
every `calls_resolved_canonical` edge must match the fixture expected-edge manifest
(**target-correctness = 1.0**, i.e. **zero** mis-resolved edges); aggregate false-edge rate is a
supporting lower bound only, never the sole gate.

## 9. Cross-language scope (explicit)

**Rust-first, not generic parity.** Canonical identity here is intrinsically Rust module-system
semantics (mod tree, `use`, `pub use`, `impl`/`Self`, turbofish generics). The *substrate* — staged_call,
post-pass, `calls_edge{resolution}`, fingerprint — is language-agnostic, but **per-language
canonicalisation is separate work**. Python/TS/C#/Go/etc. retain today's bare-name singleton behaviour
and are **untouched** by Option C. A `ModuleResolver`-style seam may be introduced with a Rust impl to
leave hooks for future languages, but Option C **ships Rust only** and must not imply parity.

## 10. Migration / rollout / rollback

- **Migration:** additive `:replace` on `function_meta` (+`canonical_path`) and `staged_call` (+raw
  qualifier, +kind, +enclosing type), via the proven `run_script_retrying` migrate/rollback pattern;
  `schema_meta` version marker gates a one-time forced re-index (087.001-T precedent).
- **Rollout:** ship in two release units (see plan) — infrastructure (precision-neutral: no new edges)
  first, then the gated "flip-on".
- **Rollback:** version-marker down-migration retracts `calls_resolved_canonical` edges and reverts to
  bare-name behaviour; additive columns are droppable; JSONL format is additive + legacy-tolerant. No
  destructive rewrite of `name`, existing edges, or the 081-S/088-F manifest.

## 11. Dependency & reconciliation findings

- **084-S / 089-F must precede Option C.** Canonical re-resolution **consumes** staged_call rows in the
  post-pass and must survive daemon restart/upgrade; Option C also **extends** the staged_call schema, so
  building on 089-F's durable+additive JSONL format avoids double format churn. **Encode as a hard
  dependency** (not prose).
- **088-F / 081-S (PR #248) relationship:** Option C's canonical approach **supersedes** the blocked
  name/spelling qualified-resolution approach on `feat/088`. Option C builds on **origin/main**; it does
  **not** require PR #248 to merge, and it re-derives the Self:: subset correctly under the unforgeable
  marker. The blocked 081-S/088-F manifest is **not mutated** here (informational links only).
- **091.002-T** (reconcile 088.005-T archived-done vs blocked full-resolver acceptance) becomes
  adjudicable **only after** the canonical resolver + identity gate land; it now depends on the enabling
  increment, still without touching 088.005-T or the 081-S manifest until a formal resumption.

## 12. Open items carried into the plan (not blockers)

- The exact seam for code-source change-detection vs the `file_hash` path (code sources bypass the
  content-ingestion skip at ingestion.rs L96-98) — verify during the fingerprint task.
- Whether an in-crate-only subset (`crate`/`super`/`self` roots, no re-export closure) ships as an
  independently sound first slice ahead of full `pub use` tracing (deliberation open question) — **yes**,
  captured as an ordered slice in the plan.

**Conclusion: GO.** Proceed to the full reviewed implementation plan, decomposition, hardening, and
adversarial review.
