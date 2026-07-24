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

For a Python call in caller module `M` (module namespace `foo/bar.py` → `foo.bar`):

* bare `name()` **defined in `M`** → canonical target `M.name`
* else `name()` **imported** via `from N import name [as alias]` → `N.name`
* `mod.name()` where `mod` is a bound imported module → `<resolved-mod>.name`
* else (star / relative / package-root / re-export / dynamic / unbound) → **DROP**

Every drop path is a fail-closed no-edge, never a guessed edge (013-D).

## Requirements Trace

| Source requirement (spike / stash FE8B3B2D) | Implementation action |
|---|---|
| Module namespace `foo/bar.py` → `foo.bar` (only on a **provable** package layout) | T1: `python_module_path_for_file(rel_path, &PackageLayout)`; REJECT `src/`-roots / implicit namespace / `__init__.py` (M5) |
| Symbol-level import bindings (Python analogue of Rust `UseGraph`) | T2: `extract_python_import_bindings` (fail closed on competing/duplicate bindings, M1) |
| Scope-correct bindings — function-local imports must not leak; module vs function scope | T2b: scoped binding model (M1) |
| Register the `unit_python_canonical` test target so verification runs | T1: `Cargo.toml` `[[test]]` entry (M3) |
| Populate `function_meta.canonical_path` for Python module-level defs | T3: Python branch in `canonical_path_for_function` (reuses `upsert_function_with_canonical`) |
| Emit `module.func()` as a canonical-eligible staged call (stop dropping it); `self`/`cls` stay dropped | T4: `python.rs` emits `is_qualified:true, raw_qualifier=<receiver>, qualifier_kind="module"`; excludes `self`/`cls` (M2) |
| Route Python cross-file bare + module-qualified calls into canonical staging | T5a: consumer Calls arm + `should_stage_provenance_call` (language-dispatched) |
| Python-aware canonical target resolution reusing the singleton fail-closed core | T5b: `python_ctx_for_staged_file` + Python branch in `canonical_target_for_staged_call` |
| Fail closed when a variable/param/any rebind shadows an imported module name | T5c: drop `mod.func()` when `mod` is re-bound in the applicable scope — assignment/augmented/`for`/`with`/`except`/walrus/param/module-level (M1) |
| Existing indexes must backfill Python canonical paths after upgrade | T7: versioned extraction hash / `index --force` re-extraction + upgrade regression test (M4) |
| Fail closed on star / relative / package-root / re-export / dynamic / duplicate | T1–T2 return no module/binding; T3 returns `""`; T5b singleton check drops duplicates |
| No schema change; reuse `function_ids_by_canonical_path` + staging queries | No `cozo_queries.rs` change (verified by T5b) |
| Do NOT resolve instance-method dispatch; do NOT touch FF7DE872 | Out of scope; documented in T6 |
| Document capability + v1 non-goals | T6 |

## Design decision — bare-call routing (spike fork resolved)

**Chosen: Option A (mirror Rust).** Route *all* canonical-eligible Python calls
through **provenance staging** and resolve them in the **existing canonical
post-pass**, leaving the name-only `reresolve_calls_edges` bare-name pass as an
**untouched conservative fallback** and keeping `cozo_queries.rs` change-free:

* In-file bare calls (caller+callee resolve in the same file) stay a **`direct`
  edge** — unchanged (code_graph.rs:900-903).
* Cross-file **bare** Python calls (callee unresolved in-file) are staged via
  `put_staged_call_with_provenance` with `qualifier_kind="python_bare"`,
  `raw_qualifier=""` — instead of the current name-only `put_staged_call`. Because
  the bare-name pass filters `qualifier_kind.is_empty()`, these are handled *only*
  by the canonical pass (no double-processing).
* `module.func()` calls are staged with `qualifier_kind="module"`,
  `raw_qualifier=<receiver>`.
* The canonical post-pass computes the target canonical path from the caller's
  Python module + import bindings and matches it against
  `function_ids_by_canonical_path` with the existing `ids.len()==1` fail-closed.

Rejected Option B (canonical fallback inside `reresolve_calls_edges`): more
surgical but edits the load-bearing, operator-gated bare-name resolver in
`cozo_queries.rs`; higher blast radius; harder to keep byte-identical for Rust.

## Implementation Units

