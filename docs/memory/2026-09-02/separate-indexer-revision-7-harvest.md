---
type: stage-session-memory
timestamp: 2026-09-03T01:10:00Z
agent: stage
session_id: stage-separate-indexer-rev7-20260902
phase: harvest-complete
supersedes: docs/memory/2026-09-02/separate-indexer-revision-6-confirmation-halt.md
---

# Stage Session — Separate Indexer Revision 7 Harvest

## Operator direction

Autonomous continuation to implementation was explicitly authorized. Scope was
limited to clearing the three mechanical blockers recorded in the Revision 6
confirmation halt. Architecture was not reopened and no broad persona cycle was
run.

## Work performed

* Step 0.0 tool gate. `TOOL_OK: backlogit CLI 1.10.1`. MCP tools not exposed in
  this invocation, so `DEGRADED_MODE` on registry-declared CLI fallbacks.
  `INDEX_SYNC_OK` (1173 artifacts at entry).
* Authored `## Remediation Revision 7 — Mechanical Corrections` and
  `## Plan Review — Revision 7` in
  `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`.
* Ran one bounded mechanical confirmation (Rust Reviewer + Architecture
  Strategist, both claude-opus-5), scoped strictly to the three corrections.
* Harvested the authoritative plan into backlogit: 1 feature, 59 tasks,
  28 subtasks, 188 task dependency edges, 10 queued shipments, 15 inter-shipment
  `blocks` edges.

## Revision 7 content

**A. P0-1 — test-manifest registration.** New foundation unit **F00** owns the
`[[test]]` registration section of root `Cargo.toml` (append-only, 49 new
blocks) plus 49 committed placeholder harness files (19 contract, 26
integration, 4 unit). Verified against the tree: 0 top-level `tests/*.rs`, 219
existing `[[test]]` blocks, naming convention
`<subdir>_<basename minus _test>`. All 49 generated names collision-checked
against the 219 existing names — no duplicates. Placeholders import nothing from
`engram`, so F00 lands GREEN and each downstream unit's RED step is genuine.
`F00 -> {49 units}` and `F00 -> F12a` edges added; F00 has no in-edges.

**B. P1-1 — F12 dependency wiring.** `crates/engram-indexer/Cargo.toml` added to
F12's file list, annotated "dependency sections only; after F12a". The
workspace-stub-before-RED order is preserved; manifest ownership is exactly
`F12a -> F12` over disjoint sections.

**C. P1-2 — missing edge.** `F02 + F17 + F38 -> F20` amended to
`F02 + F16 + F17 + F38 -> F20`.

## Confirmation result: PASS

All three items CONFIRMED RESOLVED by both reviewers. Architecture returned no
new P0/P1. Rust returned one new in-scope P1, remediated in place during this
cycle:

* **R7-A1** — assigning removal of the `integration_connection` `[[test]]` block
  to F47 left a broken window at the F46 boundary. The block is gated on
  `required-features = ["legacy-sse"]`, so `cargo dev-test` would have stayed
  GREEN and hidden the breakage while `cargo lint` and `cargo ci`
  (`--all-features`) would have failed with `couldn't read
  tests/integration/connection_test.rs` for the whole span between F46 and F47.
  Fix: ownership moved to F46 so deletion and deregistration land together, and
  the `F12a -> F46` edge added. Root `Cargo.toml` total order is now
  `F00 -> F12a -> F46 -> F47`.

Graph re-verified acyclic; a valid topological order exists; every multi-owner
file carries a declared total order. Three P3 accuracy corrections were also
folded in (F00 is one of three graph roots — F00, F02 and F38 — not the unique
root; F46 ceased to be a root once the `F12a -> F46` edge above was added;
roster is 59 units, not 58; F12a added to the exclusion table). One P2
correction: the placeholder-before-block ordering is enforced at build time,
not by `cargo metadata`, so F00 verification now rests on
`cargo test --test <target> -- --list`, `cargo dev-test`, and `cargo ci`.

Carried forward as non-blocking: F20 reads `ReadinessView` written by F18 with
no `F18 -> F20` edge; this does not break compilation because F04a declares the
seam and ships a pass-through. All Revision 6 non-blocking observations stand.

## Harvest output

* Feature: **142-F**
* Tasks: **142.001-T** through **142.059-T** (one per implementation unit)
* Subtasks: 28, under the eight oversized or high-complexity units
  (F00, F19, F04a, F08, F17, F24, F50, F54)
* Task dependency edges: 188, all `blocks`
* Shipments: **133-S** through **142-S**, all `queued`, 88 items total
* Inter-shipment `blocks` edges: 15

Shipment execution order:

```text
133-S ─┬─> 134-S ─┬─> 138-S ─┬─> 139-S ──> 140-S ─┐
       │          │          ├─> 140-S          ├─> 142-S
       │          └─> 141-S <┘                  │
       ├─> 135-S ──> 137-S ───────────────────> ┤
       └─> 136-S ─┬─> 137-S                     │
                  └─> 138-S                     │
                      141-S ────────────────────┘
```

## Degradations recorded

* **Sizing is prose, not structured.** This workspace's `task` artifact type
  defines no `size` or `complexity` field (registry `features.sizing` is absent
  from `.autoharness/backlog-registry.yaml`), so `backlogit update --size` and
  `--complexity` both fail validation with `artifact type "task" does not define
  a size field`. Every task instead carries an enum-validated
  `Size: X | Complexity: Y` line in its implementation-notes section, and every
  subtask carries `Size: X`. Registry drift worth an operator fix: the backlogit
  CLI exposes `--size`, `--size-source`, `--size-ruleset-version` and
  `--complexity`, so the workspace type config, not the tool, is the limiter.
* **MCP tools not exposed** in this invocation; all backlog work ran on
  registry-declared CLI fallbacks.

## Boundary compliance

No production source modified. No branch or worktree created. No build run. No
shipment claimed. No PR. Nothing pushed. Planning and backlog artifacts only.
P-001 and P-016 respected.

## Next step

Orchestrator handles the staging artifact gate. Ship claims **133-S** first.
RS4 and RS5 remain `ActionRisk: high` and require operator approval before Ship
implements F08/F17 (136-S) or F50/F51 (142-S). The HTTP/SSE source deletion in
F46 (135-S) requires operator approval immediately before execution.
