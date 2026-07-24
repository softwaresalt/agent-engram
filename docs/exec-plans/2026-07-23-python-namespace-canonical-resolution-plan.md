---
title: "Python module-namespace-qualified call resolution — implementation plan"
type: plan
date: 2026-07-23
source: docs/decisions/2026-07-23-python-namespace-canonical-resolution-spike.md
stash_id: FE8B3B2D
status: reviewed
requires_plan_hardening: true
layers_on: "094-F"
extends: "091-F"
sequences_after: "094-F"
independent_of: ["090-S", "095-F"]
related_bug: "FF7DE872"
tags:
  - code-graph
  - python
  - canonical-identity
  - call-graph
  - namespace-resolution
  - fail-closed
---

## Problem Frame

engram resolves cross-module same-name calls correctly for **Rust** via
canonical (crate/module-namespace) identity (091-F): `function_meta.canonical_path`
is populated per Rust def, and a post-pass resolves staged qualified calls by
exact canonical path with a singleton/duplicate fail-closed guard. **Python has
none of this.** After 094-F, Python emits `Calls` edges, but:

* cross-file **bare** Python calls resolve **name-only** via
  `reresolve_calls_edges` (`cozo_queries.rs:2182`), which is language-scoped but
  fails closed (drops) whenever **2+ same-language** defs share the callee name —
  the exact cross-module same-name case (`foo.parse` vs `bar.parse`). Recall is
  lost.
* `module.func()` attribute calls are emitted `is_method:true` with an empty
  `raw_qualifier` and are **dropped** by `should_stage_provenance_call`
  (`code_graph.rs:189-190`).

Python modules **are** namespaces (`foo/bar.py` → `foo.bar`). The spike
(`docs/decisions/2026-07-23-python-namespace-canonical-resolution-spike.md`,
conclusion **GO / high confidence**) verified against current code that the
existing 091-F canonical machinery can be extended to Python:

* the DB layer (`function_meta.canonical_path`, `function_ids_by_canonical_path`,
  `canonical_paths_for_function_name`, staging queries) is **language-agnostic and
  reused with zero schema changes**;
* the resolver's **singleton/duplicate fail-closed core** is reused unchanged;
* the only new logic is a **Python analogue** of the Rust `canonical` package
  (module path + import bindings + target resolution) plus a language dispatch at
  three seams.

This plan implements the **frozen scope**: module-level function resolution and
`module.func()` disambiguation. It **does not** implement instance-method dispatch
(`self.method()` / `obj.method()` — requires type inference), and it **does not**
subsume the split-out same-file shadowing bug **FF7DE872** (a direct-edge /
source-order fix the canonical index cannot make — it fails closed on the
duplicate canonical path instead of applying last-wins).

## Resolution rule (frozen)

**Over-arching invariant — prove-or-fail-closed (cycle-7).** A Python bare or
module-qualified call resolves to a canonical / module-qualified target **only when the
call-site-effective binding of the callee name is STATICALLY PROVABLE**; in every other
case it **fails closed** — routed to `python_bare` staging so a downstream layer (T5b/T5c)
adjudicates or drops it, never a guessed edge (013-D). "Statically provable" requires
BOTH: (a) the winning binding is **unambiguous across every scope the resolver models**
(module, enclosing-function, function-local) and is **not shadow-contested** by any
competing binding form — a named module import, a positioned **star-import invalidator**,
a **non-import module / scope rebind** (assignment / augmented / `class` / `del` /
`match`-case / `for` / `with … as` / `except … as` / walrus / parameter target), or a
**scoped (function-local / enclosing-function) import**; **and** (b) it is the
binding effective **at the call site in execution order** (the time axis, C7-1). Any axis
the resolver cannot prove — **scope OR binding form** — forces the fail-closed route.
Expressed as a **prove-the-negative default**: *route a same-file `def` to `python_bare`
staging **unless** the name is PROVABLY the sole binding across every modeled scope — no
import, no star, no scoped import, **and no non-import module/scope rebind form the
resolver models** (the **Shared rebind-form set (RFS)**, defined once in §Design decision);
if any competing binding — of any form, at any scope — cannot be proven absent, **stage,
never a direct edge**.* Closure is therefore over **both** the scope axis (module /
enclosing / function-local) **and** the binding-form axis (RFS), so neither a new scope nor
a new syntactic rebind form can silently reopen a false-edge path.

For a Python call in caller module `M` (module namespace `foo/bar.py` → `foo.bar`):

* bare `name()` — resolve to the **binding effective at the call site**. The extractor
  stages `Calls` **only from function bodies** (module-level calls are never staged — C9-3),
  so this rule is defined **over function-body calls only**; because a function body runs
  when the function is *called*, its effective module binding is statically provable **only
  when it is stable** — no later module-level rebind contests it — else **fail closed**:
  * the effective binding is the last **stable** module (or enclosing-function, per T2b)
    binding of `name`: a `from N import name` that is the last binding with **no later
    rebind** → `N.name`; a `def name` that is the last binding with **no later rebind** →
    `M.name`;
  * a **later module-level rebind** after the winning binding makes invocation order
    undecidable (the function may run before or after it) → **fail closed** (**C7-1**);
    module-level (top-level) call ordering is a **v1 non-goal** (top-level calls not
    extracted);
  * a name that is **shadow-contested** at any modeled scope (star / non-import rebind /
    scoped import competing with a same-scope `def`) is **not** resolved in-line → routed
    to staging for T5b/T5c (or dropped).
* `mod.name()` where `mod` is a bound imported module → `<resolved-mod>.name`
* else (star / relative / package-root / re-export / dynamic / unbound / any
  shadow-contested or order-uncertain binding) → **DROP** (fail closed).

Every drop path is a fail-closed no-edge, never a guessed edge (013-D). A **DROP** means
**no canonical module-qualified edge**; a *unique* cross-file bare call still keeps its
**legacy name-only edge** (Y3/F3 recall preservation) — only the canonical layer drops.

> **Function-local import ordering (Y1).** Inside a function, a function-local
> import binds the name for the **whole** function body (Python `UnboundLocalError`
> semantics): a call **before** the import fails closed (no edge); only calls
> **after** the import resolve. Uncertain control-flow ordering → fail closed. (T2b.)

> **No-module-context recall (Y3).** When caller `M` has **no provable module
> namespace** (`src/`-root / PEP 420 namespace / `__init__.py` → T1 returns `None`),
> a **bare** cross-file call is resolved by the **legacy name-only unique-match**
> (today's behavior — **recall preserved**), never dropped-to-nothing and never a
> canonical module-qualified edge. The canonical (`python_bare`) path applies **only**
> when `M` has a provable namespace; precision (no false module edge) and recall (the
> legacy edge) both hold. (T5a stages `python_bare`; T5b routes no-context bare calls
> to the legacy matcher.)

> **Import-rebind precedence (X2/X3).** In
> ```python
> def parse(): ...
> from bar import parse
> def caller(): parse()
> ```
> Python binds `parse` to `bar.parse` — the later import **shadows** the local def — and
> the call site (`caller` body, defined **after** the import) sees the import binding, so
> the bare `parse()` resolves to `bar.parse`, **not** `M.parse`. A **local-def-first**
> rule would mint the wrong edge to `M.parse` and breach the precision floor (013-D).
> Symmetrically, a local `def` that appears **after** the import (and before the call)
> wins. When source order cannot establish a clear last binding, **fail closed** (no edge).

> **Call-site-effective binding (C7-1) — function-body calls only.** "Last-binding-wins" is
> scoped to the **call site**, not the whole module. The Python extractor emits `Calls`
> **only from function bodies** (`extract_top_level` / `extract_calls_from_body`, seeded from a
> function body — `python.rs:44-79,233-277`); a **module-level (top-level) `parse()` is never
> staged**, so call-position ordering is defined **only over function-body calls**. A function
> body runs when the function is *called*, not when it is defined, so its effective module
> binding is statically provable **only when that binding is stable** — no later module-level
> rebind contests it:
> ```python
> from bar import parse
> def caller(): parse()       # no later module-level rebind → stable → bar.parse
> ```
> When a **later module-level rebind** of the same name exists, the enclosing function may run
> **before or after** the rebind, so the binding is **not statically decidable → fail closed**:
> ```python
> def parse(): ...
> def caller(): parse()       # function-body call
> from bar import parse       # later module-level rebind — caller() may run before OR after
> ```
> `caller`'s `parse` could be `M.parse` (if `caller` runs before the import) or `bar.parse`
> (if after), so T5b/T5c **fail closed** (no edge). T5b selects the last binding whose effect
> is settled when the call runs and fails closed when a later rebind makes that order
> undecidable. **v1 NON-GOAL:** module-level (top-level) call ordering is **out of scope** —
> top-level calls are **not extracted**, so such cases are **neither regressed nor newly
> handled**. (Direct application of the prove-or-fail-closed invariant to the time axis.)

> **Duplicate same-name imports (C8-1).** The call-site-effective last-binding-wins
> ordering (C7-1) disambiguates a name bound by an in-module `def` and **at most one**
> import binding (plus rebind markers) in source order — it does **not** disambiguate **two
> or more competing imports of the same name** at a scope (`from a import f; from b import
> f; f()`). T2 **fails closed on such duplicate/competing bindings (M1)** — it produces
> **no** binding — so T5b derives no canonical target and the call **fails closed** (no
> canonical module-qualified edge). Recall is not recoverable here: the F3 legacy name-only
> fallback also drops because the name is **non-unique** (≥2 candidate functions). This
> matches main's behavior today (name-only ambiguous → drop), so it is fail-closed with **no
> regression**. Ordered-import-history disambiguation across duplicate same-name imports is
> a documented **v1 non-goal** — the frozen rule, T2 (M1), and T5b all agree on fail-closed
> here.

### Resolution composition trace (cycle-7 — whole-contract closure)

Every canonical scenario yields **exactly one** unambiguous, precision-floor-safe outcome,
owned by **exactly one** layer; everything unprovable fails closed.

| # | Scenario | Effective binding | Outcome | Owning layer |
|---|---|---|---|---|
| C1 | def-only (`def f; f()`) | same-file def | `M.f` (direct fast path) | T5a in-file arm |
| C2 | import-only (`from N import f; f()`) | named import | `N.f` | T5b |
| C3 | def-before-import (`def f; from N import f; f()`) | import (last before call) | `N.f` | T5a→staging → T5b |
| C4 | import-before-def (`from N import f; def f; f()`) | def (last before call) | `M.f` | T5a→staging → T5b |
| C5 | def + star-after-def (`def f; from n import *; f()`) | star invalidator contests def | **DROP** (fail closed) | T5a→staging → T5c |
| C6 | def + non-import rebind (`def f; f = g; f()`; also `class`/`del`/`match`-case/`for`/`with`/`except`/walrus/param) | RFS rebind form contests def (T5a shared in-file scan) | **DROP** (fail closed) | T5a routing (shared RFS scan) + T5c invalidation (same RFS) |
| C7 | def + function-local import (`def f; def h(): from b import f; f()`) | scoped import shadows body | in-body → `b.f`; pre-import call → fail closed | T5a→staging → T2b/T5b/T5c |
| C8 | provable-namespace unbound (star/re-export/relative; unique name) | no canonical target | **DROP canonical**, keep **legacy name-only** edge | T5b legacy fallback (F3) |
| C9 | call-before-later-rebind (function-body call under a later module rebind) | not statically decidable (function may run before or after) | **fail closed** (module-level call ordering = v1 non-goal — top-level calls not extracted) | T5b call-site-effective (C7-1, function-body only) |

## Requirements Trace

| Source requirement (spike / stash FE8B3B2D) | Implementation action |
|---|---|
| Module namespace `foo/bar.py` → `foo.bar` (only when every ancestor is a **provable regular package**) | T1: `python_module_path_for_file(rel_path, is_regular_package)` (predicate from the indexed `__init__.py` set — **no config source**, Q3); REJECT `src/`-roots / implicit PEP 420 namespace / `__init__.py`; **source-root-aware resolution = v1 non-goal** (M5, Q3) |
| Symbol-level import bindings (Python analogue of Rust `UseGraph`) | T2: `extract_python_import_bindings` records `(canonical_path, kind∈{ModuleImport, FromImportSymbol}, source_position)` (R2 + **F14 positions**) and fails closed on competing/duplicate bindings (M1); a module-scope **`from N import *` is recorded as a positioned order-aware invalidator marker (F4)**, not a binding |
| Scope-correct bindings — function-local imports must not leak; module vs function scope; **function-local import ordering**; **enclosing-function (closure) scope** | T2b: scoped binding model (M1) + **order-aware function-local imports — a name bound by a function-local import is local for the whole body, so calls *before* the import fail closed (Y1)** + **lexical closure chain: walk enclosing-function scopes (honoring `global`/`nonlocal`) before module scope, or fail closed when an enclosing function binds the name (F5)**; a pre-import function-local use is a **poison/tombstone that forces fail-closed (F8)**, never a fall-through to module scope |
| Register the `unit_python_canonical` test target so verification runs | T1: `Cargo.toml` `[[test]]` entry (M3) |
| Populate `function_meta.canonical_path` for Python module-level defs | T3: Python branch in `canonical_path_for_function` (reuses `upsert_function_with_canonical`); **package-topology changes (add/remove `__init__.py`) reindex/invalidate affected descendants past the content-hash skip (C6-1)** |
| Emit `module.func()` as a canonical-eligible staged call (stop dropping it); `self`/`cls` stay dropped | T4: `python.rs` emits `is_qualified:true, raw_qualifier=<receiver>, qualifier_kind="module"`; excludes `self`/`cls` (M2) |
| Route Python cross-file bare + module-qualified calls into canonical staging — on **both** indexing arms; **in-file shadowed calls must not short-circuit to a direct edge** | T5a: **both** Calls-consumer arms — full-index (851-908) **and sync (1573-1643)** — + `should_stage_provenance_call` (language-dispatched); the sync arm's name-only `put_staged_call` (1639) also routes through provenance for Python so T7 re-extraction never strands edges (X1, same class as Q2/R1); **the in-file `(Some,Some)` direct-edge shortcut (900-902) is made SHADOW-AWARE over SCOPES *and* FORMS (F2 root-defect, generalized cycle-7 Z1/Z2/C7-2, non-import-rebind axis closed cycle-8 C6): a same-file `def` is routed to `python_bare` staging whenever its name is SHADOW-CONTESTED by EITHER a producer-backed import signal — a named module import (T2), a positioned star-import invalidator (T2/F4), OR a scoped function-local/enclosing-function import (T2b) — OR any non-import rebind form in the Shared rebind-form set (RFS, §Design decision) that T5a detects with its OWN in-file tree-sitter scan (the same scan T5c runs; T2/T2b produce no non-import-rebind signal); expressed as the prove-the-negative default "keep the direct edge ONLY when provably the sole binding across every scope AND with no RFS form present" so future scopes AND forms are covered by construction; T5a OWNS this `code_graph.rs:896-908` change on both arms — no new producer, task, or DAG edge**; a callee with **no** competing binding keeps the direct fast path; **no-target bare calls stay recall-preserving via the legacy matcher (Y3/F3, resolved in T5b)** |
| **Fail closed on `def` + non-import module/scope rebind (C6 — the non-import-rebind axis)** (`def parse(); parse = factory(); parse()`; also `class parse`/`del parse`/`match`-case/`for`/`with … as`/`except … as`/walrus/parameter targets) | **DROP** — T5a's in-file routing scan detects the RFS rebind form (no producer needed) and routes the same-file `def` to `python_bare` staging instead of a direct `M.parse` edge; T5c then fails closed on the same RFS (order-aware). Asserted by T5a's routing-table test (`def f; f = factory(); f()` → STAGED, **no** direct `M.f`) and T5c's rebind table; composition-trace row C6 = DROP |
| Python-aware canonical target resolution reusing the singleton fail-closed core | T5b: `python_ctx_for_staged_file` + Python branch in `canonical_target_for_staged_call`, dispatched by the T2 binding **kind** (R2); resolves to the **binding effective at the call site** — for a **function-body call** (the only call form the extractor stages, C9-3), the module/enclosing binding that is **stable when the call runs**, and **fails closed when invocation order cannot statically establish the binding** (e.g. a function-body call contested by a later module rebind — **C7-1**); **module-level (top-level) call ordering is a v1 non-goal** (top-level calls not extracted); **guard-agnostic — the T5c shadow guard wraps it downstream (R3)**; when T5b **derives no canonical target** it returns a **typed no-target reason (C9-4)** and applies the **legacy name-only unique-match** fallback (**F3**) **only** for the recall-safe reasons {`NoModuleContext` (T1 rejected `src/`-root / PEP 420 / `__init__.py`), `UnsupportedImportForm` (provable-namespace-but-unbound: star / re-export / relative)} via a **public language-scoped name→IDs helper in `cozo_queries.rs` (F9)**; **never** for {`CompetingBindings`, `Shadowed`, `DuplicateSameNameImport`} — those stay fully fail-closed (NO edge); non-unique names still fail closed |
| Fail closed when any rebind shadows an imported **module receiver OR bare callee** name | T5c: drop `mod.func()` (receiver) **and** bare `parse()` (Q1) when the name is re-bound by a binding **later than the WINNING binding T5b resolved to** (import OR def, order-aware anchor — **F1**) — in the applicable scope — assignment/augmented/`for`/`with`/`except`/walrus/param/**`def`/`class`/`del`/`match`-case (X4)**/**later `from N import *` (F4)**/module-level (M1, Q1, X4); a rebind **before** the winner does not invalidate it, so **`from bar import parse; def parse(); parse()` → `M.parse` SURVIVES (def-after-import, F1)** just as `def parse(); from bar import parse; parse()` → `bar.parse` (Y2); and the winner is **call-site-effective, not whole-module** (**C7-1**, function-body calls only — top-level call ordering is a v1 non-goal, not extracted) |
| Existing indexes must backfill Python canonical **edges** after upgrade, **in one operation** | T7: a **separate `PYTHON_CANONICAL_EXTRACTION_VERSION` marker** (NOT `content_hash`, R1) triggers re-extraction that **also runs the canonical resolution pass in the same step** (escalate sync→full path or invoke the post-pass) / `index --force`; upgrade regression asserts the **resolved edge** and that `content_hash` staleness detection is preserved (M4, Q2, R1) |
| Fail closed on star / relative / package-root / re-export / dynamic / duplicate | T1–T2 return no module/binding; T3 returns `""`; T5b singleton check drops duplicates |
| No **canonical** schema change; reuse `function_ids_by_canonical_path` + staging queries | No `function_meta`/`calls_edge`/staging schema change (verified by T5b); **T5b's F9 name→IDs helper is a read-only extraction of the resolver's inline singleton, not a schema change**; T7's extraction-version marker is orthogonal index-state, not the canonical model (R1) |
| Do NOT resolve instance-method dispatch; do NOT touch FF7DE872 | Out of scope; documented in T6 |
| Document capability + v1 non-goals | T6 |

## Design decision — bare-call routing (spike fork resolved)

**Chosen: Option A (mirror Rust).** Route *all* canonical-eligible Python calls
through **provenance staging** and resolve them in the **existing canonical
post-pass**, keeping the name-only `reresolve_calls_edges` bare-name pass as the
**conservative fallback** for empty-qualifier calls. T5b additionally invokes a
**small public language-scoped name→IDs singleton helper** exposed in
`cozo_queries.rs` (F9 — the pre-existing singleton is inline-private CozoScript
inside `reresolve_calls_edges` at `cozo_queries.rs:2191-2205,2263-2270` and filters
out `python_bare` rows at `2222-2227`, so it cannot be reused as-is; T5b extracts it
into a shared method and calls it for its no-target fallback — a **helper exposure,
not a schema change**):

* In-file bare calls stay a **`direct` edge ONLY when the callee is NOT shadow-contested
  at ANY scope OR by ANY rebind form the resolver models (F2, generalized cycle-7 —
  Z1/Z2/C7-2; non-import-rebind axis closed cycle-8 — C6).** The
  consumer at `code_graph.rs:896-908` resolves a bare callee against the current file's
  symbols *before* any staging, so a same-file `def` short-circuits to a **direct
  `M.callee` edge (`(Some,Some)`→`create_calls_edge`, 900-902)**. The cycle-6 F2 fix keyed
  the skip on **named module-level imports only**, which does **not** compose: a
  `from n import *` (a non-binding invalidator marker, Z1), a non-import module rebind
  (`parse = factory()` / `class` / `del` / `match`-case, Z2/C6), or a **function-local /
  enclosing-function import** (C7-2) all leave the predicate false, so the arm still mints
  a false direct edge and bypasses T2b/T5b/T5c. **T5a therefore makes this decision
  SHADOW-CONTEST-aware for Python via the prove-or-fail-closed invariant**, closing over
  **both scopes and binding forms**: a same-file `def` is routed to `python_bare` staging
  whenever its name is contested by **either** (i) a **producer-backed import signal** —
  a named module import, a positioned star-import invalidator (T2), or a scoped
  function-local / enclosing-function import (T2b) — **or** (ii) **any non-import
  module/scope rebind form in the Shared rebind-form set (RFS)**, which T2/T2b do **not**
  produce (they walk only import nodes), so **T5a detects these forms with its own in-file
  rebind-form scan — the SAME module/scope rebind-form scan T5c specifies (§T5c)**. Because
  `parse_source` retains no tree-sitter `Tree` (it returns only `ParseResult{symbols,
  edges}`, `parsing.rs:247-252`), **T5a re-parses the in-scope file `source: String`**
  (already cloned at the seam — full-index ~`code_graph.rs:609`, sync ~`1290`) **in-memory**,
  computing the file's **Rebind-Form-Set map ONCE PER FILE and caching it** for reuse across
  every call in that file (**not per call-site**) — one extra amortized in-memory parse per
  Python file, **no new producer, DB field, or DAG edge**. **Self-exclusion (C9-1):** the
  scan **excludes the `def` currently under consideration** (the definition
  `find_function_id` matched is the **candidate target**, never a self-competitor) and counts
  only **OTHER** competing bindings **applicable to the caller's lexical scope** (module +
  enclosing functions + the caller's own function — **not** unrelated sibling-scope
  locals/parameters). A same-file callee is kept on the **direct-edge fast path only when its
  matched def is PROVABLY the sole applicable binding** — no OTHER import/star/scoped-import
  signal **and** no OTHER RFS form in the caller's scope; if any competing binding cannot be
  proven absent, it is staged. (Without self-exclusion, `def f; f()` would self-contest via
  the `def` RFS form and the promised direct fast path would be unreachable dead code.)
  This change is on **both** arms (`code_graph.rs:896-908` full-index and
  `1573-1643` sync) and **T5a owns it**.
