---
title: "Retrieval-Eval Correctness Hardening — exec plan"
type: exec-plan
date: 2026-07-11
status: reviewed
feature: 084-F
deliberation: docs/decisions/2026-07-11-retrieval-eval-correctness-deliberation.md
plan_review: PASS
---

# Exec Plan — Retrieval-Eval Correctness Hardening

**Date:** 2026-07-11
**Status:** Reviewed / Ready for harvest — plan-review **PASS**
**Stage owner:** stage agent
**Deliberation:** `docs/decisions/2026-07-11-retrieval-eval-correctness-deliberation.md`
**Feature (umbrella):** 084-F
**Ships:** as the "Retrieval-Eval Correctness Hardening" release unit. Correctness-gates all
future retrieval/graph improvements (goes before further 082-F fan-out and any search-ranking
work), because those are judged by this ruler.

---

## 1. Objective

Make the retrieval-eval subsystem **trustworthy**: correct `resolution_recall` numerator/
denominator units and language scope, honest index/disk consistency (no silent `[0,1]`
clamp), false-edge **target-correctness** (not just dangling lower bound), thresholds that
actually gate, a canonical TSX language gate, and honest retrieval-mode reporting — plus the
in-scope completeness/perf/memory hardening and the regression tier exercising the real path.

All work is confined to the **retrieval_eval subsystem** (see deliberation "Scope boundary").
The indexer write path (`src/services/code_graph.rs`), the agent-efficiency `evaluation`
surface, and all parser/reliability work are **out of scope**.

## 2. Grounding — verified modules (2026-07-11)

| Concern | Symbol | Location |
|---|---|---|
| numerator (whole-graph, distinct `(from,to)`, no lang gate) | `count_calls_edges` | `src/db/cozo_queries.rs:2755` |
| denominator (disk-parsed, lang-gated, per-occurrence, skips unreadable) | `count_workspace_call_sites` | `src/tools/eval.rs:73` |
| false-edge (dangling-only lower bound) | `count_dangling_calls_edges` | `src/db/cozo_queries.rs:2776` |
| semantic gate (path-ext `tsx`→`typescript`) + full clone+sort per query | `evaluate_semantic`, `language_of` | `src/services/retrieval_eval.rs:192`, `:55` |
| snapshot atomicity (already correct — reference pattern) | `snapshot_parts` | `src/tools/eval.rs:46` |
| report / config models | `RetrievalEvalReport`, `RetrievalEvalConfig` | `src/models/retrieval_eval.rs`, `src/models/config.rs` |

## 3. Decomposition — 14 tasks under 084-F

Each task honors the granularity rule (**≤3 source files, ≤5 functions, ≤4 test scenarios,
single width domain, ≤2h human-equivalent**), width isolation (no code+unrelated-docs mix),
and the atomic-milestone rule. **Every code task starts with a compiling-but-failing test
harness** (Test-First, Ship executes) named in its spec.

| Slice | Intent | Tasks |
|---|---|---|
| A0 | Report/config model surface (all new fields, additive/back-compat) | 084.001-T |
| A  | `resolution_recall` correctness (D6F70DCC, 88B5FAFD, 54848E3D) | 084.002-T, 084.003-T |
| B  | false-edge target-correctness (D07F0919 ⇄ 49561F22) | 084.004-T (+ 2 subtasks) |
| C  | canonical TSX gate (2894ACB5) | 084.005-T |
| D  | thresholds gate at runtime (14B33F9F) | 084.006-T, 084.007-T |
| E  | retrieval-mode fidelity (00C7F3CC) | 084.008-T |
| F  | completeness / perf / memory (78AA205D, 4CF046A5, CA401F5F) | 084.009-T, 084.010-T, 084.011-T |
| G  | regression tier exercises real path (F137D72E) | 084.012-T |
| H  | plan-doc alignment (635EE7C0) | 084.013-T |
| —  | carry-forward merge-gate docs/instructions (3A280A4E) | 084.014-T |

### 084.001-T — Eval report + config model surface  *(width: models/schema)*
- **Files:** `src/models/retrieval_eval.rs` (report/metrics structs), `src/models/config.rs`
  (threshold-surface wiring only). Test: `tests/unit/retrieval_eval_model_test.rs` (new) or
  `tests/contract/retrieval_eval_contract_test.rs` (extend).
