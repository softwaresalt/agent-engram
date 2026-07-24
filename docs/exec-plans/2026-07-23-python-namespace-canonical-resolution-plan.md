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
| Module namespace `foo/bar.py` → `foo.bar` | T1: `python_module_path_for_file` |
| Symbol-level import bindings (Python analogue of Rust `UseGraph`) | T2: `extract_python_import_bindings` |
| Populate `function_meta.canonical_path` for Python module-level defs | T3: Python branch in `canonical_path_for_function` (reuses `upsert_function_with_canonical`) |
| Emit `module.func()` as a canonical-eligible staged call (stop dropping it) | T4: `python.rs` emits `is_qualified:true, raw_qualifier=<receiver>, qualifier_kind="module"` |
| Route Python cross-file bare + module-qualified calls into canonical staging | T5a: consumer Calls arm + `should_stage_provenance_call` (language-dispatched) |
| Python-aware canonical target resolution reusing the singleton fail-closed core | T5b: `python_ctx_for_staged_file` + Python branch in `canonical_target_for_staged_call` |
| Fail closed when a local variable shadows an imported module name | T5c: drop `mod.func()` when `mod` is re-bound (assigned/param) in the caller scope |
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
  `python_module_path_for_file(rel_path) -> Option<String>`:
  `foo/bar.py` → `Some("foo.bar")`; strip `.py`, join path segments with `.`.
  **Fail closed (`None`)** for: `__init__.py` (package root, ambiguous identity),
  any segment that is not a valid Python identifier (PEP 420 / odd layouts),
  and non-`.py` paths. `None` ⇒ the caller writes `""` (never a match target, D4).
* **Files**: `python_canonical/module_path.rs`, `python_canonical/mod.rs`,
  `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: simple `a/b.py→a.b`; nested `p/q/r.py→p.q.r`;
  `pkg/__init__.py→None`; `notes.txt→None`.
* **Verification**: `cargo test --test unit_python_canonical`; `cargo clippy
  --all-targets -- -D warnings -D clippy::pedantic`; `cargo fmt --all -- --check`.
* **Milestone**: deterministic, fail-closed module-path transform.

### T2 — Python import-binding capture (domain: code)

* **New**: `src/services/parsing/python_canonical/bindings.rs`.
  `extract_python_import_bindings(source) -> HashMap<String, String>` mapping a
  **local name** to its **canonical origin**, built by walking tree-sitter
  `import_statement` / `import_from_statement` nodes:
  * `from N import name` → `name → "N.name"`; `from N import name as p` →
    `p → "N.name"`.
  * `import a.b as c` → `c → "a.b"` (module binding); `import a.b` → `a → "a"`
    (root-name module binding).
  * **No binding (fail closed)** for: `from N import *` (star), relative imports
    (`from . import x`, leading-dot module), `importlib`/`__import__`/dynamic.
* **Files**: `python_canonical/bindings.rs`, `python_canonical/mod.rs` (re-export),
  `tests/unit/python_canonical_test.rs`.
* **Tests (RED→GREEN, 4)**: `from p import f`→`f=p.f`; `from p import f as g`→
  `g=p.f`; `import a.b as c`→`c=a.b`; `from p import *` and `from . import x`
  produce **no** binding.
* **Verification**: `cargo test --test unit_python_canonical`; clippy; fmt.
* **Milestone**: symbol-level import bindings with fail-closed omissions.

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
  `function` is an `attribute` whose `object` is a **simple identifier** `r`, emit
  `is_method:false, is_qualified:true, raw_qualifier:r, qualifier_kind:"module"`
  (candidate — the resolver fails closed if `r` is not a bound module). Attribute
  calls whose object is **not** a simple identifier (`self.x()`, `obj.attr.y()`,
  `a().b()`) stay dropped (`is_method:true`, empty qualifier). Bare identifier
  calls unchanged at the parser (their canonical routing happens in T5a).
* **Files**: `python.rs`, `tests/unit/parsing_test.rs` (`unit_parsing`).
* **Tests (RED→GREEN, 4)**: `mod.func()`→one `Calls` with
  `is_qualified:true, raw_qualifier:"mod", qualifier_kind:"module"`; `self.foo()`→
  no promoted/staged edge (empty-qualifier drop preserved); `obj.attr.bar()`→
  dropped; bare `foo()` still `is_method:false, is_qualified:false`.
* **Verification**: `cargo test --test unit_parsing`; clippy; fmt.
* **Milestone**: `module.func()` becomes a canonical-eligible staged call instead
  of a silent drop; non-module receivers still fail closed.

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

* **Why**: a local variable or parameter can shadow an imported module name
  (`import parser` … `parser = get_parser(); parser.parse()`). Static
  binding-only resolution would bind `parser.parse()` to the **module** `parser`'s
  `parse` — a false edge violating 013-D. (Found in plan-review, P1.)
* **Changes**: `src/services/code_graph.rs` Python branch of
  `canonical_target_for_staged_call` (from T5b): before resolving a
  `qualifier_kind=="module"` call, **fail closed** when the receiver name is
  re-bound in the caller — i.e. it appears as an assignment target or a parameter
  in the caller function body. Reuse the already-loaded caller source (T5b context);
  a cheap tree-sitter scan of the caller function for `assignment`
  targets / parameter identifiers matching the receiver. No new DB call.
* **Files**: `code_graph.rs`, `tests/integration/calls_recall_acceptance_test.rs`.
* **Tests (RED→GREEN, 3)**: `import bar; bar.parse()` (no local `bar`) resolves;
  `import bar; bar = f(); bar.parse()` (local re-bind) → **no** edge; `def g(bar):
  bar.parse()` (parameter shadow) → **no** edge.
* **Verification**: `cargo test --test integration_calls_recall_acceptance`; clippy;
  fmt.
* **Milestone**: module-name shadowing by locals/params cannot mint a false edge.

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
T1 (module-path) ─┬─▶ T3 (populator) ─┐
                  └─▶ T5b (resolver)  │
T2 (bindings) ──────▶ T5b (resolver)  │
T4 (parser emit) ──▶ T5a (staging) ───┼─▶ T5b ─▶ T5c (shadow guard) ─▶ T6 (docs)
T3 ───────────────────────────────────┘
```