Sequenced test-first (Constitution Principle II). Each unit is a single skill
domain, ≤3 files, ≤5 functions, ≤4 test scenarios — a verifiable milestone.
Every unit authors its failing test(s) first (RED), then the implementation
(GREEN).

### T1 — Python module-path primitive (domain: code)

* **New**: `src/services/parsing/python_canonical/module_path.rs` + `mod.rs`.
  `python_module_path_for_file(rel_path, layout: &PackageLayout) -> Option<String>`
  where `PackageLayout` carries package/source-root metadata: which ancestor dirs
  are provable **regular packages** (contain `__init__.py`) and any declared
  **source roots** (e.g. `src/`). **A pure `rel_path` transform is insufficient
  (M5)**: `src/pkg/mod.py` and an implicit PEP 420 `namespace/pkg/mod.py` contain
  only valid identifiers, so a naive transform would emit `src.pkg.mod` /
  `namespace.pkg.mod` and could mint a false canonical path.
  * Resolve `foo/bar.py` → `Some("foo.bar")` **only** when every ancestor dir is a
    provable regular package, or the leading segment is a declared source root that
    is stripped (`src/pkg/mod.py` → `pkg.mod`).
  * **Fail closed (`None`)** for: `__init__.py`; any non-identifier segment; an
    **implicit namespace package** (an ancestor dir with no `__init__.py` that is
    not a declared source root); an **undeclared/ambiguous source root**; and
    non-`.py` paths. `None` ⇒ the caller writes `""` (never a match target, D4).
    **Conservative default: when the layout cannot be proven, REJECT (fail closed).**
* **Cargo.toml (M3)**: register a `[[test]]` target
  `name = "unit_python_canonical", path = "tests/unit/python_canonical_test.rs"`.
  This repo registers **every** nested `tests/unit/*.rs` file explicitly
  (Cargo.toml ~231-237, e.g. `unit_parsing`); `unit_python_canonical` does not yet
  exist, so without this the T1/T2 verification command fails before running. (Two
  trivial config lines — not counted toward the source-file budget.)
* **Files**: `python_canonical/module_path.rs`, `python_canonical/mod.rs`,
  `Cargo.toml` (2-line test registration), `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: regular nested package `p/q/r.py` (proven `__init__.py`
  chain) → `p.q.r`; declared source root `src/pkg/mod.py` → `pkg.mod`;
  **ambiguous-layout table** → `None` (covers `__init__.py`, undeclared `src/` root,
  implicit PEP 420 namespace package, non-identifier segment); non-`.py` → `None`.
* **Verification**: `cargo test --test unit_python_canonical` (target registered
  above); `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`;
  `cargo fmt --all -- --check`.
* **Milestone**: provably-package module path; every unprovable layout fails closed.

### T2 — Python import-binding capture (domain: code)

* **New**: `src/services/parsing/python_canonical/bindings.rs`.
  `extract_python_import_bindings(source) -> ImportBindings` mapping a **local
  name** to its **canonical origin**, built by walking tree-sitter
  `import_statement` / `import_from_statement` nodes:
  * `from N import name` → `name → "N.name"`; `from N import name as p` →
    `p → "N.name"`.
  * `import a.b as c` → `c → "a.b"` (module binding); `import a.b` → `a → "a"`
    (root-name module binding).
  * **No binding (fail closed)** for: `from N import *` (star), relative imports
    (`from . import x`, leading-dot module), `importlib`/`__import__`/dynamic.
  * **Competing / duplicate binding → fail closed (M1).** If the **same local
    name** is bound by 2+ import statements in the same scope
    (duplicate/re-import), mark it **ambiguous** → **no** binding. A flat
    last-writer-wins `HashMap` is **forbidden**. (Function-vs-module scope isolation
    is T2b.)
* **Files**: `python_canonical/bindings.rs`, `python_canonical/mod.rs` (re-export),
  `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: `from p import f`→`f=p.f` **and** `from p import f as g`
  →`g=p.f`; `import a.b as c`→`c=a.b`; `from p import *` **and** `from . import x`
  → **no** binding; competing `import p` + `from q import p` → **no** binding (M1).
* **Verification**: `cargo test --test unit_python_canonical` (target registered in
  T1); clippy; fmt.
* **Milestone**: symbol-level bindings; star/relative/dynamic **and** competing
  bindings fail closed.

### T2b — Scope-aware binding isolation (domain: code)

