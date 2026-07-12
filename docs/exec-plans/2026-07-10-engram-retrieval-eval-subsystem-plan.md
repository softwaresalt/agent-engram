# Exec Plan — Portable retrieval + graph-recall eval subsystem

**Date:** 2026-07-10
**Status:** Decided / Ready for harvest
**Stage owner:** stage agent
**Deliberation:** `docs/decisions/2026-07-10-engram-retrieval-eval-subsystem-deliberation.md` (signed off 2026-07-10)
**Feature (umbrella):** 081-F
**Ships:** FIRST — before rec1-calledges (082-F), which is acceptance-gated by this subsystem's S1+S3.

---

## 1. Objective

Ship engram's **portable, in-product retrieval + graph-recall evaluation subsystem**: a
measurement substrate that runs inside *any* indexed workspace (Go/Python/Rust/…), derives
ground truth automatically (zero manual labeling), and emits structured JSON that autoharness
can consume for tuning. It is the empirical gate for `rec1-calledges` (082-F): its graph
resolution-recall + false-edge-rate metrics decide whether the call-edge recall change is a net
win.

Distinct from the existing **agent-efficiency** evaluation (`src/services/evaluation.rs`,
`EvaluationConfig`, `get_evaluation_report`). Naming is `retrieval_eval` **everywhere**; the
agent-efficiency `evaluation` surface is left untouched.

## 2. Scope Decision — 4 dependency-linked slices

The deliberation fixes four slices. Each slice is decomposed into tasks honoring the granularity
rule (≤3 source files, ≤5 functions, ≤4 test scenarios, **single width domain**). Slice 1 spans
three distinct width domains (models/schema, MCP/tools, CLI) so it is split into three tasks to
preserve width isolation; slices 2 and 3 are one compute task each; slice 4 is split into
integration/persistence and regression/graduation.

| Slice | Intent | Tasks |
|---|---|---|
| S1 | Config + report model + JSON contract + empty-state CLI/MCP wiring | 081.001-T, 081.002-T, 081.003-T |
| S2 | Semantic self-retrieval eval (precision@k/recall@k/MRR/nDCG) | 081.004-T |
| S3 | Graph resolution-recall eval (resolution recall + false-edge rate) | 081.005-T |
| S4 | Autoharness integration + feedback persistence + regression tier | 081.006-T, 081.007-T |

**Acceptance-gate export:** 082-F (rec1-calledges) depends on **S1 (081.001-T contract/model)**
and **S3 (081.005-T graph resolution-recall)** — those are the surfaces its acceptance test
consumes.

## 3. Grounding — real modules (verified 2026-07-10)

Config / model pattern (section-per-subsystem, each `#[serde(default)]`):
- `src/models/config.rs:14-56` — `WorkspaceConfig` composes `batch`/`code_graph`/`metrics`/
  `policy`/`evaluation`, each a `#[serde(default)]` field with a matching `Default` impl.
  Sibling config structs live in their own modules and are `use`d in (`use crate::models::
  evaluation::EvaluationConfig` at `config.rs:9`). A `[retrieval_eval]` section is a clean
  additive extension: a new `src/models/retrieval_eval.rs` module + one `#[serde(default)]`
  field on `WorkspaceConfig`.
- `.engram/config.toml` is committed (`.gitignore` excepts `!.engram/config.toml`); unknown
  fields are silently ignored (`config.rs:177-181`) so old configs stay valid.

Collision to avoid:
- `src/services/evaluation.rs:1-21` — `evaluate(events, &EvaluationConfig) -> EvaluationReport`
  measures **agent efficiency** (token ratios, error bursts, tool hammering). MUST NOT be
  reused/renamed. New subsystem uses `retrieval_eval` naming and its own service module.

Delivery / contract precedent (thin CLI wrapper over MCP tool):
- `src/cli/commands/report.rs:47-50` — `run_eval` = `run_tool("get_evaluation_report", None,
  flags, formatter)`. `engram eval` follows this exact shape.
- `src/bin/engram.rs:24` (`enum Command`, `#[derive(Subcommand)]`), `:118-125` (`Verify`
  variant precedent), `:259-266` (`Report` subcommand). Add an `Eval` variant + dispatch arm.
- MCP registration: `src/shim/tools_catalog.rs:284` (`Tool::new("get_evaluation_report", …)`
  schema entry) and dispatch in `src/tools/mod.rs:304-325` (`"get_evaluation_report" =>
  read::get_evaluation_report(...)`). `should_record_metrics` allow-list at `tools/mod.rs:34-55`.

