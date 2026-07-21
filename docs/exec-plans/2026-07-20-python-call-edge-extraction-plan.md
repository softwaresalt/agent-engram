---
title: "Python call-edge extraction implementation plan"
type: plan
date: 2026-07-20
source: docs/decisions/2026-07-20-python-call-edge-extraction-spike.md
stash_id: CD1EAE09
status: reviewed
requires_plan_hardening: true
tags:
  - code-graph
  - python
  - tree-sitter
  - call-graph
---

## Problem Frame

Caller→callee navigation (`map_code`, `impact_analysis`, `query_graph` over
`calls_edge`) is Rust-only today. `ExtractedEdge::Calls` is emitted in exactly one
place in the parser tree — `src/services/parsing/rust.rs:239`. The Python parser
(`src/services/parsing/python.rs`) extracts only `Defines` and `Imports`, so `.py`
files contribute symbols and imports to the graph but no call edges.

The spike (`docs/decisions/2026-07-20-python-call-edge-extraction-spike.md`,
conclusion **proceed / high confidence**) established that the downstream
`ExtractedEdge::Calls` consumer in `src/services/code_graph.rs:851-909` is
**language-agnostic**: bare identifier calls resolve by name to a direct
`create_calls_edge`, cross-file calls are staged for the deferred post-pass by
callee name, method calls are dropped, and Rust's crate-aware canonical
re-resolution (`code_graph.rs:264-361`) only ever fires for staged
method/qualified calls. Python already routes through `parse_python_source`, and
`tree-sitter-python = "0.23"` is already a declared dependency. The core extraction
work is therefore confined to the Python parser, with one hardening change to the
shared cross-file resolver (see Cross-file safety below).

This plan implements **bare-call** Python call-edge extraction — the pilot that
lights up `.py` call graphs and de-risks the shared consumer for a later
per-language rollout (spike Next Steps item 3, a separate release unit).

**Cross-file safety (added after plan-review).** The consumer stages unresolved
bare calls for a deferred post-pass that resolves them by callee **name** against
`*function_meta` with **no language filter** (`reresolve_calls_edges`,
`src/db/cozo_queries.rs:2177-2249`). Today only Rust emits staged bare calls, so
every candidate is Rust; once Python emits them, a Python `parse()` whose only
unique workspace-global match is a Rust `fn parse` would mis-bind to it. That
violates the 013-D no-false-edge invariant and the operator-signed
target-correctness gate in `docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md`
(082-F). This plan therefore also **language-scopes** the cross-file singleton
resolver (Unit 3) before Python cross-file calls are trusted (Unit 4).

## Requirements Trace

| Source requirement (spike) | Implementation action |
|---|---|
| Emit `ExtractedEdge::Calls` for bare Python calls (`foo()`) | Unit 2: `extract_calls_from_body` + `resolve_call_name` in `python.rs` |
| Mark attribute/method calls (`x.foo()`, `mod.bar()`) `is_method`, do not promote and do not stage | Unit 2: `resolve_call_name` maps `attribute` node → `is_method: true` with empty `raw_qualifier` (fails closed in `should_stage_provenance_call`) |
| Drop idiomatic builtin no-ops (`print`, `len`, …) | Unit 2: `PYTHON_CALL_BLOCKLIST` |
| Never mis-bind a cross-file bare call to a definition in another language | Unit 3: language-scope `reresolve_calls_edges` (join `function_meta`→`file_node.language`) |
| Reuse the language-agnostic consumer / storage unchanged | No change to `code_graph.rs` bare-call arm or edge schema (verified by Unit 4) |
| No new dependency | Reuse `tree-sitter-python 0.23` already in `Cargo.toml` |
| Document v1 non-goals (method bodies, decorators, dynamism) | Unit 5 |
| Prove `map_code`/`impact_analysis` traverse Python edges and cross-file resolution is target-correct | Unit 4 integration + adversarial acceptance test |

## Implementation Units

Sequenced test-first (Constitution Principle II). Each unit is a single skill
domain and a verifiable milestone.

