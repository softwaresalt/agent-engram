# Exec Plan — rec1-calledges: cross-file & method-call resolution

**Date:** 2026-07-10
**Status:** Decided — ⚠ RE-HARVEST REQUIRED before 078-S executes (Constitution Check §6 flags over-limit tasks; tracked by stash CC5D369E). Not executable as currently decomposed; 078-S is `blocked`.
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
rise AND every `calls_resolved_singleton` edge must match the fixture expected-edges manifest
(target-correctness). The aggregate false-edge rate staying within the operator threshold is a
supporting lower-bound signal only (see §§ on the metric caveat and 082.004), not the sole gate.
Mis-resolution detection beyond the fixture (calls resolved to a wrong-but-existing function, which
`false_edge_rate` cannot catch) is captured as follow-up stash `49561F22` (formerly tracked as
`D07F0919`).

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
  write `resolution = "direct"`; post-pass singleton edges write the **exact** value
  `resolution = "calls_resolved_singleton"` (one canonical string, matching the tag used by
  082.004-T's checks — never a shortened `resolved_singleton`). Add a read query (e.g.
  `count_calls_edges_by_resolution` and an enumerate-`calls_resolved_singleton`-edges query) so
  082.004-T can list the tagged edges it validates.
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
  rec1's unambiguous-name guard could introduce (tracked follow-up `D07F0919`, captured as stash
  `49561F22`). The aggregate
  metric therefore CANNOT gate mis-resolution on its own, so this task adds a **fixture
  ground-truth target-correctness assertion**: every `calls_resolved_singleton` edge produced for
  the fixture MUST match the expected-edges manifest (correct callee id), detecting wrong-target
  resolution directly and independently of the aggregate metric.
- **Integration scenarios (5):** post-change resolution_recall > pre-change; aggregate
  false_edge_rate ≤ threshold; **every `calls_resolved_singleton` edge matches the expected-edges
  manifest (no wrong-target resolution)**; `calls_resolved_singleton` edges counted; ambiguous
  names contribute no edge (skipped, not mis-resolved).
- **Depends on:** 082.003-T, **081.001-T (S1 contract/model)**, **081.005-T (S3 graph metric)**.

### 082.005-T / 082.006-T / 082.007-T — Fan-out to peer tree-sitter extractors  *(domain: parser/extraction — DEFERRED follow-on, NOT in first shipment; RE-HARVEST REQUIRED)*
Queued under 082-F for a follow-on shipment, but **NOT yet executable as a single method-arm
pattern-application**. CORRECTION (post-078-S verification): only `src/services/parsing/rust.rs`
currently emits `ExtractedEdge::Calls`; `python.rs`, `typescript.rs`, and `go_lang.rs` emit **NO
`ExtractedEdge::Calls` at all** today. There is therefore no in-file call-edge baseline to extend
with method/receiver capture — each peer language first needs the whole extraction stack built
from scratch. Each task must be **RE-HARVESTED** (re-scoped, likely split) to cover, at minimum:
1. **baseline call extraction** — walk the language's call/invocation nodes and emit
   `ExtractedEdge::Calls { caller, callee }` (the equivalent of rust.rs `extract_calls_from_body`);
