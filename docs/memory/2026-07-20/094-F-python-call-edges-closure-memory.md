---
title: 094-F closure — Python bare-call code-graph edges + language-scoped resolution
type: closure-memory
date: 2026-07-20
feature: 094-F
shipment: 089-S
tasks: [094.001-T, 094.002-T, 094.003-T, 094.004-T, 094.005-T]
pr: 277
merge_commit: 5f18b796853cb82f977494672fa280046bcbe5b8
status: shipped
harvest_source: CD1EAE09
follow_ups: [FF7DE872, FE8B3B2D, 07BFA98E]
---

## Outcome

Feature 094-F shipped via PR #277 (merge commit `5f18b79`, merge-commit
strategy) and shipment 089-S closed. This is the **pilot** for a later
per-language call-graph rollout (TS/Go/C#/Node). Before 094-F, only Rust
emitted `Calls` edges; Python files had no call graph. Now `map_code`,
`impact_analysis`, and `query_graph` light up `.py` call relationships for
bare calls (`foo()`), and the shared cross-file singleton resolver is
language-scoped so Python bare calls cannot mis-bind to same-named
definitions in another language.

Tasks completed: 094-F (feature) + 094.001-T..094.005-T (U1–U5, test-first).
The 5 leaf tasks were archived/done by the earlier Ship build run; this
session performed post-merge closure only.

## What shipped

- **Bare Python `Calls` edges** — `identifier` calls (`foo()`) promoted as
  `is_method:false`; body-scoped extraction; builtin/idiomatic callee blocklist
  to cut navigational noise.
- **Language-scoped cross-file singleton resolver (U3)** — candidate
  definitions filtered to the caller's language BEFORE the exactly-one
  unambiguity check, preventing cross-language mis-binding (013-D no-false-edge,
  082-F target-correctness).
- **Fail-closed boundaries** — attribute/method calls (`obj.f()`, `self.g()`)
  dropped (empty `raw_qualifier` → consumer drops); subscript/chained call
  shapes skipped in v1; nested/inner-function calls omitted (walk stops at
  nested callable/class scopes); parameter defaults/annotations/decorators
  excluded from the body walk.

## Files modified

- `src/services/parsing/python.rs` — `extract_calls_from_body` (body-scoped DFS,
  nested-scope stop), `resolve_call_name` (identifier/attribute classification,
  fail-closed drops), builtin blocklist.
- `src/db/cozo_queries.rs` — language-scoped singleton resolver (`same_language`
  filter-before-count); the resolved edges carry `calls_resolved_singleton`
  provenance.
- `tests/unit/parsing_test.rs`, `tests/integration/code_graph_test.rs` — RED-first
  unit harness + the discriminating mixed-language ordering proof
  `python_cross_file_call_resolves_to_python_target_amid_same_name_rust_def`.
- `docs/architecture.md` — documented the Python call-edge extraction and the
  language-scoping invariant.

## Decisions and rationale

- **Namespace-qualified resolution feasibility = YES, but it is its own
  feature.** The "proper" fix for cross-module same-name ambiguity is to extend
  engram's Rust canonical-path resolution to Python's module namespace
  (`function_meta.canonical_path` exists but is Rust-only populated). Feasible —
  Python modules are namespaces — but it needs a symbol-level import-binding
  table (Python analogue of Rust's `UseGraph`) plus a Python canonical_path
  populator and resolver. Scoped OUT of 094-F as deliberation-spike **FE8B3B2D**
  (kind=feature). Explicit non-goal: instance-method dispatch (needs type
  inference).
- **Observability waiver → justified N/A.** No new counters/metrics required for
  this pilot; the edge output is verifiable through existing
  `map_code`/`impact_analysis`/`query_graph` surfaces and unit/integration
  tests. Recorded as a justified N/A rather than an open observability gap.
- **Bug split (FF7DE872 out of FE8B3B2D).** The same-file same-name shadowing
  correctness gap (`find_function_id` returns FIRST match) is a special case the
  namespace feature would subsume, but it can and should be fixed independently
  and sooner. Split into standalone bug **FF7DE872** so it is NOT gated behind
  the namespace deliberation.
- **Harvest source reconciled.** Stash **CD1EAE09** (the 094-F harvest source,
  previously "IMPLEMENTATION DEFERRED / planning-only") was updated to reflect
  the shipped outcome and archived (DB state `removed`).

## Closure operational note (gate-evidence remediation)

`backlogit shipment ship 089-S` initially refused: member **094.001-T** carried
gate evidence pinned to an orphaned pre-amend commit (`ba284e1b`), superseded by
the identical landed commit `88facd4` via `git commit --amend` during PR prep;
`ba284e1b` is not reachable from the merge SHA. Remediation was tool-native and
append-only: reopened 094.001-T to `active`, re-ran the completion gate on
merged `main` (passed, re-recording `head_sha = 5f18b79`), then re-shipped
successfully. No audit-log rewrite; no `--force-gates` needed.

## Next steps (all LATER — do not action now)

- **FF7DE872** (bug, medium): same-file same-name shadowing target precision —
  independent follow-up, sooner than the namespace feature.
- **FE8B3B2D** (feature, medium): Python module-namespace-qualified call
  resolution — later staging cycle (sequenced after 094-F and 07BFA98E).
- **07BFA98E** (spike, medium): Spark notebook data-lineage tracking — later,
  depended on the Python calls pilot landing (now satisfied).
- **Per-language rollout** (TS/Go/C#/Node): each new language inherits the
  language-scoped singleton resolver; re-run the mixed-language ordering proof
  when adding each.
- **Operational reminder:** existing `.py` files need a **forced** reindex
  (`engram sync --force`, or equivalently `engram index --force`) to acquire the
  new edges. Plain `engram sync`, `engram index`, and `engram sync --full` all
  pass `force=false` and hash-skip unchanged files (a no-op for backfill); only
  `--force` sends `{"force": true}` and re-parses them.