### Unit 1 — Failing unit-test harness (domain: tests) — RED

* **Changes**: Add failing unit tests to `tests/unit/parsing_test.rs`, invoking the
  already-public `parse_source(source, Language::Python)` (both are already imported
  in that file; no new `pub` surface). Mirror the Rust call tests in intent, renamed
  for Python semantics. Scenarios:
  1. **Positive bare**: `def orchestrate():\n  step_one()\n  step_two()` →
     `Calls(orchestrate→step_one)` and `Calls(orchestrate→step_two)`, both
     `is_method:false`.
  2. **Attribute not promoted and not staged**: `def f():\n  obj.save()` and a
     `self`-receiver case `def g():\n  self.foo()` → assert **zero** `Calls` edges
     with `callee in {save, foo}` (strong count assertion, not "if any", so it
     cannot pass vacuously against a silent drop). Paired with scenario 1's positive
     presence.
  3. **Nested bare call inside attribute args**: `def f():\n  obj.save(compute())` →
     `Calls(f→compute)` **is** present (DFS still recurses into call arguments).
  4. **Builtin blocklist** (renamed `skips_builtin_calls_in_call_discovery`):
     `def f():\n  print(x)\n  real_call()` → callees include `real_call`, exclude
     `print`.
  5. **Nested-scope ownership**: `def outer():\n  def inner():\n    leaf()\n  top()`
     → `Calls(outer→top)` present; assert **no** `Calls(outer→leaf)` (DFS stops at
     nested `function_definition`/`lambda`/`class_definition` boundaries).
  6. **Graceful degradation**: source with unmodeled call shapes (`d["k"]()`
     subscript-call, `a().b()` chained-call) does not panic and emits no spurious
     bare edge.
* **Files**: `tests/unit/parsing_test.rs` (registered `[[test]]` target `unit_parsing`).
* **Verification**: `cargo test --test unit_parsing` compiles and the new tests **fail**.
* **Milestone**: red tests committed.

### Unit 2 — Python bare-call extraction (domain: code) — GREEN

* **Grammar pre-check (first step)**: run a one-off debug tree-walk over a real
  `.py` sample to confirm the actual `tree-sitter-python 0.23` node kinds and field
  names before coding — do not assume them from `rust.rs`. Expected: a `call` node
  with a `function` field; bare `foo()` → `function` kind `identifier`; `obj.foo()`
  → `function` kind `attribute`, whose fields are **`attribute`** (callee) and
  **`object`** (receiver) — *not* Rust's `field`/`value`.
* **Changes** in `src/services/parsing/python.rs` (deliberately diverging from a
  literal mirror of `rust.rs:229-373`):
  * `extract_calls_from_body(node, source, caller_name, edges)` — DFS over a
    function body that **stops descending at nested `function_definition`,
    `lambda`, and `class_definition` nodes**, so calls are attributed only to their
    owning top-level function.
  * `resolve_call_name`:
    * `call.function` kind `identifier` → bare call (`is_method:false,
      is_qualified:false`), promote.
    * kind `attribute` → `is_method:true`, callee = the `attribute` field text, and
      leave `raw_qualifier` **empty** (do **not** copy the `object` receiver). This
      makes `should_stage_provenance_call(true, false, "")` return `false`, so the
      call is dropped and never enters Rust-specific provenance staging — closing
      the `self`-receiver leak. Continue recursing into the call's **arguments** so
      nested bare calls are still captured.
    * other kinds (`subscript`, chained `call`) → skip in v1 (match arm returns
      `None`; forward-compatible, no panic).
  * `PYTHON_CALL_BLOCKLIST` — builtins that are idiomatic no-ops: `print, len, str,
    int, float, bool, list, dict, set, tuple, range, super, isinstance, issubclass,
    getattr, setattr, hasattr, enumerate, zip, map, filter, open, type, repr,
    format, sorted, sum, min, max, abs, next, iter, id, vars, dir`.
  * **Omit** all Rust scoped-path helpers (`scoped_call_name`,
    `scoped_path_segments`, `collect_scoped_segments`) — Python has no `::` form, so
    copying them yields `dead_code` and fails `-D warnings`. `is_qualified` is never
    true for Python.
  * Wire into `extract_top_level`: after pushing `Defines` for each
    `function_definition`, invoke `extract_calls_from_body(child, source,
    &func.name, edges)` (same placement as `rust.rs:207`).