- **Adds (all `#[serde(default)]`, no `deny_unknown_fields`, back-compat):** `retrieval_mode`
  / `fallback` enum on semantic metrics (Cluster E); `index_stale` + generation/accounting
  fields on graph metrics (Cluster A3); false-edge target-correctness fields
  (`target_correct` / `target_mismatch` counters, Cluster B); threshold-evaluation result
  fields (`thresholds_breached`, per-metric outcome, Cluster D).
- **Harness-first:** contract/unit test pins the JSON shape + asserts an absent section
  deserializes to defaults **before** field additions.
- **Verification (≤4):** (1) new fields serialize with documented names; (2) legacy report
  JSON (no new fields) round-trips to defaults; (3) legacy `.engram/config.toml` without new
  keys → defaults, no error; (4) `retrieval_eval` and agent-efficiency `evaluation` configs
  still coexist.
- **Depends on:** none (foundation).

### 084.002-T — `resolution_recall` unit + language-scope consistency  *(width: eval graph read-path)*
- **Source:** D6F70DCC + 88B5FAFD.
- **Files:** `src/db/cozo_queries.rs` (`count_calls_edges`: gate numerator to
  `config.languages` via `calls_edge → function_meta → file.language`), `src/tools/eval.rs`
  (`count_workspace_call_sites`: dedupe denominator to distinct `(caller,callee)` units).
  Test: `tests/integration/retrieval_eval_graph_test.rs` (extend).
- **Adds:** both sides count the **same unit** (distinct `(caller,callee)` relations) under the
  **same language gate**.
- **Harness-first:** integration scenario asserting the double-call / multi-language cases
  fails against current code before the fix.
- **Verification (≤4):** (1) same callee called twice → recall **not** deflated (unit parity);
  (2) multi-language workspace beyond `config.languages` → non-configured edges excluded from
  numerator, recall reflects per-language; (3) all-resolved single-language → recall ≈ 1.0;
  (4) empty/disabled → zero report, no panic.
- **Depends on:** 084.001-T.

