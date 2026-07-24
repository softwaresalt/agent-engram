---
title: "Python module-namespace-qualified call resolution — feasibility spike"
type: spike
date: 2026-07-23
time_box: "2h"
conclusion: "go"
confidence: "high"
stash_id: "FE8B3B2D"
layers_on: "094-F"       # Python Calls-edge extraction + U3 language-scoping (merged)
extends: "091-F"         # Option C canonical identity + import-aware resolution (merged)
sequences_after: "094-F"
independent_of: ["090-S", "095-F"]   # operator-ratified: NO blocking dependency
related_bug: "FF7DE872"  # same-file same-name shadowing — SPLIT OUT, independent, NOT subsumed
deliberation: "Option B ratified — product value operator-asserted; proceed via technical-feasibility spike"
tags:
  - "code-graph"
  - "python"
  - "canonical-identity"
  - "call-graph"
  - "namespace-resolution"
  - "fail-closed"
  - "013-D"
---

## Goal

Confirm — against the **real current code**, not the drifted anchors in the stash
— whether engram's existing Rust-built canonical-path resolution (091-F) can be
extended to Python's module namespace to disambiguate **cross-module same-name
calls**, without introducing new false-edge risk, and within a normal feature
envelope.

Resolution rule under test (module namespace: `foo/bar.py` → `foo.bar`):

* bare call `name()` **defined in module `M`** → canonical `M.name`
* else **imported** via `from N import name` → canonical `N.name`
* else (star import / relative / package-root / re-export / dynamic) → **DROP**

