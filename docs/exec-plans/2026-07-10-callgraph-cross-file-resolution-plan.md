# Exec Plan — rec1-calledges: cross-file & method-call resolution

**Date:** 2026-07-10
**Status:** Decided / Ready for harvest
**Stage owner:** stage agent
**Deliberation:** `docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md` (signed off 2026-07-10)
**Feature (umbrella):** 082-F
**Ships:** SECOND — acceptance-gated by 081-F slices S1 (081.001-T) + S3 (081.005-T).

---

## 1. Objective

Raise call-edge **recall** for idiomatic Rust — cross-file calls and method/receiver calls
(`x.foo()`, `self.bar()`) — that are currently silently dropped, WITHOUT introducing false edges.
Implement **Option B** from the deliberation: capture method calls at extraction, record
unresolved `Calls` edges instead of dropping them, then resolve them in a deferred post-pass that
mirrors the existing `reresolve_references_edges` machinery, creating an edge **only when exactly
one function** in the workspace has that name (unambiguous-name-only guard). Post-pass-resolved
edges are tagged with distinct provenance (`calls_resolved_singleton`) so the eval subsystem and
other consumers can weight them.

Acceptance is **measured by 081-F** (the retrieval-eval subsystem): graph resolution-recall must
rise and false-edge rate must stay within the operator-chosen threshold.

## 2. Operator-confirmed design parameters (2026-07-10)

1. **Precision policy** = unambiguous-name-only (precision-first; skip ambiguous names entirely).
2. **Edge provenance** = tag post-pass-resolved calls distinctly (`calls_resolved_singleton`).
3. **Performance** = gate the global post-pass behind **full / `--force` index only**;
   incremental sync **skips** it.
4. **Language scope** = **Rust-first as slice 1**; fan out to peer tree-sitter extractors as
   follow-on tasks under this same feature (not in the first shipment).

## 3. Grounding — real modules (verified 2026-07-10)

Extraction (method-call capture):
- `src/services/parsing/rust.rs:251-266` `resolve_call_name` handles `identifier` (bare `foo()`)
  and `scoped_identifier` (`a::b::foo()` → last segment); `_ => None` currently drops
  `field_expression` (method/receiver calls `x.foo()`, `self.foo()`). `CALL_BLOCKLIST`
  (`:247-249`, `new/default/into/clone/from/unwrap/expect/ok/err`) must be preserved.
- `extract_calls_from_body` (`:224-245`) walks `call_expression` nodes and emits
  `ExtractedEdge::Calls { caller, callee }`.

Resolution (record-not-drop + post-pass):
- `src/services/code_graph.rs:466-475` (index path) resolves `Calls` with
  `find_function_id(&function_ids, callee)` where `function_ids` holds **only this file's**
  symbols ("Resolve names to IDs within this file's symbols") — every cross-file call is dropped
  silently. Sync path mirror at `:1070-1077`.
- `cross_file_edges_dropped` counter at `:52 / :496 / :1099` (currently only `Imports` are
  counted as deferred cross-file).
- **Reuse target** — the existing deferred post-pass: `code_graph.rs:543` (index) / `:1152`
  (sync) call `queries.reresolve_references_edges()`, implemented at
  `src/db/cozo_queries.rs:1357` (`reresolve_references_edges(&self) -> Result<ReresolveResult,
  EngramError>`). The new `Calls` post-pass mirrors this pattern.
- `find_function_id` helper at `code_graph.rs:1427-1431` (name → id, file-local today).

Acceptance measurement (from 081-F):
- `src/services/retrieval_eval.rs` (081.005-T) computes **resolution_recall** (resolved ÷ visible
  call sites from `extract_calls_from_body`) and **false_edge_rate**. This plan's success is
  defined against those metrics.

Safety: `Result<T, EngramError>` everywhere; no `unwrap`/`expect`; `#![forbid(unsafe_code)]`;
clippy pedantic. Test tiers under `tests/{unit,integration}`.

## 4. Duplicate / overlap determination — NOT a duplicate

- `decision-013 - Cross-File-Call-Edges-Deferred.md` records that cross-file call edges were a
  **known deferred limitation**, not a bug — this feature is the deliberate lift of that deferral.
- The `References` post-pass (`reresolve_references_edges`) resolves a **different** edge type
  (SQL references), but its machinery/pattern is the reuse target — pattern reuse, not overlap.
