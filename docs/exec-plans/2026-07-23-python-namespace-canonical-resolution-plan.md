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
| Module namespace `foo/bar.py` → `foo.bar` (only when every ancestor is a **provable regular package**) | T1: `python_module_path_for_file(rel_path, is_regular_package)` (predicate from the indexed `__init__.py` set — **no config source**, Q3); REJECT `src/`-roots / implicit PEP 420 namespace / `__init__.py`; **source-root-aware resolution = v1 non-goal** (M5, Q3) |
| Symbol-level import bindings (Python analogue of Rust `UseGraph`) | T2: `extract_python_import_bindings` records `(canonical_path, kind∈{ModuleImport, FromImportSymbol})` (R2) and fails closed on competing/duplicate bindings (M1) |
| Scope-correct bindings — function-local imports must not leak; module vs function scope | T2b: scoped binding model (M1) |
| Register the `unit_python_canonical` test target so verification runs | T1: `Cargo.toml` `[[test]]` entry (M3) |
| Populate `function_meta.canonical_path` for Python module-level defs | T3: Python branch in `canonical_path_for_function` (reuses `upsert_function_with_canonical`) |
| Emit `module.func()` as a canonical-eligible staged call (stop dropping it); `self`/`cls` stay dropped | T4: `python.rs` emits `is_qualified:true, raw_qualifier=<receiver>, qualifier_kind="module"`; excludes `self`/`cls` (M2) |
| Route Python cross-file bare + module-qualified calls into canonical staging | T5a: consumer Calls arm + `should_stage_provenance_call` (language-dispatched) |
| Python-aware canonical target resolution reusing the singleton fail-closed core | T5b: `python_ctx_for_staged_file` + Python branch in `canonical_target_for_staged_call`, dispatched by the T2 binding **kind** (R2); **guard-agnostic — the T5c shadow guard wraps it downstream (R3)** |
| Fail closed when any rebind shadows an imported **module receiver OR bare callee** name | T5c: drop `mod.func()` (receiver) **and** bare `parse()` (Q1) when the name is re-bound in the applicable scope — assignment/augmented/`for`/`with`/`except`/walrus/param/module-level (M1, Q1) |
| Existing indexes must backfill Python canonical **edges** after upgrade, **in one operation** | T7: a **separate `PYTHON_CANONICAL_EXTRACTION_VERSION` marker** (NOT `content_hash`, R1) triggers re-extraction that **also runs the canonical resolution pass in the same step** (escalate sync→full path or invoke the post-pass) / `index --force`; upgrade regression asserts the **resolved edge** and that `content_hash` staleness detection is preserved (M4, Q2, R1) |
| Fail closed on star / relative / package-root / re-export / dynamic / duplicate | T1–T2 return no module/binding; T3 returns `""`; T5b singleton check drops duplicates |
| No **canonical** schema change; reuse `function_ids_by_canonical_path` + staging queries | No `function_meta`/`calls_edge`/staging schema change (verified by T5b); T7's extraction-version marker is orthogonal index-state, not the canonical model (R1) |
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
  name** to its **canonical origin _and binding kind_ (R2)** — a
  `(canonical_path, kind)` where `kind ∈ {ModuleImport, FromImportSymbol}` — built by
  walking tree-sitter `import_statement` / `import_from_statement` nodes:
  * `from N import name` → `name → ("N.name", FromImportSymbol)`; `from N import name
    as p` → `p → ("N.name", FromImportSymbol)`.
  * `import a.b as c` → `c → ("a.b", ModuleImport)`; `import a.b` → `a → ("a",
    ModuleImport)` (root-name module binding).
  * **No binding (fail closed)** for: `from N import *` (star), relative imports
    (`from . import x`, leading-dot module), `importlib`/`__import__`/dynamic.
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
* **Tests (RED→GREEN, 4)**: `from p import f`→`(p.f, FromImportSymbol)` **and** `from p
  import f as g`→`(p.f, FromImportSymbol)`; `import a.b as c`→`(a.b, ModuleImport)`
  (kind asserted, R2); `from p import *` **and** `from . import x` → **no** binding;
  competing `import p` + `from q import p` → **no** binding (M1).
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
  call sites (721, 1446) parallel to `rust_ctx`; that context carries the
  **regular-package predicate** T1 needs — a set of dirs containing `__init__.py`,
  derived once from the already-indexed file set (Q3: **no `CodeGraphConfig` change,
  no source-root config**). Reuse `upsert_function_with_canonical` unchanged (no DB
  change).