* **Why (M1)**: a file-wide flat binding map **leaks a function-local import into
  other callers** and cannot represent module-level rebinding. Bindings must be
  scoped: module-level imports apply to every function; a function-local import
  applies **only** within its own function and must never leak to a sibling.
* **Changes**: extend `bindings` (T2) to a **scoped model** — module-level bindings
  + per-function-scope bindings — so a resolver consults a caller function's local
  bindings first, then module-level. A name whose applicable scope holds a competing
  binding fails closed (reuses the M1 rule). Nested-function locals do not leak to
  the enclosing scope.
* **Files**: `python_canonical/bindings.rs`, `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: a function-local `from x import f` is visible inside its
  function; the **same** call site in a sibling function that lacks the import gets
  **no** binding (no leak); a module-level import is visible in all functions; a
  nested-function local import does not leak to its enclosing function.
* **Verification**: `cargo test --test unit_python_canonical`; clippy; fmt.
* **Milestone**: bindings are scope-correct; function-local imports cannot leak.

### T3 — Python canonical_path populator (domain: code)

* **Changes**: `src/services/code_graph.rs`. Add a language dispatch to
  `canonical_path_for_function` (145-169) so Python module-level defs get
  `canonical_path = "<module>.<name>"` via T1 (`""` when `python_module_path_for_file`
  returns `None`). Build a lightweight Python per-file context at the two populator
  call sites (721, 1446) parallel to `rust_ctx`. Reuse
  `upsert_function_with_canonical` unchanged (no DB change).
* **Files**: `code_graph.rs`, `tests/integration/code_graph_test.rs`
  (`integration_code_graph`).
* **Tests (RED→GREEN, 3)**: a `.py` module def gets `canonical_path="mod.f"`; an
  `__init__.py` def gets `""`; two same-file defs of `f` both persist their
  (identical) canonical path — proving the **duplicate** state the resolver later
  fails closed on (ties to FF7DE872 non-subsumption).
* **Verification**: `cargo test --test integration_code_graph`; clippy; fmt.
* **Milestone**: Python defs carry exact module-qualified canonical identity.

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

* **Changes**: `src/services/code_graph.rs` Calls consumer arm (851-908) +
  `should_stage_provenance_call` (188-194), language-dispatched on the caller
  file's language:
  * accept `qualifier_kind=="module"` for Python into provenance staging;
  * for Python **bare** calls whose callee is unresolved in-file, stage via
    `put_staged_call_with_provenance` with `qualifier_kind="python_bare"` instead
    of the name-only `put_staged_call`. In-file bare calls remain `direct` edges.
  * **Rust behavior is untouched** (dispatch guards every new branch on
    `Language::Python`).
* **Files**: `code_graph.rs`, `tests/integration/code_graph_test.rs`.
* **Tests (RED→GREEN, 4)**: Python cross-file bare call produces a staged row with
  `qualifier_kind=="python_bare"`; Python `mod.func()` produces a staged row with
  `qualifier_kind=="module"`; in-file bare Python call still yields a `direct`
  edge; a Rust cross-file bare call is **still** name-only staged (regression).
* **Verification**: `cargo test --test integration_code_graph`; existing Rust
  calls tests unaffected; clippy; fmt.
* **Milestone**: Python canonical-eligible calls reach the canonical post-pass;
  Rust staging is provably unchanged.

### T5b — Python canonical target resolution in the post-pass (domain: code)

* **Changes**: `src/services/code_graph.rs`
  `reresolve_calls_edges_with_canonical_context` (264-368). Add
  `python_ctx_for_staged_file` (module path via T1 + bindings via T2, read from
  source like `rust_ctx_for_staged_file`) and a Python branch dispatched by the
  **caller file's language**: compute the target canonical path —
  * `qualifier_kind=="python_bare"`: `M.callee` if `callee` defined in `M`, else
    binding for `callee` (`N.callee`), else fail closed;
  * `qualifier_kind=="module"`: resolve `raw_qualifier` to a bound module, then
    `<module>.callee`, else fail closed;
  then reuse the existing `canonical_index.get(&target)` **singleton** match
  (`ids.len()==1`) — dropping on zero, ambiguous, or **duplicate** canonical path.
  No `cozo_queries.rs` change.
* **Files**: `code_graph.rs`, `tests/integration/calls_recall_acceptance_test.rs`
  (`integration_calls_recall_acceptance`).
* **Tests (RED→GREEN, 4)**: two modules both define `parse`; caller does
  `bar.parse()` → edge resolves to **bar's** exact `parse` id (target-identity,
  not row-existence); caller does bare `parse()` with `from bar import parse` →
  resolves to bar's `parse`; **star import** `from bar import *` then bare
  `parse()` where 2+ `parse` exist → **no** edge; two same-file `parse` defs
  (duplicate canonical path) → **no** canonical edge (fail closed, FF7DE872 stays
  unfixed here).
* **Verification**: `cargo test --test integration_calls_recall_acceptance --test
  integration_code_graph`; clippy; fmt.
* **Milestone**: cross-module same-name Python calls resolve to exact targets;
  every ambiguity fails closed.

### T5c — Module-receiver shadow guard (domain: code)

* **Why**: a variable or parameter can shadow an imported module name at **any**
  scope (`import bar; bar = factory(); def g(): bar.parse()` — module-level rebind;
  or `def g(bar): bar.parse()` — parameter). Static binding-only resolution would
  bind `bar.parse()` to the **module** `bar`'s `parse` — a false edge violating
  013-D. (Plan-review P1; **expanded by PR #285 M1**.)
* **Changes**: `src/services/code_graph.rs` Python branch of
  `canonical_target_for_staged_call` (from T5b): before resolving a
  `qualifier_kind=="module"` call, **fail closed** when the receiver name is
  re-bound anywhere in the **applicable scope** — the caller function **and** module
  level — consuming the **T2b** scope model. The rebind scan must cover the **full
  target set (M1)**, not just plain assignment/parameter:
  * plain `assignment` targets **and augmented assignment** (`+=`, `-=`, …);
  * `for` targets (`for bar in …`), `with … as bar`, `except … as bar`;
  * **walrus** `(bar := …)`; function **parameters**;
  * **module-level** rebinding of an imported name.

  Reuse the already-loaded caller/module source (T5b/T2b context) — a cheap
  tree-sitter scan; no new DB call.
* **Files**: `code_graph.rs`, `tests/integration/calls_recall_acceptance_test.rs`.
* **Tests (RED→GREEN, 4)**: clean `import bar; bar.parse()` (no rebind) → resolves;
  parameter shadow `def g(bar): bar.parse()` → **no** edge; **module-level rebind**
  `import bar; bar = factory(); def g(): bar.parse()` → **no** edge (M1 example); a
  **rebind-target table** {`for`, `with … as`, `except … as`, `:=`, augmented `+=`}
  for the receiver → **no** edge.
* **Verification**: `cargo test --test integration_calls_recall_acceptance`; clippy;
  fmt.
* **Milestone**: module-name shadowing by any rebind form, in any scope, cannot mint
  a false edge.

### T7 — Rollout: versioned re-extraction + backfill trigger (domain: code)

* **Why (M4)**: existing indexes will **not** acquire Python canonical paths through
  normal incremental indexing. Both the full-index and sync paths skip files whose
  content hash is unchanged (`code_graph.rs:590-599` and `1252-1263`), `force`
  defaults to `false`, and the canonical call post-pass runs on the **full-index
  path only** (`code_graph.rs:985-992`). After upgrade, unchanged `.py` files keep
  **empty** canonical paths and stale staging unless a rebuild is forced. The
  earlier "no migration" claim was unsafe.
* **Changes**: fold a **versioned extraction constant** into the Python file
  content-hash comparison (the *versioned-extraction-hash* option — **no new schema
  column**): bumping the constant invalidates the cached hash for `.py` files, so
  the full-index path re-extracts them **and** the canonical post-pass re-runs.
  Documented fallback: a one-shot forced backfill (`index --force`). Replace the
  plan's "no migration" language with this real backfill trigger.
* **Files**: `src/services/code_graph.rs`, `tests/integration/code_graph_test.rs`.
* **Tests (RED→GREEN, 3)**: an already-indexed `.py` file with an **unchanged**
  content hash but an **older** extraction version is re-extracted and acquires its
  Python `canonical_path` (upgrade regression); a file already at the current
  version is still skipped (fast-path preserved, no perf regression); Rust files are
  unaffected (regression).
* **Verification**: `cargo test --test integration_code_graph`; clippy; fmt.
* **Milestone**: upgrades backfill Python canonical identity deterministically; the
  content-hash fast-path is preserved for current-version files.

### T6 — Documentation (domain: docs)

* **Changes**: document Python namespace-qualified call resolution and its **v1
  non-goals** in `docs/ARCHITECTURE.md` / `docs/QUALITY_SCORE.md`: instance-method
  dispatch NOT resolved (needs type inference); re-exports, relative/package-root
  (`__init__.py`, PEP 420), star, and dynamic imports fail closed; a local variable
  or parameter shadowing an imported module name fails closed (T5c); FF7DE872
  (same-file shadowing) is a separate, independent fix.
* **Files**: `docs/ARCHITECTURE.md` (and/or `docs/QUALITY_SCORE.md`).
* **Verification**: prose review; no build impact.
* **Milestone**: documented capability + honest limitations.

## Dependency Graph

```text
T1 (module-path) ─┬─▶ T3 (populator) ─┬─▶ T7 (rollout/backfill) ─┐
                  └─▶ T5b (resolver)  │                          │