* **Files**: `src/services/parsing/python.rs`.
* **Verification**: `cargo test --test unit_parsing` (Unit 1 tests now **pass**);
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` clean;
  `cargo fmt --all -- --check`.
* **Milestone**: green unit tests.

### Unit 3 — Language-scoped cross-file singleton resolution (domain: code)

* **Why**: `reresolve_calls_edges` (`src/db/cozo_queries.rs:2177-2249`) builds a
  `name → [id]` index from `*function_meta { id, name }` and creates a
  `calls_resolved_singleton` edge whenever a staged bare callee name has exactly one
  workspace-global definition — **regardless of language**. Once Python emits staged
  cross-file bare calls, this can mis-bind a Python call to a lone same-named Rust
  function (P1). This unit closes that before Unit 4 trusts cross-file Python.
* **Changes**: language-scope the resolver. Build the candidate index as
  `name → [(id, language)]` by joining `function_meta { id, name, file_path }` with
  `file_node { path: file_path, language }`. Derive the **caller's** language from
  the caller function's file the same way. Filter candidates to the caller's
  language **before** the `ids.len() == 1` unambiguity check, so "exactly one" means
  "exactly one same-language definition".
* **Regression safety**: today every staged call originates from Rust and resolves
  to Rust; the same-language filter is a **no-op** for that population. Add a
  Rust-path regression assertion that existing singleton resolution is unchanged.
* **Files**: `src/db/cozo_queries.rs` (`reresolve_calls_edges` + a joined-index
  helper); `src/services/code_graph.rs` only if caller-language plumbing is needed.
* **Verification**: `cargo test --test integration_calls_recall_acceptance --test integration_code_graph`
  green; existing Rust calls tests unaffected;
  `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` clean.
* **Milestone**: cross-file resolution provably same-language.

### Unit 4 — Integration + adversarial acceptance (domain: tests)

* **Changes**: add Python fixtures and integration cases:
  1. **Intra-file** bare call indexes and produces a `direct` `calls_edge`;
     `map_code`/`impact_analysis` return the caller/callee relationship.
  2. **Cross-file, target-correct**: two `.py` files where file A's `orchestrate()`
     calls `helper()` defined in file B → assert the edge resolves to **B's exact
     `helper` id** (target-identity assertion, not mere row-existence — per the
     082-F acceptance gate).
  3. **Adversarial cross-language**: a `.py` file calling `parse()` where the only
     workspace-global `parse` is a **Rust** `fn parse` → assert **no** `calls_edge`
     is created from the Python caller to the Rust function (proves Unit 3).
* **Files**: `tests/integration/code_graph_test.rs` and/or
  `tests/integration/calls_recall_acceptance_test.rs` (targets `integration_code_graph`,
  `integration_calls_recall_acceptance`) plus `.py` fixtures under an existing
  test-fixtures path.
* **Verification**: `cargo test --test integration_calls_recall_acceptance --test integration_code_graph`
  passes.
* **Milestone**: end-to-end edge creation, correct cross-file target, and no
  cross-language mis-binding.

### Unit 5 — Documentation (domain: docs)

* **Changes**: document Python call-graph support and its **v1 limitations**:
  bare-call-only promotion; attribute/method calls neither promoted nor staged;
  calls inside class method bodies not captured (methods aren't indexed as symbols);
  **decorated top-level defs** (`@decorator` → `decorated_definition`) are not
  indexed and their bodies are skipped — a named v1 non-goal with a noted recall
  impact; nested-function calls are attributed only to their own scope; cross-file
  Python resolution is language-scoped (no cross-language edges); Python dynamism
  lowers precision vs. Rust.
* **Files**: `docs/ARCHITECTURE.md` and/or `docs/QUALITY_SCORE.md`.
* **Verification**: prose review; no build impact.
* **Milestone**: documented capability + honest limitations.

## Dependency Graph

```text
Unit 1 (red harness) ──▶ Unit 2 (bare extraction, green) ─┐
                                                          ├─▶ Unit 4 (integration + adversarial)