* **Files**: `code_graph.rs`, `tests/integration/code_graph_test.rs`
  (`integration_code_graph`).
* **Tests (RED→GREEN, 4)**: a `.py` def in a proven regular-package chain gets
  `canonical_path="mod.f"`; an `__init__.py` def gets `""`; a def under a `src/`
  source-root or an implicit PEP 420 namespace package gets `""` (Q3 fail-closed);
  two same-file defs of `f` both persist their (identical) canonical path — proving
  the **duplicate** state the resolver later fails closed on (ties to FF7DE872
  non-subsumption).
* **Verification**: `cargo test --test integration_code_graph`; clippy; fmt.
* **Milestone**: Python defs carry exact canonical identity only on a provable
  regular-package chain; every other layout stays `""` (fail closed).

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
  `python_ctx_for_staged_file` (module path via T1 + **scope-aware bindings via
  T2b**, read from source like `rust_ctx_for_staged_file`) and a Python branch
  dispatched by the **caller file's language** that computes the target canonical path
  **using the T2 binding _kind_ (R2)**. This stage is **shadow-guard-agnostic (R3): it
  does NOT invoke the T5c guard** — shadow-rebind handling is added downstream by T5c,
  which keeps T5b independently completable.
  * `qualifier_kind=="python_bare"`: `M.callee` if `callee` defined in `M`, else a
    **`FromImportSymbol`** binding for `callee` (→ `N.callee`), else fail closed. A
    `ModuleImport`-kind name used as a bare callee is not a function → fail closed.
  * `qualifier_kind=="module"`: the receiver must resolve to a **`ModuleImport`**
    binding (`import pkg` / `import a.b as c`) → `<module>.callee`; a
    **`FromImportSymbol`** receiver (`from pkg import parse; parse.tokenize()`) is an
    attribute access on an object, **not** a module → fail closed (R2); else fail closed.
  then reuse the existing `canonical_index.get(&target)` **singleton** match
  (`ids.len()==1`) — dropping on zero, ambiguous, or **duplicate** canonical path.
  No `cozo_queries.rs` change.
* **Files**: `code_graph.rs`, `tests/integration/calls_recall_acceptance_test.rs`
  (`integration_calls_recall_acceptance`).
* **Tests (RED→GREEN, 4)** — **no shadowing here (R3; shadow cases live in T5c)**: two
  modules both define `parse`; caller does `bar.parse()` with `import bar` → edge
  resolves to **bar's** exact `parse` id (target-identity, not row-existence); caller
  does bare `parse()` with `from bar import parse` → resolves to bar's `parse`; a
  **`from pkg import parse`** receiver used as `parse.tokenize()` → **no** module edge
  (R2 kind fail-closed); fail-closed ambiguity — **star** `from bar import *` then bare
  `parse()` with 2+ `parse`, **and** two same-file `parse` defs (duplicate canonical
  path) → **no** canonical edge (FF7DE872 stays unfixed here).
* **Verification**: `cargo test --test integration_calls_recall_acceptance --test
  integration_code_graph`; clippy; fmt.