* **Shared rebind-form set (RFS) — single source of truth (cycle-8, C6).** The set of
  syntactic forms that count as a **rebinding of a name** is defined **once, here**, and is
  consumed **identically** by **T5a's in-file routing scan** (does a competing binding
  exist? → stage) **and** by **T5c's order-aware invalidation scan** (does a competing
  binding land in the winner→call window? → fail closed). Neither task may enumerate its
  own divergent list. **RFS =** { plain `assignment` (`name = …`); **annotated assignment
  (`name: T = …`)** (a distinct tree-sitter node from plain assignment); augmented assignment
  (`name += …`, etc.); `class name`; `def name`; `del name`; `match`/`case` capture pattern
  binding `name`; `for name in …`; `with … as name`; `except … as name`; walrus
  `(name := …)`; function **parameter** `name`; a module-scope `from N import *` star
  invalidator (F4) }, at module scope **or** the applicable function scope. **Self-exclusion
  (C9-1):** when T5a runs the scan for an in-file bare call, the `def name` form belonging to
  the **matched definition under consideration** is the candidate target and is **excluded**
  from the competing-binding count — only bindings of `name` **other** than that def, and
  **applicable to the caller's lexical scope**, count as contests. T5a treats the presence of
  **any** such OTHER RFS form (that it cannot prove absent) as a route-to-staging trigger
  (order-agnostic — precision-safe over-approximation); T5c applies the **order-aware**
  winner→call-window rule (Y2 + F1 + C7-1) to the **same** RFS to decide the final drop.
  Because both scans read the same RFS definition, T5a's routing and T5c's invalidation
  **cannot drift** on which forms count.
* Cross-file **bare** Python calls (callee unresolved in-file) are staged via
  `put_staged_call_with_provenance` with `qualifier_kind="python_bare"`,
  `raw_qualifier=""` — instead of the current name-only `put_staged_call`. Because
  the bare-name pass filters `qualifier_kind.is_empty()`, these are handled *only*
  by the canonical pass (no double-processing).
* **No-target legacy fallback (Y3 + F3):** whenever T5b **derives no canonical
  target** for a `python_bare` call — either because the caller has no provable module
  namespace (T1 `None`) **or** because the module namespace is provable but the callee
  has **no import binding and no in-module def** (star-imported / re-exported / relative
  / otherwise resolvable today only by the global name-only singleton, **F3**) — T5b
  falls back to the **legacy name-only unique-match** via the shared language-scoped
  name→IDs helper (F9), preserving today's recall and never emitting a false
  module-qualified edge; a non-unique name still fails closed. Gating the fallback on
  `T1==None` alone would DROP the provable-namespace-but-unbound calls, because the
  `python_bare` stamp excludes them from `reresolve_calls_edges`
  (`qualifier_kind.is_empty()` filter, `cozo_queries.rs:2222-2227`) — the F3 regression.
* `module.func()` calls are staged with `qualifier_kind="module"`,
  `raw_qualifier=<receiver>`.
* The canonical post-pass computes the target canonical path from the caller's
  Python module + import bindings and matches it against
  `function_ids_by_canonical_path` with the existing `ids.len()==1` fail-closed.

