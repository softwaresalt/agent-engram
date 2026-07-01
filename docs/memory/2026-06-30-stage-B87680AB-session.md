# Stage session — 2026-06-30 — stash B87680AB (engram evals/gates)

## Summary
Processed stash B87680AB ("Deterministic gates, telemetry & evaluation engine — agent-engram
scope") end-to-end: triage → duplicate check → deliberation → impl-plan → plan-harden →
plan-review → harvest → queued shipment. Not a duplicate. Scoped a defensible first slice
(Phase 1a `engram verify` CLI) and deferred the rest.

## Deliverables
- **Deliberation:** 011-D (linked to stash B87680AB).
- **Exec-plan:** `docs/exec-plans/2026-06-30-engram-evals-gates-verify-cli-plan.md`
  (Constitution Check + plan-harden + plan-review PASS).
- **Feature:** 064-F "Deterministic gates & telemetry — engram Structural Authority + Telemetry sink".
- **Tasks:**
  - 064.001-T Phase 1a — verify linter core service + result model (dep: none)
  - 064.002-T Phase 1a — `engram verify <path>` CLI subcommand + exit-code/stderr contract (dep: 064.001-T)
  - 064.003-T Phase 1a — cross-platform path normalization + subprocess integration test (dep: 064.002-T)
  - 064.004-T Phase 1b — reactive sync gated on verify conformance (DEFERRED; dep: 064.001-T)
  - 064.005-T Phase 2c — ExecutionEpoch CozoDB telemetry schema (DEFERRED)
  - 064.006-T Phase 2d — ExecutionEpoch JSONL ingestion + Task/Code linking (DEFERRED; dep: 064.005-T)
- **Shipment:** 052-S (queued) = [064-F, 064.001-T, 064.002-T, 064.003-T]. Ready for Ship to claim.
- **Stash:** B87680AB archived.

## Scope decision (why Phase 1a first)
Entry spans 4 width domains (linter / daemon / schema / ingestion) — too large for one shipment.
Phase 1a `engram verify` CLI chosen: self-contained, local/no-daemon (manifest analog), no schema
or daemon change, and the critical unblocker for the autoharness `pre_task_completion` gate.
Phases 1b/2c/2d deferred as dependency-linked tasks under 064-F (not re-stashed, so they stay visible).

## Duplicate determination — NOT a duplicate
- No `engram verify` command exists (all `verify` refs are pidfile `verify_alive`/doc comments).
- No ExecutionEpoch/telemetry relation in `src/db/cozo_backend/schema.rs`.
- Related prior work is reuse-surface, not overlap: 052-F/052.004-T/038-S = ADVISORY md-lint
  (non-blocking, retrieval normalization); 040-S/054-F = engram→backlogit telemetry (opposite
  direction from Phase 2d autoharness→engram ingestion).

## Key grounding (real modules)
- CLI: `src/bin/engram.rs` (clap Command enum → `cli::commands::*` `run_* -> i32`), local no-daemon
  analog `src/cli/commands/manifest.rs`; register in `src/cli/commands/mod.rs`.
- Parsing reuse: `src/services/parsing/frontmatter.rs::parse` (GAP: returns None for BOTH absent
  and malformed YAML — verify must split these), `src/services/parsing/markdown.rs` (advisory lint),
  `src/services/ingestion.rs` (content-record path).
- Phase 1b scaffolding already exists: `src/daemon/watcher.rs` + `src/daemon/debounce.rs::adapt_event
  -> ServiceAction::ReingestContent` → 1b is an enhancement, not greenfield.
- Path normalization: no shared util; convention is `.replace('\\','/')`; `src/db/workspace.rs::normalize_canonical`.
- Tests: `tests/{contract,integration,unit,fixtures,helpers}`; analogs gate_test.rs,
  evaluation_contract_test.rs, cli_direct_test.rs.

## Risks / assumptions
- Exit-code contract (0 pass / 1 non-conformant / 2 error) is a public dependency of the autoharness
  gate — pinned via contract test in 064.002-T.
- Malformed-vs-absent frontmatter policy: malformed = hard fail; absent = pass (Phase 1a).
- Autoharness invokes verify per-file; no glob/dir in Phase 1a.
- engram daemon-status was unresponsive this session; used file-based research grounding (engram MCP
  tools not in Stage's tool surface).

## Next steps (Ship)
1. Claim shipment 052-S; build Phase 1a test-first (064.001 → 064.002 → 064.003).
2. After Phase 1a merges, pick up deferred 064.004/005/006 (respect dep links; keep width-isolated).

## Role-boundary note
Stage stayed within planning/backlog scope: no source/test/config edits, no build/branch/PR. Only
docs artifacts (exec-plan, this memory) + backlog mutations were written.