* **Milestone**: cross-module same-name Python calls resolve to exact targets **via the
  T2 binding kind**; every ambiguity fails closed. **Shadow-rebind handling is deferred
  to T5c (R3).**

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
  candidate canonical target, T5c **fails closed** when the resolved name is re-bound
  anywhere in the **applicable scope** (caller function **and** module level), consuming
  the **T2b** scope model. This wrapping keeps the dependency one-directional (T5c
  depends on T5b, never the reverse). The guard applies to **both**:
  * `qualifier_kind=="module"` — the **receiver** name (`bar` in `bar.parse()`);
  * `qualifier_kind=="python_bare"` — the **bare callee** name (`parse` in
    `parse()`), when T5b resolved it via a `FromImportSymbol` binding or an in-module
    def (Q1).

  The rebind scan covers the **full target set (M1)**: plain `assignment` **and
  augmented assignment** (`+=`, …); `for` targets; `with … as`; `except … as`;
  **walrus** `(:=)`; function **parameters**; and **module-level** rebinding. Reuse
  the already-loaded caller/module source (T5b/T2b context) — a cheap tree-sitter
  scan; no new DB call.
* **Files**: `code_graph.rs`, `tests/integration/calls_recall_acceptance_test.rs`.
* **Tests (RED→GREEN, 4)**: **(a)** clean cases resolve — `import bar; bar.parse()`
  and `from bar import parse; parse()` (no rebind); **(b)** module-receiver rebind
  table {assignment, module-level `import bar; bar = factory(); def g(): bar.parse()`,
  `for`/`with … as`/`except … as`/`:=`/augmented/parameter} → **no** edge;
  **(c) bare-import assignment shadow** `from bar import parse; parse = factory();
  parse()` → **no** edge (Q1); **(d) bare-import parameter shadow**
  `from bar import parse` with `def g(parse): parse()` → **no** edge (Q1).
* **Verification**: `cargo test --test integration_calls_recall_acceptance`; clippy;
  fmt.
* **Milestone**: shadowing of **either** a module receiver **or** a bare imported
  callee, by any rebind form in any scope, cannot mint a false edge — **enforced
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
* **Files** (≤3): `src/services/code_graph.rs` (extraction-version constant + marker
  read/compare/persist + one-step resolution), the index-state seam that holds the
  marker, `tests/integration/code_graph_test.rs`.