Rejected Option B (canonical fallback inside `reresolve_calls_edges`): more
surgical but rewrites the load-bearing, operator-gated bare-name resolver in
`cozo_queries.rs`; higher blast radius; harder to keep byte-identical for Rust. (F9's
helper exposure is a **minimal extraction** of that resolver's inline singleton into a
shared read-only method — it does **not** alter the resolver's logic, distinct from
Option B's rewrite.)

## Implementation Units

Sequenced test-first (Constitution Principle II). Each unit is a single skill
domain, ≤3 files, ≤5 functions, ≤4 test scenarios — a verifiable milestone.
Every unit authors its failing test(s) first (RED), then the implementation
(GREEN).

### T1 — Python module-path primitive (domain: code)

* **New**: `src/services/parsing/python_canonical/module_path.rs` + `mod.rs`.
  `python_module_path_for_file(rel_path, is_regular_package: &impl Fn(&Path) -> bool)
  -> Option<String>` — resolve `foo/bar.py` → `Some("foo.bar")` **only** when **every
  ancestor directory in the path is a provable regular package** (contains an
  `__init__.py`), a predicate the caller **derives from the already-indexed file
  set** (no config). A top-level module (`mod.py`, no ancestor dirs) → `Some("mod")`.
  * **Fail closed (`None`)** for: `__init__.py`; any non-identifier segment; **any
    ancestor dir lacking `__init__.py`** — which conservatively **rejects both
    implicit PEP 420 namespace packages and `src/`-style source-root layouts** (T1
    cannot prove a `src/` root is a strippable source root, so it does not resolve
    those); and non-`.py` paths. `None` ⇒ the caller writes `""` (never a match
    target, D4).
  * **v1 NON-GOAL (Q3/Q6 — NARROWED):** source-root-aware resolution (stripping a
    declared `src/` root to yield `pkg.mod`) is **explicitly out of scope for v1**.
    There is no production source for source-root declarations (`CodeGraphConfig`
    has no such field, `config.rs:98-120`), so rather than invent speculative config
    (Constitution VI) v1 **fails closed** on `src/`-layouts and namespace packages —
    no `PackageLayout`/source-root machinery. A future iteration may add a
    `source_roots` config and wire it here.
* **Cargo.toml (M3)**: register a `[[test]]` target
  `name = "unit_python_canonical", path = "tests/unit/python_canonical_test.rs"`
  (repo registers every nested `tests/unit/*.rs`; `unit_parsing` exists ~231-237,
  this target did not). Two trivial config lines.
* **Files**: `python_canonical/module_path.rs`, `python_canonical/mod.rs`,
  `Cargo.toml` (2-line test registration), `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: regular nested package `p/q/r.py` with a proven
  `__init__.py` chain → `p.q.r`; top-level `mod.py` → `mod`; **fail-closed table** →
  `None` (covers `__init__.py`, a `src/`-root where `src/` lacks `__init__.py`, an
  implicit PEP 420 namespace package, a non-identifier segment); non-`.py` → `None`.
* **Verification**: `cargo test --test unit_python_canonical`; `cargo clippy
  --all-targets -- -D warnings -D clippy::pedantic`; `cargo fmt --all -- --check`.
* **Milestone**: a dotted path **only** for a provable regular-package chain; every
  unprovable layout (namespace / `src/`-root / `__init__.py`) fails closed.

### T2 — Python import-binding capture (domain: code)

* **New**: `src/services/parsing/python_canonical/bindings.rs`.
  `extract_python_import_bindings(source) -> ImportBindings` mapping a **local
  name** to its **canonical origin, binding kind, _and source position_ (R2 + F14)** — a
  `(canonical_path, kind, position)` where `kind ∈ {ModuleImport, FromImportSymbol}` —
  built by walking tree-sitter `import_statement` / `import_from_statement` nodes. The
  **position** (byte/line order) is required by T5c's order-aware winner anchor (F1) and
  by the F4 star invalidator; every binding record carries it (F14).
  * `from N import name` → `name → ("N.name", FromImportSymbol, pos)`; `from N import
    name as p` → `p → ("N.name", FromImportSymbol, pos)`.
  * `import a.b as c` → `c → ("a.b", ModuleImport, pos)`; `import a.b` → `a → ("a",
    ModuleImport, pos)` (root-name module binding).
  * **No binding (fail closed)** for: relative imports (`from . import x`, leading-dot
    module), `importlib`/`__import__`/dynamic.
  * **Module-scope `from N import *` (star) → a positioned _order-aware invalidator
    marker_, not a binding (F4).** T2 records the star's module-scope position so T5c
    can fail closed when a star occurs **after** the winning binding (`from bar import
    parse; from n import *; parse()` → **no** edge — the later star may rebind `parse`).
    A star **before** the winner does not invalidate it. The star itself never mints a
    binding (it stays a fail-closed drop for its own name resolution).
  * **Competing / duplicate binding → fail closed (M1).** If the **same local
    name** is bound by 2+ import statements in the same scope
    (duplicate/re-import), mark it **ambiguous** → **no** binding. A flat
    last-writer-wins `HashMap` is **forbidden**. (Function-vs-module scope isolation
    is T2b.)
  * **Why the kind (R2)**: T5b must tell a **module receiver** (`import pkg` →
    `pkg.func()`) from an **imported symbol** (`from pkg import parse` → `parse()` or
    the out-of-scope attribute `parse.tokenize()`). Without the kind, a from-import is
    mis-resolved as a module and mints a wrong edge.
* **Files**: `python_canonical/bindings.rs`, `python_canonical/mod.rs` (re-export),
  `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: `from p import f`→`(p.f, FromImportSymbol, pos)` **and**
  `import a.b as c`→`(a.b, ModuleImport, pos)` (kind + position asserted, R2/F14);
  `from . import x` → **no** binding, **and** a module-scope `from p import *` records a
  positioned **invalidator marker** (F4 — not a binding); **F4 order-aware** — a star
  **after** a `from bar import parse` marks `parse` invalidatable, a star **after** a
  `def parse` likewise (star-after-def), while a star **before** does not; competing
  `import p` + `from q import p` → **no** binding (M1).
* **Verification**: `cargo test --test unit_python_canonical` (target registered in
  T1); clippy; fmt.
* **Milestone**: symbol-level bindings; star/relative/dynamic **and** competing
  bindings fail closed.

### T2b — Scope-aware binding isolation (domain: code)

* **Why (M1, Y1, F5)**: a file-wide flat binding map **leaks a function-local import into
  other callers** and cannot represent module-level rebinding. Bindings must be
  scoped: module-level imports apply to every function; a function-local import
  applies **only** within its own function and must never leak to a sibling. **And a
  function-local import binds its name for the *whole* function body (Python
  `UnboundLocalError` semantics), so a call *before* the import must fail closed (Y1).**
  **A two-level (module/function) model is insufficient (F5): Python resolves names
  through the *lexical closure chain* — the enclosing-function scopes between a nested
  function and the module — so a name bound by an enclosing function must be seen by the
  inner call, or the resolution is wrong (precision) / wrongly dropped (recall).**
* **Changes**: extend `bindings` (T2) to a **scoped model** — module-level bindings
  + a **lexical closure chain** of per-function scopes (innermost → enclosing →
  module) — so a resolver consults a caller function's local bindings, then each
  **enclosing function** scope, then module-level, **honoring `global` / `nonlocal`
  declarations** (which redirect a name to the module / nearest-enclosing scope). A name
  whose applicable scope holds a competing binding fails closed (reuses the M1 rule).
  Nested-function locals do not leak to the enclosing scope. **When an enclosing function
  binds the name in a way the chain cannot resolve unambiguously, fail closed (F5).**
  **Track each function-local import's position and each call site's position: a name
  bound by a function-local import LATER in the function emits NO binding for call sites
  BEFORE that import (fail closed — UnboundLocalError). A pre-import function-local use is
  a poison/tombstone that forces fail-closed — it must NOT fall through to the module
  scope (F8). Only calls AFTER the import resolve. When control flow makes the ordering
  uncertain, fail closed (Y1).**
* **Files**: `python_canonical/bindings.rs`, `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: **(1)** a function-local `from x import f` is visible for a
  call **after** it, but a call **before** the function-local import
  (`def g(): f(); from x import f`) gets **no** binding — a poison/tombstone that does
  **not** fall through to a module-level `f` (fail closed — Y1/F8); **(2)** the **same**
  call site in a sibling function that lacks the import gets **no** binding (no leak, M1);
  **(3) closure chain (F5)** — a nested function's bare call resolves against an
  **enclosing function's** import binding (walk the chain), while a nested-function local
  import does **not** leak to its enclosing function, and a `nonlocal`/`global`-declared
  name is redirected to the correct scope; **(4)** when a call's position relative to a
  function-local binding can't be established (branchy control flow) **or** an enclosing
  scope binds the name ambiguously → **no** edge (fail closed, Y1/F5).
* **Verification**: `cargo test --test unit_python_canonical`; clippy; fmt.
* **Milestone**: bindings are scope-correct (**module + full closure chain, F5**) **and
  order-correct**; function-local imports cannot leak, a call preceding a function-local
  import fails closed (poison, F8), and enclosing-function binds are honored or fail
  closed.

### T3 — Python canonical_path populator (domain: code)

* **Changes**: `src/services/code_graph.rs`. Add a language dispatch to
  `canonical_path_for_function` (145-169) so Python module-level defs get
  `canonical_path = "<module>.<name>"` via T1 (`""` when `python_module_path_for_file`
  returns `None`). Build a lightweight Python per-file context at the two populator
  call sites (721, 1446) parallel to `rust_ctx`; that context carries the
  **regular-package predicate** T1 needs — a set of dirs containing `__init__.py`,
  derived once from the already-indexed file set (Q3: **no `CodeGraphConfig` change,
  no source-root config**). Reuse `upsert_function_with_canonical` unchanged (no DB
  change).
  * **Package-topology invalidation (C6-1):** the regular-package predicate depends on
    **other files** (`__init__.py` presence in every ancestor dir), but sync
    **content-hash-skips unchanged descendants** (`code_graph.rs:1252-1263`), so adding
    or removing a `p/__init__.py` would leave **stale** `canonical_path` values on
    descendant defs (false/missing edges). T3 must **treat an add/remove of any
    `__init__.py` as a package-topology change that reindexes/invalidates the affected
    descendants** (recompute their canonical paths past the content-hash skip), for
    **both** the add transition (namespace/`src` → regular package) and the delete
    transition (regular package → namespace).
* **Files**: `code_graph.rs`, `tests/integration/code_graph_test.rs`
  (`integration_code_graph`).
* **Tests (RED→GREEN, 4)**: a `.py` def in a proven regular-package chain gets
  `canonical_path="mod.f"`; an `__init__.py` def gets `""` while a def under a `src/`
  source-root or an implicit PEP 420 namespace package gets `""` (Q3 fail-closed);
  two same-file defs of `f` both persist their (identical) canonical path — proving
  the **duplicate** state the resolver later fails closed on (ties to FF7DE872
  non-subsumption); **C6-1 topology transitions** — **adding** `p/__init__.py`
  recomputes descendant canonical paths from `""`→`p.mod.f` and **removing** it
  invalidates them back to `""`, both past the content-hash skip.
* **Verification**: `cargo test --test integration_code_graph`; clippy; fmt.
* **Milestone**: Python defs carry exact canonical identity only on a provable
  regular-package chain; every other layout stays `""` (fail closed); **package-topology
  edits invalidate stale descendant canonical paths (C6-1).**

### T4 — Emit canonical-eligible Python calls with provenance (domain: code)

* **Changes**: `src/services/parsing/python.rs`. In `resolve_call_name`, when
  `function` is an `attribute` whose `object` is a **simple identifier** `r` **that
  is not `self` or `cls`** (M2), emit
  `is_method:false, is_qualified:true, raw_qualifier:r, qualifier_kind:"module"`
  (candidate — the resolver fails closed if `r` is not a bound module).
  * **`self.foo()` / `cls.bar()` (M2)**: the object *is* a simple identifier, but
    `self`/`cls` are explicitly **excluded** — they are instance/class receivers
    (out of scope, need type inference), so they stay dropped (`is_method:true`,
    empty qualifier). This keeps T4's acceptance criterion (`self.foo()` unstaged)
    internally consistent.
  * Attribute calls whose object is **not** a simple identifier (`obj.attr.y()`,
    `a().b()`) also stay dropped.
  * Bare identifier calls unchanged at the parser (canonical routing in T5a).
* **Files**: `python.rs`, `tests/unit/parsing_test.rs` (`unit_parsing`).
* **Tests (RED→GREEN, 4)**: `mod.func()`→one `Calls` with
  `is_qualified:true, raw_qualifier:"mod", qualifier_kind:"module"`; `self.foo()`
  **and** `cls.bar()` → **no** promoted/staged edge (empty-qualifier drop; M2);
  `obj.attr.bar()`→dropped; bare `foo()` still `is_method:false, is_qualified:false`.
* **Verification**: `cargo test --test unit_parsing`; clippy; fmt.
* **Milestone**: `module.func()` becomes a canonical-eligible staged call; `self`/
  `cls`/non-module receivers still fail closed.

### T5a — Route Python calls into canonical staging (domain: code)

* **Changes**: `src/services/code_graph.rs` — **both** Calls-consumer sites so
  version-triggered re-extraction (T7) can never strand Python calls on the legacy
  name-only path (X1, same class as Q2/R1): the **full-index arm (851-908)** **and the
  sync arm (1573-1643)** — plus `should_stage_provenance_call` (188-194),
  language-dispatched on the caller file's language:
  * accept `qualifier_kind=="module"` for Python into provenance staging;
  * for Python **bare** calls whose callee is unresolved in-file, stage via
    `put_staged_call_with_provenance` with `qualifier_kind="python_bare"` instead
    of the name-only `put_staged_call` — **on both arms** (the sync arm's bare-call
    `put_staged_call` at 1639 must also route through provenance for Python).
  * **In-file bare calls are SHADOW-CONTEST-aware over SCOPES *and* FORMS (F2 root-defect,
    generalized cycle-7 Z1/Z2/C7-2; non-import-rebind axis closed cycle-8 C6):** the
    `(Some,Some)` branch at `code_graph.rs:900-902` currently mints a
    **direct `M.callee` edge** whenever the callee resolves against the current file's
    symbols — *before* any `python_bare` staging. For `def parse(); from bar import parse;
    parse()` that emits a **false direct `M.parse` edge** (frozen rule requires
    `bar.parse`) and renders T5b's last-binding-wins branches **unreachable**. The cycle-6
    F2 fix keyed the skip on **named module-level imports only**; cycle-7 extended it to
    star/scoped imports — but **both keyed only on producer-backed import signals**. The
    **non-import module/scope rebind axis (C6)** — `parse = factory()` / `class parse` /
    `del parse` / `match`-case / `for`/`with … as`/`except … as`/walrus/parameter targets —
    has **no producer**: T2 and T2b walk only import nodes, so these forms leave the
    predicate false and the arm still mints a false direct `M.parse` edge that bypasses
    T2b/T5b/T5c. **T5a owns modifying `code_graph.rs:896-908` on BOTH arms** so that, for
    Python, a same-file `def` is routed through `python_bare` staging **whenever its name is
    contested by EITHER**: (i) a **producer-backed import signal** — a named module import,
    a positioned **star-import invalidator** (T2), or a **scoped function-local /
    enclosing-function import** (T2b); **OR** (ii) **any non-import rebind form in the
    Shared rebind-form set (RFS, §Design decision)** that T5a detects with **its own in-file
    rebind-form scan — the SAME module/scope rebind-form scan T5c specifies (§T5c)**. Since
    `parse_source` retains no tree-sitter `Tree` (`ParseResult{symbols, edges}` only,
    `parsing.rs:247-252`), **T5a re-parses the in-scope file `source: String`** (already
    cloned at the seam — full-index ~`code_graph.rs:609`, sync ~`1290`) **in-memory**,
    building the file's **Rebind-Form-Set map ONCE PER FILE (cached, reused across all calls
    in the file, not per call-site)** — **no new producer, DB field, or DAG edge**.
    **Self-exclusion + lexical scope (C9-1):** the scan **excludes the matched `def` under
    consideration** (that def is the candidate target, not a self-competitor) and counts only
    **OTHER** bindings of the name **applicable to the caller's lexical scope** (module +
    enclosing functions + the caller's own function — not unrelated sibling-scope
    locals/parameters). A same-file callee keeps the `direct`-edge fast path **only when its
    matched def is PROVABLY the sole applicable binding** — no OTHER import/star/scoped signal
    **and** no OTHER RFS form in the caller's scope; else it is staged. (Without self-exclusion
    the `def` RFS form would self-contest and the direct fast path would be unreachable dead
    code.) The prior "unchanged (code_graph.rs:900-903)" claim is **removed**.
  * **Rust behavior is untouched** on both arms (dispatch guards every new branch on
    `Language::Python`).
  * **No-module-context / no-binding recall (Y3 + F3):** T5a still stamps
    `python_bare` for cross-file bare calls on both arms; recall is **preserved by
    T5b**, which routes any bare call for which it **derives no canonical target** — a
    layout T1 rejects (`src/`-root / PEP 420 namespace / `__init__.py`) **or** a
    provable-namespace module whose callee has no import binding and no in-module def
    (star / re-export / relative, **F3**) — to the **legacy name-only unique-match** (no
    regression, no false module edge). The recall is asserted in T5b's tests, not
    restaged here.
* **Files**: `code_graph.rs` (two consumer sites), `tests/integration/code_graph_test.rs`.
* **Depends on T2 and T2b** (in addition to T4): T5a consults **T2's `ImportBindings`**
  (named module imports + positioned star markers) **and T2b's scoped-binding model**
  (function-local / enclosing-function imports) to decide whether a same-file callee is
  shadow-contested by an **import** at any modeled scope. Cycle-6 added the **T2→T5a**
  edge (F2); cycle-7 added the **T2b→T5a** edge (Z1/Z2/C7-2 generalization). The
  **non-import rebind forms (RFS, C6) have no producer** — T2/T2b walk only import nodes —
  so **T5a scans for them itself** by re-parsing the in-scope file `source: String` in-memory
  at the consumer site (`parse_source` keeps no `Tree`; source is already cloned at the seam),
  caching the Rebind-Form-Set map once per file (the same scan T5c runs). **No new producer,
  task, or DAG edge is required** for the C6 closure. Both existing edges are stated in the
  Dependency Graph; acyclicity re-verified (T2b is upstream of T5b/T5c, so T5a←T2b introduces
  no cycle).
* **Tests (RED→GREEN, 4)**: (1) Python cross-file bare call produces a staged row with
  `qualifier_kind=="python_bare"` **on both the full-index and sync arms** (the sync arm no
  longer name-only-stages Python bare calls); (2) Python `mod.func()` produces a staged row
  with `qualifier_kind=="module"`; (3) **shadow-contest routing table (F2 + Z1/Z2/C7-2 +
  C6)** — a same-file `def parse` whose name is contested by **any** of {a later named
  import; a `from n import *` after the def; a **non-import module rebind
  `parse = factory()`** (and `class parse` / `del parse` / `match`-case capture); a
  function-local import `def g(): from bar import parse; parse()`; an enclosing-function
  import} is **routed to `python_bare` staging (assert NO direct `M.parse` edge)** on both
  arms — the `def f; f = factory(); f()` case in particular MUST NOT mint a direct `M.f`
  edge — whereas an in-file callee **provably the sole binding** (no import/star/scoped
  signal and no RFS form) still yields a `direct` edge; (4) a Rust cross-file bare call is
  **still** name-only staged on both arms (regression). *(Vectors consolidated into one
  parametrized routing-table scenario to stay within the 2-hour rule while covering every
  scope and form axis by construction.)*
* **Verification**: `cargo test --test integration_code_graph`; existing Rust
  calls tests unaffected; clippy; fmt.
* **Milestone**: Python canonical-eligible calls reach the canonical post-pass **from
  both indexing arms**; Rust staging is provably unchanged.

### T5b — Python canonical target resolution in the post-pass (domain: code)

* **Changes**: `src/services/code_graph.rs`
  `reresolve_calls_edges_with_canonical_context` (264-368). Add
  `python_ctx_for_staged_file` (module path via T1 + **scope-aware bindings via
  T2b**, read from source like `rust_ctx_for_staged_file`) and a Python branch
  dispatched by the **caller file's language** that computes the target canonical path
  **using the T2 binding _kind_ (R2)**. This stage is **shadow-guard-agnostic (R3): it
  does NOT invoke the T5c guard** — shadow-rebind handling is added downstream by T5c,
  which keeps T5b independently completable.
  * `qualifier_kind=="python_bare"` — **call-site-effective binding (X2/X3 + C7-1),
    prove-or-fail-closed**, defined **over function-body calls only** (the extractor stages
    `Calls` only from function bodies — C9-3): T5b resolves to the module / enclosing binding
    of `callee` that is **statically stable when the call runs**, else fails closed.
    Concretely, using the T2/T2b **source positions (F14)**:
    * a function body runs when the function is *called*, so its module binding is provable
      **only when it is not contested by a later module-level rebind** of `callee` (the
      function may run before or after such a rebind). When stable, pick the last
      module-level binding of `callee`: a `FromImportSymbol` (no later rebind) →
      `N.callee` (import WINS); a module-local `def` (no later rebind) → `M.callee`; an
      enclosing-function binding shadows the module binding per T2b (F5);
    * a **later module-level rebind** of `callee` after the winning binding makes the
      invocation order undecidable → **fail closed** (C7-1);
    * a `ModuleImport`-kind name used as a bare callee is not a function → fail closed;
      when execution order can't establish a stable call-site binding, **fail closed**. (A
      local-def-first rule would mint `M.parse` for the X2/X3 `def parse(); from bar import
      parse; parse()` counterexample — a false edge; a whole-module last-binding-wins that
      ignored call-time stability would mint an edge for a function-body call contested by a
      later rebind — also false.) **Module-level (top-level) call ordering is a v1 non-goal**
      — top-level calls are not extracted (C9-3).
    * **duplicate same-name imports → fail closed (C8-1):** when a scope has **two or more
      competing imports of the same name** (`from a import f; from b import f; f()`), T2
      produces **no** binding (M1), so there is no resolvable last import binding and T5b
      **fails closed** (no canonical edge) with the typed **`DuplicateSameNameImport`**
      no-target reason (C9-4). The call-site ordering above resolves `def`-vs-**single**-import
      + rebinds, **not** import-vs-import of the same name; the F3 legacy fallback is **not
      taken** for this reason (emitting the sole indexed `a.f` would leak a wrong-callee edge
      when Python's effective target is the later `b.f`). No regression vs main (name-only
      ambiguous → drop today).
  * `qualifier_kind=="module"`: the receiver must resolve to a **`ModuleImport`**
    binding (`import pkg` / `import a.b as c`) → `<module>.callee`; a
    **`FromImportSymbol`** receiver (`from pkg import parse; parse.tokenize()`) is an
    attribute access on an object, **not** a module → fail closed (R2); else fail closed.
  * **No-target legacy fallback (Y3 + F3) — gated on a TYPED no-target reason (C9-4):**
    T5b's resolution outcome carries a **typed no-target reason** whenever it derives no
    canonical target, and the **legacy name-only unique-match** fallback fires **only** for
    the two recall-safe reasons:
    * **`NoModuleContext`** — `python_ctx_for_staged_file` yields no module path (T1 rejected
      the `src/`-root / PEP 420 / `__init__.py` layout);
    * **`UnsupportedImportForm`** — a module path exists but the callee has **no import
      binding and no in-module def** (star-imported / re-exported / relative — **F3**).

    Gating the fallback on `T1==None` alone would DROP the provable-namespace-but-unbound
    calls: the `python_bare` stamp excludes them from `reresolve_calls_edges`
    (`qualifier_kind.is_empty()` filter, `cozo_queries.rs:2222-2227`), stranding a unique
    cross-file edge that resolves **today** — the F3 recall regression. The fallback uses a
    **public language-scoped name→IDs singleton helper (F9)** extracted from the
    inline-private CozoScript at `cozo_queries.rs:2191-2205,2263-2270`; a non-unique name
    still fails closed (legacy parity).

    The fallback is **NEVER** taken for the fail-closed reasons **{`CompetingBindings`,
    `Shadowed`, `DuplicateSameNameImport`}** — a call dropped because a **competing/duplicate
    same-name import** (C8-1: `from a import f; from b import f`), a **later shadowing rebind**,
    or any other contested binding that could not be statically resolved stays **fully
    fail-closed (NO edge)**. Emitting the sole indexed legacy `a.f`/`M.f` there would leak a
    **wrong-callee** edge when Python's effective target is the later `b.f`. This makes C8-1
    genuinely complete: the duplicate-import case carries `DuplicateSameNameImport` and gets
    **no fallback**. The legacy name-only edge (for the two allowed reasons only) is a
    name-only edge (not binding-derived), so it is **outside the T5c canonical shadow-guard
    scope** — unchanged legacy semantics.
  then reuse the existing `canonical_index.get(&target)` **singleton** match
  (`ids.len()==1`) — dropping on zero, ambiguous, or **duplicate** canonical path.
  **F9: T5b adds a small public `function_ids_by_name`-style language-scoped helper in
  `cozo_queries.rs`** (extraction of the resolver's inline singleton — a read-only
  helper exposure, **not** a schema change); the earlier "No `cozo_queries.rs` change"
  claim is **removed**.
* **Files (3)**: `code_graph.rs`, `cozo_queries.rs` (the F9 helper exposure),
  `tests/integration/calls_recall_acceptance_test.rs`
  (`integration_calls_recall_acceptance`). (Three files, like T1's four — acceptable for
  a single-domain code task; ≤4 scenarios preserved.)
* **Tests (RED→GREEN, 4)** — **no shadowing here (R3; shadow cases live in T5c)**: two
  modules both define `parse`; caller does `bar.parse()` with `import bar` → edge
  resolves to **bar's** exact `parse` id (target-identity, not row-existence); **call-site
  ordering (X2/X3 + C7-1)** — bare `parse()` with `from bar import parse` → bar's `parse`;
  with BOTH a local `def parse` AND a **later** `from bar import parse` (function-body call;
  the import is the last **stable** module binding) the import rebind wins → bar's `parse`;
  and a **function-body call with a local `def parse` and NO later rebind** binds the def →
  `M.parse` (C7-1 stable binding), while a **function-body call contested by a later module
  rebind fails closed** (C7-1 undecidable order; module-level call ordering is a v1 non-goal —
  top-level calls not extracted, C9-3); **fail-closed
  vectors** — a **`from pkg import parse`** receiver used as `parse.tokenize()` → **no**
  module edge (R2 kind), **and** ambiguity (**star** `from bar import *` then bare
  `parse()` with 2+ `parse`, **and** two same-file `parse` defs / duplicate canonical
  path) → **no** canonical edge (FF7DE872 stays unfixed here); **no-target legacy
  recall (Y3 + F3)** — a **unique** cross-file bare call still gets its **legacy
  name-only** edge in **both** the T1-rejected-layout case (`src/`-root / PEP 420) **and
  the provable-namespace-but-unbound case** (star / re-export / relative with no
  binding), while a non-unique one still fails closed (legacy parity).
* **Verification**: `cargo test --test integration_calls_recall_acceptance --test
  integration_code_graph`; clippy; fmt.
* **Milestone**: cross-module same-name Python calls resolve to exact targets **via the
  T2 binding kind**; every ambiguity fails closed; **every bare call for which T5b
  derives no canonical target keeps its legacy name-only recall (Y3 + F3)**.
  **Shadow-rebind handling is deferred to T5c (R3), which wraps this resolution with an
  order-aware guard anchored on the winning binding (Y2 + F1).**

### T5c — Shadow guard: module receiver + bare import (domain: code)

* **Why**: an imported name — whether used as a **module receiver**
  (`import bar; bar = factory(); bar.parse()`) **or as a bare callee**
  (`from bar import parse; parse = factory(); parse()` — Q1) — can be shadowed by a
  later binding. Static binding-only resolution would still bind the call to
  `bar.parse`, a false edge violating 013-D. The bare-callee vector was left open by
  cycle-1 (the guard covered only module receivers). (Plan-review P1; M1; **Q1**.)
* **Changes (R3 — guard lives ENTIRELY here; T5b stays guard-agnostic)**:
  `src/services/code_graph.rs` — **wrap** T5b's guard-agnostic resolution in the Python
  branch of `canonical_target_for_staged_call` so that, **after** T5b computes a
  candidate canonical target, T5c **fails closed** when the resolved name is re-bound by
  a **later** binding — one that occurs **after the WINNING binding T5b actually
  resolved to** (the import **or** the in-module `def` — **F1, order-aware anchor**) in
  binding order — in the **applicable scope** (caller function **and** module level),
  consuming the **T2b** scope model. **F1 re-anchor:** the guard is anchored on the
  *winner*, **not** on "the import". For **def-after-import**
  (`from bar import parse; def parse(): ...; parse()`) T5b's last-binding-wins resolves
  to **`M.parse`** (the def is the later binding); since **no** rebind follows that
  winning def, T5c must let the edge **SURVIVE** — anchoring on "the import" (cycle-5 Y2)
  would wrongly treat the winning def as a post-import rebind and drop it. This wrapping
  keeps the dependency one-directional (T5c depends on T5b, never the reverse). The
  guard applies to **both**:
  * `qualifier_kind=="module"` — the **receiver** name (`bar` in `bar.parse()`);
  * `qualifier_kind=="python_bare"` — the **bare callee** name (`parse` in
    `parse()`), when T5b resolved it via a `FromImportSymbol` binding or an in-module
    def (Q1).

  The rebind scan covers the **Shared rebind-form set (RFS)** defined once in §Design
  decision and consumed **identically** by **T5a's routing scan** (C6) — so the two scans
  can never diverge on which forms count. The RFS is the **full target set (M1, X4)**:
  plain `assignment` **and augmented assignment** (`+=`, …); `for` targets; `with … as`;
  `except … as`; **walrus** `(:=)`; function **parameters**; **`def` / `class`
  definitions**; **`del` statements**; **`match` / `case` capture patterns**; a later
  module-scope **`from N import *` star import (F4)**; and **module-level** rebinding.
  **T5c applies this set ORDER-AWARE** (below), whereas T5a's routing use is the coarser
  order-agnostic "any RFS form present → stage"; the **shared artifact is the syntactic
  form set, not the order logic**.
  **Order-aware (Y2 + F1 + C7-1): each of these invalidates the resolved edge ONLY when
  it occurs AFTER the WINNING binding (import or def) AND within the window that can affect
  the call site** — i.e. strictly **between the winning binding and the call** at module
  scope (a *later* rebind that shadows the winner before the call executes). A
  `def`/`class`/`del`/`match`-capture/star or any rebind **BEFORE** the winner does NOT
  invalidate it — the winner is then the last binding and T5b's resolution stands.
  **C7-1 (call-site-effective, function-body calls only):** since the extractor stages
  `Calls` only from function bodies (module-level calls are never staged — C9-3), a
  **function-body call whose module binding is contested by a later module-level rebind**
  (invocation order unknown — the function may run before or after it) is not statically
  provable → **fail closed**. Module-level (top-level) call ordering is a documented **v1
  non-goal** (top-level calls not extracted). Two symmetric module-scope cases both
  survive: `def parse(); from bar import parse; parse()` → **`bar.parse`** (winner =
  import, no shadowing rebind before the call; Y2) **and** `from bar import parse; def
  parse(): ...; parse()` → **`M.parse`** (winner = def, no later rebind; **F1**). A rebind
  in the invalidating window still drops the edge (e.g. `import bar; class bar: ...;
  bar.parse()` → **no** edge, X4; `from bar import parse; from n import *; parse()` →
  **no** edge, **F4** star-after-import). Effective binding order is module source order at
  module scope and, within a function, Python's function-local-for-whole-body rule (T2b,
  Y1). Reuse the already-loaded caller/module source (T5b/T2b context) — a cheap
  tree-sitter scan; no new DB call.
* **Files**: `code_graph.rs`, `tests/integration/calls_recall_acceptance_test.rs`.
* **Tests (RED→GREEN, 4)**: **(a)** clean & ordered cases resolve — `import bar;
  bar.parse()` and `from bar import parse; parse()` (no rebind), **plus BOTH winner-
  anchor survivals**: `def parse(); from bar import parse; parse()` → **`bar.parse`**
  (winner = import, Y2) **and** `from bar import parse; def parse(): ...; parse()` →
  **`M.parse`** (winner = def; **F1 def-after-import survives**), **and the C7-1 function-body
  fail-closed** — a function-body call whose winning module binding is contested by a **later
  module-level rebind** (`def parse(): ...; def g(): parse(); from bar import parse`) → **no
  edge** (undecidable invocation order); module-level (top-level) call ordering is a **v1
  non-goal** (top-level calls not extracted, C9-3); **(b)** module-receiver
  rebind table — **each rebind occurring AFTER the winner** {assignment, module-level
  `import bar; bar = factory(); def g(): bar.parse()`,
  `for`/`with … as`/`except … as`/`:=`/augmented/parameter, **`def`/`class`/`del`/
  `match`-case capture (X4)** e.g. `import bar; class bar: ...; bar.parse()`, **and a
  later `from n import *` (F4)** e.g. `from bar import parse; from n import *; parse()`}
  → **no** edge;
  **(c) bare-import assignment shadow** `from bar import parse; parse = factory();
  parse()` → **no** edge (Q1); **(d) bare-import parameter shadow**
  `from bar import parse` with `def g(parse): parse()` → **no** edge (Q1).
* **Verification**: `cargo test --test integration_calls_recall_acceptance`; clippy;
  fmt.
* **Milestone**: shadowing of **either** a module receiver **or** a bare imported
  callee, by any rebind form **that occurs after the WINNING binding** in the applicable
  scope (order-aware, Y2 + F1), cannot mint a false edge — while an **earlier** rebind
  (and a def-after-import that IS the winner) leaves the resolution intact — **enforced
  downstream of T5b (R3), keeping T5b independently completable.**

### T7 — Rollout: versioned re-extraction + one-step resolution backfill (domain: code)

* **Why (M4 + Q2)**: existing indexes will **not** acquire Python canonical edges
  through normal incremental indexing. Full-index and sync skip files whose content
  hash is unchanged (`code_graph.rs:590-599` and `1252-1263`), `force` defaults to
  `false`, and the canonical call post-pass runs on the **full-index path only**
  (`code_graph.rs:985-992`). **Q2:** a versioned-hash bump also fires on startup
  auto-**sync**, but sync re-extracts + clears prior resolved edges/staging and then
  **exits without running the post-pass** — leaving staged calls **unresolved** until
  a later full index (a regression window). Re-extraction alone is not enough.
* **R1 — do NOT mix the extraction version into `file_node.content_hash`.**
  `retrieval_eval::is_index_stale` (`src/services/retrieval_eval.rs:717-718`) compares
  `file_node.content_hash` **byte-for-byte** against the raw source SHA-256
  (`source_content_hash`, 699-700), and the indexer writes `content_hash` as that raw
  SHA (`file_node {…, content_hash, …}`, `cozo_queries.rs:609` / `schema.rs:573`).
  Folding a version into it would break staleness detection — a hard repo invariant.
* **Changes**: track a `PYTHON_CANONICAL_EXTRACTION_VERSION` constant in a **dedicated,
  separate index-state marker** — **not** `content_hash` — following the existing
  versioned-index precedent (`TMDL_DAX_INDEX_VERSION` + `compute_tmdl_dax_index_hash`,
  `powerbi_indexer.rs:60-81`, which persists its versioned hash in a **separate** record,
  never `file_node.content_hash`). On index/sync, when the stored extraction version
  differs from the code constant, re-extract the affected `.py` files; `file_node.
  content_hash` stays the raw source SHA (staleness detection intact). **Crucially (Q2),
  the same operation must also run the full canonical resolution pass** — either escalate
  the run to the full-index path (which runs the post-pass) or invoke the canonical
  post-pass from the sync path — so resolved edges are **restored in one operation**,
  never left staged-but-unresolved; then persist the new extraction version. Documented
  fallback: a one-shot forced `index --force`. Replace the "no migration" claim with this
  real, single-step backfill trigger. **No canonical-schema change** (`function_meta` /
  `calls_edge` unchanged); the extraction-version marker is orthogonal index-state, not
  the canonical data model, and does not alter the `content_hash` contract.
* **C7-3 — persist the new marker ONLY on a fully-successful pass (partial-failure gate).**
  Sync records per-file read/parse failures in `SyncResult.errors` and **continues**
  (`code_graph.rs:1215-1221`). If the extraction-version marker is persisted after a
  **partial** run, the next sync sees a current version and **hash-skips the failed,
  unchanged files indefinitely** — a *stale-forever* migration hole (their Python canonical
  edges never backfill). The upgrade gate must therefore **keep the OLD extraction-version
  marker whenever the pass had ANY per-file failure** (a non-empty `SyncResult.errors` for
  the affected `.py` set), so migration **retries on the next sync**; the new version is
  persisted **only** after every affected Python file AND the resolution pass succeed. When
  the pass outcome is uncertain, keep the old marker (fail closed toward retry, never
  toward stale-skip).
* **Files** (≤3): `src/services/code_graph.rs` (extraction-version constant + marker
  read/compare/persist + one-step resolution), the index-state seam that holds the
  marker, `tests/integration/code_graph_test.rs`.
* **Tests (RED→GREEN, 4)**: **upgrade regression** — after an extraction-version-bump
  **sync** of an unchanged-hash `.py` file, the cross-module **resolved edge is present**
  (assert the RESOLVED EDGE, not merely the def's `canonical_path`) (Q2); **content-hash
  contract (R1)** — the same file's `file_node.content_hash` still equals the raw source
  SHA and `is_index_stale` returns false for unchanged source (version tracked
  separately, staleness intact); **partial-failure gate (C7-3)** — when the version-bump
  pass hits a per-file read/parse failure (non-empty `SyncResult.errors`), the extraction
  version is **NOT** advanced and the **next** sync **re-attempts** the failed unchanged
  file (no stale-forever skip); a file already at the current extraction version is still
  skipped (fast-path preserved) and Rust files are unaffected.
* **Verification**: `cargo test --test integration_code_graph`; clippy; fmt.
* **Milestone**: an upgrade backfills Python canonical **edges** in one operation with
  no unresolved-edge window; `content_hash` staleness detection is preserved (version in
  a separate marker); **the new version persists only on a fully-successful pass so a
  partial failure retries next sync (C7-3)**; the fast-path is preserved for
  current-version files.

### T6 — Documentation (domain: docs)

* **Changes**: document Python namespace-qualified call resolution and its **v1
  non-goals** in `docs/ARCHITECTURE.md` / `docs/QUALITY_SCORE.md`: instance-method
  dispatch NOT resolved (needs type inference); re-exports, relative/package-root
  (`__init__.py`, PEP 420), star, and dynamic imports fail closed; **`src/`-layout /
  source-root package resolution is a v1 non-goal (Q3) — those defs fail closed to
  no canonical path**; a module receiver **or a bare imported callee** shadowed by a
  local/parameter/any rebind fails closed (T5c); the upgrade backfill trigger (T7);
  FF7DE872 (same-file shadowing) is a separate, independent fix.
* **Files**: `docs/ARCHITECTURE.md` (and/or `docs/QUALITY_SCORE.md`).
* **Verification**: prose review; no build impact.
* **Milestone**: documented capability + honest limitations.

## Dependency Graph

```text
T1 (module-path) ─┬─▶ T3 (populator) ──▶ T7 (rollout/backfill) ─┐
                  └─▶ T5b (resolver)                            │
T2 (bindings) ─┬─▶ T2b (scope isolation) ─┬─▶ T5a (staging) ────┼─▶ T6 (docs)
               │                          ├─▶ T5b               │
               └─▶ T5a (staging) ─────────┴─▶ T5c (guard)       │
T4 (parser emit) ─▶ T5a ─▶ T5b ─▶ T5c (shadow guard) ───────────┘
```

No cycles. T1 and T2 are independent primitives. T2b refines T2 (scope model). T3
needs T1. T4 is parser-independent of T1–T3. **T5a needs T4, T2 AND T2b** — T5a's
shadow-aware in-file routing consults **T2's `ImportBindings`** (named module imports +
positioned star markers, the **T2→T5a** edge added cycle-6 for F2) **and T2b's
scoped-binding model** (function-local / enclosing-function imports, the **T2b→T5a** edge
added **cycle-7** for the Z1/Z2/C7-2 generalization) to decide whether a same-file callee
is shadow-contested at any modeled scope. T5b needs T1, T2b, T3, T5a — and is
**guard-agnostic (R3): it does not depend on T5c and is independently completable**. T5c
**wraps** T5b's resolution with the shadow guard (downstream) and consumes T2b's scope
model — the dependency is strictly **T5c → T5b**, never the reverse. T7 (rollout/backfill)
needs the populator (T3) and resolver (T5b) to be real. T6 needs the capability real (T3,
T5b, T5c, T7). Edge list (acyclic): T1→{T3,T5b}; T2→{T2b,**T5a (F2)**};
T2b→{**T5a (Z1/Z2/C7-2, cycle-7)**,T5b,T5c}; T4→T5a; T5a→T5b; T3→{T5b,T7,T6};
T5b→{T5c,T7}; {T3,T5b,T5c,T7}→T6. **This cycle-7 revision adds exactly one edge —
T2b→T5a — so that T5a's routing predicate can consult the scoped-binding signals; T2b is
already upstream of T5b/T5c and only depends on T2, so adding T5a←T2b introduces NO cycle
(no back-edge into T2b). The task graph is otherwise unchanged and remains acyclic
(cycle-6's T2→T5a and T3→T6 reconciliations stand).**

## Decisions and Rationale

* **Zero DB/schema change.** `function_meta.canonical_path`,
  `function_ids_by_canonical_path`, `canonical_paths_for_function_name`, and the
  staging queries are language-agnostic; reusing them removes the highest-blast
  layer from the change. Verified in the spike.
* **Option A (provenance staging) over Option B (bare-name-pass edit).** Keeps the
  operator-gated `reresolve_calls_edges` **resolver logic** untouched and all canonical
  logic in one place; keeps Rust output byte-identical. **F9 nuance:** T5b exposes a
  small **read-only language-scoped name→IDs helper** in `cozo_queries.rs` (a minimal
  extraction of the resolver's inline-private singleton, used for the no-target legacy
  fallback) — this is a **helper exposure, not a resolver rewrite or schema change**,
  and is distinct from Option B's rewrite of the bare-name pass.
* **Parser emits module-qualified candidates; the resolver disambiguates.** The
  parser cannot know whether an attribute receiver is a module or an object, so it
  emits a `qualifier_kind="module"` candidate with the receiver, and the
  binding-aware resolver fails closed when the receiver is not a bound module. This
  keeps the parser simple and the receiver-classification honest. **`self`/`cls`
  receivers are excluded at the parser (M2)** — they are instance/class dispatch
  (out of scope), so they never become module candidates.
* **Scope-aware, fail-closed bindings (M1, Q1).** Import bindings are modeled with
  module-level vs per-function scope (T2b), a function-local import never leaks to a
  sibling, and any competing/duplicate binding — or any rebind of the **module receiver
  OR bare imported callee** name in the applicable scope (assignment/augmented/`for`/
  `with`/`except`/walrus/param/module-level, T5c) — fails closed. A flat file-wide
  `HashMap` is forbidden.
* **Module path requires a provable regular-package chain (M5, Q3).** T1 derives a
  regular-package predicate (dirs with `__init__.py`) from the indexed file set — **no
  config/source-root field** (none exists, `config.rs:98-120`); `src/`-roots, implicit
  PEP 420 namespace packages, and `__init__.py` are REJECTED (fail closed).
  Source-root-aware resolution is an explicit **v1 non-goal**.
* **Binding kind disambiguates module vs symbol (R2).** `ImportBindings` records a
  `kind ∈ {ModuleImport, FromImportSymbol}` beside the canonical path so T5b resolves a
  **module receiver** (`import pkg; pkg.func()` → `pkg.func`) distinctly from an
  **imported symbol** (`from pkg import parse` → bare `parse()` = `pkg.parse`; the
  attribute `parse.tokenize()` = a call on an object → fail closed, out of scope).
  Without the kind, a from-import would be mis-resolved as a module and mint a wrong edge.
* **Shadow guard is downstream of resolution (R3).** T5b produces guard-agnostic
  binding-kind resolution; T5c **wraps** it with the rebind shadow check. The dependency
  is one-directional (T5c → T5b), so T5b is independently completable and the DAG stays
  acyclic while the guard still covers both module receivers and bare imports (Q1).
* **Upgrades backfill edges in one operation via a SEPARATE version marker (M4, Q2, R1).**
  Existing indexes do **not** gain Python canonical paths through content-hash-skipping
  incremental indexing, and **sync re-extracts but never runs the post-pass**. The
  extraction version is tracked in a **dedicated index-state marker
  (`PYTHON_CANONICAL_EXTRACTION_VERSION`), NOT mixed into `file_node.content_hash`** —
  which `retrieval_eval::is_index_stale` compares byte-for-byte against the raw source
  SHA (`retrieval_eval.rs:717-718`); mixing would break staleness detection. Following
  the `TMDL_DAX_INDEX_VERSION` precedent (`powerbi_indexer.rs:60-81`, a separate
  versioned record), a version mismatch triggers re-extraction **and runs the canonical
  resolution pass in the same step** (escalate sync→full path or invoke the post-pass) —
  or a documented forced `index --force`. Resolved edges are restored in one operation
  (T7); `content_hash` stays the raw SHA. No silent "no migration" claim.
* **Fail-closed everywhere.** Star/relative/package-root/re-export/dynamic produce
  no module or binding; ambiguous/duplicate canonical paths are dropped by the
  reused singleton check. The canonical path is exact, so the feature only *adds*
  precision over name-only matching.
* **FF7DE872 explicitly not fixed.** Same-file duplicates yield identical
  canonical paths → the singleton check drops them (not last-wins). Documented and
  test-pinned (T3, T5b) so the boundary can't silently erode.
* **Test-first, split by domain.** Each unit observes red before green and touches
  one skill domain, satisfying Principle II and width isolation.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| **Wrong module-path mints a false edge** (the one real correctness risk): `__init__.py`, PEP 420 namespace packages, `src/`-layout roots | **T1 resolves a dotted path only when every ancestor dir is a provable regular package (`__init__.py`), predicate derived from the indexed file set — no config source (M5, Q3)**: `__init__.py`, `src/`-style roots, and implicit PEP 420 namespace packages REJECT → `None`→`""` (never a match target, D4); T1 tests pin `src/`-layout + implicit-namespace + `__init__.py`; T3/T5b tests pin `__init__.py`→`""` |
| **Recall trade-off: `src/`-layout & namespace-package defs get no canonical path (Q3 narrowing)** | Deliberate fail-closed narrowing (Constitution VI — no speculative source-root config; none exists at `config.rs:98-120`): those calls fall back to the existing **name-only** unique-match — **T5b routes no-module-context bare calls to the legacy matcher (Y3)** since T5a's `python_bare` stamp would otherwise exclude them — no regression vs today, never a false edge; source-root-aware resolution is a documented **v1 non-goal** (T6) for a future iteration with a real config source |
| **Any rebind shadows an imported module receiver OR bare callee** (`import bar; bar = f(); bar.parse()` **or** `from bar import parse; parse = f(); parse()` **or** `import bar; class bar: ...; bar.parse()`) → false edge | **T5c fails closed on the full rebind-target set — but only for a rebind that occurs AFTER the WINNING binding T5b resolved to (import OR def) in effective order (order-aware anchor, Y2 + F1; an earlier rebind — and a `def`-after-import that IS the winner, e.g. `from bar import parse; def parse(); parse()` → `M.parse` — leaves the resolution intact) — in the applicable scope for BOTH the receiver AND the bare callee name (M1, Q1, X4)**: assignment, augmented (`+=`), `for`/`with … as`/`except … as`/walrus/parameter, **`def`/`class`/`del`/`match`-case capture (X4)**, a **later `from N import *` star (F4)**, and module-level rebind; consumes T2b's scope model |
| **Root defect: in-file `(Some,Some)` direct-edge shortcut mints a false `M.callee` edge for a shadow-contested same-file def and hides last-binding-wins (F2; generalized cycle-7 Z1/Z2/C7-2; non-import-rebind axis closed cycle-8 C6)** (`def parse(); from bar import parse; parse()` → wrongly a direct `M.parse` edge at `code_graph.rs:900-902`; cycle-6's named-import-only predicate also missed `from n import *` after a def (Z1), non-import rebinds (Z2/C6 — `parse = factory()`/`class`/`del`/`match`-case), and function-local/enclosing imports (C7-2)) | **T5a makes the in-file bare-call decision SHADOW-CONTEST-aware over SCOPES *and* FORMS on BOTH arms (896-908 full-index + 1573-1643 sync) via the prove-or-fail-closed invariant: a same-file `def` is routed to `python_bare` staging whenever its name is contested by EITHER a producer-backed import signal — a named module import (T2), a positioned star-import invalidator (T2/F4), OR a scoped function-local/enclosing-function import (T2b) — OR any non-import rebind form in the Shared rebind-form set (RFS) that T5a detects with its OWN in-file tree-sitter scan (the same scan T5c runs; T2/T2b produce no non-import-rebind signal). Expressed as the prove-the-negative default "keep the direct edge ONLY when provably the sole binding across every scope AND with no RFS form present", so future scopes AND forms are covered by construction; a callee with no competing binding keeps the direct fast path. T5a OWNS this code change; the C6 closure needs no new producer, task, or DAG edge (T5a self-serves via an in-memory re-parse of the in-scope file source, cached once per file; the matched def is self-excluded from the contest — C9-1/C9-2).** |
| **Call-position ignored — a function-body call whose module binding is contested by a later module-level rebind is invocation-order-ambiguous (C7-1)** (`def parse(): ...; def g(): parse(); from bar import parse` — `g` may run before or after the import) | **T5b resolves to the binding EFFECTIVE AT THE CALL SITE, defined over function-body calls only (the extractor stages calls only from function bodies — top-level calls are not extracted, C9-3): a function-body call resolves to its module/enclosing binding ONLY when that binding is stable (no later module-level rebind of the name); a function-body call contested by a later module-level rebind FAILS CLOSED (prove-or-fail-closed on the time axis). Module-level (top-level) call ordering is a documented v1 non-goal — such cases are neither regressed nor newly handled. Stable-binding and fail-closed cases pinned in T5b/T5c.** |
| **Stale-forever migration: persisting the extraction-version marker after a PARTIAL upgrade run hash-skips the failed unchanged files indefinitely (C7-3)** (sync records per-file failures in `SyncResult.errors` and continues, `code_graph.rs:1215-1221`) | **T7 keeps the OLD `PYTHON_CANONICAL_EXTRACTION_VERSION` marker whenever the pass had ANY per-file failure (non-empty `errors` for the affected `.py` set), so migration retries on the next sync; the new version persists ONLY after every affected file AND the resolution pass succeed. A partial-failure regression test asserts the marker is not advanced and the failed file is re-attempted next sync.** |
| **Function-local import leaks / competing bindings overwrite** (flat file-wide map) → false edge from the wrong module | **T2 fails closed on competing/duplicate bindings; T2b scopes module-level vs per-function bindings so a local import never leaks (M1)** |
| **Function-local import resolves a call that textually precedes it** (`def g(): f(); from x import f`) → false edge to `x.f`, though Python raises `UnboundLocalError` at that call | **T2b tracks binding AND call positions: a name bound by a function-local import is local for the whole body — calls BEFORE the import fail closed (a poison/tombstone that does NOT fall through to module scope, F8), only calls AFTER resolve; uncertain control-flow ordering fails closed (Y1)** |
| **Enclosing-function (closure) binding wrong-scoped by a two-level model** → wrong-module resolution or wrongly-dropped edge (F5) | **T2b models the lexical closure chain (innermost → enclosing → module), honoring `global`/`nonlocal`, or fails closed when an enclosing function binds the name ambiguously (F5); nested-function scenarios pinned** |
| **From-import symbol mis-resolved as a module receiver** (`from pkg import parse; parse.tokenize()` treated as module `parse`) → wrong edge | **T2 records the binding kind (`ModuleImport` vs `FromImportSymbol`) (R2); T5b resolves a module receiver ONLY from a `ModuleImport` binding and fails closed on a `FromImportSymbol` receiver** (attribute-on-object, out of scope); T5b test pins `parse.tokenize()` → no edge |
| **Local-def-first mints a false edge when a later import shadows the local def** (`def parse(): ...; from bar import parse; parse()` → wrongly `M.parse`) (X2/X3) | **T5b applies call-site-effective last-binding-wins (X2/X3 + C7-1): a `FromImportSymbol` binding that rebinds the name over a local `def` and is the last STABLE module binding WINS → `N.callee`; the local def wins when it is the last stable binding; a function-body call contested by a later module-level rebind fails closed (C7-1); when execution order can't establish a stable call-site binding, fail closed**; T5b test pins the counterexample → `bar.parse`, **not** `M.parse`. **(This branch is only reachable because T5a's F2 fix routes the shadow-contested same-file call to staging instead of a direct edge — see the F2 root-defect row.)** |
| **`self`/`cls` wrongly staged as a module candidate** | **T4 explicitly excludes `self`/`cls` receivers (M2)**; they stay dropped (empty qualifier); T4 test pins `self.foo()`/`cls.bar()` unstaged |
| New `python_canonical` module trips `-D warnings` dead_code when landed before its consumers | T1/T2 ship with same-crate unit tests exercising each public fn (counts as use under `cargo test`/clippy `--all-targets`); T3/T5b add production call sites |
| tree-sitter-python node/field names differ from assumptions (`import_from_statement`, `dotted_name`, `aliased_import`, `wildcard_import`, `relative_import`) | T2 grammar pre-check via a debug tree-walk on real `.py` before coding; tests assert positive presence so a mis-mapping fails loudly |
| Duplicate canonical path silently binds wrong target | Reused `ids.len()==1` singleton fail-closed; T5b duplicate-def test asserts **no** edge |
| Bare Python cross-file call regresses (was name-only, now provenance) | T5a routes shadowed in-file calls to staging and keeps unshadowed in-file `direct` edges (F2); T5b covers cross-file **when a module namespace + binding resolve**, and **falls back to the legacy name-only unique-match whenever it derives NO canonical target — T1-rejected layout OR provable-namespace-but-unbound (star/re-export/relative), F3 — not only when no module namespace exists** so no unique cross-file bare call regresses (a non-unique one still fails closed); name-only pass still runs for non-Python staged calls; T5a Rust regression assertion |
| Re-export chains (`from a import b` re-exported) resolve wrong | v1 non-goal — not traced; fails closed (drop); documented (T6) |
| Modifying shared consumer / post-pass regresses Rust resolution | Every new branch guarded on `Language::Python`; Rust-path regression assertions (T5a); full ordered gate suite before merge |
| Low precision on dynamic Python | **Precision is verified by a manifest-backed target-identity gate — integration fixtures assert the exact resolved callee id on an adversarial corpus (import-after-def, def-after-import, later-star, shadow rebinds, from-import receiver) with a non-zero module-qualified edge count (C6-4/5/F6/F7)** — plus `get_retrieval_eval_report` for the dangling-edge signal only; numeric floor not claimed from the report alone; v1 non-goals documented (T6) |
| **Package-topology change leaves stale canonical paths** (adding/removing `__init__.py` while sync content-hash-skips unchanged descendants, `code_graph.rs:1252-1263`) → false/missing edges (C6-1) | **T3 treats an add/remove of any `__init__.py` as a package-topology change that reindexes/invalidates affected descendants past the content-hash skip; T3 tests pin BOTH the add transition (`""`→`p.mod.f`) and the delete transition (back to `""`)** |
| Existing indexes keep **empty** canonical paths **and staged-but-unresolved edges** after upgrade (content-hash skip at `code_graph.rs:590-599`/`1252-1263`; post-pass full-index-only at `985-992`; `force` defaults false; **sync re-extracts but never runs the post-pass, Q2**) | **T7 (M4, Q2, R1): a SEPARATE `PYTHON_CANONICAL_EXTRACTION_VERSION` index-state marker (NOT `file_node.content_hash` — that stays the raw SHA `is_index_stale` reads byte-for-byte, `retrieval_eval.rs:717-718`; TMDL precedent `powerbi_indexer.rs:60-81`) forces re-extraction AND runs the canonical resolution pass in the SAME operation so resolved edges are restored in one step — no unresolved-edge window; documented `index --force` fallback; upgrade regression asserts the RESOLVED cross-module edge AND that content_hash staleness detection is preserved** |
| **Mixing an extraction version into `content_hash` would break staleness detection** (R1) | **T7 keeps `file_node.content_hash` = raw source SHA; the extraction version lives in a dedicated marker; a T7 test asserts `is_index_stale` still returns false for unchanged source after a version bump** |

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | All new code returns `Result`/`Option`; no `unwrap`/`expect`/`panic`, no `unsafe`, no new lossy casts. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` is a per-unit gate. Satisfied. |
| II. Test-First (NON-NEGOTIABLE) | Every unit authors failing tests before implementation (RED→GREEN); T5b authors the target-identity + fail-closed acceptance assertions. Satisfied. |
| III / IV. Workspace / CLI containment | Parser + resolver over already-indexed content; source reads confined to the workspace via existing `ws_path.join(rel_path)`. Satisfied. |
| V. Structured Observability | Edge creation flows through existing indexing logs; no new silent path. Satisfied. |
| VI. Single Responsibility / no new dependency | Reuses `tree-sitter-python 0.23`; new `python_canonical` module mirrors the existing `canonical` package boundary. Satisfied. |
| VII / VIII. Destructive approval / Safety modes | No destructive command; additive parser + language-guarded resolver branches; enter **freeze-scope** on `src/services/parsing/` + `src/services/code_graph.rs`. Satisfied. |
| IX. Git-Friendly Persistence | Plan + docs are Markdown/YAML; edges regenerate on index. Satisfied. |
| X. Context Efficiency | No tool response-shape change; additive edges only. Satisfied. |
| XI. Merge Commit History | Process-level; observed at ship time. N/A to plan content. |

No justified violations.

## Plan Hardening (REQUIRED)

This plan edits correctness-critical shared graph-resolution code
(`code_graph.rs` consumer + canonical post-pass) and expands the
`calls_resolved_canonical` population to a new language governed by the
operator-signed 082-F target-correctness gate and the 013-D no-false-edge
invariant. That elevates it above a parser-local change.

* **ProposedAction** — populate Python `canonical_path`, emit `module.func()` as a
  staged canonical-eligible call, route Python canonical-eligible calls into
  provenance staging, and resolve them via a Python branch of the canonical
  post-pass reusing the singleton fail-closed core.
* **ActionRisk** — `moderate`. Touches shared resolution code, but every new branch
  is guarded on `Language::Python`; the Rust-only staged population is untouched
  (byte-identical output asserted).
* **Verification** — (a) T5a Rust-path regression assertion (staging behavior
  unchanged for Rust); (b) T5b **target-identity** acceptance (exact callee id, not
  row-existence) for the cross-module same-name case; (c) T5b fail-closed
  assertions for star import, unbound receiver, and **duplicate canonical path**;
  (d) T1 package-layout fail-closed pins — `__init__.py`, undeclared `src/` root,
  implicit PEP 420 namespace all → `""` (M5); (e) T4 `self`/`cls` unstaged (M2);
  (f) T2/T2b competing-binding + function-local-leak fail-closed and T5c full
  rebind-target-set shadow guard — assignment/augmented/`for`/`with`/`except`/walrus/
  parameter/**`def`/`class`/`del`/`match`-case (X4)**/module-level — for **both** a
  module receiver **and** a bare imported callee (M1, Q1, X4), plus T5b
  **last-binding-wins** so a later import rebind beats a local def (X2/X3); (g) T7
  upgrade regression — after a version-bump **sync**, the cross-module **resolved edge
  is present** (not merely `canonical_path`), restored in one operation (M4, Q2), staged
  via provenance on **both** the full-index and sync arms so re-extraction never strands
  a Python edge (X1); (h) full ordered quality-gate suite before merge.
* **Rollback** — additive and reversible: revert the T3/T4/T5a/T5b/T5c/T7 (and
  T2/T2b primitive) commits. No schema change, migration, or destructive step; edges
  regenerate on the next index; the versioned-extraction-hash bump is inert until a
  re-index. **Rollback triggers**: any false Python→X edge observed in
  `map_code`/`impact_analysis` acceptance; any Rust singleton/canonical regression;
  a `__init__.py`/namespace-package/`src/`-root def acquiring a non-empty
  `canonical_path`; a function-local import resolving in a sibling function.
* **ActionResult** — `planned` (execution deferred; this remains planning-only).

### Hardening Signals

* **Public API / schema / contract change** — *absent*. No `calls_edge` or
  `function_meta` schema change; no MCP tool schema change; `map_code`/
  `impact_analysis` additively include Python canonical edges after re-index.
* **Security / auth / permission / compliance** — *absent*.
* **Migration / backfill / destructive / irreversible** — **present (backfill,
  non-destructive) (M4, Q2)**. Existing indexes need a version-gated re-extraction —
  a versioned extraction hash bump (or documented `index --force`) — to acquire
  Python canonical paths (T7); the same operation **also runs the canonical
  resolution pass in one step** (escalate sync→full path or invoke the post-pass)
  so resolved edges are restored without an unresolved-edge window (Q2); additive,
  reversible, no destructive step.
* **External integration / operator checkpoint / external dependency** — *absent*.
  No new dependency.
* **High runtime / rollout / rollback risk** — *moderate*: shared resolution code,
  hardened by the language-guarded dispatch, Rust regression assertions, and
  fail-closed acceptance tests above.

**Requires plan hardening: yes — satisfied inline (this section).**

### PR #285 plan-review hardening (cycle 1)

Five substantive plan-review findings (M1–M5) were addressed by hardening the plan
**and** the harvested task files (no scope expansion — module-level namespace only,
fail-closed; FF7DE872 stays independent; no 090-S/095-F dependency):

* **M1 — scope-aware, fail-closed bindings.** A flat `HashMap<String,String>` cannot
  preserve Python scope (function-local imports leak; duplicate/rebinds overwrite;
  the T5c scan missed rebind forms). Split **T2b** (scope isolation: module vs
  per-function) out of T2; T2 now fails closed on competing/duplicate bindings; T5c
  now fails closed on the full rebind-target set (assignment, augmented, `for`,
  `with … as`, `except … as`, walrus, parameter, module-level). Example neutralized:
  `import bar; bar = factory(); def g(): bar.parse()` → no edge. (Tasks 096.002-T,
  096.007-T, new 096.009-T.)
* **M2 — `self`/`cls` excluded.** T4 explicitly excludes `self`/`cls` receivers from
  module candidates; they stay dropped, keeping T4's acceptance criterion consistent.
  (Task 096.004-T.)
* **M3 — test target registered.** `unit_python_canonical` did not exist; T1 now
  registers the `[[test]]` entry in `Cargo.toml` and lists it in its file set;
  T1/T2 verification commands are correct. (Tasks 096.001-T, 096.002-T.)
* **M4 — real backfill trigger.** "No migration" was unsafe (content-hash skip +
  full-index-only post-pass). New rollout task **T7** re-extracts stale-version `.py`
  files (or documented `index --force`) with an upgrade regression test. (New task
  096.010-T; DAG updated.) *(Cycle-1 wording said "folds a versioned hash into the `.py`
  content-hash"; **corrected by cycle-3 R1** — the version lives in a SEPARATE index-state
  marker, never `file_node.content_hash`. See the cycle-3 addendum.)*
* **M5 — provable package layout.** T1 takes package/source-root metadata and
  REJECTS (fail closed) `src/`-roots, implicit namespace packages, and `__init__.py`
  that cannot be proven a regular-package dotted path; tests pin those layouts.
  (Task 096.001-T.)

Task count went 8 → 10 (added T2b/096.009-T and T7/096.010-T); the DAG and queued
shipment 091-S were updated to match.

### PR #285 plan-review hardening (cycle 2)

Six cycle-2 review comments (Q1–Q6; Q6 duplicates Q3) — three substantive gaps
introduced by the cycle-1 hardening (Q1–Q3) plus two consistency updates (Q4–Q5) —
were addressed by hardening the plan **and** the harvested task files. No scope
expansion (module-level namespace only, fail-closed; FF7DE872 independent; no
090-S/095-F dependency). **Task count stays 10.**

* **Q1 — bare-import shadow gap (P1).** The cycle-1 shadow guard covered only
  module-qualified receivers, leaving `from bar import parse; parse = factory();
  parse()` resolving through T5b's `python_bare` binding to `bar.parse` — a false
  edge. **T5c** now fails closed when a **bare imported callee** is re-bound by any
  form (assignment/augmented/`for`/`with … as`/`except … as`/walrus/parameter/
  module-level) in the applicable scope, and **T5b**'s `python_bare` resolution
  invokes that guard on the callee name; T5c adds bare-import assignment + parameter
  shadow tests. (Tasks 096.007-T, 096.006-T.) *(Cycle-2 wording; **revised by cycle-3
  R3**: the guard lives entirely in T5c, which wraps T5b's guard-agnostic resolution —
  see the cycle-3 addendum. Guard coverage still includes bare imports per Q1.)*
* **Q2 — sync backfill left edges unresolved (P1).** T7's versioned hash also fires
  on startup auto-**sync**, but sync re-extracts and clears prior resolved
  edges/staging without running the full-index-only post-pass — a regression window.
  **T7** now runs/triggers the canonical resolution pass in the **same operation**
  (escalate the sync to the full-index path or invoke the post-pass from sync); the
  upgrade regression test asserts the **RESOLVED cross-module edge** post-upgrade,
  not merely `canonical_path`. (Task 096.010-T.)
* **Q3 + Q6 — PackageLayout had no production source (P1). Decision: NARROW**
  (Constitution VI, no speculative additions — `CodeGraphConfig` has no source-root
  field at `config.rs:98-120`). Removed the cycle-1 `PackageLayout`/source-root
  machinery and the over-claimed `src/pkg/mod.py → pkg.mod` promise. **T1** now
  resolves a dotted path **only when every ancestor dir is a provable regular
  package** (`__init__.py`), via a predicate derived from the indexed file set (no
  config); it **fails closed** on `__init__.py`, `src/`-style roots, and implicit
  PEP 420 namespace packages. **T3** builds that predicate from the indexed
  `__init__.py` set (no `CodeGraphConfig` change). Source-root-aware package
  resolution is documented as an explicit **v1 non-goal** (T6). No new task; both
  Q3 and Q6 resolved by this one decision. (Tasks 096.001-T, 096.003-T; Risks row.)
* **Q4 — DoD stale (P3).** Feature 096-F's Definition of Done now enumerates all
  **ten** tasks (T1, T2, T2b, T3, T4, T5a, T5b, T5c, T6, T7).
* **Q5 — artifact count stale (P3).** The plan's artifact list/task range and the
  session record now reflect the 10-task harvest (adds 096.009-T/T2b and
  096.010-T/T7); shipment 091-S already lists all ten tasks + the feature.

DAG unchanged (still 10 tasks, acyclic): T3←T1; T5a←T4; T5b←{T1,T3,T5a,T2b};
T5c←{T5b,T2b}; T6←{T3,T5b,T5c,T7}; T2b←T2; T7←{T3,T5b}. The DoD (Q4), the artifact
count (Q5), the shipment 091-S manifest, and the DAG all agree on the same final
ten-task list.

### PR #285 plan-review hardening (cycle 3)

Three cycle-3 review comments (R1–R3) — plan-consistency gaps in the cycle-2 hardening
— were addressed. No scope change (module-level namespace only, fail-closed; FF7DE872
independent; no 090-S/095-F dependency). **Task count stays 10; DAG unchanged & acyclic.**

* **R1 — extraction version must not corrupt the `content_hash` contract (P1).** T7's
  cycle-2 wording folded a versioned extraction constant into the `.py` **content hash**.
  But `retrieval_eval::is_index_stale` (`retrieval_eval.rs:717-718`) compares
  `file_node.content_hash` **byte-for-byte** against the raw source SHA
  (`source_content_hash`, 699-700), and the indexer writes `content_hash` as that raw SHA
  (`cozo_queries.rs:609` / `schema.rs:573`) — mixing a version in would break staleness
  detection. **T7** now tracks a **dedicated `PYTHON_CANONICAL_EXTRACTION_VERSION` index-
  state marker** (following the `TMDL_DAX_INDEX_VERSION` / `compute_tmdl_dax_index_hash`
  separate-record precedent, `powerbi_indexer.rs:60-81`), leaves `content_hash` as the raw
  SHA, and adds a regression asserting `is_index_stale` still holds after a version bump.
  (Task 096.010-T; Risks + Requirements-Trace rows.)
* **R2 — `ImportBindings` lacked a binding kind (P1).** A bare canonical string cannot tell
  a **module receiver** (`import pkg` → `pkg.func()`) from an **imported symbol**
  (`from pkg import parse` → `parse()` / `parse.tokenize()`), so a from-import would be
  mis-resolved as a module. **T2** now records `(canonical_path, kind∈{ModuleImport,
  FromImportSymbol})`; **T5b** dispatches on the kind — a module receiver resolves only from
  a `ModuleImport` binding, and a `FromImportSymbol` receiver (`parse.tokenize()`) fails
  closed (attribute-on-object, out of scope). T5b adds a `parse.tokenize()` → no-edge test.
  (Tasks 096.002-T, 096.006-T; Risks + Requirements-Trace rows.)
* **R3 — T5b/T5c circular completability (P1).** The cycle-2 Q1 edit made **T5b**'s
  acceptance require invoking the **T5c** shadow guard, while T5c depends on T5b — a
  completability cycle. **Fix:** the guard lives **entirely in T5c (downstream)**; **T5b**
  is now **guard-agnostic** (its acceptance does not call the guard) and produces
  binding-kind resolution; **T5c wraps** T5b's resolution with the shadow check. The
  frontmatter dependency stays one-directional (T5c → T5b), so each task is independently
  completable and the DAG is acyclic. Guard **coverage still includes bare imports** per
  Q1. (Tasks 096.006-T, 096.007-T; Dependency Graph.)

Final DAG (acyclic, unchanged): T3←T1; T5a←T4; T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b};
T6←{T3,T5b,T5c,T7}; T2b←T2; T7←{T3,T5b}. **T5b does not depend on T5c** (R3). Counts,
DoD, shipment 091-S manifest, and the DAG all agree on the same 10-task list.

