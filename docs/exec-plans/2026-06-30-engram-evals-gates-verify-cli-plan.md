# Exec Plan — Deterministic Gates & Telemetry (engram scope): Phase 1a `engram verify` CLI

**Date:** 2026-06-30
**Status:** Decided / Ready for harvest
**Stage owner:** stage agent
**Stash:** B87680AB (priority: high, kind: feature)
**Deliberation:** 011-D
**Design contract:** `docs/design-docs/autoharness-evals-gates-design.md`
**Feature (umbrella):** 064-F
**First shipment slice:** Phase 1a — tasks 064.001-T, 064.002-T, 064.003-T

---

## 1. Objective

Implement engram's share of the autoharness *Deterministic Gates, Telemetry & Evaluation
Engine* design. Engram is the **Structural Authority** (document conformance for graph
ingestion) and the **Telemetry sink** (ExecutionEpoch ingestion). The design partitions
engram work into four workstreams:

| Phase | Design item | engram scope |
|---|---|---|
| 1a | `engram verify <path>` | Structural AST / YAML-frontmatter linter CLI; non-zero exit blocks `pre_task_completion` |
| 1b | Reactive sync daemon | File-watcher syncs *valid, mutated* markdown into CozoDB nodes |
| 2c | Telemetry schema | CozoDB relational schema for `ExecutionEpoch` events |
| 2d | Ingestion path | Consume autoharness JSONL telemetry; link epochs to Task + Code nodes |

**This plan delivers Phase 1a only as the first queued shipment.** Phases 1b/2c/2d are
carried as dependency-linked follow-on tasks under the same umbrella feature (see §6).

## 2. Scope Decision (why Phase 1a first)

The stash entry spans four domains (linter, daemon event loop, schema evolution, ingestion).
Bundling them violates the **2-Hour Rule** and **Width Isolation**. Phase 1a is chosen as the
first slice because it is:

- **Self-contained & local** — a per-file linter that runs in-process with **no daemon and no
  DB** (architecturally identical to the existing `engram manifest` local command), so it has
  the lowest dependency and blast-radius risk of the four workstreams.
- **The critical unblocker** — autoharness `pre_task_completion` gates cannot block on
  `engram verify {file_path}` until this CLI exists and honors an exit-code contract.
- **Zero schema / zero daemon change** — no CozoDB relations added, no watcher loop touched.

Rejected alternatives (see 011-D): (A) all four phases in one shipment — violates task
granularity; (C) Phase 2c schema first — higher blast radius, no immediate gate value.

## 3. Grounding — real modules (verified 2026-06-30)

CLI registration & dispatch:
- `src/bin/engram.rs` — clap `Command` enum (`#[derive(Subcommand)]`); each variant dispatches
  to a `cli::commands::*` `run_*` fn returning `i32`, then `std::process::exit(code)`.
- `src/cli/commands/mod.rs` — module registry (`indexing`, `lifecycle`, `manifest`, `report`,
  `search`).
- `src/cli/commands/manifest.rs` — **local, no-daemon** command analog: `run_manifest(&flags,
  &fmt) -> i32`. `verify` follows this shape.
- `src/cli/output.rs` — `OutputFormatter` (json/format/quiet, `cli_error`).
- `src/cli/flags.rs` — `GlobalFlags` (`resolve_workspace`, workspace containment).

Structural parsing (reuse targets):
- `src/services/parsing/frontmatter.rs::parse(&str) -> FrontmatterDocument { metadata:
  Option<serde_yaml::Mapping>, body }`. **Known gap:** returns `metadata: None` for BOTH absent
  AND malformed frontmatter — verify must distinguish these (malformed = hard fail).
- `src/services/parsing/markdown.rs` — `MarkdownChunk` (advisory `lint_summary`/suggestions),
  heading/AST extraction; `chunk_markdown_document_with_title_hint` used by ingestion.
- `src/services/ingestion.rs` — `ingest_all_sources`, `build_markdown_content_records`,
  `upsert_content_record`; defines what "conformant for graph ingestion" means in practice.

Cross-platform paths:
- No single normalization utility exists; the established convention is `.replace('\\', "/")`
  (used in `file_tracker.rs`, `ingestion.rs`, `code_graph.rs`, `watcher.rs`, `backlog_indexer.rs`).
- `src/db/workspace.rs::normalize_canonical(PathBuf) -> PathBuf`.
- Workspace containment / traversal rejection is Constitution Principle III (reuse `GlobalFlags`
  workspace resolution).

Test tiers: `tests/{contract,integration,unit,fixtures,helpers}`; analogs
`tests/contract/gate_test.rs`, `tests/contract/evaluation_contract_test.rs`,
`tests/integration/cli_direct_test.rs`.

