# Stage session — 2026-07-10 — 081-F eval subsystem + 082-F rec1-calledges harvest

**Agent:** Stage · **Repo:** softwaresalt/agent-engram · **Branch:** main @ ae7bf17
**Pipeline run:** impl-plan → plan-review → harvest for TWO operator-signed-off deliberations.

## What shipped from this session (planning + backlog only; no code, no builds, no PRs)

### Deliberations marked DECIDED (operator sign-off 2026-07-10)
- `docs/decisions/2026-07-10-engram-retrieval-eval-subsystem-deliberation.md` → `status: decided`, harvested_to 081-F.
- `docs/decisions/2026-07-08-callgraph-cross-file-resolution-deliberation.md` → `status: decided`, harvested_to 082-F.

### Exec plans authored (Constitution Check + Plan-Harden + Plan-Review each)
- `docs/exec-plans/2026-07-10-engram-retrieval-eval-subsystem-plan.md` (081-F, 4 slices → 7 tasks). Plan-Review: **PASS**.
- `docs/exec-plans/2026-07-10-callgraph-cross-file-resolution-plan.md` (082-F, Rust-first slice 1). Plan-Review: **PASS**.

### Feature 081-F — Portable retrieval + graph-recall eval subsystem (ships FIRST)
- 081.001-T retrieval_eval config section + report data model  (S1, width:models)
- 081.002-T run_retrieval_eval / get_retrieval_eval_report MCP tools (empty-state)  (S1, width:mcp)
- 081.003-T `engram eval` CLI subcommand + JSON-stdout contract  (S1, width:cli)
- 081.004-T Semantic self-retrieval eval compute (precision@k/recall@k/MRR/nDCG)  (S2)
- 081.005-T Graph resolution-recall + false-edge-rate compute  (S3 — rec1 acceptance metric)
- 081.006-T Status/manifest exposure + .engram/eval persistence  (S4)
- 081.007-T Regression test tier + docs/eval graduated baseline  (S4)

### Feature 082-F — rec1-calledges cross-file & method-call resolution (ships SECOND)
- 082.001-T Capture method/receiver calls in resolve_call_name (field_expression)
- 082.002-T Record unresolved Calls edges instead of dropping them
- 082.003-T Unambiguous cross-file post-pass + calls_resolved_singleton provenance
- 082.004-T Acceptance verification via 081-F eval (recall up, false-edge within threshold)
- 082.005-T [Deferred] Fan-out to peer language extractors — NOT in first shipment

## Dependency wiring
- 081-F chain: 001→{002→003}, 001→004, 001→005, {004,005}→006→007.
- 082-F chain: 001→002→003→004; 003→005 (deferred).
- **Cross-feature acceptance gate (rec1 → eval S1+S3):** 082.004-T depends_on 081.001-T (S1) AND 081.005-T (S3).
- Feature-level: 082-F depends_on 081-F.

## Shipments (both queued for Ship to claim)
- **077-S = SHIP-1** — 081-F eval subsystem (081-F + 001..007-T). covering_feature 081-F.
- **078-S = SHIP-2** — 082-F rec1 (082-F + 001..004-T; 082.005-T deferred, excluded). covering_feature 082-F.
- **Recommended Ship order: 077-S (SHIP-1) then 078-S (SHIP-2).** SHIP-2 acceptance (082.004-T) cannot pass until SHIP-1 delivers 081.001-T + 081.005-T.

## Grounding anchors used (for Ship)
- Config pattern: `src/models/config.rs:14-56` (#[serde(default)] section-per-subsystem) → new `src/models/retrieval_eval.rs`.
- Collision to avoid: `src/services/evaluation.rs` (agent-efficiency) — keep `retrieval_eval` naming distinct.
- CLI wrapper: `src/cli/commands/report.rs:47-50`; `src/bin/engram.rs:24/118-125/259-266`.
- MCP: `src/shim/tools_catalog.rs:284`, dispatch `src/tools/mod.rs:304-325`, allow-list `:34-55`.
- Semantic: `src/services/search.rs:254 hybrid_search`; `tests/integration/relevance_test.rs:154-207`.
- Graph GT: `src/services/parsing/rust.rs:224-266 (extract_calls_from_body / resolve_call_name, CALL_BLOCKLIST)`; `src/services/code_graph.rs:466-475/1070-1077 (Calls resolve), :543/:1152 + cozo_queries.rs:1357 (reresolve_references_edges post-pass), :1427 find_function_id`.
- Status exposure: `src/tools/lifecycle.rs:60-72/464-523 (WorkspaceStatus / get_workspace_status)`.

## State / guardrails observed
- ALL_TOOLS_OK (backlogit MCP + CLI). INDEX_SYNC_OK at start and end.
- Did NOT touch the two intentional protected deletions (`.github/agents/auto-mergeinstall.agent.md`, `auto-tune.agent.md`) — still unstaged `D`.
- No branches, no PRs, no cargo/builds (Ship's role). All artifacts left uncommitted in the working tree for operator/Ship.

## Next steps for Ship
1. Claim 077-S; execute 081.001-T→081.007-T test-first in dependency order; ship eval subsystem.
2. Then claim 078-S; execute 082.001-T→082.004-T; 082.004-T gates on the 081-F eval metrics.
3. After SHIP-2, Stage decomposes 082.005-T into per-language tasks (python/ts/go) for a follow-on shipment.