### PR #285 plan-review hardening (cycle 4)

Cycle-4 review (operator chose **Option A = fix substantively**) surfaced five plan/task
consistency gaps (X1–X5; X6 is the Orchestrator-owned PR body — untouched here). No scope
change (module-level namespace only, fail-closed; FF7DE872 independent; no 090-S/095-F
dependency). **Task count stays 10; DAG unchanged and acyclic.**

* **X1 — sync-arm staging bypasses canonical resolution.** The Calls-edge sync consumer
  (`code_graph.rs:1573-1643`) name-only-stages bare Python calls via `put_staged_call`
  (line 1639), so version-triggered re-extraction (T7) strands them unresolved — the same
  class as Q2/R1, now a **third** location. **T5a** now routes Python bare + module-
  qualified calls through provenance staging on **both** the full-index arm (851-908)
  **and** the sync arm (1573-1643). (Task 096.005-T; Requirements Trace; Hardening (f).)
* **X2/X3 — local-def-first mints a false edge under import shadowing.** `def parse();
  from bar import parse; def caller(): parse()` binds `parse` to `bar.parse` (later import
  shadows the local def), yet a local-def-first rule emits `M.parse` — a false edge. The
  resolution contract and **T5b** now apply **last-binding-wins**: a `FromImportSymbol`
  binding that rebinds the name over a local `def` **wins** → `N.callee`; when source order
  can't establish a clear last binding, fail closed. (Resolution rule; T5b; Task 096.006-T;
  Risks.)