2. **caller attribution** — associate each call site with its enclosing function/method id;
3. **language-specific member/selector/call node handling** — Python `attribute`/`call`,
   TypeScript `member_expression`/`call_expression`, Go `selector_expression`/`call_expression`
   (each grammar names these differently from Rust's `call_expression`/`field_expression`);
4. **method/receiver capture + record-unresolved + post-pass** wiring (the rec1 pattern) layered
   on top of 1–3;
5. **tests** per tier for each language.
These are NOT "≤3 files, single width, same pattern" — that earlier framing was incorrect. A Stage
re-harvest MUST re-scope (and probably split) 082.005-T / 082.006-T / 082.007-T before any of them
is routed to Ship. They are marked `blocked` in the queue pending that re-harvest.
- **082.005-T — Python:** build the call-extraction stack (1–5 above) for
  `src/services/parsing/python.rs` (currently emits no `Calls`).
- **082.006-T — TypeScript:** build the call-extraction stack for
  `src/services/parsing/typescript.rs` (currently emits no `Calls`).
- **082.007-T — Go:** build the call-extraction stack for
  `src/services/parsing/go_lang.rs` (currently emits no `Calls`).
- **Depends on:** 082.003-T (all three) **plus a Stage re-harvest** re-scoping each task.

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
| Task Granularity | ⚠ **Three units exceed limits and MUST be split/reduced by a Stage re-harvest before 078-S executes:** 082.002-T defines 6 scenarios (>4), 082.003-T touches 4 source files (>3, after the provenance-storage addition), and 082.004-T defines 5 integration scenarios (>4, see lines 157–160). The remaining units comply (≤3 files / ≤4 scenarios, single width). Language fan-out is deferred and per-language split (082.005/006/007-T). |

> **Harvest follow-up (Copilot review #239):** before 078-S is routed to Ship, a Stage re-harvest
> MUST (a) split 082.002-T into staging *capture* vs *lifecycle* (clear-before-reindex + deletion +
> stale-resolved-edge retraction), 082.003-T into provenance *storage* vs *post-pass resolution*,
> and reduce/split 082.004-T (5 scenarios → ≤4) so every executable task is single-width and
> ≤4 scenarios, (b) propagate the full `staged_call` lifecycle + `resolution` migration + read-query
> contract into the executable queue task items (not just this plan), and (c) carry the call-edge
> retraction design from §7. Tracked as stash follow-up CC5D369E. 078-S is `blocked` until done.

## 7. Plan-Harden (risk-triggered)

Trigger: **core-indexing change** that alters graph semantics and edge counts for **every**
indexed workspace; false-edge risk cannot be validated on a single workspace by inspection.

**Safety mode:** `careful` + `freeze-scope` to `src/services/parsing/rust.rs`,
`src/services/code_graph.rs` (Calls resolution + post-pass invocation), `src/db/cozo_queries.rs`
(staging + `reresolve_calls_edges`), `src/db/cozo_backend/schema.rs` (`staged_call` relation +
`calls_edge.resolution` migration), `src/models/code_edge.rs` (`resolution` field), and `tests/`.
Peer-language extractors (082.005-T Python, 082.006-T TypeScript, 082.007-T Go) are **out of scope**
for the first shipment.

**Stale-edge retraction (Copilot review #239):** clearing `staged_call` rows alone does not retract
a `calls_edge` that a prior singleton post-pass created. Reindex/deletion paths
(`src/services/code_graph.rs:293-300`, `:1196-1204`) delete function metadata but do NOT clear call
edges, so a changed/deleted caller or callee would leave the old resolved edge dangling. The staging
*lifecycle* task (see §6 split follow-up) MUST retract a file's prior `calls_resolved_singleton`
edges while the old symbol IDs still exist, before re-staging, and test changed/deleted callers and
callees after a post-pass.

| Risk | Blast radius | Mitigation |
|---|---|---|
| False edges from ambiguous names | High (pollutes graph for all workspaces) | Unambiguous-name-only guard (exactly-one-match); 082.003 scenario asserts ambiguous → skipped, no-def → no edge. **Primary detection is the 082.004 fixture ground-truth target-correctness assertion** (every `calls_resolved_singleton` edge must match the expected-edges manifest); the aggregate 081.005 `false_edge_rate` is only a conservative lower-bound signal (dangling targets, not wrong-but-existing targets — follow-up `D07F0919`). |
| Method-call over-capture (common names) | Medium | Keep `CALL_BLOCKLIST`; under-recall of `count/get/new` accepted (better than false edges) per deliberation. |
| Post-pass latency on large (10k+ symbol) workspaces | Medium | Gate global post-pass behind full/`--force` index only; incremental sync skips it (082.003). |
| Index/sync path divergence | Medium | 082.002 asserts sync-path parity for recording; post-pass intentionally index-only (documented + tested). |
| Unmeasured recall/precision tradeoff | High (product decision) | Acceptance defined by 082.004: resolution recall must rise AND every `calls_resolved_singleton` edge must match the fixture expected-edges manifest (target-correctness). The aggregate 081.005 `false_edge_rate ≤ threshold` is a supporting lower-bound signal only, not the sole gate. |
| Provenance tag consumers unaware of new edge kind | Low | `calls_resolved_singleton` distinct tag; consumers/eval can weight; existing direct `calls` edges unchanged. |

**Rollback:** NOT purely additive once the persisted `calls_edge.resolution` provenance field
lands. Reverting the code removes the `field_expression` arm, the unresolved-call staging, and the
`reresolve_calls_edges` post-pass, but the current reindex paths (`src/services/code_graph.rs:293-300`,
`:1181-1204`) do NOT clear `calls_edge` rows, so a full reindex alone leaves prior
`calls_resolved_singleton` edges behind and the reverted (old-schema) writer may be incompatible
with the migrated relation. The Stage re-harvest (stash CC5D369E) MUST define and test an explicit
pre-revert cleanup / down-migration: retract `calls_resolved_singleton` edges and drop/ignore the
`resolution` column before or during the reverting reindex. Until that down-migration exists, treat
rollback as requiring a manual edge cleanup step, not a plain reindex.

## 8. Plan-Review (gate)

Multi-persona review (Rust-safety, indexing/graph correctness, performance/ops, test-first,
API/contract), 1 cycle.

| # | Persona | Finding | Severity | Disposition |
|---|---|---|---|---|
| 1 | Indexing/graph | Ambiguous-name resolution could inject false edges across all workspaces | P1 | RESOLVED — exactly-one-match guard; 082.003 scenarios pin ambiguous→skip, no-def→no-edge. |
| 2 | Test-first / product | Recall/precision tradeoff must be empirically validated, not asserted | P1 | RESOLVED — 082.004 gates on recall↑ AND the fixture expected-edges manifest target-correctness check (every `calls_resolved_singleton` edge matches the correct callee). The aggregate 081.005 `false_edge_rate ≤ threshold` is a lower-bound signal only — it detects dangling targets, NOT wrong-but-existing targets (lines 149–156; follow-up D07F0919) — so it cannot close this finding alone. Hard dependency on S1+S3. |
| 3 | Performance/ops | Global post-pass cost on large workspaces | P1 | RESOLVED — gated to full/`--force` index only; incremental sync skips (082.003 scenario). |
| 4 | Rust-safety | `field_expression` extraction must not regress blocklist or panic on odd trees | P2 | RESOLVED — `CALL_BLOCKLIST` preserved; `resolve_call_name` returns `Option`; 082.001 blocklist + chained scenarios. |
| 5 | API/contract | New edge provenance could confuse existing `calls` consumers | P2 | RESOLVED — distinct `calls_resolved_singleton` tag; direct edges unchanged; documented. |
| 6 | Ops | Language fan-out risks width creep if bundled | P3 | RESOLVED — 082.005 deferred + split per-language at harvest; not in first shipment. |

**Gate outcome (original review): PASS** — no unresolved P0/P1 at first review.

> **SUPERSEDED by Copilot review #239 (2026-07-11):** this plan is NO LONGER "proceed to harvest"
> as-is. A subsequent Stage re-harvest is REQUIRED before 078-S executes (see the **Status** line
> and Constitution Check §6): three units exceed granularity limits (082.002-T, 082.003-T,
> 082.004-T), the executable task bodies must carry the full `staged_call` lifecycle + `resolution`
> migration + read-query contract, and the rollback needs an explicit down-migration (§7). 078-S is
> `blocked` until the re-harvest rewrites the manifest. Tracked by stash CC5D369E.

## 9. Assumptions & open items

- Acceptance is only meaningful once 081-F S1 (081.001-T) + S3 (081.005-T) exist; 082-F ships
  **after** 081-F. The dependency edges are wired at task level (082.004-T → 081.001-T,
  082.004-T → 081.005-T) and enforced by shipment ordering.
- The operator-chosen false-edge-rate threshold is read from `RetrievalEvalConfig.thresholds`
  (081.001-T); 082.004 asserts against it.
- Peer-language extractors are queued as 082.005-T (Python), 082.006-T (TypeScript), and
  082.007-T (Go) under 082-F, but each is **`blocked` pending a Stage re-harvest**. Post-078-S
  verification found the earlier "same pattern, ≤3 files, single width" framing incorrect: only
  `rust.rs` emits `ExtractedEdge::Calls` today, so `python.rs` / `typescript.rs` / `go_lang.rs`
  each need the full call-extraction stack (baseline extraction + caller attribution +
  language-specific member/selector/call node handling + tests) built before the rec1
  method/receiver + post-pass pattern applies. These tasks therefore require **re-harvest and
  likely a further split** — they are NOT ready to route to Ship as-is (see §5, tasks
  082.005/006/007-T).
