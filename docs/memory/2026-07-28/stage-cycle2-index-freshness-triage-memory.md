# Stage session memory — Cycle 2: post-100-F follow-up stash triage (2026-07-28)

## Context
Fresh 4-entry stash produced during the merged 092-S / 100-F shipment
(PR #291, merge SHA `8a6c6e32`; feature 100-F done, shipment 092-S archived).
Orchestrator routed triage → deliberation/spike → impl-plan → plan-harden →
plan-review → harvest → queued shipment(s). Operator approved processing.

Index refreshed at session start (`backlogit.exe sync --cwd .` → 770 artifacts).
Tool gate: `ALL_TOOLS_OK` (backlogit MCP 1.7.0 + CLI fallbacks per
`.autoharness/backlog-registry.yaml`; engram CLI + daemon healthy).

## Triage outcomes (4 entries)

| Stash | Kind / prio | Decision | Artifacts |
|---|---|---|---|
| 5765BAAB | bug / medium | DEFERRED — investigate-first spike | deliberation **015-D**; spike doc `docs/decisions/2026-07-28-daemon-index-singleton-nonpersist-ipc-hang-spike.md` |
| 8DD29746 | task / medium | ADVANCED — full pipeline → shipment | plan `docs/exec-plans/2026-07-28-versioned-codegraph-revalidation-backfill-plan.md`; feature **101-F** + **101.001/002/003-T**; shipment **094-S** (queued) |
| B94772CB | feature / low | DEFERRED — deliberation, low, defer indefinitely | deliberation **016-D** |
| F97D51DF | task / low | ADVANCED (maintenance) → shipment | feature **102-F** + task **102.001-T**; shipment **093-S** (queued) |

## Key technical findings

### 5765BAAB — stash root-cause REFUTED (→ spike, not a plan)
Stash blamed `src/cli/direct.rs:162` (`use_index = full || force`) routing
`engram index` to the sync path (which skips the cross-file post-pass). Static
analysis **contradicts** this: `engram index` routes to `index_workspace` on
BOTH paths — daemon via `run_index` → `run_tool_timed("index_workspace")`
(`cli/commands/indexing.rs`), and `--direct` via `run_direct_sync(full=true)`
→ `use_index=true` → `index_workspace`. The index path runs the UNCONDITIONAL
cross-file post-pass (`reresolve_calls_edges_with_canonical_context`,
`code_graph.rs:1815`). So `direct.rs:162` is NOT the cause for `engram index`.
Real defect is on the DAEMON path (commit boundary / IPC response completion),
UNPROVEN → spike (H1 commit boundary, H2 IPC framing/backpressure, H3 daemon
routing divergence to debounce/sync, H4 staging left unresolved). Fix plan
deferred until hands-on daemon repro lands (runtime execution out of Stage scope).

### 8DD29746 — versioned code-graph revalidation / stale-edge backfill (101-F)
Content-hash skip keys on file CONTENT, not extractor generation, so pre-100-F
WRONG direct edges stay stale until `--force`. Fix mirrors 096-F T7:
durable `schema_meta` key `code_graph_extraction_generation` + opt-in gated
revalidation backfill (advance marker only on full success, C7-3). Precedent
machinery: `schema_meta_flag`/`set_schema_meta_flag`,
`PYTHON_CANONICAL_EXTRACTION_VERSION_KEY` (`src/db/cozo_backend/schema.rs`);
`python_extraction_version`/`set_python_extraction_version`
(`src/db/cozo_queries.rs`). Design fork LOCKED: opt-in (NOT auto heavy
re-extraction) to avoid churn (C12-5). Units U1 (RED) → U2 (GREEN marker +
gated backfill) → U3 (upgrade acceptance + docs), dependency-chained.

### B94772CB — Python-only last-wins recall recovery (016-D, defer)
100-F chose fail-closed (Option A, language-agnostic) for same-file duplicate
top-level defs. This would RECOVER recall by minting the edge to the effective
(last) def for Python only. Documented v1 non-goal (docs/architecture.md).
Recommend Option A/C hybrid IF recall data later justifies; MUST stay
zero-false-edge (013-D); Rust stays fail-closed (last-wins unsound for
inline-mod-per-file). Sequence AFTER 8DD29746 and AFTER the 5765BAAB spike/fix.

### F97D51DF — cargo audit remediation (102-F)
10 pre-existing transitive advisories, all at PR #291 base (Cargo.lock
byte-identical, 0 deps added by 100-F). Single dependency width. Ship-executed
(needs `cargo audit`/build/test). Prefer `cargo update -p <crate> --precise`;
escalate to direct bump only if the transitive range blocks the patch; defer
major-version breaks as separate items.

## Grouping decision
5765BAAB (daemon/IPC width, spike) and 8DD29746 (code-graph/db-marker width,
plannable) do NOT share a fix surface → separate, NOT one shipment.
8DD29746 is independently shippable (no dependency on 5765BAAB). F97D51DF
(dependency width) kept separate from 101-F.

## Deliverables
- Queued shipment **094-S** — 101-F versioned revalidation/backfill (medium).
- Queued shipment **093-S** — 102-F cargo audit remediation (low).
- Deferred: 5765BAAB (015-D spike — needs daemon repro), B94772CB (016-D low).
- Stash: harvested entries 8DD29746 + F97D51DF archived; deferred entries
  5765BAAB + B94772CB remain active (deliberation-linked).
- Semantic link: 101-F related_to 015-D.

## Recommended Ship execution order
1. **094-S** FIRST — correctness/freshness (medium); mirrors proven 096-F T7 pattern.
2. **093-S** — opportunistic low-priority maintenance.
3. Next investigative step: schedule the **5765BAAB / 015-D** hands-on daemon
   spike (highest operator-assessed value, but root cause unproven — a fix plan
   on the misattributed `direct.rs:162` cause would be low quality).

## Notes
- All docs/backlog artifacts are in the WORKING TREE only — NOT committed.
  Offer to commit on request; do not push without confirmation.
- Ship already closed 092-S (see `docs/memory/2026-07-28-ship-092S-...closure.md`).