* **X4 — shadow-guard rebind set incomplete.** The T5c rebind enumeration omitted `def`,
  `class`, `del`, and `match`/`case` capture patterns, so `import bar; class bar: ...;
  bar.parse()` kept a stale import binding → false edge. **T5c** now includes those forms
  as name-rebinding invalidators. (T5c; Task 096.007-T; Risks; Hardening (f).)
* **X5 — no monitoring contract for runtime-affecting work.** Added a **Monitoring &
  Rollback** section (mirroring 090-S A5 / 095-F Fork-A precision floors): SLI = Python
  module-qualified edge precision via `get_retrieval_eval_report`; baseline = 1.000 / zero
  false edges; alert = precision < 1.0; rollback trigger = any confirmed false edge on the
  eval corpus after indexing; observation window = first release cycle (≈2 weeks) + first
  real-package cohort; owner = code-graph parsing/resolution area. (Plan-only.)

DAG unchanged and acyclic; DoD, artifact count, shipment 091-S manifest, and the DAG all
agree on the same 10-task list. **X6 (PR body) is Orchestrator-owned and not touched.**

### PR #285 plan-review hardening (cycle 5)

Cycle-5 review (operator chose **Option A = fix Y1+Y2+Y3, then hard-stop**) surfaced
three plan/task-consistency findings — **Y2 and Y3 are self-inflicted contradictions
introduced by cycle-4's X4 and X1**. No scope change (module-level namespace only,
fail-closed; FF7DE872 independent; no 090-S/095-F dependency). **Task count stays 10;
DAG unchanged and acyclic** (Y3 is placed in T5b, which already depends on T1, so **no
new dependency edge is required**).