### 084.003-T — `resolution_recall` index/generation consistency gate  *(width: eval graph read-path)*
- **Source:** 54848E3D.
- **Files:** `src/tools/eval.rs` (`count_workspace_call_sites`: compare inventory source to the
  indexed generation; account — don't silently drop — unreadable indexed files),
  `src/services/retrieval_eval.rs` (populate `index_stale` from the model). Test:
  `tests/integration/retrieval_eval_graph_test.rs` (extend).
- **Adds:** honest staleness signal **instead of** silent `[0,1]` clamp when working tree has
  drifted from the indexed revision or indexed files are unreadable.
- **Note (deferred alt):** persisting the call-site inventory at index generation is the
  heavier fix but touches the indexer write path (out of 084-F scope) — see deliberation
  "deferred heavier alternative". This task delivers the eval-scoped generation gate only.
- **Harness-first:** scenario mutating a file after index → run flags `index_stale`.
- **Verification (≤4):** (1) clean index → `index_stale=false`, recall computed; (2) file
  edited after index → `index_stale=true`, no silent clamp; (3) unreadable indexed file →
  accounted (not silently dropped from denominator); (4) empty → zero report.
- **Depends on:** 084.001-T, 084.002-T (same denominator function).

### 084.004-T — false-edge **target-correctness** (fixture manifest)  *(width: eval graph read-path + fixture)*
- **Source:** **D07F0919 ⇄ 49561F22 (consolidated; 49561F22 superseded/archived).**
- **Files:** `src/db/cozo_queries.rs` (retain resolved-callee provenance read for
  `calls_resolved_singleton`), `src/services/retrieval_eval.rs` (assert resolved target vs
  expected-target manifest by exact identity; keep `count_dangling_calls_edges` as the labeled
  lower-bound aggregate). Test + fixture under `tests/`.
- **Acceptance nuance (from `2026-07-08-callgraph-cross-file-resolution-deliberation.md:25-33`):**
  `false_edge_rate` (dangling-only) is a **lower bound**; correctness requires **manifest
  target assertions by exact identity**, not mere existence. Production-scale sampling of all
  resolved singletons beyond the fixture (unbounded part of 49561F22) is the follow-on, not
  this shipment.
- **Subtasks:**
  - **084.004.001-ST** — build the fixture workspace + **expected-target manifest** (ground
    truth: caller → expected callee identity, incl. an intentional ambiguous/wrong-but-existing
    case). *(width: test fixture)*
  - **084.004.002-ST** — provenance retention + manifest target assertion path + labeled
    lower-bound aggregate. *(width: eval graph read-path)* Depends on 084.004.001-ST.
- **Verification (≤4):** (1) all singletons match manifest → `target_mismatch=0`; (2) a
  wrong-but-existing target → `target_mismatch>0` even though dangling rate stays ~0 (proves
  the lower-bound gap is now covered); (3) a dangling target → still counted by the labeled
  aggregate; (4) empty graph → zero report.
- **Depends on:** 084.001-T.

### 084.005-T — canonical TSX language gate  *(width: eval semantic gate)*
- **Source:** 2894ACB5.
- **Files:** `src/services/retrieval_eval.rs` (`language_of`/gate returns canonical `tsx`, so
  both semantic and graph paths share one gate). Test: `tests/unit/retrieval_eval_...` (unit).
- **Harness-first:** unit asserting `languages=['tsx']` includes TSX functions in semantic eval.
- **Verification (≤3):** (1) `languages=['tsx']` → TSX functions included; (2)
  `languages=['typescript']` → `.ts` included, `.tsx` gated by canonical id (documented
  behavior); (3) empty `languages` → all gated in (opt-in unchanged).
- **Depends on:** none (isolated one-function change; no new model fields).

### 084.006-T — thresholds enforced in `run_retrieval_eval` + report  *(width: eval service)*
- **Source:** 14B33F9F (service half).
- **Files:** `src/services/retrieval_eval.rs` (`run_retrieval_eval` consults
  `parts.config.thresholds`, calls `check_thresholds`, records outcome in report). Test:
  `tests/integration/retrieval_eval_thresholds_test.rs` (new).
- **Harness-first:** integration scenario where a breached threshold currently still passes.
- **Verification (≤4):** (1) threshold met → `thresholds_breached=false`; (2) threshold
  breached → recorded in report; (3) no thresholds configured → no gating, back-compat;
  (4) disabled/empty run → exits the contract unchanged (no false breach).
- **Depends on:** 084.001-T.

### 084.007-T — thresholds CLI exit-code surfacing  *(width: CLI)*
- **Source:** 14B33F9F (CLI half).
- **Files:** `src/cli/commands/eval.rs` (map `thresholds_breached` → non-zero exit, mirroring
  `engram verify`). Test: `tests/integration/eval_cli_...` (extend/new).
- **Harness-first:** CLI test asserting breach → non-zero exit, empty run → exit 0.
- **Verification (≤3):** (1) breach → non-zero exit; (2) pass → exit 0; (3) disabled/empty
  run → exit 0 (contract preserved).
- **Depends on:** 084.006-T.

### 084.008-T — retrieval-mode fidelity / fallback detection  *(width: eval service/semantic)*
- **Source:** 00C7F3CC.
- **Files:** `src/services/retrieval_eval.rs` (`evaluate_semantic`: detect + record hybrid vs
  silent keyword-only fallback; either propagate embedding failure for eval runs or set the
  `retrieval_mode`/`fallback` field). Test: `tests/integration/retrieval_eval_semantic_test.rs`
  (extend).
- **Harness-first:** scenario forcing embedding unavailability → run records `keyword_only`.
- **Verification (≤4):** (1) embeddings available → `retrieval_mode=hybrid`; (2) embedding
  path unavailable → `retrieval_mode=keyword_only` recorded (not masked); (3) reports across
  modes are distinguishable; (4) empty → zero report.
- **Depends on:** 084.001-T.

### 084.009-T — semantic corpus completeness (LEFT JOIN + name fallback)  *(width: eval semantic corpus)*
- **Source:** 78AA205D.
- **Files:** `src/services/retrieval_eval.rs` and/or the `all_functions` read in
  `src/db/cozo_queries.rs` (INNER JOIN → LEFT JOIN + name-fallback query derivation so
  docstring-less symbols are in the denominator). Test: semantic integration (extend).
- **Verification (≤3):** (1) docstring-less function is in the corpus denominator; (2)
  name-fallback query derives for it; (3) counts unchanged for docstring-bearing symbols.
- **Depends on:** 084.001-T.

### 084.010-T — semantic eval top-k selection (perf)  *(width: eval semantic)*
- **Source:** 4CF046A5.
- **Files:** `src/services/retrieval_eval.rs` (`evaluate_semantic`: bounded top-k
  heap/selection instead of full clone+sort per query). Test: unit/bench-style assertion of
  identical ranks with bounded work.
- **Verification (≤3):** (1) ranks identical to prior implementation on a fixture; (2) work
  bounded by k (no full corpus clone+sort); (3) empty → zero report.
- **Depends on:** 084.009-T (same function region; serialize to avoid conflict).

### 084.011-T — graph eval bounded/incremental parsing (memory)  *(width: eval graph read-path)*
- **Source:** CA401F5F.
- **Files:** `src/tools/eval.rs` (`count_workspace_call_sites`: parse in bounded batches
  instead of accumulating every source into memory). Test: integration asserting counts
  unchanged.
- **Verification (≤3):** (1) call-site count unchanged vs prior; (2) peak memory bounded
  (batched); (3) empty → zero.
- **Depends on:** 084.003-T (same function; serialize).

### 084.012-T — graph regression tier exercises the real path  *(width: tests)*
- **Source:** F137D72E.
- **Files:** `tests/integration/retrieval_eval_regression_test.rs` (build/index a fixture
  workspace and assert graph metrics from the **real** `run_retrieval_eval` path, not
  injected `compute_graph_metrics(10,9,0)`).
- **Verification (≤4):** (1) real-path recall matches expected on fixture; (2) real-path
  target-correctness asserted; (3) a seeded regression in count path fails the tier; (4)
  baseline stays green only when the real path is correct.
- **Depends on:** 084.002-T, 084.003-T, 084.004-T (asserts the corrected real path).

### 084.013-T — align eval plan doc with narrowed + corrected contract  *(width: docs)*
- **Source:** 635EE7C0.
- **Files:** `docs/exec-plans/2026-07-10-engram-retrieval-eval-subsystem-plan.md` (≈`:157`,
  `:171`): functions-only + bare-name fallback (as delivered); dangling-only lower bound
  **plus** the 084-F fixture target-correctness addition.
- **Verification:** doc references match delivered + 084-F behavior; no unresolved `{{...}}`;
  cross-references valid.
- **Depends on:** 084.004-T, 084.009-T (documents the corrected contract).

### 084.014-T — carry-forward: merge-gate compound learning + instruction rule  *(width: docs/instructions)*
- **Source:** 3A280A4E (operator carry-forward).
- **Files:** create `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`
  from the preserved draft
  (`.copilot/session-state/e571e150-14e3-4ff8-b6a0-6290b8c3c0c4/files/2026-07-11-merge-gate-compound-learning-draft.md`);
  append the `commit_id == current HEAD` review-completion rule to
  `.github/instructions/github-pr-automation.instructions.md` §1.2 (Completion signal) and
  reference it from the pr-lifecycle merge step.
- **Rule encoded:** before merge, a Copilot review whose `commit_id == current HEAD` must have
  completed **AND** Copilot removed from `requested_reviewers` **AND** 0 unresolved threads
  **AND** `mergeable_state == clean`; re-check after **every** push.
- **Verification:** compound doc present + renamed; §1.2 amended; pr-lifecycle merge step
  references it; no unresolved `{{...}}`; markdown structure valid.
- **Depends on:** none (docs/instructions only; kept separate to preserve width isolation on
  the code tasks).

## 4. Dependency graph

```
084.001-T (models)
├── 084.002-T ── 084.003-T ── 084.011-T
│        └────────────┴──────────────── 084.012-T
├── 084.004-T (└ 084.004.001-ST → 084.004.002-ST) ── 084.012-T, 084.013-T
├── 084.006-T ── 084.007-T
├── 084.008-T
└── 084.009-T ── 084.010-T
                 └────────── 084.013-T
084.005-T  (independent)
084.014-T  (independent, docs/instructions carry-forward)
```

## 5. Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | All new code Rust 2024, `Result<T, EngramError>` via `?`, no `unwrap`/`expect`, `#![forbid(unsafe_code)]` inherited, clippy `-D warnings -D pedantic`. New report/config fields derive serde like existing peers. |
| II. Test-First | Every code task names a unit/contract/integration harness authored **before** impl (compiling-but-failing). Model task pins JSON shape first. |
| III. Workspace Isolation | Eval reads/writes stay under the workspace; `count_workspace_call_sites` keeps its canonical-root containment check; no traversal introduced. |
| IV. CLI Workspace Containment | `engram eval` remains a read/compute + in-workspace-write command; exit-code change adds no out-of-workspace writes. |
| V. Destructive Command Approval | None — all changes additive (new fields, corrected queries, new tests/fixtures). |
| VI. Safety Modes | Elevated blast radius (metric-semantics change consumed by 082-F acceptance + threshold gating that affects CLI exit code) → `careful` + `freeze-scope` (see §6). |
| Task Granularity | Each task ≤3 source files / ≤5 fns / ≤4 scenarios, single width domain; thresholds split service/CLI; model surface isolated first. |

## 6. Plan-Harden (risk-triggered)

**Triggers:** (a) changes the **meaning** of `resolution_recall` (the acceptance ruler for
082-F); (b) thresholds become a **gate** that can flip CI/CLI exit codes; (c) new committed
report/config fields consumed by autoharness.

**Safety mode:** `careful` + `freeze-scope` to `src/models/retrieval_eval.rs`,
`src/models/config.rs` (thresholds field only), `src/services/retrieval_eval.rs`,
`src/tools/eval.rs`, the eval **read** queries in `src/db/cozo_queries.rs`
(`count_calls_edges`, `count_dangling_calls_edges`), `src/cli/commands/eval.rs`, and `tests/`.
**Explicitly do not** touch `src/services/code_graph.rs` (indexer write path),
`src/services/evaluation.rs` (agent-efficiency), or any parser module.

| Risk | Blast radius | Mitigation |
|---|---|---|
| Recall unit change misread by 082-F acceptance | High (mis-judges a core graph change) | Unit documented in report/module docs; 084.002 scenarios pin double-call + multi-language; 084.012 asserts real-path recall on fixture. |
| `index_stale` gate hard-fails opt-in eval | Medium | Emit-with-flag, non-fatal; empty/disabled run contract preserved (084.003 scenario 1/4). |
| Threshold exit code breaks disabled/empty contract | High (breaks harness consumer) | 084.007 pins disabled/empty → exit 0; breach → non-zero only when thresholds configured. |
| New serde fields break old `.engram/config.toml`/reports | Medium | `#[serde(default)]`, no `deny_unknown_fields`; 084.001 round-trips legacy JSON/TOML. |
| Same-function edits across tasks conflict | Low | Dependencies serialize `count_workspace_call_sites` (002→003→011) and `evaluate_semantic` (009→010, 005/008 isolated). |
| Fixture target-correctness flaky | Medium | Deterministic fixture + exact-identity assertion; ambiguous case explicitly seeded (084.004.001-ST). |

**Rollback:** every task is additive (new fields, corrected read queries, new tests/fixtures,
one CLI exit-code branch, two docs files). Reverting the feature branch removes them with no
schema migration to unwind (config/report sections are optional/defaulted).

## 7. Plan-Review (gate) — multi-persona

Personas: **Rust-safety**, **API/contract & serde back-compat**, **indexing/graph-metric
correctness**, **test-first / regression**, **blast-radius/ops & scope-freeze**, **docs/traceability**.
Cycle 1. Findings and resolutions:

| # | Persona | Severity | Finding | Resolution |
|---|---|---|---|---|
| F1 | Graph-metric correctness | Advisory | Deduping the denominator changes the metric's *meaning* (call-site → call-relation recall); consumers could misread it. | Resolved — meaning documented in report/module docs (084.001/084.002); deliberation open-question #1 records the choice. |
| F2 | Blast-radius/scope | Advisory | 54848E3D's "canonical" fix (persist inventory at index time) touches the indexer write path, outside the freeze. | Resolved — 084.003 delivers the eval-scoped generation gate; heavier alt explicitly deferred and noted on the task + deliberation. |
| F3 | API/contract | Advisory | Threshold exit-code change risks the disabled-by-default / empty-run contract used by autoharness. | Resolved — 084.007 verification pins disabled/empty → exit 0; breach non-zero only when thresholds configured. |
| F4 | Test-first / regression | Advisory | False-edge target-correctness (084.004) is meaningless unless the regression tier drives the real path. | Resolved — 084.012 depends on 084.002/003/004 and asserts real-path target-correctness; injected-metric anti-pattern removed. |
| F5 | Docs/traceability | Advisory | Dedup of D07F0919/49561F22 must not silently orphan the callgraph plan's reference. | Resolved — supersede recorded in deliberation dedup table; 084.004 body cites both IDs. |

**Gate result: PASS** — 0 blocking, 0 critical, 5 advisory (all resolved within cycle 1).
Proceed to harvest.

## 8. Definition of Ready for Ship

- 14 tasks (+2 subtasks) created under 084-F with dependencies wired.
- One `queued` shipment covering 084-F and all children (incl. 084.014-T carry-forward).
- 49561F22 superseded/archived; D07F0919 retained as canonical false-edge source.
- 30CE5DD6 and 2C420C96 left active in stash (out of eval-correctness scope).