Semantic ground truth + metrics:
- `src/services/search.rs:254` `hybrid_search(query, candidates, limit)` and unified search
  types (`UnifiedSearchResult` at `search.rs:86`, `merge_unified_results` at `:165`) are the
  query engine. Symbol docstrings/qualified names are the auto-derived known-item queries
  (`ExtractedFunction.docstring` produced by `extract_docstring` in `parsing/rust.rs:280`).
- `tests/integration/relevance_test.rs:154-207` — existing precision@5 harness over a synthetic
  corpus via `hybrid_search`; S2 generalizes this pattern (add recall@k, MRR, nDCG) over
  workspace symbols rather than a fixed corpus.

Graph ground truth + metrics:
- `src/services/parsing/rust.rs:224-245` `extract_calls_from_body` inventories syntactic call
  sites per file (peer extractors do the same) — the **denominator** for resolution recall.
- `src/services/code_graph.rs:466-475` (index) / `:1070-1077` (sync) resolve `Calls` edges;
  `cross_file_edges_dropped` counter at `:52/:496/:1099`. Resolved `calls` edges are the
  **numerator**. Metrics = resolution recall (resolved ÷ visible call sites) + false-edge rate.

Status / manifest exposure:
- `src/tools/lifecycle.rs:60-72` `WorkspaceStatus` struct; handler
  `get_workspace_status` at `:464-523`. Add `retrieval_eval_enabled: bool` populated from
  `WorkspaceConfig`, so autoharness discovers the capability without parsing `.engram/config.toml`.

Error model / safety: `Result<T, EngramError>` everywhere; `unwrap`/`expect` denied;
`#![forbid(unsafe_code)]`; clippy pedantic. Test tiers under `tests/{unit,contract,integration}`.

## 4. Duplicate / overlap determination — NOT a duplicate

- No `retrieval_eval` config, service, CLI, or MCP tool exists (grep: only agent-efficiency
  `evaluation`/`get_evaluation_report`). Greenfield subsystem.
- `tests/integration/relevance_test.rs` is a **synthetic ranking-regression guard**, not a
  portable measurement over a real indexed workspace — it is a reuse pattern, not overlap.
- `get_evaluation_report` is agent-efficiency, orthogonal in both direction and meaning.

Conclusion: proceed; reuse `hybrid_search`, the config section pattern, the report.rs CLI
wrapper, and `extract_calls_from_body`; build a **new** `services::retrieval_eval` module.

## 5. Task decomposition

Each task is test-first (Ship authors the harness before impl), ≤3 source files, ≤5 functions,
≤4 test scenarios, single width domain.

### Slice 1 — foundation (config + contract + empty-state wiring)

#### 081.001-T — `[retrieval_eval]` config section + report data model  *(domain: models/schema)*
- **Files:** `src/models/retrieval_eval.rs` (new), `src/models/config.rs` (wire field + Default),
  `src/models/mod.rs` (register module). Test: `tests/unit/retrieval_eval_model_test.rs` (new).
- **Adds:** `RetrievalEvalConfig { enabled: bool, languages: Vec<String>, k: usize, sample_size:
  usize, thresholds: RetrievalEvalThresholds }` with `#[serde(default)]`, **disabled by
  default**, following the `EvaluationConfig`/`CodeGraphConfig` shape. `RetrievalEvalReport`
  (+ `SemanticMetrics { precision_at_k, recall_at_k, mrr, ndcg }`, `GraphMetrics {
  resolution_recall, false_edge_rate, call_sites, resolved }`, `evaluated_at`, `branch`).
  Wire `#[serde(default)] pub retrieval_eval: RetrievalEvalConfig` into `WorkspaceConfig`
  (`config.rs:14-42`) and its `Default` (`:44-56`).
- **Unit scenarios (4):** default → `enabled=false`; `[retrieval_eval]` TOML parses; report JSON
  round-trips (serde); unknown field tolerated (no `deny_unknown_fields`).
- **Depends on:** none (foundational).