* **Y1 — function-local import fail-closed ordering (P1).** `def g(): f(); from x
  import f` — because `f` is imported inside `g`, Python treats it as function-local for
  the **whole** body, so the first call `f()` (before the import) raises
  `UnboundLocalError` at runtime; a scope-only lookup would wrongly emit an edge to
  `x.f`. **T2b** now tracks binding **and** call positions: a name bound by a
  function-local import LATER in the function emits **no** edge for call sites **before**
  the import (fail closed); only calls **after** it resolve; uncertain control-flow
  ordering fails closed. (Task 096.009-T; Resolution rule note; Risks row.)
* **Y2 — order-aware rebind guard (P1).** X4 put `def`/`class`/`del`/`match`-capture into
  T5c's "re-bound anywhere" set, which **contradicted** the X2/X3 frozen last-binding-wins
  rule (it would drop `def parse; from bar import parse; parse()`, which must resolve to
  `bar.parse`). **T5c** is now **order-aware**: a rebind invalidates the import binding
  **only when it occurs AFTER the import** in effective binding order (a *later* rebind);
  a `def`/`class`/`del`/`match`-capture or any rebind **before** the import does **not**
  invalidate it — the import is then the last binding and T5b's resolution stands. Added
  the import-after-def resolves case to T5c scenario (a) and cross-referenced T5b↔T5c.
  (Task 096.007-T; T5c section; Requirements-Trace + Risks rows.)
* **Y3 — legacy fallback preserves recall (P1).** T5a stamps every unresolved cross-file
  bare call `python_bare`, which **excludes** it from the legacy name-only matcher; when
  **T1 rejects** a `src/`/namespace layout (no module context), T5b had no context → the
  call dropped to **no edge**, regressing a unique bare call that resolves **today** via
  the legacy name-only matcher. **T5b** now falls back to the **existing legacy name-only
  unique-match** whenever `python_ctx_for_staged_file` yields no module path — preserving
  today's recall with no false module-qualified edge; canonical resolution applies only
  when a module path exists. Placed in **T5b (which already depends on T1)**, so **no new
  DAG edge** and the legacy pass stays untouched. Added the no-context recall test to T5b
  (consolidating its fail-closed vectors into one scenario to stay at 4) and a cross-ref
  note in T5a. (Tasks 096.006-T + 096.005-T; T5a/T5b sections; Design-decision Option A;
  Resolution rule note; Requirements-Trace + Risks rows.)

**T5a/T5b/T5c reconciliation (one principle — last-binding-wins, order-aware; when
ambiguous or uncertain, fail closed):** T5b resolves a bare call to the **last effective
binding** (import-after-def → `N.name`; def-after-import → `M.name`) and is
guard-agnostic; T5c **wraps** T5b and fails closed **only** on a rebind that lands
**after** the import; T2b supplies the order model, including the function-local
`UnboundLocalError` rule. The three tasks do not contradict each other.

DAG unchanged and acyclic (T5a←{T4}; T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b}; **T5b independent
of T5c**); DoD, artifact count, shipment 091-S manifest, and the DAG all agree on the same
10-task list. **This is plan-review-fix cycle 5 (operator-directed; Option A — hard-stop
after push).**

### PR #285 adversarial reconciliation (cycle 6 — structural revision)

A 3-model adversarial review (Opus 4.8 + GPT-5.6-Sol + Gemini 3.1 Pro, source-verified
against merged main) returned **NOT CONVERGED** with two gate-blocking P0s plus verified
P1s and Copilot cycle-6 threads. Operator chose **Option A = fix substantively, then push
and STOP**. The root defect: **last-binding-wins was adjudicated in T5b/T5c, but the layer
BELOW them — the in-file direct-edge path at `code_graph.rs:900-902` — mints a false
`M.callee` edge and renders T5b's local-def branches unreachable.** All fixes are made
mutually consistent as **one coherent resolution contract**: T5a routes shadowed in-file
calls to staging; T5b adjudicates last-binding-wins and falls back on no-target; T5c fails
closed only on rebinds after the winning binding.

* **F2 (P0, root defect) — T5a (096.005-T) + §Design-decision + §T5a + Risks.** Removed the
  "in-file bare calls stay direct — unchanged (code_graph.rs:900-903)" claim. The in-file
  `(Some,Some)` decision is now **import/shadow-aware**: a same-file callee that is ALSO a
  module import (T2 predicate) is routed to `python_bare` staging (not a direct edge) so
  T5b/T5c adjudicate; an unshadowed callee keeps the direct fast path. **T5a OWNS the
  `code_graph.rs:896-908` change on BOTH arms (full-index 851-908 + sync 1573-1643).**
* **F1 (P0, contradiction) — T5c (096.007-T) + §T5c + Requirements-Trace.** Re-anchored the
  order-aware guard on the **WINNING binding T5b resolved to (import OR def)**, failing
  closed only on rebinds AFTER that winner. `from bar import parse; def parse(); parse()` →
  `M.parse` now **SURVIVES** (def-after-import; cycle-5 Y2 only covered def-before-import).
* **F3 (P1, recall) — T5b (096.006-T) + §T5b + Risks.** T5b fires the legacy name-only
  fallback whenever it derives **NO canonical target** — not only when T1==None, but also
  for a provable-namespace module whose callee is star/re-export/relative (unbound). Gating
  on T1==None alone would DROP those (python_bare is filtered out of `reresolve_calls_edges`
  at `cozo_queries.rs:2222-2227`).
* **F9 (P1, impossible-as-written) — T5b (096.006-T).** Chose **option (a)**: expose a
  public read-only language-scoped name→IDs singleton helper in `cozo_queries.rs` (the
  inline-private CozoScript at 2191-2205 cannot be reused as-is) and add it to T5b's scope;
  removed the "No cozo_queries.rs change" claim. (Option (b) rejected — it cannot serve the
  F3 provable-namespace-no-binding case, discovered only at resolution time.)
* **F4 (P1) — T2 (096.002-T) + §T5c.** A module-scope `from N import *` occurring AFTER the
  winning binding is an order-aware invalidator (fail closed); T2 records the star with its
  position; star-after-import and star-after-def fail-closed tests added.
* **F5 (P1) — T2b (096.009-T).** Modeled the lexical closure chain (enclosing-function
  scopes, honoring `global`/`nonlocal`) or fail closed when an enclosing function binds the
  name; folded F8 (pre-import function-local use is a poison/tombstone, no fall-through) and
  F14 (positions for all bindings). Nested-function scenarios added.
* **C6-1 — T3 (096.003-T).** Package-topology changes (add/remove `__init__.py`) reindex/
  invalidate affected descendants past the content-hash skip (`code_graph.rs:1252-1263`);
  BOTH add- and delete-transition tests added.
* **C6-4/5 (+F6/F7) — 096-F DoD + T6 (096.008-T) + §Monitoring + Risks.** Replaced the
  report-only 1.000-precision claim with a **manifest-backed target-identity gate** (exact
  callee id on an adversarial corpus, non-zero module-qualified edge count) + a manual
  audit + an F7 recall-parity signal; `get_retrieval_eval_report` is kept only as a
  dangling-edge tripwire.
* **F18 / F19 (consistency).** Reconciled the DAG edge-list (T3→T6 added to the main list to
  match the trace) and qualified the "Option B" naming collision at `096-F:21`.

**DAG change (the ONLY one this cycle): T2→T5a (F2)** — T5a must consult T2's
`ImportBindings` at staging time to decide the shadow-aware in-file route. New edge list
(acyclic; T1/T2 remain source primitives, no back-edge): T5a←{T2,T4}; T5b←{T1,T3,T5a,T2b};
T5c←{T5b,T2b}; T2b←{T2}; T3←{T1}; T6←{T3,T5b,T5c,T7}; T7←{T3,T5b}. **Task count stays 10;
shipment 091-S stays 11 items** (096-F + 10 tasks). **F16 (split T5b) declined** — advisory/
LOW, and the operator directed minimal DAG change; F3+F9 stay in T5b (3 files, ≤4 scenarios,
mirroring T1's 4-file precedent). **F17 (`import a.b` root binding) treated as a
defensive-test note** — the reviewer states it is likely already fail-closed. This is
plan-review-fix **cycle 6 (operator-directed; Option A — structural revision, hard-stop
after push)**.

### PR #285 final consolidated pass (cycle 7 — prove-or-fail-closed generalization)

A focused 3-model adversarial re-check validated cycle-6 (**F1/F2/F3/F9/F5 all PASS**),
then triangulated **one remaining root defect** plus bounded adjacent items. The root:
cycle-6's F2 fix keyed T5a's routing predicate too narrowly — on **named module-level
import bindings only** — so it did **not compose** with the other shadow classes. Operator
chose **Option A = fix substantively, then push and STOP** (LAST structural pass). All
fixes are folded into one coherent contract governed by a single **prove-or-fail-closed
invariant**.

* **Z1 + Z2 + C7-2 (unified — generalize T5a) — 096.005-T + §Resolution-rule invariant +
  §Design-decision + §T5a + Requirements-Trace + Risks.** Added the over-arching invariant
  *"a Python bare call resolves to a canonical target ONLY when the call-site-effective
  binding is statically provable; otherwise fail closed."* Rewrote T5a's `(Some,Some)`
  routing predicate to skip the direct-edge shortcut whenever the same-file `def`'s name is
  **shadow-contested at ANY scope the resolver models** — named import (Z-none), positioned
  star-import invalidator (Z1), non-import module rebind (Z2), OR scoped function-local /
  enclosing-function import (C7-2). Expressed as an abstraction (*"shadow-contested at any
  modeled scope"*), **not** axis-by-axis, so future axes are covered by construction. This
  closes the F4 remainder (star-after-def now routes to staging, so T5c's star guard is
  reachable) in the same abstraction.
* **C7-1 (call-position / time axis) — 096-F resolution rule + T5b (096.006-T) + T5c
  (096.007-T) + §Resolution-rule + Requirements-Trace + Risks.** "Last-binding-wins" is now
  **call-site-effective**: T5b resolves to the last provable binding **preceding the call**
  in execution order (module-level source order), so a module-level call before a later
  import binds the earlier def (`def parse(); parse(); from bar import parse` → `M.parse`);
  T5c's invalidation window is strictly **between the winner and the call**, and a
  function-body call whose module binding is contested by a later module-level rebind
  **fails closed** (invocation order unprovable). Before-rebind and after-rebind
  target-identity cases added. *(Superseded by cycle-9 C9-3: module-level calls are not
  extracted — call-position ordering is narrowed to function-body calls only and top-level
  call ordering is a documented v1 non-goal; see the cycle-9 addendum.)*
* **C7-3 (upgrade-gate partial-failure) — T7 (096.010-T) + §T7 + Risks.** The
  `PYTHON_CANONICAL_EXTRACTION_VERSION` marker persists **only** after every affected
  Python file AND the resolution pass succeed; on any per-file failure
  (`SyncResult.errors` non-empty, `code_graph.rs:1215-1221`) the **old** marker is kept so
  migration retries next sync (no stale-forever hash-skip). Partial-failure regression test
  added.
* **C7-4 (spike prose) — spike doc.** Added a clearly-labeled **SUPERSEDED** note to the
  spike's "no changes to `cozo_queries.rs`" conclusion + change-surface table row (T5b/F9
  adds a public name→IDs helper); spike history preserved, not rewritten.
* **Z3 (096-F resolution-rule prose) — 096-F.** Qualified the star/re-export/unbound
  "→ DROP" to "DROP (no canonical module-qualified edge; a unique legacy name-only edge is
  preserved per F3)", reconciled with the F3 recall goal.