Unit 3 (language-scoped resolver) ────────────────────────┘
Unit 2 ──▶ Unit 5 (docs)
Unit 3 ──▶ Unit 5 (docs)
```

No cycles. Unit 3 is independent of Units 1–2 in code but MUST land before Unit 4
trusts cross-file Python (and before any Python cross-file edge is shipped). Unit 4
depends on both Unit 2 (Python emission) and Unit 3 (language scoping). Unit 5
depends on the capability being real (Units 2 and 3).

## Decisions and Rationale

* **Bare-call-only promotion in v1.** Name-only resolution cannot safely map
  `x.foo()`/`mod.bar()` to a definition (Python methods are not indexed as
  symbols; module targets are ambiguous). Promoting them risks false singleton
  edges — the same hazard `rust.rs` guards against. Extract-and-mark (`is_method`)
  keeps the door open for future method-aware resolution without emitting wrong
  edges now.
* **Attribute calls are marked but never staged.** Copying the receiver into
  `raw_qualifier` (as `rust.rs::method_call_name` does) would make a `self`-receiver
  call satisfy `should_stage_provenance_call(true, false, "self")` and route Python
  data into Rust-specific canonical staging (`code_graph.rs:188-194`). v1 leaves
  `raw_qualifier` empty so attribute calls fail closed and never pollute the
  provenance table — and never risk the v2 method-body work flooding staging.
* **Language-scoped cross-file resolution (Unit 3).** The shared singleton
  post-pass resolves by bare name across all languages. Rather than special-case
  Python in the consumer, scope the resolver itself to same-language candidates.
  This is regression-safe for the current Rust-only population (a no-op), closes the
  cross-language false-edge vector for every future language, and upholds the 013-D
  no-false-edge invariant and the 082-F target-correctness gate.
* **Reuse the language-agnostic bare-call consumer untouched.** Emitting only bare
  calls routes through `code_graph.rs:896-908` (direct edge / staged-by-name); the
  only shared change is the resolver's candidate filter (Unit 3), not the consumer's
  bare-call arm.
* **A Python builtin blocklist** mirrors Rust's `CALL_BLOCKLIST` to suppress
  high-frequency no-op callees that would add graph noise without navigational
  value.
* **Test-first, split by domain.** Separating the red harness (Unit 1) from the
  implementation (Units 2, 3) satisfies both Principle II (observe red before green)
  and width isolation.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Tree-sitter-python node/field names differ from assumptions — esp. `attribute` node fields (`attribute`/`object`, not Rust's `field`/`value`) | Unit 2 grammar pre-check via a debug tree-walk against a real file before coding; Unit 1 scenario 2 asserts a positive `is_method:true`/skip outcome so a silent-drop mapping error fails loudly |
| Cross-file bare call mis-binds to a same-named definition in another language | Unit 3 language-scopes the resolver; Unit 4 adversarial case asserts no Python→Rust edge |
| DFS attributes nested-scope calls to the wrong (outer) function | Unit 2 stops descent at nested `function_definition`/`lambda`/`class_definition`; Unit 1 scenario 5 covers it |
| Low precision on dynamic/OOP-heavy Python (methods, decorators, callbacks) | Documented as v1 non-goals (Unit 5); measure with `run_retrieval_eval` / `get_retrieval_eval_report` rather than asserting a target |
| Decorated top-level defs (`decorated_definition`) skipped → recall gap | Documented as a named v1 non-goal (Unit 5); deferred to avoid widening `extract_top_level` scope in this pilot |
| Calls inside class method bodies not captured (methods aren't symbols) | Explicit v1 limitation; a later unit could index class methods as symbols and extend extraction |
| Existing indexes lack Python edges until re-index | Normal incremental indexing behavior (`sync_workspace` / `index_workspace`); not a migration |
| Modifying the shared `reresolve_calls_edges` regresses Rust resolution | Same-language filter is a no-op for the Rust-only staged population; Unit 3 adds a Rust-path regression assertion |
| Noise from over-broad blocklist or missing common builtins | Keep blocklist conservative; tune via integration/eval evidence |

## Constitution Check

| Principle | Assessment |
|---|---|
| I. Safety-First Rust | All new code returns `Result`/`Option`; no `unwrap`/`expect`/`panic`, no `unsafe`, no new casts. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` is a per-unit gate. Satisfied. |
| II. Test-First (NON-NEGOTIABLE) | Unit 1 authors failing unit tests before Unit 2 implements; Unit 4 authors the cross-file/adversarial assertions. Extractor behavior is observed red→green. Satisfied. |
| III / IV. Workspace / CLI containment | Parser + resolver over already-indexed content; no filesystem path ops, no writes outside cwd. Satisfied. |
| V. Structured Observability | Edge creation flows through existing indexing logs; no new silent path. Satisfied. |
| VI. Single Responsibility | No new dependency (`tree-sitter-python 0.23` already declared). Satisfied. |
| VII / VIII. Destructive approval / Safety modes | No destructive command; additive parser + no-op-for-Rust resolver filter. N/A. |
| IX. Git-Friendly Persistence | Plan is Markdown + YAML frontmatter; Unit 5 docs follow the convention. Satisfied. |
| X. Context Efficiency | No change to tool response shape; additive edges only. Satisfied. |
| XI. Merge Commit History | Process-level; observed at ship time. N/A to plan content. |