- Method-call capture (`field_expression`) has no existing handling (`_ => None`). Greenfield.

Conclusion: proceed; extend `resolve_call_name`, record unresolved `Calls`, add a
`reresolve_calls_edges` post-pass modeled on `reresolve_references_edges`.

## 5. Task decomposition (Rust-first — first shipment)

Each task is test-first, ≤3 source files, ≤5 functions, ≤4 test scenarios, single width domain.

### 082.001-T — Capture method/receiver calls in `resolve_call_name`  *(domain: parser/extraction)*
- **Files:** `src/services/parsing/rust.rs` (add `field_expression` arm to `resolve_call_name`).
  Test: `tests/unit/rust_method_call_extraction_test.rs` (new).
- **Behavior:** extract the method name from a `field_expression` receiver call (`x.foo()` →
  `foo`, `self.bar()` → `bar`); keep the `CALL_BLOCKLIST` filter so `x.clone()`/`x.unwrap()`
  stay dropped. No resolution change — extraction only.
- **Unit scenarios (4):** `x.foo()` → callee `foo`; `self.bar()` → `bar`; blocklisted
  `x.clone()` → None; chained `a.b().c()` → captures both non-blocklisted method names.
- **Depends on:** none.

### 082.002-T — Record unresolved `Calls` edges instead of dropping them  *(domain: core indexing)*
- **Files:** `src/services/code_graph.rs` (index path `:466-475` + sync path `:1070-1077`:
  when `find_function_id` misses, stage the unresolved callee name rather than discarding),
  `src/db/cozo_backend/schema.rs` (new `staged_call` relation) + `src/db/cozo_queries.rs`
  (persist/retrieve/**clear** staging rows). Test:
  `tests/integration/unresolved_calls_capture_test.rs` (new).
- **Staging relation & lifecycle:** define `staged_call { caller_id, callee_name, source_file => created_at }`
  keyed so every staged row carries its **source file**. A file's staged rows MUST be cleared
  before that file is (re-)indexed — both the full-index path and the incremental sync path clear
  a file's prior `staged_call` rows before re-staging, so a changed or removed call never leaves a
  stale staged row that a later forced post-pass could resolve into a stale edge. File deletion
  clears the deleted file's staged rows.
- **Behavior:** a within-file resolvable call still creates a direct `calls` edge immediately
  (unchanged, provenance `direct` — see 082.003-T); an unresolved callee is recorded (caller id +
  callee name + source file) for the post-pass. Both index and sync paths behave identically.
  Preserve `cross_file_edges_dropped` accounting semantics.
- **Integration scenarios (6):** cross-file callee is recorded (not silently dropped); in-file
  callee still yields a direct edge; sync path parity with index path; blocklisted names never
  recorded; **re-indexing a file whose call changed clears the old staged row (no stale carry-over)**;
  **deleting a file clears its staged rows**.
- **Depends on:** 082.001-T.

### 082.003-T — Unambiguous post-pass resolution + `calls_resolved_singleton` provenance  *(domain: core indexing/post-pass)*
- **Files:** `src/db/cozo_backend/schema.rs` + `src/models/code_edge.rs` (**provenance storage**,
  see below), `src/db/cozo_queries.rs` (new `reresolve_calls_edges` modeled on
  `reresolve_references_edges:1357`; resolve each staged callee against a workspace-global
  `name → [function_id]` index, create an edge **only when exactly one** match, tag provenance
  `calls_resolved_singleton`), `src/services/code_graph.rs` (invoke the post-pass in the
  **full/`--force` index** path only — alongside `reresolve_references_edges` at `:543` — and
  **NOT** in the incremental sync path at `:1152`). Test:
  `tests/integration/calls_postpass_resolution_test.rs` (new).
- **Provenance storage (prerequisite for the tag AND for 082.004-T edge enumeration):** today
  `calls_edge` stores only `(from, to) => created_at` (`src/db/cozo_backend/schema.rs:319`) and
  `CodeEdge` has no provenance field (`src/models/code_edge.rs:34`). Extend the `calls_edge`
  schema with a `resolution: String` attribute (migration: pre-existing rows default to
  `"direct"`) and add `CodeEdge.resolution: Option<String>`. Direct in-file edges (082.002-T)
  write `resolution = "direct"`; post-pass singleton edges write `resolution =
  "resolved_singleton"`. Add a read query (e.g. `count_calls_edges_by_resolution` and an
  enumerate-`resolved_singleton`-edges query) so 082.004-T can list the tagged edges it validates.
  Cover with schema-migration, `CodeEdge` model round-trip, and query tests.
- **Behavior:** unique cross-file name (e.g. `get_health_report_for_daemon`) → one tagged edge;
  ambiguous name (≥2 defs) → skipped (bounds false edges); non-existent def → no edge;
  incremental sync does not run the global post-pass (performance gate).
- **Integration scenarios (4):** unique cross-file name → edge + `calls_resolved_singleton` tag;
  ambiguous name (2 defs) → skipped; name with no def → no edge (no false edge); incremental
  sync path → post-pass skipped.
- **Depends on:** 082.002-T.

### 082.004-T — Acceptance verification via 081-F eval subsystem  *(domain: verification/eval)*
- **Files:** `tests/integration/calls_recall_acceptance_test.rs` (new — index a fixture
  before/after, run the 081.005-T graph resolution-recall + false-edge-rate metric, assert
  recall **rises** and false-edge rate stays within the operator threshold) plus a hand-authored
  **expected-edges manifest** for the fixture (ground-truth `caller → callee` id pairs). Small doc
  note in the plan/changelog is Ship-side.
- **Behavior:** this is the empirical gate that closes the deliberation's acceptance criterion.
  It consumes the eval subsystem's graph metric (081.005-T) and the report contract/model
  (081.001-T). **Metric caveat:** the 081-F `false_edge_rate` (`count_dangling_calls_edges`,
  `src/db/cozo_queries.rs:2409`) is a conservative lower bound — it detects *dangling* callees
  only, not calls resolved to an existing-but-wrong function, which is exactly the false edge
  rec1's unambiguous-name guard could introduce (tracked follow-up `D07F0919`). The aggregate
  metric therefore CANNOT gate mis-resolution on its own, so this task adds a **fixture
  ground-truth target-correctness assertion**: every `calls_resolved_singleton` edge produced for
  the fixture MUST match the expected-edges manifest (correct callee id), detecting wrong-target
  resolution directly and independently of the aggregate metric.
- **Integration scenarios (5):** post-change resolution_recall > pre-change; aggregate
  false_edge_rate ≤ threshold; **every `calls_resolved_singleton` edge matches the expected-edges
  manifest (no wrong-target resolution)**; `calls_resolved_singleton` edges counted; ambiguous
  names contribute no edge (skipped, not mis-resolved).
- **Depends on:** 082.003-T, **081.001-T (S1 contract/model)**, **081.005-T (S3 graph metric)**.

### 082.005-T / 082.006-T / 082.007-T — Fan-out to peer tree-sitter extractors  *(domain: parser/extraction — DEFERRED follow-on, NOT in first shipment)*
Decomposed one task per language (each ≤3 files, single width, all depend on 082.003-T), queued
under 082-F for a follow-on shipment:
- **082.005-T — Python:** apply the `field_expression`/method-capture + record-unresolved +
  post-pass pattern to `src/services/parsing/python.rs`.
- **082.006-T — TypeScript:** same pattern for `src/services/parsing/typescript.rs`.
- **082.007-T — Go:** same pattern for `src/services/parsing/go.rs`.
- **Depends on:** 082.003-T (all three).

**Dependency chain (first shipment 078-S):** 082.001-T → 082.002-T → 082.003-T → 082.004-T;
082.004-T additionally depends on 081.001-T and 081.005-T. The peer-language tasks (082.005-T
Python, 082.006-T TypeScript, 082.007-T Go) are deferred to a follow-on shipment; 078-S ships only
082.001-004-T and does **NOT** archive 082-F, which stays active until the peer-language tasks
complete.

## 6. Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | New/changed code Rust 2024, `Result<T, EngramError>`, no `unwrap`/`expect`, `#![forbid(unsafe_code)]`, clippy pedantic. |
| II. Test-First | Each task pairs impl with a named tier file authored before impl by Ship. |
| III. Workspace Isolation | Post-pass resolves only within the current workspace's symbol index; no cross-workspace reach. |
| IV. CLI Workspace Containment | No new CLI surface; behavior rides existing index/sync commands. |
| V. Destructive Command Approval | None — additive edges + provenance tag; no deletions of existing edge semantics. |
| VI. Safety Modes | Elevated blast radius (core-indexing change affecting every workspace's edge counts) → `careful` + `freeze-scope` (see §7). |
| Task Granularity | Each task ≤3 source files / ≤5 fns / ≤4 scenarios, single width; language fan-out deferred and per-language split. |

## 7. Plan-Harden (risk-triggered)

Trigger: **core-indexing change** that alters graph semantics and edge counts for **every**
indexed workspace; false-edge risk cannot be validated on a single workspace by inspection.

**Safety mode:** `careful` + `freeze-scope` to `src/services/parsing/rust.rs`,
`src/services/code_graph.rs` (Calls resolution + post-pass invocation), `src/db/cozo_queries.rs`
(staging + `reresolve_calls_edges`), and `tests/`. Peer-language extractors are **out of scope**
for the first shipment (082.005-T deferred).

| Risk | Blast radius | Mitigation |
|---|---|---|
| False edges from ambiguous names | High (pollutes graph for all workspaces) | Unambiguous-name-only guard (exactly-one-match); 082.003 scenario asserts ambiguous → skipped, no-def → no edge; false-edge rate empirically bounded by 081.005 metric. |
| Method-call over-capture (common names) | Medium | Keep `CALL_BLOCKLIST`; under-recall of `count/get/new` accepted (better than false edges) per deliberation. |
| Post-pass latency on large (10k+ symbol) workspaces | Medium | Gate global post-pass behind full/`--force` index only; incremental sync skips it (082.003). |
| Index/sync path divergence | Medium | 082.002 asserts sync-path parity for recording; post-pass intentionally index-only (documented + tested). |
| Unmeasured recall/precision tradeoff | High (product decision) | Acceptance defined by 081-F metrics (082.004): recall must rise, false-edge rate ≤ threshold. |
| Provenance tag consumers unaware of new edge kind | Low | `calls_resolved_singleton` distinct tag; consumers/eval can weight; existing direct `calls` edges unchanged. |

**Rollback:** additive. Reverting removes the `field_expression` arm, the unresolved-call
staging, and the `reresolve_calls_edges` post-pass; a subsequent full reindex restores prior edge
counts (no destructive migration).

## 8. Plan-Review (gate)

Multi-persona review (Rust-safety, indexing/graph correctness, performance/ops, test-first,
API/contract), 1 cycle.

| # | Persona | Finding | Severity | Disposition |
|---|---|---|---|---|
| 1 | Indexing/graph | Ambiguous-name resolution could inject false edges across all workspaces | P1 | RESOLVED — exactly-one-match guard; 082.003 scenarios pin ambiguous→skip, no-def→no-edge. |
| 2 | Test-first / product | Recall/precision tradeoff must be empirically validated, not asserted | P1 | RESOLVED — 082.004 gates on 081.005 metrics (recall↑, false-edge ≤ threshold) — hard dependency on S1+S3. |
| 3 | Performance/ops | Global post-pass cost on large workspaces | P1 | RESOLVED — gated to full/`--force` index only; incremental sync skips (082.003 scenario). |
| 4 | Rust-safety | `field_expression` extraction must not regress blocklist or panic on odd trees | P2 | RESOLVED — `CALL_BLOCKLIST` preserved; `resolve_call_name` returns `Option`; 082.001 blocklist + chained scenarios. |
| 5 | API/contract | New edge provenance could confuse existing `calls` consumers | P2 | RESOLVED — distinct `calls_resolved_singleton` tag; direct edges unchanged; documented. |
| 6 | Ops | Language fan-out risks width creep if bundled | P3 | RESOLVED — 082.005 deferred + split per-language at harvest; not in first shipment. |

**Gate outcome: PASS.** No unresolved P0/P1. Two P2 resolved. Proceed to harvest.

## 9. Assumptions & open items

- Acceptance is only meaningful once 081-F S1 (081.001-T) + S3 (081.005-T) exist; 082-F ships
  **after** 081-F. The dependency edges are wired at task level (082.004-T → 081.001-T,
  082.004-T → 081.005-T) and enforced by shipment ordering.
- The operator-chosen false-edge-rate threshold is read from `RetrievalEvalConfig.thresholds`
  (081.001-T); 082.004 asserts against it.
- Peer-language extractors (082.005-T) are a deferred follow-on, decomposed per language later.