#### 081.002-T — `run_retrieval_eval` / `get_retrieval_eval_report` MCP tools (empty-state)  *(domain: MCP/tools)*
- **Files:** `src/tools/read.rs` (or new `src/tools/eval.rs`) handlers, `src/tools/mod.rs`
  (dispatch arms + `should_record_metrics`), `src/shim/tools_catalog.rs` (two `Tool::new`
  schemas near `:284`). Test: `tests/contract/retrieval_eval_tools_test.rs` (new).
- **Behavior:** register both tools; when disabled or no run exists, return a well-formed
  **empty** `RetrievalEvalReport` (mirrors `evaluate`'s empty-events branch,
  `services/evaluation.rs:22-31`). No compute yet — wiring + schema + empty contract only.
- **Contract scenarios (4):** both schemas present in catalog; `run_retrieval_eval` while
  disabled → empty report + `enabled:false`; `get_retrieval_eval_report` before any run → empty
  report; unknown params tolerated.
- **Depends on:** 081.001-T.

#### 081.003-T — `engram eval` CLI subcommand + JSON-stdout contract  *(domain: CLI)*
- **Files:** `src/cli/commands/eval.rs` (new: `run_eval_retrieval`, thin `run_tool` wrapper),
  `src/cli/commands/mod.rs` (register), `src/bin/engram.rs` (`Eval` variant + match arm). Test:
  `tests/contract/cli_eval_test.rs` (new).
- **Behavior:** `engram eval` → `run_tool("run_retrieval_eval", …)`; structured JSON to
  **stdout**; honor `--format json|quiet` via `OutputFormatter`. Engram owns CLI + output
  contract; autoharness owns invocation (064-F `engram verify` precedent).
- **Contract scenarios (4):** exit 0 on empty/disabled run; JSON emitted to stdout;
  `--format quiet` suppresses; subcommand registered in help.
- **Depends on:** 081.002-T.

### Slice 2 — semantic self-retrieval eval

#### 081.004-T — Semantic self-retrieval eval compute  *(domain: eval-compute/semantic)*
- **Files:** `src/services/retrieval_eval.rs` (new: query derivation + metrics),
  `src/services/mod.rs` (register), `src/tools/read.rs` (handler invokes semantic compute). Test:
  `tests/integration/retrieval_eval_semantic_test.rs` (new).
- **Adds:** derive known-item queries from each symbol's docstring / fully-qualified name; run
  `hybrid_search`/unified search; compute **precision@k, recall@k, MRR, nDCG**. Language-agnostic
  (drives off indexed symbols, gated by `RetrievalEvalConfig.languages`).
  - **Delivered contract (084-F correction):** the semantic corpus is **functions-only** in this
    baseline — it drives off the indexed `Function` inventory, not every indexed symbol kind.
    Each query is derived from a function's docstring first line, falling back to its **bare
    function name** (not a fully-qualified name). Known-item ranking uses the bounded top-k probe
    `hybrid_rank_of` (084.010-T), which is provably rank-identical to `hybrid_search` but avoids a
    full clone+sort per query. The corpus is completeness-preserving: a partially-written function
    (a `function_meta` row lacking a code/embedding row) still counts toward the denominator via a
    LEFT JOIN (084.009-T). The effective retrieval mode (hybrid vs keyword-only fallback) is
    recorded honestly rather than assumed hybrid (084.008-T).
- **Integration scenarios (4):** known-item hit@1 → MRR=1.0; injected miss → recall@k drops;
  nDCG rewards correct ordering; empty/disabled corpus → zero report (no panic).
- **Depends on:** 081.001-T.

### Slice 3 — graph resolution-recall eval  *(this is the rec1 acceptance metric)*

#### 081.005-T — Graph resolution-recall + false-edge-rate compute  *(domain: eval-compute/graph)*
- **Files:** `src/services/retrieval_eval.rs` (extend with graph metric), `src/db/cozo_queries.rs`
  (read query: resolved `calls` counts / edges by provenance), `src/tools/read.rs` (handler
  populates graph section). Test: `tests/integration/retrieval_eval_graph_test.rs` (new).