**DAG change (the ONLY one this cycle): T2b→T5a** — T5a's generalized predicate must
consult T2b's scoped-binding signals (function-local / enclosing imports), not just T2's
module-level `ImportBindings`. New edge list (acyclic): T5a←{T2,T2b,T4}; T5b←{T1,T3,T5a,T2b};
T5c←{T5b,T2b}; T2b←{T2}; T3←{T1}; T6←{T3,T5b,T5c,T7}; T7←{T3,T5b}. **Acyclicity:** T2b only
depends on T2 and is already upstream of T5b/T5c, so adding T5a←T2b introduces no back-edge
into T2b. **Task count stays 10; shipment 091-S stays 11 items** (096-F + 10 tasks); every
routing/time-axis vector is consolidated into existing task scenarios (≤4 each, 2-hour rule
intact) — no new task. This is plan-review-fix **cycle 7 (operator-directed; Option A —
FINAL consolidated pass, hard-stop after push)**.

### PR #285 narrow closure pass (cycle 8 — non-import-rebind axis, C6)

The cycle-7 FINAL pass validated C7-1/C7-3/C7-4/Z3 and **8 of 9 composition vectors**; a
targeted 3-model re-check found **one** remaining failing vector — **C6: `def` +
non-import module/scope rebind** (`def parse(); parse = factory(); parse()`; also
`class`/`del`/`match`-case/`for`/`with … as`/`except … as`/walrus/parameter targets). Root
cause: cycle-7's "shadow-contested at any modeled **scope**" abstraction closed over
**scopes** but **not over binding FORMS**, and the non-import-rebind axis has **no producer**
(T2/T2b walk only import nodes), so that case still hit the `(Some,Some)` arm and minted a
false direct `M.parse` edge that never reached T5c's (correct) rebind guard. Operator chose
**Option B** — minimal fix, no new producer/task/DAG edge — since **T5a can self-serve the
scan by re-parsing the in-scope file source** already cloned at the consumer seam.

* **C6 (def + non-import rebind) — 096.005-T (T5a) + 096.007-T (T5c) + §Resolution-rule
  invariant + §Design-decision + §T5a + composition-trace C6 + Requirements-Trace.**
  Generalized the invariant to be closed over **BOTH scopes AND binding forms**
  (prove-the-negative default: keep the direct edge **only** when provably the sole binding
  across every modeled scope **and** with no non-import rebind form present; otherwise
  stage). Defined the **Shared rebind-form set (RFS)** **once** in §Design decision, consumed
  **identically** by T5a's in-file routing scan (order-agnostic "any RFS form present →
  stage") and T5c's order-aware invalidation scan (same forms, winner→call window) — so the
  two scans can never diverge. T5a now runs the **same rebind-form scan T5c specifies** by
  re-parsing the in-scope file source in-memory (cached once per file), on **both** arms
  (896-908 + 1573-1643). Added
  routing-table vector `def f; f = factory(); f()` → **STAGED (no direct `M.f`)** to T5a and
  the def+non-import-rebind → DROP row to the Requirements-Trace; composition-trace **C6 now
  resolves to DROP** (owning layer = T5a routing (shared RFS scan) → T5c invalidation).

**No DAG change this cycle:** the non-import rebind forms have **no producer** and T5a
detects them with its own in-file scan (T5a re-parses the in-scope file source it already
has at the seam), so **no new task and
no new DAG edge** are required. Edge list unchanged (T5a←{T2,T2b,T4}; T5b←{T1,T3,T5a,T2b};
T5c←{T5b,T2b}; T2b←{T2}; T3←{T1}; T6←{T3,T5b,T5c,T7}; T7←{T3,T5b}; acyclic). **Task count
stays 10; shipment 091-S stays 11 items.** This is plan-review-fix **cycle 8
(operator-directed; Option B — narrow closure, hard-stop after push)**.

**Folded into the same cycle-8 commit — two adjacent internal-consistency threads:**

* **C8-1 (duplicate same-name imports — narrowed rule) — §Resolution-rule (new C8-1
  blockquote) + §T5b (new fail-closed sub-bullet) + Task `096.002-T` (T2, M1) + Task
  `096.006-T` (T5b).** The frozen call-site last-binding rule (C7-1), read literally, could
  seem to disambiguate `from a import f; from b import f; f()` by source order, contradicting
  T2's fail-closed-on-duplicate-bindings behavior (M1). Reconciled via **Option B (narrowed
  rule)** — the cheaper, strictly fail-closed choice consistent with prove-or-fail-closed: the
  last-binding ordering disambiguates `def`-vs-**single**-import + rebinds, **not**
  import-vs-import of the same name; **≥2 competing imports of a name at a scope → T2 yields no
  binding (M1) → T5b derives no target → fail closed** (no canonical edge; the F3 legacy
  fallback also drops, name non-unique). No regression vs main (name-only ambiguous → drop
  today). Ordered-import-history disambiguation is a documented **v1 non-goal**. T2 was already
  fail-closed; this pass makes the frozen rule + T5b + both task specs **explicitly agree** and
  makes the M1 example cover the two-from-import case. No new code behavior, no new scenario
  (folded into T5b's existing ambiguity fail-closed vector — 2-hour rule intact).
* **C8-2 (spike DROP rows vs the F3 legacy-edge contract) — spike doc.** The spike's
  resolution-rule-under-test and Fail-closed matrix stated `→ DROP` outright, which the final
  T5b/F3 contract superseded (a *unique* cross-file bare call keeps its legacy name-only edge).
  The cycle-7 C7-4 SUPERSEDED note only corrected the `cozo_queries.rs` change surface, leaving
  the behavioral DROP rows contradictory. Added **two clearly-labeled "⚠️ Refined (C7-4/F3)"
  notes** — one under the resolution-rule list, one under the Fail-closed matrix — qualifying
  every `DROP` as "no canonical module-qualified edge; a unique legacy name-only edge may
  remain (non-unique still fails closed)". Spike history preserved (additive notes), now
  consistent with the plan + `096-F`.

Both threads were posted by Copilot cycle-8 at `689da02a` and are **folded into the same commit
as the C6 closure** (one push / one re-check). Neither adds scope; both make the frozen contract
self-consistent. Task count stays 10; shipment 091-S stays 11 items.

### PR #285 final spec-polish pass (cycle 9 — C9-1..C9-4 + nits, planning-only)

An independent 3-model adversarial re-check at `1d4f9caf` returned **CONVERGED** (C6 genuinely
closed, all 9 vectors compose, no P0/P1/P2, no false-edge hole). Four Copilot cycle-9 threads
(LOW/P3 spec-coherence + implementability) plus two re-check nits are folded into **one commit**
atop `1d4f9caf`. **None create a false edge** — C9-1/C9-4 only tighten HOW a vector is decided
(not its precision-floor-safe result), and C9-2/C9-3 correct factual / implementability claims.
**No new task, producer, or DAG edge; task count stays 10; shipment 091-S stays 11 items.**

* **C9-1 (RFS self-contest / over-routing) — §Design-decision (RFS) + §T5a + `096.005-T`.**
  The RFS includes `def name`, so T5a's "any RFS form present → stage" would match the def's
  OWN binding for every `def f; f()`, defeating the promised direct fast path (dead code — **no
  false edge**). Fix: T5a's in-file scan **excludes the matched `def` under consideration** (that
  def is the candidate target, not a self-competitor) and counts only bindings of the name
  **OTHER** than that def, **applicable to the caller's lexical scope** (module + enclosing
  functions + the caller's own function — not unrelated sibling-scope locals/params).
  "Shadow-contested" now means "contested by a binding OTHER than the def under consideration,
  within the caller's applicable lexical scope"; the direct fast path is taken **iff** the matched
  def is provably the sole applicable binding.
* **C9-2 + re-check nit A (the "T5a already holds the AST" claim was FALSE) — §Design-decision +
  §T5a + Risks + cycle-8 addendum + `096.005-T`.** `parse_source` returns
  `ParseResult{symbols, edges}` only (`parsing.rs:247-252`) — **no tree-sitter `Tree` is
  retained**. But the file's `source: String` IS in scope at the seam (cloned ~`code_graph.rs:609`
  full-index / ~`1290` sync). Fix: T5a **re-parses the in-scope source in-memory**, computing the
  file's Rebind-Form-Set map **ONCE PER FILE and caching it** (reused across all calls in the file,
  not per call-site) — one extra amortized parse per Python file. The load-bearing conclusion stays
  TRUE: still **no** new producer, DB field, task, or DAG edge. Every "AST already held" phrase for
  this seam is purged from §T5a / `096.005-T` / Risks / the cycle-8 addendum.
* **C9-3 (C7-1 module-level call ordering cannot run) — §Resolution-rule (C7-1 blockquote) + §T5b
  + composition-trace C9 + Requirements-Trace + Risks + `096.006-T`.** The Python extractor emits
  `Calls` only from FUNCTION BODIES (`extract_top_level`/`extract_calls_from_body`,
  `python.rs:44-79,233-277`); a module-level `parse()` is **never staged**. Fix (narrow,
  fail-closed): C7-1 call-position ordering is defined **over function-body calls only** — a
  function-body call resolves to its module/enclosing binding only when that binding is **stable**
  (no later module-level rebind), else fails closed. Module-level (top-level) call ordering is a
  documented **v1 non-goal** — neither regressed nor newly handled. The prior prose promised a
  module-level `f()` → `M.f` edge that the extractor **cannot produce**; the extractable outcome
  (fail-closed / no false edge for the function-body form) is **unchanged** — only the unreachable
  prose is corrected. Composition-vector **C9 now reads "function-body call under a later module
  rebind → fail closed"**, module-level out of scope.
* **C9-4 (F3 legacy fallback must not leak an edge for competing bindings) — §T5b +
  Requirements-Trace + `096.006-T`.** "Fall back to the sole indexed legacy edge whenever no
  canonical target" would, for `from a import f; from b import f` (T2 = no binding), emit the sole
  indexed `a.f`/`M.f` even though Python's effective target is the later `b.f`. Fix: T5b's outcome
  carries a **typed no-target reason**; the F3 name-only fallback is allowed **only** for
  `{NoModuleContext, UnsupportedImportForm}` and **never** for
  `{CompetingBindings, Shadowed, DuplicateSameNameImport}` — those stay fully fail-closed (NO
  edge). The C8-1 duplicate-import case carries `DuplicateSameNameImport` and gets **no fallback**,
  making C8-1 genuinely complete.
* **Re-check nit B (annotated assignment) — §Design-decision (RFS) + `096.005-T`.** The RFS
  "assignment" form now explicitly enumerates **annotated assignment** (`name: T = …`, a distinct
  tree-sitter node) alongside plain and augmented assignment. No behavior change — a subtype
  already intended.

**No DAG change (cycle 9):** C9-2's honest resolution is a self-serve in-memory re-parse, so no
new producer/edge. Edge list unchanged (T5a←{T2,T2b,T4}; T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b};
T2b←{T2}; T3←{T1}; T6←{T3,T5b,T5c,T7}; T7←{T3,T5b}; acyclic). `096.005-T` deps stay
{`096.002-T`,`096.004-T`,`096.009-T`}; `096.006-T` deps unchanged. All 9 composition vectors
re-verified; C9-1/C9-4 tighten only the decision mechanism, not the outcome. This is
plan-review-fix **cycle 9 (operator-directed; final spec-polish, hard-stop after push)**.





```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test
cargo audit
```

Per-unit `cargo test --test <target>` (targets `unit_python_canonical`,
`unit_parsing`, `integration_code_graph`, `integration_calls_recall_acceptance`)
drive the red/green loop; the full ordered suite is the merge gate.

## Runtime Verification and Closure

* **Changed runtime surface**: `map_code`, `impact_analysis`, `query_graph` over
  `calls_edge` begin returning canonical-resolved edges for cross-module same-name
  Python calls that were previously dropped. Indexing adds a **version-gated
  re-extraction** for Python files (T7) so upgrades backfill; the content-hash
  fast-path is preserved for files already at the current extraction version.
* **Runtime verification** (before absorbed): index a small real Python package
  with two modules defining the same function name; call `map_code`/
  `impact_analysis` on a caller and confirm the edge points at the **correct**
  module's function and that a genuinely ambiguous call yields no edge. T5b covers
  this in automated form; a manual daemon check confirms the live tool surface.
* **Operational closure**: record the behavioral expansion (Python cross-module
  same-name resolution) and the documented fail-closed non-goals. No feature flag or
  dashboard required for this additive change; the one upgrade action is a
  version-gated re-extraction / `index --force` backfill (T7). Precision is governed by
  the **Monitoring & Rollback** contract below (the **manifest-backed target-identity
  gate** + manual audit; `get_retrieval_eval_report` is a secondary dangling-edge
  tripwire only). Ownership: code-graph parsing/resolution area.

## Monitoring & Rollback

This feature changes runtime edge output (013-D no-false-edge, 082-F
target-correctness), so — mirroring 090-S's **A5 precision floor** and 095-F's **Fork-A**
precision floor — it ships with an explicit precision-floor observability contract, not
an "additive ⇒ no trigger" claim. **Measurability caveat (C6-4/5 / adversarial F6/F7):
`get_retrieval_eval_report` alone CANNOT certify the 1.000 precision floor — it reports
a dangling-edge rate, cannot detect a canonical edge that points at the WRONG existing
function, and does not isolate Python module-qualified edges. The precision floor is
therefore certified by a manifest-backed target-identity gate + a specified manual
audit; the report is retained only for its dangling-edge signal.**

* **Primary SLI — target-identity precision (measurable, C6-4/5/F6):** a
  **manifest-backed integration gate** over a fixed Python eval corpus asserts, for each
  emitted module-qualified / canonical bare-call edge, that it points at the **exact
  expected callee id** (target-identity, not row-existence). The corpus MUST include the
  adversarial vectors this plan turns on — import-after-def (`bar.parse`), def-after-import
  (`M.parse`, F1), later-star (F4), receiver/bare shadow rebinds (T5c), from-import
  receiver (`parse.tokenize()` → no edge), duplicate canonical path — **plus the cycle-7
  shadow-routing + time-axis vectors (Z1/Z2/C7-2/C7-1)**: def+star-after-def
  (`def parse(); from n import *; parse()` → **DROP**), def+non-import-rebind
  (`def parse(); parse = factory(); parse()` → **DROP**), def+function-local-import
  (`def parse(); def g(): from bar import parse; parse()` → in-body `bar.parse`; pre-import
  call fail-closed), def+enclosing-scope-import (closure import shadows nested call),
  function-body call with a stable local def and no later rebind
  (`def parse(); def caller(): parse()` → **`M.parse`**), and a
  function-body-call-under-later-rebind (**DROP**, order-ambiguous; module-level call
  ordering is a v1 non-goal — top-level calls not extracted, C9-3) — **and** assert a
  **non-zero count of module-qualified edges** (so a silently-empty result cannot pass).
  The same routing vectors are also pinned in T5a's unit routing tests (staging decision)
  so the precision gate and the routing predicate are exercised on the identical corpus.
* **Secondary signal — dangling-edge rate** via `get_retrieval_eval_report` over the
  `run_retrieval_eval` corpus (detects edges to non-existent targets only; used as a
  cheap regression tripwire, NOT as the precision certificate).
* **Recall-parity signal (F7):** the eval gate also asserts that every unique cross-file
  bare call that resolved via the legacy name-only matcher **before** this feature still
  resolves **after** it (Y3/F3 fallback), so the `python_bare` routing introduces no
  recall regression.
* **Baseline (spike floor)** — **precision = 1.000 / zero false module-qualified edges**
  on the manifest gate. The spike set a hard zero-false-edge floor (013-D); v1 emits an
  edge only on an unambiguous singleton canonical match and fails closed on every
  ambiguity.
* **Alert threshold** — **any** manifest-gate target-identity mismatch (a module-qualified
  edge to the wrong callee id), any dangling module-qualified edge, or a module-qualified
  edge count of **zero** on a corpus known to contain resolvable cases → **precision < 1.0
  / measurement invalid** → alert. There is no soft band.
* **Rollback trigger** — a confirmed target-identity mismatch (wrong-callee module-qualified
  edge) **or** a confirmed dangling module-qualified edge on the eval corpus after
  indexing → **disable/revert** the Python canonical branches (revert T3/T4/T5a/T5b/T5c/T7
  + the T2/T2b primitives; the additive `calls_resolved_canonical` Python rows regenerate
  empty on the next index, Rust resolution untouched). A **recall** shortfall is **not** a
  rollback trigger — recall is a documented v1 non-goal envelope; only a **precision**
  breach (a wrong or dangling edge, measured by the manifest gate) reverts.
* **Observation window** — the **first release cycle (≈ the first two weeks post-merge)**
  **and** the **first cohort of indexed real Python packages**, whichever spans more
  indexing activity; the manifest gate is re-run and a **manual audit** of a sampled set
  of live module-qualified edges (via `map_code`/`impact_analysis`) is performed after
  that cohort indexes.
* **Owner** — **code-graph parsing/resolution area** (Stage-harvested; **Ship-executed
  and Ship-owned at runtime**). The manifest-backed target-identity gate plus the manual
  audit are the operational guardrail; no dashboard or feature flag required for the
  additive change.

## Following Steps (outside this plan)

1. **Ship handoff (this plan is reviewed and harvested):** the plan is `plan-review`-ed
   (see `## Plan Review`) and already `harvest`-ed into feature `096-F` + the **ten
   units** (T1, T2, T2b, T3, T4, T5a, T5b, T5c, T6, T7; stash `FE8B3B2D`) assembled into
   the **queued** shipment `091-S`. **Ship** claims `091-S` and executes the tasks in DAG
   order (T1/T2/T4 primitives first; T5a owns the F2 `code_graph.rs:896-908` change on
   both arms; T5b owns the F9 `cozo_queries.rs` helper; T5c the F1 winner-anchored guard),
   opens the PR, and runs the pre-merge gate + Monitoring contract. Stage's boundary ends
   at the reviewed, harvested, queued backlog.