No cycles. T1 and T2 are independent primitives. T3 needs T1. T4 is
parser-independent of T1–T3. T5a needs T4. T5b needs T1, T2, T3, T5a. T5c refines
T5b (the module-qualified branch). T6 needs the capability to be real (T3, T5b, T5c).

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
  keeps the parser simple and the receiver-classification honest.
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
| **Wrong module-path mints a false edge** (the one real correctness risk): `__init__.py`, PEP 420 namespace packages, `src/`-layout roots | T1 fails closed to `None`→`""` (never a match target, D4); T3/T5b tests pin `__init__.py`→`""`; hardened below |
| **Local variable / parameter shadows an imported module name** (`import parser; parser = f(); parser.parse()`) → false module edge | T5c fails closed when the receiver is re-bound in the caller scope (P1 from plan-review) |
| New `python_canonical` module trips `-D warnings` dead_code when landed before its consumers | T1/T2 ship with same-crate unit tests exercising each public fn (counts as use under `cargo test`/clippy `--all-targets`); T3/T5b add production call sites |
| tree-sitter-python node/field names differ from assumptions (`import_from_statement`, `dotted_name`, `aliased_import`, `wildcard_import`, `relative_import`) | T2 grammar pre-check via a debug tree-walk on real `.py` before coding; tests assert positive presence so a mis-mapping fails loudly |
| Duplicate canonical path silently binds wrong target | Reused `ids.len()==1` singleton fail-closed; T5b duplicate-def test asserts **no** edge |
| Bare Python cross-file call regresses (was name-only, now provenance) | T5a keeps in-file `direct` edges; T5b covers cross-file; name-only pass still runs for any non-Python staged calls; T5a Rust regression assertion |
| Re-export chains (`from a import b` re-exported) resolve wrong | v1 non-goal — not traced; fails closed (drop); documented (T6) |
| Modifying shared consumer / post-pass regresses Rust resolution | Every new branch guarded on `Language::Python`; Rust-path regression assertions (T5a); full ordered gate suite before merge |
| Low precision on dynamic Python | Measured via `run_retrieval_eval` / `get_retrieval_eval_report`, not asserted as a numeric target; v1 non-goals documented (T6) |
| Existing indexes lack Python canonical edges until re-index | Normal incremental indexing; additive; no migration |

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
  (d) T1/T3 `__init__.py`→`""` package-root fail-closed pin; (e) full ordered
  quality-gate suite before merge.
* **Rollback** — additive and reversible: revert the T3/T4/T5a/T5b commits. No
  schema change, migration, or destructive step; edges regenerate on the next
  index. **Rollback triggers**: any false Python→X edge observed in
  `map_code`/`impact_analysis` acceptance; any Rust singleton/canonical regression;
  a `__init__.py`/namespace-package def acquiring a non-empty `canonical_path`.
* **ActionResult** — `planned` (execution deferred; this remains planning-only).

### Hardening Signals

* **Public API / schema / contract change** — *absent*. No `calls_edge` or
  `function_meta` schema change; no MCP tool schema change; `map_code`/
  `impact_analysis` additively include Python canonical edges after re-index.
* **Security / auth / permission / compliance** — *absent*.
* **Migration / backfill / destructive / irreversible** — *absent*. Edges
  regenerate via normal incremental indexing.
* **External integration / operator checkpoint / external dependency** — *absent*.
  No new dependency.
* **High runtime / rollout / rollback risk** — *moderate*: shared resolution code,
  hardened by the language-guarded dispatch, Rust regression assertions, and
  fail-closed acceptance tests above.

**Requires plan hardening: yes — satisfied inline (this section).**

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
  Python calls that were previously dropped. Indexing path is structurally
  unchanged.
* **Runtime verification** (before absorbed): index a small real Python package
  with two modules defining the same function name; call `map_code`/
  `impact_analysis` on a caller and confirm the edge points at the **correct**
  module's function and that a genuinely ambiguous call yields no edge. T5b covers
  this in automated form; a manual daemon check confirms the live tool surface.
* **Operational closure**: record the behavioral expansion (Python cross-module
  same-name resolution) and the documented fail-closed non-goals. No feature flag
  or dashboard required for this additive change; precision trackable via
  `get_retrieval_eval_report`. Ownership: code-graph parsing/resolution area.

## Following Steps (outside this plan)

1. `plan-review` this plan (see `## Plan Review`), then `harvest` into a feature +
   the seven units above (stash `FE8B3B2D`), assembled into a **queued** shipment.
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
  Resolved by **T5c**: fail closed when the receiver is re-bound (assignment target
  or parameter) in the caller scope; documented as a caveat in T6.

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