No justified violations.

## Plan Hardening (REQUIRED)

Unit 3 modifies `reresolve_calls_edges`, a correctness-critical shared post-pass
governed by the operator-signed 082-F target-correctness gate. That elevates this
plan above a pure parser-local change, so it carries hardening detail.

* **ProposedAction** — language-scope the cross-file singleton resolver so staged
  bare calls resolve only to same-language definitions; emit Python bare-call edges.
* **ActionRisk** — `moderate`. It touches shared graph-resolution code, but the
  same-language filter is a proven no-op for the current Rust-only staged
  population, so Rust resolution behavior does not change.
* **Verification** — (a) Rust-path regression assertion that existing singleton
  resolution is unchanged (Unit 3); (b) adversarial cross-language acceptance test
  proving no Python→Rust mis-binding (Unit 4); (c) cross-file target-identity
  assertion, not row-existence (Unit 4); (d) full ordered quality-gate suite before
  merge (below).
* **Rollback** — additive and reversible: revert the Unit 2/Unit 3 commits. No
  migration, schema change, or destructive step; edges regenerate on the next index.
* **ActionResult** — `planned` (execution deferred; this remains planning-only).

### Hardening Signals

* **Public API / schema / contract change** — *absent*. No `calls_edge` schema or
  MCP tool schema change; `map_code`/`impact_analysis` additively include Python
  edges after re-index.
* **Security / auth / permission / compliance** — *absent*.
* **Migration / backfill / destructive / irreversible** — *absent*. Edges are
  regenerated by normal incremental indexing.
* **External integration / operator checkpoint / external dependency** — *absent*.
  No new dependency.
* **High runtime / rollout / rollback risk** — *low but non-trivial*: Unit 3 edits
  shared resolution code, hardened by the Rust-path regression + adversarial tests
  above.

**Requires plan hardening: yes — satisfied inline (this section).**

## Quality Gates (pre-merge, constitutional order)

