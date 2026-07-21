---
title: "Add Python Calls edge extraction to the code graph"
type: spike
date: 2026-07-20
time_box: "2h"
conclusion: "proceed"
confidence: "high"
linked_parent_work_item: "094-F"
promoted_to: ["queue", "plan", "backlog"]
plan_artifact: docs/exec-plans/2026-07-20-python-call-edge-extraction-plan.md
harvested_to: "094-F"
stash_id: "CD1EAE09"
sequences_before: "07BFA98E"
tags:
  - "code-graph"
  - "python"
  - "tree-sitter"
  - "call-graph"
  - "notebook"
  - "multi-language"
---

## Goal

Is it feasible and worthwhile to extract `Calls` edges from Python source so that
`map_code`, `impact_analysis`, and `query_graph` expose caller/callee navigation
for `.py` files, and what is the smallest repo-aligned approach? This work is the
prerequisite for any future Jupyter notebook call-graph support (Spark lineage
spike `07BFA98E` is sequenced strictly after this one).

## Success Criteria

* A clear yes/no on feasibility grounded in the current code-graph pipeline.
* An identified change surface (which files, which functions) that fits the
  2-hour granularity rule.
* Explicit precision limitations documented so downstream planning does not
  over-promise.
* Confirmation of whether a new dependency is required.

## Scope Constraints

Read-only investigation. No parser or schema changes made during the spike. The
notebook and Spark-lineage work is explicitly out of scope and deferred to the
sequenced follow-on spike `07BFA98E`.

## Investigation Approach

1. Confirm which languages currently emit `Calls` edges and how they are stored.
2. Trace the full path from parser output to a persisted `calls_edge` row.
3. Determine how much of that path is language-agnostic versus Rust-specific.
4. Read the Rust extractor as the reference pattern and map it onto the Python
   tree-sitter grammar.
5. Enumerate Python-specific precision risks.

## Findings

### What Was Discovered

**Only Rust emits `Calls` edges today.** A search of `src/services/parsing/`
shows `ExtractedEdge::Calls { .. }` is pushed only in
`src/services/parsing/rust.rs:239` (`extract_calls_from_body`). The Python
parser (`src/services/parsing/python.rs`) extracts only `Defines` (functions,
classes) and `Imports` — no call edges, and its module doc states method bodies
are not indexed ("Tier 1 implementation"). So there is no Python call graph even
for plain `.py` files; the notebook gap is a symptom of this larger gap.

**The downstream consumer is language-agnostic.** The edge-promotion loop in
`src/services/code_graph.rs:851-909` matches `ExtractedEdge::Calls` without any
language check:

* Bare calls (`is_method == false && is_qualified == false`) are resolved by
  name within the file's symbols via `find_function_id`. A locally resolved
  callee becomes a **direct** `create_calls_edge(from_id, to_id)`; a
  caller-resolved but callee-unresolved (cross-file) call is handed to
  `put_staged_call` for the deferred cross-file post-pass (082.002-T), keyed by
  bare callee name.
* Method calls are dropped by `should_stage_provenance_call`
  (`code_graph.rs:188`), so they never create a false edge.
* Qualified calls are staged with provenance
  (`put_staged_call_with_provenance`), but the re-resolution pass
  (`reresolve_calls_edges_with_canonical_context`) is Rust-crate-specific
  (`crates`, `rust_ctx`, `unsafe_prefixes`). A Python qualified call staged here
  would simply never match and produce no edge — safe, if slightly wasteful.

The practical consequence: if the Python extractor emits **only bare-identifier
calls** and marks attribute/method calls as `is_method`, everything flows through
the existing safe, language-agnostic direct/staged path and never touches the
Rust canonical machinery.

**Routing, storage, and dependency already exist.** Python files already reach
`parse_python_source` today (they are indexed for `Defines`/`Imports`), so
`Language::try_from("python")` and `language_from_path` ("py" -> "python",
`code_graph.rs:1967`) already work. `tree-sitter-python = "0.23"` is already a
declared dependency in `Cargo.toml` — **no new dependency is required**
(Constitution Principle VI satisfied). The `calls_edge` relation, keyed by
`(from, to)`, and `create_calls_edge`/`put_staged_call` already exist.