- **Adds:** denominator = parser call-site inventory (`extract_calls_from_body`); numerator =
  resolved `calls` edges. **resolution_recall** = resolved ÷ visible call sites;
  **false_edge_rate** = edges to names with no matching def (or ambiguous) ÷ resolved. This is
  the empirical surface that gates 082-F.
  - **Delivered contract (084-F correction):** the denominator counts **distinct `(caller,callee)`
    relations** (not raw call occurrences), excludes method/receiver and path-qualified calls
    (which are never promoted to edges), and is gated to the same configured caller languages as
    the numerator (084.002-T) so recall is a ratio of commensurable units. `false_edge_rate` is a
    **dangling-only lower bound**: `count_dangling_calls_edges` counts only resolved edges whose
    callee matches **no** known definition — it does **not** detect mis-resolution to an
    existing-but-wrong function. True **target-correctness** is therefore asserted separately,
    against a hand-authored expected-target manifest over an indexed fixture (084.004-T /
    084.012-T), never inferred from `false_edge_rate` alone. Recall is additionally surfaced with
    honest `index_stale` / `unreadable_files` signals when the re-read tree drifts from the
    indexed revision (084.003-T).
- **Integration scenarios (4):** all-local resolved → recall≈1.0; cross-file drop → recall<1.0;
  edge to non-existent def → false_edge_rate>0; empty graph → zero report.
- **Depends on:** 081.001-T.

### Slice 4 — autoharness integration + feedback

#### 081.006-T — Status/manifest exposure + `.engram/eval/` persistence  *(domain: integration/persistence)*
- **Files:** `src/tools/lifecycle.rs` (`WorkspaceStatus.retrieval_eval_enabled` + populate in
  `get_workspace_status`), `src/services/retrieval_eval.rs` (persist each run to
  `.engram/eval/{branch}/…json`; `get_retrieval_eval_report` reads the last run). Test:
  `tests/contract/retrieval_eval_status_test.rs` (new).
- **Behavior:** expose `enabled` for autoharness discovery (no file parsing); raw runs land under
  `.engram/eval/{branch}/` (tool-managed, gitignored); `get_retrieval_eval_report` returns the
  latest persisted run.
- **Contract scenarios (4):** status reflects `enabled=true` when configured; `=false` by default;
  a run writes JSON under `.engram/eval/{branch}/`; report tool reads the latest run.
- **Depends on:** 081.004-T, 081.005-T.

#### 081.007-T — Regression test tier + `docs/eval/` graduated baseline  *(domain: test/regression + docs)*
- **Files:** `tests/integration/retrieval_eval_regression_test.rs` (new tier — runs eval on a
  fixture workspace, asserts baseline thresholds), `docs/eval/README.md` + baseline JSON (new
  committed dir — does not yet exist), `src/services/retrieval_eval.rs` (small
  threshold-compare helper). Test tier IS the deliverable.
- **Behavior:** graduated baselines live in committed `docs/eval/` for cross-session tuning input;
  the regression test guards against metric regressions on the fixture.
- **Regression scenarios (4):** fixture eval meets semantic baseline; meets graph baseline;
  threshold breach fails the tier; baseline JSON round-trips.
- **Depends on:** 081.006-T.

**Dependency chain (first shipment):** 081.001-T → {081.002-T → 081.003-T}, 081.001-T →
081.004-T, 081.001-T → 081.005-T; {081.004-T, 081.005-T} → 081.006-T → 081.007-T.

## 6. Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | All new code Rust 2024, `Result<T, EngramError>`, no `unwrap`/`expect`, `#![forbid(unsafe_code)]` inherited, clippy pedantic. New config/report structs derive serde like existing peers. |
| II. Test-First | Every task pairs impl with a named tier file (unit/contract/integration) authored before impl by Ship. |
| III. Workspace Isolation | `.engram/eval/{branch}/` writes are inside the workspace `.engram` dir; report reads are workspace-scoped; no traversal. |
| IV. CLI Workspace Containment | `engram eval` is a read/compute + in-workspace-write command using `GlobalFlags` workspace resolution; no writes outside the workspace. |
| V. Destructive Command Approval | None — eval is additive; writes only under `.engram/eval/` and committed `docs/eval/` baselines. |
| VI. Safety Modes | Elevated blast radius (new public CLI + MCP contract, new committed config schema) → `careful` + `freeze-scope` (see §7). |
| Task Granularity | Each task ≤3 source files / ≤5 fns / ≤4 scenarios, single width domain; S1 split across 3 width domains. |

## 7. Plan-Harden (risk-triggered)

Triggers: **new public CLI + MCP contract** consumed by autoharness; **new committed config
schema** (`[retrieval_eval]` in `.engram/config.toml`); the subsystem's graph metric is the
**acceptance gate for a core-indexing change** (082-F).