Run in order; do not skip:

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings -D clippy::pedantic
cargo dev-test
cargo audit
```

Per-unit `cargo test --test <target>` invocations (targets `unit_parsing`,
`integration_code_graph`, `integration_calls_recall_acceptance`) drive the red/green
loop; the full ordered suite above is the merge gate.

## Runtime Verification and Closure

* **Changed runtime surface**: the code-graph MCP tools `map_code`,
  `impact_analysis`, and `query_graph` will begin returning call edges for `.py`
  files (previously empty). CLI/daemon indexing path is unchanged structurally.
* **Runtime verification** (before absorbed): index a small real Python module,
  then call `map_code` and `impact_analysis` on a known top-level function and
  confirm expected callers/callees appear and no spurious builtin edges are
  present. Unit 4 covers this in automated form (including the adversarial
  cross-language case); a manual daemon check confirms the live tool surface.
* **Operational closure**: record the behavioral expansion (Python now yields
  call edges) and the documented precision limits. No feature flag, monitoring
  dashboard, or rollback trigger is required for this additive parser change;
  precision can be tracked over time via `get_retrieval_eval_report`. Ownership:
  code-graph parsing area.

## Following Steps (outside this plan)

1. `plan-review` this plan (done — see `## Plan Review`), then `harvest` it into a
   feature + the five units above under the Python-calls work (stash `CD1EAE09`).
2. After the Python-calls feature ships, pick up Spark-lineage spike `07BFA98E`
   (notebook DataFrame/temp-view/table lineage — distinct from call graph).
3. Optionally, a separate release unit generalizes this extractor pattern to
   TypeScript → Go → C# → Node (spike Next Steps item 3). That rollout inherits the
   language-scoped resolver from Unit 3.

## References

* Spike: `docs/decisions/2026-07-20-python-call-edge-extraction-spike.md`
* Reference extractor: `src/services/parsing/rust.rs:229-373`
* Target parser: `src/services/parsing/python.rs`
* Language-agnostic bare-call consumer: `src/services/code_graph.rs:851-909`;
  provenance gate `should_stage_provenance_call` at `code_graph.rs:188-194`
* Cross-file singleton resolver (Unit 3 target): `src/db/cozo_queries.rs:2177-2249`
* Function/file language linkage: `function_meta { id, name, file_path }` ⋈
  `file_node { path, language }`
* Edge shape: `src/services/parsing.rs:184-243`
* Existing calls tests to mirror: `tests/unit/parsing_test.rs:250-346`
* Test targets: `unit_parsing`, `integration_code_graph`,
  `integration_calls_recall_acceptance` (Cargo.toml)
* Target-correctness acceptance gate (082-F):
  `docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md`
* No-false-edge invariant (013-D) and canonical-identity context:
  `docs/decisions/2026-07-15-091-001-canonical-identity-spike.md`,
  `docs/decisions/decision-013 - Cross-File-Call-Edges-Deferred.md`

## Plan Review

**Reviewed**: 2026-07-20 · **Skill**: plan-review · **Personas**: Constitution
Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher (always-on),
Architecture Strategist (cross-model, gpt-5.6-sol). Security-Lens and Agent-Native
Parity personas were not triggered (no auth/API/secrets/data store; additive
`map_code`/`impact_analysis` output only, no tool-contract change).

**Gate decision (initial): FAIL** — one P1 (cross-model corroborated) on the
pre-revision plan. **Gate decision (after revision): PASS** — the P1 and all
actionable P2/P3 findings are resolved in the units above; see Resolution.

**Plan hardening**: initially declared "no"; review found the plan expands the
`calls_resolved_singleton` population to a new language governed by the
operator-signed 082-F acceptance gate and (after adding Unit 3) edits shared
resolution code. Hardening requirement is now **yes — satisfied** by the
`## Plan Hardening` section.

### Findings by severity

**P1 (gate-blocking) — resolved**

* **Cross-language false-edge / missing target-correctness gate.** The cross-file
  singleton resolver (`reresolve_calls_edges`, `cozo_queries.rs:2177-2249`) matches
  staged bare callee names against `*function_meta` with no language filter. Once
  Python emits staged bare calls, a Python `parse()` whose only unique
  workspace-global match is a Rust `fn parse` mis-binds to it — violating the 013-D
  no-false-edge invariant and the 082-F target-correctness gate, which requires
  exact-target-identity assertions rather than row-existence. Flagged independently
  by **Architecture Strategist** (cross-language mis-binding) and **Learnings
  Researcher** (082-F acceptance gate) — high confidence, verified in code.

**P2 (moderate) — resolved**