**Net change surface is essentially one file.** The only new production code is
call extraction inside `src/services/parsing/python.rs`, mirroring the Rust
reference (`extract_calls_from_body` + `resolve_call_name` +
`CALL_BLOCKLIST`). The consumer, DB queries, language routing, and cross-file
post-pass are all reused unchanged.

**Grammar node mapping (Rust -> Python).** The Rust extractor keys off
`call_expression` with a `function` field of `identifier` / `scoped_identifier`
/ `field_expression`. In the tree-sitter-python grammar the equivalents are:

| Concept | Rust node | Python node |
|---|---|---|
| Call site | `call_expression` | `call` |
| Bare call `foo()` | `function` = `identifier` | `function` = `identifier` -> promote |
| Attribute/method `x.foo()` | `field_expression` | `function` = `attribute` (`attribute` field is the method) -> mark `is_method`, do not promote |
| Chained/other | — | `subscript` / nested `call` -> extract best-effort or skip |

Python has no `::` path form, so `is_qualified` is largely unused; module-qualified
calls appear as `attribute` (`mod.func()`) and are treated like method calls in
v1 (conservative, not promoted).

### This is a Rust-only gap, not a Python-only gap

Confirmed mechanically: `ExtractedEdge::Calls` is emitted in exactly one place in
the entire `src/services/parsing/` tree — `rust.rs:239`. Every other language
parser extracts only symbol and import edges:

| Parser | Emits `Calls`? | Edge kinds emitted |
|---|---|---|
| `rust.rs` | yes | Calls, Defines, Imports, InheritsFrom |
| `typescript.rs` | no | Defines, Imports |
| `javascript.rs` (Node) | no | Defines, Imports |
| `go_lang.rs` | no | Defines, Imports |
| `csharp.rs` | no | Defines, Imports |
| `c.rs` | no | Defines, Imports |
| `cpp.rs` | no | Defines, Imports |
| `swift.rs` | no | Defines, Imports |
| `kotlin.rs` | no | (no `Calls`) |
| `sql.rs` | no | Defines, References |
| `markdown.rs` | no | Imports |

So caller/callee navigation via `map_code`, `impact_analysis`, and `query_graph`
is Rust-only across the whole workspace today — TypeScript, Go, Node/JS, and C#
share the exact same gap as Python.

### Generalization: Python is a pilot, the pattern is reusable

Because the promotion path in `code_graph.rs:851-909` matches `ExtractedEdge::Calls`
with no language check, the extractor pattern established here is a **reusable
template**, not a Python-specific hack. Each additional language needs only its
own parser-local call extraction (its tree-sitter call/attribute node mapping plus
a language builtin blocklist); the consumer, `calls_edge` storage, cross-file
post-pass, and graph queries are all shared.

Recommended framing: treat the Python work as the **pilot** that both lights up
`.py` call graphs and de-risks the shared consumer for non-Rust languages. A
follow-on could then roll out the same pattern one parser at a time — a natural
ordering by likely value/effort is TypeScript, then Go, then C#, then JavaScript
(Node), with C/C++/Swift/Kotlin last. This should be a **separate release unit**
from the Python pilot (width isolation, single-parser-per-task granularity), not
folded into `CD1EAE09`. The only shared code that might be touched by a rollout
is the per-language builtin blocklist convention; the core pipeline stays
untouched.

Deliberately **out of scope** for both the Python pilot and any per-language
rollout: Rust's canonical/crate-aware re-resolution
(`reresolve_calls_edges_with_canonical_context`, `code_graph.rs:264-361`) is
Rust-specific and should not be generalized — non-Rust languages rely on the
language-agnostic direct-edge + cross-file-by-name resolution only.

### What Was Tried and Failed

Nothing was implemented (read-only spike). One approach was considered and
rejected: routing Python attribute calls (`mod.func()`) through the qualified
provenance-staging path to recover module-qualified targets. Rejected for v1
because that path's re-resolution is Rust-crate-specific and would add Python
handling to canonical-resolution code for little near-term gain. Keeping v1 to
bare-call promotion isolates the change to the parser.

### Remaining Unknowns

* Exact tree-sitter-python node/field names for chained calls and calls inside
  comprehensions/decorators — to be confirmed against the grammar during
  implementation (does not affect feasibility).