2. FF7DE872 (same-file shadowing) remains an independent backlog item — not gated
   behind this feature.
3. Optional future work (separate release unit): instance-method dispatch via type
   inference (`self.method()`/`obj.method()`) — the documented non-goal here.

## References

* Spike: `docs/decisions/2026-07-23-python-namespace-canonical-resolution-spike.md`
* Target parser: `src/services/parsing/python.rs` (Calls emission 94-F; flat
  `extract_import` 141-143; attribute drop 309-321)
* Populator: `src/services/code_graph.rs:145-169` (`canonical_path_for_function`);
  call sites 721, 1446
* Consumer + provenance gate: `src/services/code_graph.rs:851-908`,
  `should_stage_provenance_call` 188-194
* Canonical post-pass: `src/services/code_graph.rs:264-368`
  (`reresolve_calls_edges_with_canonical_context`); Rust context builder
  `rust_ctx_for_staged_file` 252-260; target seam
  `canonical_target_for_staged_call` 197-246
* Reused DB (no change): `src/db/cozo_queries.rs:924-940`
  (`function_meta.canonical_path`), `1014-1066`
  (`canonical_paths_for_function_name` + `function_ids_by_canonical_path`),
  `2182-2289` (`reresolve_calls_edges`, bare-name fallback)
* Mirror target: `src/services/parsing/canonical/` (`module_path.rs`,
  `use_graph.rs`, `resolver.rs`)
* Prior art: 094-F `docs/decisions/2026-07-20-python-call-edge-extraction-spike.md`
  + `docs/exec-plans/2026-07-20-python-call-edge-extraction-plan.md`; 091-F
  `docs/decisions/2026-07-15-091-001-canonical-identity-spike.md`,
  `docs/decisions/2026-07-15-option-c-canonical-identity-deliberation.md`,
  `docs/exec-plans/2026-07-15-091-F-option-c-canonical-identity-plan.md`
* Invariants: 013-D no-false-edge; 082-F target-correctness gate
  (`docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md`)
* Split-out independent bug: stash `FF7DE872` (same-file shadowing; `find_function_id`
  first-match, `code_graph.rs:2037`)

## Plan Review

**Reviewed**: 2026-07-23 · **Skill**: plan-review · **Personas**: Constitution
Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher (always-on),
Architecture Strategist. Security-Lens and Agent-Native Parity personas were not
triggered (no auth/API/secrets/data store; additive `map_code`/`impact_analysis`
output only, no tool-contract or schema change).

**Gate decision (initial): FAIL** — one P1 (module-name shadowing false-edge
vector) on the pre-revision plan. **Gate decision (after revision): PASS** — the P1
and all actionable P2/P3 findings are resolved in the units above. One P1 is below
the 3-P0/P1 threshold, so full multi-model adversarial review is **not** required
for this scoped, fail-closed feature.

**Plan hardening**: **yes — satisfied** by the `## Plan Hardening` section (shared
resolution code + expansion of the 082-F-gated canonical population to a new
language).

### Findings by severity

**P1 (gate-blocking) — resolved**

* **Module-name shadowed by a local variable / parameter → false edge**
  (Architecture Strategist, corroborated by Learnings Researcher against the 013-D
  no-false-edge invariant). `import parser` followed by `parser = get_parser();
  parser.parse()` would statically bind `parser.parse()` to the **module** `parser`.
  Resolved by **T5c**: fail closed when the receiver is re-bound in the applicable
  scope; documented as a caveat in T6. *(Expanded by PR #285 M1 to the full
  rebind-target set — assignment/augmented/`for`/`with`/`except`/walrus/parameter/
  module-level — consuming the T2b scope model; further extended by cycle-2 **Q1**
  to cover a shadowed **bare imported callee**, not just a module receiver.)*

**P2 (moderate) — resolved**

* **Incremental build-gate fidelity** (Rust Reviewer): the new `python_canonical`
  module would trip `-D warnings` dead_code if landed before its consumers.
  Resolved: T1/T2 ship same-crate unit tests that exercise every public fn (a use
  under `cargo test` / clippy `--all-targets`); T3/T5b add production call sites.
  Added to Risks.
* **Parser cannot classify attribute receivers** (Architecture Strategist): emitting
  every simple-identifier-receiver attribute call as a `qualifier_kind="module"`
  candidate is safe only because the binding-aware resolver fails closed on unbound
  receivers. Confirmed intentional (Decisions §"Parser emits candidates; the
  resolver disambiguates") and now reinforced by the T5c shadow guard.
* **Quality-gate order/fidelity** (Constitution Reviewer): verified the ordered
  suite `fmt → clippy --all-targets → cargo dev-test → cargo audit` and per-unit
  `--test` targets are present and consistent with AGENTS.md.

**P3 (minor) — resolved / accepted**

* **tree-sitter-python node/field names** (Learnings Researcher): confirm
  `import_from_statement`, `dotted_name`, `aliased_import`, `wildcard_import`,
  `relative_import` empirically before coding — folded into T2's grammar pre-check.
* **`import a.b` (no alias) with multi-segment receiver** (`a.b.func()`): the
  receiver is an `attribute`, not a simple identifier, so T4 drops it (fail closed);
  documented as a v1 non-goal in T6. Accepted.
* **Re-export chains** not traced in v1 (fail closed): accepted v1 non-goal (T6).

### Scope Boundary Auditor — PASS

Confirmed the plan stays within the frozen IN scope (module-level functions +
`module.func()` disambiguation), explicitly excludes instance-method dispatch, and
does **not** subsume FF7DE872 (T3/T5b pin the duplicate-canonical fail-closed that
makes last-wins impossible here). No dependency wired to 090-S/095-F. No scope
creep introduced by the parser's candidate emission (resolver fails closed).

### Resolution

All P1/P2 findings are resolved inline (T5c added; Risks + Requirements Trace +
Dependency Graph + T6 updated). P3 items are folded into existing units or accepted
as documented v1 non-goals. **Plan is APPROVED for harvest.**

### PR #285 review addendum (plan-review-fix cycle 1)

Post-harvest, PR #285 returned five substantive plan-hardening findings (M1–M5). All
were valid execution gaps and are resolved by the hardening captured above (see
`## Plan Hardening → PR #285 plan-review hardening (cycle 1)` for the per-finding
map). Net structural change: two tasks split out to respect the 2-hour rule — **T2b**
(scope-aware binding isolation, from M1) and **T7** (versioned re-extraction /
backfill, from M4) — task count 8 → 10, DAG and queued shipment **091-S** updated to
match. No scope expansion: module-level namespace resolution only, fail-closed on
ambiguity; FF7DE872 stays independent; no 090-S/095-F dependency. Gate remains
**PASS** (M1–M5 are hardening refinements, not new P0/P1 vectors; still below the
3-P0/P1 multi-model threshold).

### PR #285 review addendum (plan-review-fix cycle 2)

The cycle-2 Copilot review returned six comments (Q1–Q6; Q6 duplicates Q3): three
substantive gaps introduced by the cycle-1 hardening (Q1–Q3) plus two consistency
updates (Q4–Q5). All were addressed by hardening the plan **and** the task files
(see `## Plan Hardening → PR #285 plan-review hardening (cycle 2)` for the per-finding
map). Summary: **Q1** extends the T5c shadow guard to imported **bare callees** (T5b
invokes it); **Q2** makes T7's backfill run the canonical resolution pass in the
**same operation** (no unresolved-edge window; regression asserts the resolved edge);
**Q3+Q6** are resolved by **NARROWING** — T1 fails closed on `src/`-roots / implicit
PEP 420 namespace packages / `__init__.py` and the source-root machinery is dropped
(source-root-aware resolution = documented v1 non-goal, Constitution VI); **Q4/Q5**
align the DoD and artifact count to the final **ten-task** set. **No new task — task
count stays 10;** DAG unchanged and acyclic. No scope expansion: module-level
namespace resolution only, fail-closed; FF7DE872 independent; no 090-S/095-F
dependency. Gate remains **PASS** (Q1–Q6 are refinements/consistency fixes, not new
P0/P1 vectors; still below the 3-P0/P1 multi-model threshold). **This is
plan-review-fix cycle 2 of 3.**

### PR #285 review addendum (plan-review-fix cycle 3)

The cycle-3 review returned three substantive plan-consistency findings (R1–R3), all
gaps in the cycle-2 hardening (see `## Plan Hardening → PR #285 plan-review hardening
(cycle 3)` for the per-finding map). Summary: **R1** moves T7's extraction version out of
`file_node.content_hash` (which `is_index_stale` reads byte-for-byte) into a dedicated
index-state marker (TMDL-version precedent), preserving staleness detection; **R2** adds a
binding **kind** to `ImportBindings` so T5b tells a module receiver from an imported symbol
(no from-import mis-resolved as a module); **R3** removes the T5b/T5c completability cycle —
the shadow guard lives entirely in T5c, which wraps a now-**guard-agnostic** T5b (dependency
strictly T5c → T5b). **No new task — count stays 10;** DAG unchanged and acyclic (**T5b does
not depend on T5c**). No scope expansion: module-level namespace resolution only, fail-closed;
FF7DE872 independent; no 090-S/095-F dependency. Gate remains **PASS** (R1–R3 are consistency
refinements, not new P0/P1 vectors; still below the 3-P0/P1 multi-model threshold). **This is
plan-review-fix cycle 3 of 3 (the cap).** Per operator directive, if a cycle-4 review still
surfaces NEW substantive gaps, the plan is accepted as-is and residual items become Ship
execution-time considerations.

### PR #285 review addendum (plan-review-fix cycle 4)

The operator elected **Option A (fix substantively)** on the cycle-4 review, which returned
five plan/task-consistency findings (X1–X5). X6 is the **Orchestrator-owned PR body** and is
**not** touched here. Summary (see `## Plan Hardening → PR #285 plan-review hardening (cycle
4)` for the per-finding map): **X1** routes Python call staging through provenance on **both**
the full-index and sync arms (`code_graph.rs:1573-1643`) so version-triggered re-extraction
never strands an edge (same class as Q2/R1); **X2/X3** make bare-call resolution
**last-binding-wins** so a later import rebind beats a local `def` (`def parse; from bar import
parse; parse()` → `bar.parse`, not `M.parse`); **X4** adds `def`/`class`/`del`/`match`-case
capture to the T5c shadow-guard rebind set; **X5** adds a **Monitoring & Rollback** contract
(precision-floor SLI via `get_retrieval_eval_report`, baseline 1.000 / zero false edges, alert
= precision < 1.0, rollback = any confirmed false edge post-index, observation window + owner)
mirroring 090-S A5 / 095-F Fork-A. **No new task — count stays 10;** DAG unchanged and acyclic.
No scope expansion: module-level namespace resolution only, fail-closed; FF7DE872 independent;
no 090-S/095-F dependency. Gate remains **PASS** (X1–X5 are precision/consistency/observability
refinements, not new P0/P1 vectors; still below the 3-P0/P1 multi-model threshold). **This is
plan-review-fix cycle 4 (operator-directed beyond the cycle-3 cap; Option A).**

### PR #285 review addendum (plan-review-fix cycle 5)

The operator elected **Option A (fix Y1+Y2+Y3, then hard-stop)** on the cycle-5 review,
which returned three P1 plan/task-consistency findings — **Y2 and Y3 are self-inflicted
contradictions introduced by cycle-4's X4 and X1**. Summary (see `## Plan Hardening → PR
#285 plan-review hardening (cycle 5)` for the per-finding map): **Y1** makes T2b fail
closed on a call that precedes a function-local import (Python `UnboundLocalError` — the
name is function-local for the whole body); **Y2** makes the T5c shadow guard
**order-aware** so only a rebind *after* the import invalidates it (reconciling X4 with the
X2/X3 last-binding-wins rule — `def parse; from bar import parse; parse()` still resolves
to `bar.parse`); **Y3** makes T5b fall back to the **legacy name-only unique-match** when
the caller has no provable module namespace (T1 rejects the layout), preserving today's
recall that the `python_bare` stamp would otherwise strand. The T5a/T5b/T5c reconciliation
is stated as one principle — **last-binding-wins, order-aware; when ambiguous or uncertain,
fail closed** — so the three tasks are mutually consistent. **No new task — count stays
10;** Y3 lives in T5b (already dependent on T1) so **the DAG is unchanged and acyclic (T5b
independent of T5c)**. No scope expansion: module-level namespace resolution only,
fail-closed; FF7DE872 independent; no 090-S/095-F dependency. Gate remains **PASS** (Y1–Y3
are precision/recall/consistency refinements, not new P0/P1 vectors; still below the
3-P0/P1 multi-model threshold). **This is plan-review-fix cycle 5 (operator-directed;
Option A — hard-stop after push).**

### PR #285 review addendum (adversarial reconciliation — cycle 6)

A **3-model adversarial review** (Opus 4.8 + GPT-5.6-Sol + Gemini 3.1 Pro, source-verified
against merged main) returned **🔴 NOT CONVERGED** with **two gate-blocking P0s (F1, F2)**
plus verified P1s (F3, F9, F4, F5) and Copilot cycle-6 threads (C6-1..C6-5). The operator
elected **Option A (fix substantively, then push and STOP)**. Unlike cycles 2–5 (consistency
refinements), this cycle is a **structural revision**: the root defect (F2) is that the
in-file direct-edge path at `code_graph.rs:900-902` mints a false `M.callee` edge for a
shadowed same-file def **and** renders T5b's last-binding-wins branches unreachable — a
precision-floor breach the single-model cycles never surfaced. See `## Plan Hardening → PR
#285 adversarial reconciliation (cycle 6 — structural revision)` for the per-finding map.

The fixes form **one coherent resolution contract**: **T5a** makes the in-file bare-call
decision import/shadow-aware and routes shadowed same-file calls to `python_bare` staging on
both arms (F2); **T5b** adjudicates last-binding-wins (now reachable) and falls back to the
legacy name-only unique-match whenever it derives **no** canonical target, via a newly
exposed public `cozo_queries.rs` helper (F3 + F9); **T5c** fails closed only on rebinds
**after the winning binding** (import OR def), so `from bar import parse; def parse();
parse()` → `M.parse` survives (F1). F4 (later-star invalidator), F5 (closure chain + poison
+ positions), C6-1 (package-topology invalidation), and C6-4/5 (manifest-backed
target-identity precision gate + manual audit + recall parity) complete the set.

**One DAG edge added — T2→T5a (F2)** — the only graph change this cycle; the task graph is
otherwise unchanged and **acyclic** (T5a←{T2,T4}; T5b←{T1,T3,T5a,T2b}; T5c←{T5b,T2b}; T2 and
T1 remain source primitives). **Task count stays 10; shipment 091-S stays 11 items.** No
scope expansion: module-level namespace resolution only, fail-closed; FF7DE872 independent;
no 090-S/095-F dependency. **F16 (split T5b) declined** (advisory/LOW; minimal-DAG directive)
and **F17 treated as a defensive-test note** (reviewer says likely already fail-closed) —
both reported to the operator rather than silently skipped. Because this cycle addressed **≥2
P0s from a multi-model adversarial pass**, the gate is recorded as **hardened to convergence
pending re-review** (operator owns the re-review); the plan is internally consistent across
plan + 10 task specs. **This is plan-review-fix cycle 6 (operator-directed; Option A —
structural revision, hard-stop after push).**

### PR #285 review addendum (final consolidated pass — cycle 7)

A focused **3-model adversarial re-check** confirmed cycle-6's **F1/F2/F3/F9/F5 all PASS**
(every original P0/P1 genuinely resolved), then triangulated **one remaining root defect**
plus bounded adjacent items. Root: cycle-6's F2 predicate was keyed on **named module-level
imports only**, so three findings are the **same class** — Z1 (star-after-def, P0, 3/3),
Z2 (non-import module rebind, P1), C7-2 (function-local / enclosing import, P1) — each
leaving the `(Some,Some)` arm to mint a false direct edge that bypasses T2b/T5b/T5c. The
operator elected **Option A (fix substantively, then push and STOP)**; this is the **LAST
structural pass**.

The fixes generalize the contract via one **prove-or-fail-closed invariant** and keep
F1/F2/F3/F9/F5 intact:

* **Z1/Z2/C7-2 (unified) — T5a (096.005-T) + §Resolution-rule invariant + §T5a.** T5a's
  routing predicate now skips the direct shortcut whenever the same-file `def` is
  **shadow-contested at ANY modeled scope** (named import / positioned star / non-import
  rebind / scoped import), expressed as an abstraction so future axes are covered by
  construction — this also closes the F4 remainder.
* **C7-1 — 096-F rule + T5b + T5c.** Resolution is **call-site-effective** (last provable
  binding preceding the call in execution order); function-body calls contested by a later
  module rebind **fail closed** (time-axis application of the invariant).
* **C7-3 — T7 (096.010-T).** The extraction-version marker persists **only** on a
  fully-successful pass; any per-file failure keeps the old marker (migration retries),
  closing the stale-forever hole.
* **C7-4 — spike doc; Z3 — 096-F.** Prose reconciliation: a labeled SUPERSEDED note on the
  spike's `cozo_queries.rs`-untouched conclusion (F9 helper), and the 096-F DROP wording
  qualified to preserve the unique legacy name-only edge (F3).

**Whole-contract composition re-verified** (see the *Resolution composition trace* table,
§Resolution rule): def-only, import-only, def-before-import, import-before-def,
def+star-after-def, def+non-import-rebind, def+function-local-import,
provable-namespace-unbound, and call-before-later-rebind each yield **exactly one**
unambiguous, precision-floor-safe outcome owned by **exactly one** layer; everything
unprovable fails closed. **One DAG edge added — T2b→T5a** (T5a consults T2b's scoped-binding
signals); acyclic (T2b←{T2} only, already upstream of T5b/T5c — no back-edge). **Task count
stays 10; shipment 091-S stays 11 items**; every new vector consolidated into existing ≤4
scenario tasks (2-hour rule intact) — no new task, no scope expansion (module-level
namespace only, fail-closed; FF7DE872 independent; no 090-S/095-F dependency). **No findings
judged false positives** — all were source-verified. The plan is internally consistent
across plan + 10 task specs + spike + 096-F. **This is plan-review-fix cycle 7
(operator-directed; Option A — FINAL consolidated pass, hard-stop after push).**