* **`self`-receiver leak into Rust provenance staging** (Architecture + Rust). A
  literal mirror of `rust.rs::method_call_name` sets `raw_qualifier="self"`, which
  satisfies `should_stage_provenance_call(true,false,"self")` and routes Python data
  into Rust-specific canonical staging.
* **Attribute node field names** `attribute`/`object`, not Rust's `field`/`value`
  (Rust) — a verbatim mirror silently drops attribute calls and lets the hedged
  Unit 1 assertion pass vacuously.
* **Unrestricted DFS crosses nested function/lambda/class scopes**, mis-attributing
  calls to the outer function (Architecture); nested **bare** calls inside
  unpromoted attribute calls must still be captured (Learnings).
* **Quality-gate fidelity** (Constitution): clippy omitted `--all-targets`;
  verification omitted the full ordered suite (`fmt → clippy --all-targets →
  cargo dev-test → cargo audit`) and was out of order.
* **Missing `Constitution Check` section** (Constitution governance requirement).
* **Empirical node-kind discovery + graceful degradation** (Learnings): confirm
  grammar node/field names via a debug tree-walk; degrade on unmodeled kinds.
* **Negative-assertion vacuity** (Learnings + Rust): pair "no bare `save` edge" with
  a positive `is_method:true`/skip assertion.

**P3 (advisory) — resolved or accepted**

* Dead `scoped_*` helpers would fail `-D warnings` — omit them (Rust).
* Use public `parse_source(_, Language::Python)`; no new `pub` wrapper (Rust).
* Wrong test target name `parsing_test` → `unit_parsing` (Constitution); integration
  targets named `integration_code_graph` / `integration_calls_recall_acceptance`.
* Decorated top-level defs (`decorated_definition`) skipped → recall gap: documented
  as a named v1 non-goal (Rust).
* Test naming `skips_macro_invocations` → `skips_builtin_calls` for Python semantics
  (Rust).
* Integration behavior authored green-first: mitigated — extractor logic is red-first
  in Unit 1; Unit 4 adds adversarial assertions (Constitution).
* Keep extraction parser-local; defer a shared trait until a second language exists
  (Architecture, Scope) — accepted; no shared trait introduced.
* Scope Boundary Auditor: plan well-bounded (2×P3, provisional blocklist size and
  the attribute extract-and-mark tradeoff) — both accepted; blocklist tuned via eval,
  attribute calls now skipped rather than mis-staged.

### Resolution (revision applied before harvest)

| Finding | Resolution in revised plan |
|---|---|
| P1 cross-language mis-binding / 082-F gate | New **Unit 3** language-scopes `reresolve_calls_edges` (join `function_meta`→`file_node.language`, no-op for Rust); **Unit 4** adds adversarial cross-language + exact-target-identity assertions |
| P2 `self` leak | Unit 2 leaves `raw_qualifier` empty for attribute calls → fails closed at `should_stage_provenance_call` |
| P2 attribute field names | Unit 2 uses `attribute`/`object`; grammar pre-check step added |
| P2 DFS nested scopes | Unit 2 stops descent at nested callable/class; Unit 1 scenarios 3 & 5 |
| P2 gate fidelity | `## Quality Gates` in constitutional order with `--all-targets`, `cargo dev-test`, `cargo audit` |
| P2 Constitution Check | `## Constitution Check` section added |
| P2 node-kind discovery | Unit 2 grammar pre-check; Unit 1 scenario 6 graceful degradation |
| P2 negative-assertion vacuity | Unit 1 scenario 2 uses strong count assertions paired with positive scenario 1 |
| P3s | Public `parse_source`; omit scoped helpers; correct target names; decorator non-goal documented; renamed builtin test |

**Runtime verification & closure**: present and adequate (`## Runtime Verification
and Closure`), now referencing Unit 4's adversarial coverage. No gaps.

**Outcome**: revised plan is **harvest-ready** (feature + 5 tasks under stash
`CD1EAE09`), respecting the dependency graph U1→U2, U3, (U2,U3)→U4, (U2,U3)→U5.