**Safety mode:** `careful` + `freeze-scope` to `src/models/retrieval_eval.rs`,
`src/services/retrieval_eval.rs`, `src/cli/commands/eval.rs`, `src/tools/` (read/eval + catalog +
dispatch), `src/tools/lifecycle.rs` (status field only), `src/models/config.rs` (one field +
Default only), and `tests/`. Explicitly **do not** touch `src/services/evaluation.rs` or the
agent-efficiency surface.

| Risk | Blast radius | Mitigation |
|---|---|---|
| Naming collision with agent-efficiency `evaluation` | High (semantic confusion, broken reports) | `retrieval_eval` naming everywhere; 081.001 unit test asserts both configs coexist; freeze-scope excludes `evaluation.rs`. |
| Config schema drift breaks old `.engram/config.toml` | Medium | `#[serde(default)]` + no `deny_unknown_fields` (config.rs:177-181); default disabled; 081.001 scenario asserts absent-section → default. |
| MCP/CLI contract drift (autoharness depends on JSON shape) | High (breaks harness consumer) | `RetrievalEvalReport` schema pinned by 081.002 contract test + 081.003 CLI JSON test; empty-state shape defined before compute. |
| Self-retrieval over-rewards name/doc recall (proxy metric) | Medium (misleading semantic score) | Report as **one signal** alongside graph metrics + optional labeled augmentation; documented in module docs (per deliberation con). |
| Graph metric miscount inflates/deflates rec1 acceptance | High (gates a core change) | Denominator strictly from `extract_calls_from_body`; numerator from persisted `calls` edges; 081.005 scenarios pin all-resolved / cross-file-drop / false-edge / empty. |
| Eval run cost on large workspaces | Medium | `sample_size`/`k` config bounds; disabled by default; run is opt-in per workspace. |
| `.engram/eval/` unbounded growth | Low | Per-branch dir, tool-managed; graduation to `docs/eval/` is explicit + bounded. |

**Rollback:** every task is additive (new modules + new CLI variant + new MCP tools + one
config field + one status field). Reverting the feature branch removes them with no schema
migration to unwind (config section is optional/defaulted).

## 8. Plan-Review (gate)

Multi-persona review (Rust-safety, API/contract, indexing/graph correctness, test-first,
blast-radius/ops), 1 cycle.

| # | Persona | Finding | Severity | Disposition |
|---|---|---|---|---|
| 1 | API/contract | Empty-state report shape must be defined before compute so autoharness can integrate early | P1 | RESOLVED — 081.002 defines empty `RetrievalEvalReport`; JSON pinned by contract tests. |
| 2 | Indexing/graph | Graph metric is rec1's acceptance gate; denominator/numerator must be unambiguous | P1 | RESOLVED — 081.005 fixes denominator = `extract_calls_from_body`, numerator = persisted `calls` edges; 4 scenarios pin behavior. |
| 3 | Rust-safety | Naming collision with `evaluation`/`EvaluationConfig`/`get_evaluation_report` | P1 | RESOLVED — `retrieval_eval` throughout; freeze-scope excludes `evaluation.rs`; coexistence unit test. |
| 4 | Blast-radius/ops | New committed config schema could break existing configs | P2 | RESOLVED — `#[serde(default)]`, disabled default, unknown-field tolerance; scenario asserts absent-section default. |
| 5 | Test-first | Self-retrieval is a proxy; risk of over-trusting semantic score | P2 | ACCEPTED w/ mitigation — reported as one signal + optional labeled augmentation; documented. |
| 6 | Ops | Eval cost / `.engram/eval/` growth on large workspaces | P3 | RESOLVED — `sample_size`/`k` bounds; opt-in; per-branch tool-managed dir. |

**Gate outcome: PASS.** No unresolved P0/P1. One P2 accepted with a bounded mitigation. Proceed
to harvest.

## 9. Assumptions & open items

- Autoharness invokes `engram eval` (or `run_retrieval_eval`) per workspace and reads
  `retrieval_eval_enabled` from `get_workspace_status`/manifest to decide whether to run.
- Graduated baselines live in committed `docs/eval/`; raw runs in gitignored `.engram/eval/`.
- Peer-language semantic/graph extraction beyond the config `languages` set is out of scope for
  this feature (driven by whatever the workspace indexes).
- 082-F (rec1-calledges) consumes S1 (081.001-T contract/model) + S3 (081.005-T graph metric) as
  its acceptance gate; those two tasks are the exported dependency surface.