Error model: `src/errors` — `Result<T, EngramError>` everywhere; `unwrap`/`expect` denied.

## 4. Duplicate / overlap determination — NOT a duplicate

- **No `engram verify` command exists** (grep: all `verify` refs are pidfile `verify_alive` or
  doc comments).
- **No `ExecutionEpoch`/telemetry relation** in `src/db/cozo_backend/schema.rs` (~25 `:create`
  relations, none for epochs) — Phase 2 is greenfield.
- Related prior work is a **reuse surface, not overlap**:
  - `052-F` / `052.004-T` / `038-S` — *advisory* markdown lint + heading guardrails for
    retrieval normalization (non-blocking). Distinct from a **deterministic** ingestion-
    conformance gate with an exit-code contract.
  - `040-S` / `054-F` — engram's **own** tool telemetry emitted **to backlogit** — the
    *opposite direction* of Phase 2d (autoharness JSONL → engram CozoDB).

Conclusion: proceed with planning; reuse the advisory lint + frontmatter parser, but build a
new deterministic `services::verify` for the binary gate.

## 5. Phase 1a task decomposition (first shipment)

Each task is test-first (harness authored by Ship before impl), ≤3 source files, ≤5 functions,
≤4 test scenarios, single width domain.

### 064.001-T — Verify linter core service + result model  *(domain: linter logic)*
- **Files:** `src/services/verify.rs` (new), `src/services/mod.rs` (register), `tests/unit/verify_core_test.rs` (new).
- **Adds:** `VerifyReport { conformant: bool, findings: Vec<VerifyFinding> }`,
  `VerifyFinding { rule, message, line: Option<usize> }`, and
  `pub fn verify_markdown(rel_path: &str, content: &str) -> Result<VerifyReport, EngramError>`
  composing `parsing::frontmatter::parse` (treat **present-but-malformed** YAML as a hard
  finding — closes the silent-`None` gap) + markdown structure checks reusing
  `parsing::markdown` (unresolved `{{...}}` template variables, unparseable/empty body).
- **Unit scenarios (4):** valid md+frontmatter → conformant; present-but-malformed YAML →
  non-conformant finding; unresolved `{{var}}` → finding; valid md w/o frontmatter → conformant.
- **Depends on:** none (foundational).

### 064.002-T — `engram verify <path>` CLI subcommand + exit-code/stderr contract  *(domain: CLI wiring)*
- **Files:** `src/cli/commands/verify.rs` (new: `run_verify`), `src/cli/commands/mod.rs`
  (register), `src/bin/engram.rs` (add `Verify { path }` variant + match arm);
  `tests/contract/verify_test.rs` (new).
- **Behavior:** local/no-daemon (manifest analog). Reads file, calls
  `services::verify::verify_markdown`, prints findings to **stderr** (so autoharness injects
  diagnostics into agent context), returns exit codes: **0** conformant, **1** non-conformant,
  **2** I/O/usage error. Non-markdown target → **0** (no graph-md to validate in Phase 1a).
- **Contract scenarios (≤4):** conformant → 0; malformed → non-zero + stderr carries findings;
  missing file → 2; non-markdown → 0.
- **Depends on:** 064.001-T.

### 064.003-T — Cross-platform path normalization + subprocess integration test  *(domain: cross-platform/integration)*
- **Files:** `tests/integration/cli_verify_test.rs` (new), `tests/fixtures/verify/*` (fixtures),
  minor `src/cli/commands/verify.rs` path-normalization refinement.
- **Behavior:** normalize input path to forward-slash convention (`.replace('\\', "/")`) and
  enforce workspace containment (reject traversal per Principle III); end-to-end subprocess
  invocation of the built binary asserting real exit codes.
- **Integration scenarios (≤4):** conformant fixture → 0; malformed-frontmatter fixture →
  non-zero; backslash Windows-style path input accepted; outside-workspace/`..` path rejected.
- **Depends on:** 064.002-T.

**Dependency chain (shipment):** 064.001-T → 064.002-T → 064.003-T.

## 6. Deferred phases (same feature, NOT in first shipment)

Tracked as queued tasks under 064-F with dependency links; not added to the first shipment.

- **064.004-T (Phase 1b)** — Reactive sync validity gating: gate `debounce.rs::adapt_event`
  `ReingestContent` for markdown on `verify_markdown` conformance (skip + log on failure).
  *Domain: daemon.* **Depends on 064.001-T.**
- **064.005-T (Phase 2c)** — `ExecutionEpoch` CozoDB schema: add relation(s) to
  `src/db/cozo_backend/schema.rs` + `src/models/epoch.rs` + bootstrap contract test.
  *Domain: schema.* Independent; sequence after Phase 1a lands.