* **Tests (RED→GREEN, 4)**: **upgrade regression** — after an extraction-version-bump
  **sync** of an unchanged-hash `.py` file, the cross-module **resolved edge is present**
  (assert the RESOLVED EDGE, not merely the def's `canonical_path`) (Q2); **content-hash
  contract (R1)** — the same file's `file_node.content_hash` still equals the raw source
  SHA and `is_index_stale` returns false for unchanged source (version tracked
  separately, staleness intact); a file already at the current extraction version is
  still skipped (fast-path preserved); Rust files unaffected.
* **Verification**: `cargo test --test integration_code_graph`; clippy; fmt.
* **Milestone**: an upgrade backfills Python canonical **edges** in one operation with
  no unresolved-edge window; `content_hash` staleness detection is preserved (version in
  a separate marker); the fast-path is preserved for current-version files.

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
T1 (module-path) ─┬─▶ T3 (populator) ─┬─▶ T7 (rollout/backfill) ─┐
                  └─▶ T5b (resolver)  │                          │
T2 (bindings) ─▶ T2b (scope isolation)┤                          ├─▶ T6 (docs)
T4 (parser emit) ─▶ T5a (staging) ────┼─▶ T5b ─▶ T5c (shadow) ───┘
T2b ──────────────────────────────────┘         (guard)
```

No cycles. T1 and T2 are independent primitives. T2b refines T2 (scope model). T3
needs T1. T4 is parser-independent of T1–T3. T5a needs T4. T5b needs T1, T2b, T3,
T5a — and is **guard-agnostic (R3): it does not depend on T5c and is independently
completable**. T5c **wraps** T5b's resolution with the shadow guard (downstream) and
consumes T2b's scope model — the dependency is strictly **T5c → T5b**, never the
reverse. T7 (rollout/backfill) needs the populator (T3) and resolver (T5b) to be real.
T6 needs the capability real (T3, T5b, T5c, T7). Edge list (acyclic): T1→{T3,T5b};
T2→T2b; T2b→{T5b,T5c}; T4→T5a; T5a→T5b; T3→{T5b,T7}; T5b→{T5c,T7}; {T5b,T5c,T7}→T6.

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
| **Recall trade-off: `src/`-layout & namespace-package defs get no canonical path (Q3 narrowing)** | Deliberate fail-closed narrowing (Constitution VI — no speculative source-root config; none exists at `config.rs:98-120`): those calls fall back to the existing **name-only** matcher (no regression vs today), never a false edge; source-root-aware resolution is a documented **v1 non-goal** (T6) for a future iteration with a real config source |
| **Any rebind shadows an imported module receiver OR bare callee** (`import bar; bar = f(); bar.parse()` **or** `from bar import parse; parse = f(); parse()`) → false edge | **T5c fails closed on the full rebind-target set in the applicable scope for BOTH the receiver AND the bare callee name (M1, Q1)**: assignment, augmented (`+=`), `for`/`with … as`/`except … as`/walrus/parameter, and module-level rebind; consumes T2b's scope model |
| **Function-local import leaks / competing bindings overwrite** (flat file-wide map) → false edge from the wrong module | **T2 fails closed on competing/duplicate bindings; T2b scopes module-level vs per-function bindings so a local import never leaks (M1)** |
| **From-import symbol mis-resolved as a module receiver** (`from pkg import parse; parse.tokenize()` treated as module `parse`) → wrong edge | **T2 records the binding kind (`ModuleImport` vs `FromImportSymbol`) (R2); T5b resolves a module receiver ONLY from a `ModuleImport` binding and fails closed on a `FromImportSymbol` receiver** (attribute-on-object, out of scope); T5b test pins `parse.tokenize()` → no edge |
| **`self`/`cls` wrongly staged as a module candidate** | **T4 explicitly excludes `self`/`cls` receivers (M2)**; they stay dropped (empty qualifier); T4 test pins `self.foo()`/`cls.bar()` unstaged |
| New `python_canonical` module trips `-D warnings` dead_code when landed before its consumers | T1/T2 ship with same-crate unit tests exercising each public fn (counts as use under `cargo test`/clippy `--all-targets`); T3/T5b add production call sites |
| tree-sitter-python node/field names differ from assumptions (`import_from_statement`, `dotted_name`, `aliased_import`, `wildcard_import`, `relative_import`) | T2 grammar pre-check via a debug tree-walk on real `.py` before coding; tests assert positive presence so a mis-mapping fails loudly |
| Duplicate canonical path silently binds wrong target | Reused `ids.len()==1` singleton fail-closed; T5b duplicate-def test asserts **no** edge |
| Bare Python cross-file call regresses (was name-only, now provenance) | T5a keeps in-file `direct` edges; T5b covers cross-file; name-only pass still runs for any non-Python staged calls; T5a Rust regression assertion |
| Re-export chains (`from a import b` re-exported) resolve wrong | v1 non-goal — not traced; fails closed (drop); documented (T6) |
| Modifying shared consumer / post-pass regresses Rust resolution | Every new branch guarded on `Language::Python`; Rust-path regression assertions (T5a); full ordered gate suite before merge |
| Low precision on dynamic Python | Measured via `run_retrieval_eval` / `get_retrieval_eval_report`, not asserted as a numeric target; v1 non-goals documented (T6) |
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
  rebind-target-set shadow guard for **both** a module receiver **and** a bare
  imported callee (M1, Q1); (g) T7 upgrade regression — after a version-bump
  **sync**, the cross-module **resolved edge is present** (not merely
  `canonical_path`), restored in one operation (M4, Q2); (h) full ordered
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