* Whether a Python-specific `CALL_BLOCKLIST` (e.g. `print`, `len`, `str`, `int`,
  `list`, `dict`, `range`, `super`, `isinstance`, `getattr`) is worth the noise
  reduction, or whether builtins should be filtered another way.
* Precision on real Python repos is unmeasured; a retrieval-eval fixture pass is
  the right place to quantify it.

## Recommendation

**Conclusion**: proceed
**Confidence**: high

Add bare-call `Calls` extraction to `python.rs`, mirroring the Rust extractor,
and rely entirely on the existing language-agnostic consumer, DB layer, and
cross-file post-pass. This is a contained, single-parser change with no new
dependency and no schema change, and it lights up `map_code`/`impact_analysis`/
`query_graph` for every `.py` file in the workspace — not just notebooks.

**Deliberate v1 non-goals** (to keep precision honest and scope small):

* No promotion of method/attribute calls (`x.foo()`, `mod.bar()`) to edges —
  extract-and-mark only, matching Rust's conservative stance.
* No calls extracted from method bodies inside classes until class methods are
  indexed as symbols (they are not today), so OOP-heavy call graphs stay
  incomplete in v1. Document this limitation.
* No attempt to resolve dynamic dispatch, decorators, `getattr`, or
  higher-order/callback calls.

**Precision caveats to carry into the plan**: name-only resolution risks the same
false-singleton edges Rust guards against; the existing `is_method`/`is_qualified`
gating plus a Python builtin blocklist are the mitigations. Python's dynamism
means recall/precision will be lower than Rust — set expectations accordingly and
measure with a retrieval-eval fixture rather than asserting a target.

## Next Steps

1. Promote this spike to `impl-plan` for the Python-calls feature. Suggested
   implementation units (each within the 2-hour rule):
   * Unit 1 — add `extract_calls_from_body` + `resolve_call_name` +
     `CALL_BLOCKLIST` to `python.rs`; emit bare `Calls`, mark attribute calls
     `is_method`. Unit tests in `tests/unit/parsing_test.rs` asserting
     `ExtractedEdge::Calls` for bare calls and no promotion for attribute calls.
   * Unit 2 — integration coverage in `tests/integration/code_graph_test.rs` /
     `calls_recall_acceptance_test.rs` proving a `.py` fixture yields
     `calls_edge` rows and `map_code`/`impact_analysis` traverse them.
   * Unit 3 — docs: note Python call-graph support and its v1 limitations
     (method bodies, dynamism) in the architecture/quality docs.
2. Only after the Python-calls feature ships, pick up Spark-lineage spike
   `07BFA98E`. That spike should evaluate a lineage edge type over PySpark +
   `%%sql` notebook cells (DataFrame/temp-view/table read->write), which is a
   distinct abstraction from the function call graph and should reuse the
   notebook extractor (`src/services/notebook_extract.rs`) rather than the
   tree-sitter code-graph path.
3. Optional (separate release unit, sequenced after the Python pilot proves the
   shared consumer): generalize the call-extraction pattern to other languages
   one parser at a time — TypeScript, Go, C#, then JavaScript (Node), with
   C/C++/Swift/Kotlin last. Each is a single-parser task; do NOT extend Rust's
   canonical re-resolution to them. This closes the workspace-wide call-graph
   gap, not just the Python one.

## References

* `src/services/parsing/python.rs` — current Python extractor (Defines/Imports
  only; no Calls)
* `src/services/parsing/rust.rs:229-340` — reference `Calls` extraction pattern
* `src/services/parsing.rs:184-243` — `ExtractedEdge::Calls` shape and semantics
* `src/services/code_graph.rs:851-909` — language-agnostic Calls consumer
  (direct edge / staged cross-file / provenance)
* `src/services/code_graph.rs:188` — `should_stage_provenance_call`
* `src/services/code_graph.rs:264-361` — Rust-specific canonical re-resolution
  (not used by bare Python calls)
* `src/services/code_graph.rs:1962-1983` — `language_from_path` ("py" ->
  "python")
* `Cargo.toml` — `tree-sitter-python = "0.23"` already declared
* `docs/decisions/2026-05-22-jupyter-notebook-source-support-spike.md` — original
  notebook spike that deferred code-graph symbol/edge extraction from cells
* Stash intake: `CD1EAE09` (this spike), `07BFA98E` (sequenced Spark-lineage
  follow-on)