> **⚠️ Refined (C7-4/F3) — "DROP" = no canonical edge, legacy edge may remain.** In the
> final implementation contract (plan + `096-F`), **DROP** means **no canonical
> module-qualified edge**; a **unique** cross-file bare call still keeps its **legacy
> name-only edge** (T5b's F3 no-target fallback — recall preserved). Only the canonical
> layer drops; a non-unique name still fails closed. This qualifies every "DROP" in this
> spike, including the Fail-closed matrix below (see the matching note there). The spike's
> initial "dropped outright" phrasing is superseded by the T5b/F3 legacy-edge contract.

## Scope boundary (FROZEN — do not let planning creep)

* **IN** — module-level functions and `module.func()` disambiguation (the
  same-name-different-module case).
* **OUT (documented non-goal)** — instance-method dispatch (`self.method()` /
  `obj.method()`): requires type inference. Not in this feature.
* **Related bug FF7DE872** (same-file same-name shadowing; direct-edge /
  source-order / last-wins fix) is **SPLIT OUT and INDEPENDENT** — NOT subsumed
  here, NOT in scope. See "Why FF7DE872 is not fixed here" below.

## Investigation approach

Read-only inspection of the live tree on `stage/py-namespace` (off `main`
`6d6c9d9c`). Every anchor claimed in stash `FE8B3B2D` was re-verified against
current line numbers; drift is recorded. No parser, schema, or resolver code was
modified during the spike.

## Findings — verified against current code

### The Python parser already emits Calls edges (094-F merged) and captures imports as a FLAT string

`src/services/parsing/python.rs` has advanced past the 094-F spike doc: it now
emits `ExtractedEdge::Calls` via `extract_calls_from_body` +
`resolve_call_name` + `PYTHON_CALL_BLOCKLIST`.

* Bare `foo()` (`function` = `identifier`) → `is_method:false` → promoted (direct
  edge or bare-name staging).
* Attribute `obj.foo()` / `self.bar()` / `mod.func()` (`function` = `attribute`)
  → `is_method:true` with an **empty** `raw_qualifier` (python.rs:309-321) → the
  consumer's `should_stage_provenance_call(true, false, "")` returns `false`
  (code_graph.rs:189-190) → **dropped, fails closed**. This is exactly the
  `module.func()` case this feature must instead *route into canonical staging*.
* Imports: `extract_import` (python.rs:141-143) returns **only the full node
  text** as a flat `import_path` string — confirmed. There is **no symbol-level
  import-binding table** (no Python analogue of Rust's `UseGraph`). This is the
  extraction gap.

### The canonical DB layer is already language-agnostic — REUSE, no schema change

* `function_meta.canonical_path` column exists (cozo_queries.rs:924-940 upsert;
  `:put` at 940). It is additive and `""` for non-Rust/unresolved defs.
* `function_ids_by_canonical_path()` (cozo_queries.rs:1046) returns a
  `canonical_path → [id]` index, filtering empty paths (D4: "empty is never a
  match target"). **Pure DB read — no language logic.** Directly reusable.
* `canonical_paths_for_function_name()` (cozo_queries.rs:1026) preserves one row
  per definition so two defs sharing a `canonical_path` cannot collapse — this is
  the **duplicate-canonical fail-closed** primitive (comment 1021-1025). Reusable.
* The staging queries (`put_staged_call_with_provenance`,
  `list_staged_calls_with_provenance`, `create_calls_edge_with_resolution`,
  `retract_canonical_edges_from_caller`) are all language-neutral.

**Conclusion:** `cozo_queries.rs` needs **no changes**. The entire DB/persistence
surface for canonical resolution is reused as-is. This materially de-risks the
feature (the highest-blast-radius layer — the schema — is untouched).

> **⚠️ SUPERSEDED (cycle-7, C7-4) — see the implementation plan's hardening.** This
> "no changes to `cozo_queries.rs`" conclusion was an **initial spike finding** and is
> **superseded by the implementation plan** (`docs/exec-plans/2026-07-23-python-namespace-canonical-resolution-plan.md`).
> Plan-hardening (adversarial F9) found the pre-existing name→IDs singleton is
> **inline-private CozoScript inside `reresolve_calls_edges`** (`cozo_queries.rs:2191-2205,2263-2270`)
> that **filters out `python_bare` rows** (`2222-2227`), so it cannot serve T5b's
> no-target legacy-recall fallback as-is. T5b therefore **exposes a small public,
> read-only language-scoped name→IDs helper in `cozo_queries.rs`** — a **helper
> exposure, NOT a schema change** (the canonical `function_meta` / `calls_edge` / staging
> schema is still untouched). The spike's core claim (**zero canonical *schema* change**)
> holds; only the "no *file* changes to `cozo_queries.rs`" phrasing is corrected. History
> is preserved above — this note is additive.

### The cross-file resolver has exactly TWO Rust-specific seams; the singleton match is shared

`reresolve_calls_edges_with_canonical_context` (code_graph.rs:264) is the
full-index post-pass. Its **language-agnostic core is the canonical-index
singleton lookup** (code_graph.rs:328, 347-350): `canonical_index.get(&target)`
resolving **only** when `ids.len() == 1` — i.e. it already **fails closed on
duplicate canonical paths**. The only Rust-specific parts are:

1. **Context build** — `rust_ctx_for_staged_file` (code_graph.rs:252-260) →
   `rust_canonical_ctx(.., Language::Rust, ..)` builds `(ModulePath, UseGraph)`.
   Python needs a parallel `python_ctx_for_staged_file` → `(PyModulePath,
   PyImportBindings)`, dispatched by the caller file's language.
2. **Target computation** — `canonical_target_for_staged_call`
   (code_graph.rs:197-246) maps a staged call's `qualifier_kind`
   (`"module"|"type"|"self"|"method"`) + `raw_qualifier` to a canonical string
   via `canonical::resolve_qualifier` (Rust crate/use-graph aware). Python needs a
   parallel target resolver over its module-path + import-binding table.

Notably `canonical_target_for_staged_call` **already handles a `"module"`
qualifier_kind** for Rust (`mod.func()` → `Qualifier::Path`). Python
`module.func()` is the same *shape*; only the module/import resolution differs.

### The legacy bare-name pass is language-SCOPED but NAME-ONLY — this is the recall gap

`reresolve_calls_edges()` (cozo_queries.rs:2182) resolves a staged **bare** call
only when **exactly one same-language** function carries the callee name
(`same_language.len() == 1`, line 2270). Two same-language defs of the same name
(e.g. `foo.parse` and `bar.parse`) → the `else` branch **retracts / drops**
(line 2283-2285). So today a cross-module same-name Python bare call is
**silently dropped**: correct (no false edge) but **low recall**.

**This is the precise gap FE8B3B2D closes**: replace name-only cross-file
matching with **canonical (module-qualified) matching**, recovering recall on the
same-name-different-module case *without* loosening fail-closed guarantees (the
canonical path is exact; the singleton/duplicate check still fails closed).

### The populator injection point is clean

`canonical_path_for_function` (code_graph.rs:145-169) returns `""` unless a Rust
`ctx` is present. It is called at code_graph.rs:721 (main index) and 1446
(incremental) with `rust_ctx`. Adding a Python branch — canonical path =
`python_module_path(rel_path) + "." + name` for module-level defs, `""` (never a
match target) for ambiguous package roots — slots directly into these two call
sites behind a language dispatch. No new call sites; no DB change.

### The Rust `canonical` package is the mirror target

`src/services/parsing/canonical/` is cleanly factored: `module_path.rs` (904 loc),
`use_graph.rs` (730), `resolver.rs` (776), `reexport.rs`, `generics.rs`, `mod.rs`.
The Python analogue is a **new sibling module** (e.g.
`src/services/parsing/python_canonical/`) providing three small primitives:
`python_module_path_for_file`, an import-binding extractor (the `UseGraph`
analogue), and `canonical_target_for_python_call`. It does **not** modify the Rust
canonical package — width isolation is preserved.

## Feasibility gate answers

**(1) Can Python import-binding capture + a Python `canonical_path` populator + a
Python-aware canonical resolver be built on 091-F machinery without new false-edge
risk?** — **YES.**

* Import-binding capture: new parser-local work in `python.rs` /
  `python_canonical`, structurally analogous to Rust's `extract_use_graph`.
* Populator: one Python branch in `canonical_path_for_function` (pure path→module
  transform for defs).
* Resolver: two Python branches (context build + target compute) behind a
  language dispatch in the existing post-pass; the singleton/duplicate fail-closed
  core is **reused unchanged**.
* No schema/DB change; no new dependency (tree-sitter-python already declared).

**No new false-edge risk** because every ambiguity fails closed (matrix below),
the canonical path is exact, and the duplicate-canonical singleton check is
inherited. The **one genuine correctness risk** is an *incorrect module-path
computation* (e.g. `__init__.py`, PEP 420 namespace packages, `src/` layouts)
producing a canonical string that collides with a real def. Mitigation: return
`""` (never a match target, D4) for any ambiguous package root. This is a
plan-harden item, not a feasibility blocker.

**(2) Is the effort within a normal feature envelope?** — **YES.** Change surface:

| Area | Change | Blast radius |
|---|---|---|
| `python.rs` | import-binding capture; emit `module.func()` / bare calls with provenance (`raw_qualifier`, `qualifier_kind`) instead of dropping | parser-local |
| `python_canonical/` (new) | `python_module_path_for_file`, import-binding graph, `canonical_target_for_python_call` | new module, isolated |
| `code_graph.rs` | Python branch in populator (721/1446); Python `*_ctx_for_staged_file`; Python branch in `canonical_target_for_staged_call`; language dispatch | additive seams |
| `cozo_queries.rs` | **none** *(superseded — see C7-4 note: T5b later adds a read-only public name→IDs helper for the F3/F9 legacy-recall fallback; still no schema change)* | query-local, additive |
| tests | unit (module-path, bindings, populator, fail-closed) + integration (cross-module same-name `.py` fixture resolves; ambiguous drops) | test-only |

Roughly 4–6 tasks at the 2-hour granularity. Normal feature envelope.

## Fail-closed matrix (013-D no-false-edge)

| Case | Behaviour | Mechanism |
|---|---|---|
| `from mod import *` (star) | DROP | no binding produced → callee unresolved |
| relative import `from . import x`, `from ..pkg import y` | DROP | package-root ambiguity → module path `""` |
| `__init__.py` / PEP 420 namespace package | DROP | ambiguous package identity → canonical `""` (D4) |
| re-export (`from a import b as c` re-exported) | DROP | binding not traced through re-export chains in v1 |
| `importlib` / `__import__` / dynamic | DROP | not a static binding → unresolved |
| duplicate canonical_path (2+ defs) | DROP | `function_ids_by_canonical_path` singleton check (ids.len()!=1) |
| unknown / unbound qualifier `x.func()` where `x` not an imported module | DROP | no module binding → unresolved (never a false edge) |

> **⚠️ Refined (C7-4/F3) — "DROP" here means no canonical (module-qualified) edge; a
> unique legacy name-only edge may remain.** Every **DROP** in this matrix drops the
> **canonical** module-qualified edge only. Per the final T5b/F3 contract (plan + `096-F`),
> a **unique** cross-file bare call still keeps its **legacy name-only edge** — the
> star-import, relative-import, re-export, and `__init__.py`/PEP-420 rows in particular fall
> through to T5b's no-target legacy unique-match (recall preserved), while a **non-unique**
> name (e.g. duplicate `canonical_path`, or two competing same-name imports) still fails
> closed with **no** edge at all. This qualifies the "Behaviour" column above so the spike
> matches the plan's fail-closed matrix + Requirements-Trace; the earlier "dropped outright"
> reading is **superseded** by the T5b/F3 legacy-edge contract. (Spike history is preserved;
> this note is additive.)

## Why FF7DE872 is NOT fixed here (scope guard)

FF7DE872 is **same-file** same-name shadowing on the **direct-edge** path
(`find_function_id` first-match, code_graph.rs:2037; consumed at 896-908). Same-file
bare calls take the direct-edge path **before** any canonical/singleton
post-processing, and the canonical index this feature extends **fails closed on
duplicate canonical_path** (two same-file defs → identical canonical path → 2 ids
→ dropped, not last-wins). So the namespace feature *cannot* apply the
source-order/last-wins semantics FF7DE872 needs. It is correctly an **independent
direct-edge/source-order fix** and stays out of this feature.

## Design fork to resolve in impl-plan (NOT a blocker)

Bare Python calls currently take the direct-edge / bare-name-staging path
(code_graph.rs:896-908), **not** the provenance path. To canonically resolve them,
choose one in the plan:

* **Option A (mirror Rust — preferred):** have `python.rs` stage
  canonical-eligible calls (`module.func()` and cross-module bare calls) via
  `put_staged_call_with_provenance` with `raw_qualifier` + a Python
  `qualifier_kind`, and add Python branches to the existing canonical post-pass.
  Keeps `reresolve_calls_edges` (bare-name singleton fallback) **untouched**;
  lowest blast radius; canonical pass upgrades/adds edges over the singleton pass.
* **Option B:** add a canonical disambiguation step *inside* the bare-name pass
  when `same_language.len() > 1`. More surgical but edits load-bearing
  `reresolve_calls_edges` in `cozo_queries.rs`.

Recommend **Option A** for width isolation and to keep the DB layer change-free.

## Recommendation

**Conclusion: GO — with caveats.**
**Confidence: high.**

Proceed to impl-plan. The feature layers cleanly on 091-F/094-F: the DB/persistence
layer is reused with **zero schema changes**, the resolver's fail-closed singleton
core is reused unchanged, and all new logic is a well-bounded Python analogue of an
existing Rust pattern. Recall is *recovered* (not merely preserved) with no
loosening of the 013-D no-false-edge invariant.

**Caveats to carry into impl-plan / plan-harden:**

1. **Package-root correctness is the crux.** `__init__.py`, PEP 420 namespace
   packages, and `src/`-layout roots must fail closed to `""`. This is the single
   place a wrong module path could mint a false edge; make it an explicit,
   test-covered invariant.
2. **Bare-call routing is a design fork** (Option A vs B above). Decide in the
   plan; Option A recommended (keeps `cozo_queries.rs` untouched).
3. **Re-exports and aliased re-binds** are v1 non-goals (DROP); document so
   downstream planning does not over-promise recall.
4. **Precision is unmeasured on real Python repos** — add a retrieval-eval fixture
   asserting the cross-module same-name case resolves and the ambiguous cases drop,
   rather than asserting a numeric target.

## Next steps

1. Promote to `impl-plan` (grounded in the 094-F/091-F patterns). New work =
   Python import-binding capture + Python `canonical_path` populator +
   Python-aware canonical resolver. Include Constitution Check, verification
   criteria, rollback triggers.
2. `plan-harden` — fail-closed correctness (package-root identity) is high-stakes;
   apply.
3. `plan-review` — standard multi-persona gate; must PASS before harvest.
4. `harvest` — decompose into a feature + 2-hour-scoped tasks; assemble a QUEUED
   shipment for Ship to claim.

## References (verified this spike)

* `src/services/parsing/python.rs` — Calls extraction (94-F); flat `extract_import`
  (141-143); attribute calls dropped fail-closed (309-321).
* `src/services/code_graph.rs:145-169` — `canonical_path_for_function` (Rust-only
  populator; injection point).
* `src/services/code_graph.rs:188-246` — `should_stage_provenance_call` +
  `canonical_target_for_staged_call` (Rust-specific target seam; already handles a
  `"module"` qualifier_kind).
* `src/services/code_graph.rs:252-260` — `rust_ctx_for_staged_file` (context seam).
* `src/services/code_graph.rs:264-368` — `reresolve_calls_edges_with_canonical_context`
  (post-pass; language-agnostic singleton core at 328/347-350). NOTE: stash located
  this in `cozo_queries.rs` — it is in `code_graph.rs`.
* `src/services/code_graph.rs:721`, `:1446` — populator call sites.
* `src/services/code_graph.rs:851-908` — language-agnostic Calls consumer (direct /
  staged / provenance).
* `src/services/code_graph.rs:2037` — `find_function_id` first-match (FF7DE872 locus;
  out of scope).
* `src/db/cozo_queries.rs:924-940` — `function_meta.canonical_path` upsert.
* `src/db/cozo_queries.rs:1014-1066` — `canonical_paths_for_function_name`
  (duplicate fail-closed) + `function_ids_by_canonical_path` (reused).
* `src/db/cozo_queries.rs:2182-2289` — `reresolve_calls_edges` (bare-name,
  language-scoped, name-only; the recall gap).
* `src/services/parsing/canonical/` — Rust canonical package (mirror target).
* Prior art: `docs/decisions/2026-07-20-python-call-edge-extraction-spike.md`,
  `docs/exec-plans/2026-07-20-python-call-edge-extraction-plan.md` (094-F);
  `docs/decisions/2026-07-15-091-001-canonical-identity-spike.md`,
  `docs/decisions/2026-07-15-option-c-canonical-identity-deliberation.md`,
  `docs/exec-plans/2026-07-15-091-F-option-c-canonical-identity-plan.md` (091-F).
* Stash intake: `FE8B3B2D` (this feature); `FF7DE872` (split-out independent bug).