T2 (bindings) ─▶ T2b (scope isolation)┤                          ├─▶ T6 (docs)
T4 (parser emit) ─▶ T5a (staging) ────┼─▶ T5b ─▶ T5c (shadow) ───┘
T2b ──────────────────────────────────┘         (guard)
```

No cycles. T1 and T2 are independent primitives. T2b refines T2 (scope model). T3
needs T1. T4 is parser-independent of T1–T3. T5a needs T4. T5b needs T1, T2b, T3,
T5a. T5c refines T5b and consumes T2b's scope model. T7 (rollout/backfill) needs the
populator (T3) and resolver (T5b) to be real. T6 needs the capability real (T3, T5b,
T5c, T7). Edge list: T1→{T3,T5b}; T2→T2b; T2b→{T5b,T5c}; T4→T5a; T5a→T5b; T3→{T5b,T7};
T5b→{T5c,T7}; {T5b,T5c,T7}→T6.

## Decisions and Rationale

* **Zero DB/schema change.** `function_meta.canonical_path`,
  `function_ids_by_canonical_path`, `canonical_paths_for_function_name`, and the
  staging queries are language-agnostic; reusing them removes the highest-blast
  layer from the change. Verified in the spike.
* **Option A (provenance staging) over Option B (bare-name-pass edit).** Keeps the
  operator-gated `reresolve_calls_edges` untouched and all canonical logic in one
  place; keeps `cozo_queries.rs` change-free; keeps Rust output byte-identical.
* **Parser emits module-qualified candidates; the resolver disambiguates.** The
  parser cannot know whether an attribute receiver is a module or an object, so it
  emits a `qualifier_kind="module"` candidate with the receiver, and the
  binding-aware resolver fails closed when the receiver is not a bound module. This
  keeps the parser simple and the receiver-classification honest. **`self`/`cls`
  receivers are excluded at the parser (M2)** — they are instance/class dispatch
  (out of scope), so they never become module candidates.
* **Scope-aware, fail-closed bindings (M1).** Import bindings are modeled with
  module-level vs per-function scope (T2b), a function-local import never leaks to a
  sibling, and any competing/duplicate binding — or any rebind of the receiver name
  in the applicable scope (assignment/augmented/`for`/`with`/`except`/walrus/param/
  module-level, T5c) — fails closed. A flat file-wide `HashMap` is forbidden.
* **Module path requires provable package layout (M5).** T1 takes package/source-root
  metadata; `src/`-roots, implicit PEP 420 namespace packages, and `__init__.py`
  that cannot be proven a regular-package dotted path are REJECTED (fail closed).
* **Upgrades backfill via a versioned extraction hash (M4).** Existing indexes do
  **not** gain Python canonical paths through content-hash-skipping incremental
  indexing; folding a versioned extraction constant into the `.py` content-hash (no
  schema column) — or a documented forced `index --force` — triggers re-extraction +
  the canonical post-pass (T7). No silent "no migration" claim.
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
| **Wrong module-path mints a false edge** (the one real correctness risk): `__init__.py`, PEP 420 namespace packages, `src/`-layout roots | **T1 requires provable package/source-root metadata (M5)**: unprovable layouts (`__init__.py`, undeclared `src/` root, implicit PEP 420 namespace) REJECT → `None`→`""` (never a match target, D4); T1 tests pin `src/`-layout + implicit-namespace + `__init__.py`; T3/T5b tests pin `__init__.py`→`""` |
| **Local variable / parameter / any rebind shadows an imported module name** (`import bar; bar = f(); def g(): bar.parse()`) → false module edge | **T5c fails closed on the full rebind-target set in the applicable scope (M1)**: assignment, augmented (`+=`), `for`/`with … as`/`except … as`/walrus/parameter, and module-level rebind; consumes T2b's scope model |
| **Function-local import leaks / competing bindings overwrite** (flat file-wide map) → false edge from the wrong module | **T2 fails closed on competing/duplicate bindings; T2b scopes module-level vs per-function bindings so a local import never leaks (M1)** |
| **`self`/`cls` wrongly staged as a module candidate** | **T4 explicitly excludes `self`/`cls` receivers (M2)**; they stay dropped (empty qualifier); T4 test pins `self.foo()`/`cls.bar()` unstaged |
| New `python_canonical` module trips `-D warnings` dead_code when landed before its consumers | T1/T2 ship with same-crate unit tests exercising each public fn (counts as use under `cargo test`/clippy `--all-targets`); T3/T5b add production call sites |
| tree-sitter-python node/field names differ from assumptions (`import_from_statement`, `dotted_name`, `aliased_import`, `wildcard_import`, `relative_import`) | T2 grammar pre-check via a debug tree-walk on real `.py` before coding; tests assert positive presence so a mis-mapping fails loudly |
| Duplicate canonical path silently binds wrong target | Reused `ids.len()==1` singleton fail-closed; T5b duplicate-def test asserts **no** edge |
| Bare Python cross-file call regresses (was name-only, now provenance) | T5a keeps in-file `direct` edges; T5b covers cross-file; name-only pass still runs for any non-Python staged calls; T5a Rust regression assertion |
| Re-export chains (`from a import b` re-exported) resolve wrong | v1 non-goal — not traced; fails closed (drop); documented (T6) |
| Modifying shared consumer / post-pass regresses Rust resolution | Every new branch guarded on `Language::Python`; Rust-path regression assertions (T5a); full ordered gate suite before merge |
| Low precision on dynamic Python | Measured via `run_retrieval_eval` / `get_retrieval_eval_report`, not asserted as a numeric target; v1 non-goals documented (T6) |
| Existing indexes keep **empty** canonical paths after upgrade (content-hash skip at `code_graph.rs:590-599`/`1252-1263`; post-pass full-index-only at `985-992`; `force` defaults false) | **T7 (M4): versioned re-extraction marker forces re-extraction + canonical post-pass for stale-version Python files (or documented `index --force` backfill); upgrade regression test asserts an unchanged-hash file acquires canonical_path** |

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
  rebind-target-set shadow guard (M1); (g) T7 upgrade regression — an unchanged-hash
  `.py` file acquires `canonical_path` after a version bump (M4); (h) full ordered
  quality-gate suite before merge.
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
  non-destructive) (M4)**. Existing indexes need a version-gated re-extraction —
  a versioned extraction hash bump (or documented `index --force`) — to acquire
  Python canonical paths (T7); additive, reversible, no destructive step.
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
  full-index-only post-pass). New rollout task **T7** folds a versioned extraction
  hash into the `.py` content-hash (or documented `index --force`) with an upgrade
  regression test. (New task 096.010-T; DAG updated.)
* **M5 — provable package layout.** T1 takes package/source-root metadata and
  REJECTS (fail closed) `src/`-roots, implicit namespace packages, and `__init__.py`
  that cannot be proven a regular-package dotted path; tests pin those layouts.
  (Task 096.001-T.)

Task count went 8 → 10 (added T2b/096.009-T and T7/096.010-T); the DAG and queued
shipment 091-S were updated to match.

## Quality Gates (pre-merge, constitutional order)

Run in order; do not skip:

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
  version-gated re-extraction / `index --force` backfill (T7). Precision trackable
  via `get_retrieval_eval_report`. Ownership: code-graph parsing/resolution area.

## Following Steps (outside this plan)

1. `plan-review` this plan (see `## Plan Review`), then `harvest` into a feature +
   the **ten units** above (T1, T2, T2b, T3, T4, T5a, T5b, T5c, T6, T7; stash
   `FE8B3B2D`), assembled into a **queued** shipment.
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
  module-level — consuming the T2b scope model.)*

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