- **064.006-T (Phase 2d)** — ExecutionEpoch JSONL ingestion: consume autoharness JSONL, upsert
  epoch nodes, link to `backlog_node` (Task) + `function_meta`/`file_node` (Code) via edges.
  *Domain: ingestion.* **Depends on 064.005-T.**

## 7. Constitution Check

| Principle | Compliance in this plan |
|---|---|
| I. Safety-First Rust | New code is Rust 2024, `Result<T, EngramError>`, no `unwrap`/`expect`, `#![forbid(unsafe_code)]` inherited; must pass clippy pedantic. `verify_markdown` returns `Result`. |
| II. Test-First | Each task pairs impl with a named test tier file (unit/contract/integration) authored before impl by Ship. Three-tier layout honored. |
| III. Workspace Isolation | 064.003-T explicitly enforces workspace containment + traversal rejection on the `<path>` argument; reuses `GlobalFlags` resolution. |
| IV. CLI Workspace Containment | verify only reads the target file; no writes outside cwd. |
| V. Destructive Command Approval | None — verify is read-only. |
| VI. Safety Modes | Plan carries elevated blast radius (new public CLI contract + cross-platform paths); **careful + freeze-scope** declared in §8. |
| Task Granularity | Each task ≤3 files / ≤5 fns / ≤4 scenarios, single width domain. |

## 8. Plan-Harden (risk-triggered)

Trigger: new **public CLI contract** that an external system (autoharness gate) depends on, plus
**cross-platform path** handling. (Schema + daemon risks are deferred with Phases 2c/1b.)

**Safety mode:** `careful` + `freeze-scope` to `src/services/verify.rs`, `src/cli/commands/`,
`src/bin/engram.rs` (variant/arm only), and `tests/`.

| Risk | Blast radius | Mitigation |
|---|---|---|
| Exit-code contract drift (autoharness depends on non-zero = fail) | High (breaks external gate) | Contract test 064.002-T pins 0/1/2 semantics; documented in verify.rs module docs. |
| Silent-`None` frontmatter gap → malformed files pass the gate | High (defeats the gate) | 064.001-T distinguishes malformed vs absent; unit scenario asserts malformed → non-conformant. |
| Windows/Linux path divergence | Medium | 064.003-T normalizes to forward-slash convention + subprocess test on backslash input. |
| Path traversal via `<path>` arg | Medium (Principle III) | Containment check + outside-workspace rejection scenario. |
| Over-broad "conformance" definition causing false failures | Medium (agent churn) | Phase 1a conformance = what `ingestion.rs` needs (parseable frontmatter + parseable body + no unresolved template vars); non-markdown = pass. |
| Width creep into a shared path-normalization util | Low | Explicitly deferred (011-D open Q4); reuse existing convention. |

**Rollback:** all three tasks are additive (new module + new CLI variant); reverting the feature
branch removes the `Verify` variant and `services::verify` with no schema/daemon state to unwind.

## 9. Plan-Review (gate)

Self-review against the design contract + constitution (persona checklist; 1 cycle).

| # | Finding | Severity | Disposition |
|---|---|---|---|
| 1 | Exit-code semantics must be explicit for the autoharness gate | P1 | RESOLVED — 0/1/2 pinned in 064.002-T contract test + module docs. |
| 2 | frontmatter::parse silently swallows malformed YAML | P1 | RESOLVED — 064.001-T treats present-but-malformed as hard finding; unit scenario added. |
| 3 | Cross-platform path handling required by design §6 Q2 | P1 | RESOLVED — dedicated task 064.003-T with subprocess + traversal scenarios. |
| 4 | Risk of duplicating existing advisory md-lint | P2 | RESOLVED — reuse `parsing::markdown`/`frontmatter`; build binary gate distinct from advisory suggestions (011-D). |
| 5 | "Conformant for graph ingestion" under-specified | P2 | ACCEPTED w/ mitigation — bound to `ingestion.rs` requirements; refine in Phase 1b if watcher surfaces new cases. |
| 6 | Non-markdown file behavior ambiguous | P3 | RESOLVED — Phase 1a: non-markdown → exit 0 (documented). |

**Gate outcome: PASS.** No unresolved P0/P1 blocking findings. One P2 accepted with a bounded
mitigation. Proceed to harvest.

## 10. Assumptions & open items

- Autoharness invokes `engram verify {file_path}` **per modified file** (design §5 config);
  Phase 1a is per-file (no glob/dir).
- The `.autoharness/config.yaml` `validation_gates` contract lives on the autoharness side; engram
  only owns the CLI + exit code + stderr diagnostics.
- Absent frontmatter is permitted in Phase 1a unless a required-frontmatter doc class is later
  configured (deferred).
- Pre-execution sizing / infinite-correction-loop handling (design §4, §6 Q1) are
  autoharness/backlogit-side and out of engram scope.
