---
title: "Shipment manifest integrity & GI/GR reconciliation"
description: "Harness-side defense-in-depth against over-scoped shipment manifests after the 003-S incident"
topic: "Stage/Ship workflow guards + GI/GR reconciliation skill to prevent recurrence of 003-S manifest drift"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md"
  - "docs/closure/003-s-cozodb-phase2-closure.md"
stash_ids:
  - "A1B2C3D4"
  - "B2C3D4E5"
  - "C3D4E5F6"
  - "D4E5F6A7"
tags:
  - workflow-integrity
  - shipment
  - harness
  - chore
---

## Problem Frame

### Incident triggering this work

Shipment **003-S** (CozoDB migration) was harvested at planning time with all 8 phases (50 items) but only Phases 0-2 were ever executed. When `backlogit_ship_shipment` was called post-merge, the tool deleted 27 unbuilt queue files from disk without verifying their `status: done` and without moving them to archive. Discovery only happened during a post-hoc audit. Recovery required restoring 27 files from a pre-ship git commit.

Full incident analysis: `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md`.

### Who cares

* **Operators** lose work-tracking history when items vanish silently
* **Ship agent** produces incorrect closure summaries when manifest doesn't match reality
* **Stage agent** harvests over-scoped manifests at planning time, compounding the risk
* **Future audits** cannot reconcile what was actually shipped vs claimed

### Constraints

* The `backlogit` tool is external (registered via `.autoharness/backlog-registry.yaml` as `command: "backlogit mcp"`). Its source is **not in this repo**, so tool-level fixes must be escalated upstream or wrapped at the workflow layer.
* All defenses must work without modifying the backlogit binary
* Solution must be testable without live MCP calls (skill-level + agent-instruction-level enforcement)
* Recovery procedures must remain documented (operators may still hit unfixed older versions of the tool)

### Success criteria

1. A repeat of the 003-S incident is structurally prevented at the harness layer
2. Stage no longer harvests multi-phase plans into a single over-broad shipment
3. Ship verifies manifest-vs-reality before AND after archiving
4. Failed reconciliation halts the workflow with operator-actionable detail
5. The compound learning is operationalized as enforced workflow steps, not just documentation

### Out of scope

* Replacing the backlogit tool
* Fundamentally restructuring the Stage/Ship two-agent split
* Recovering historical shipments other than 003-S (already done manually)
* Building a UI for shipment audit reports

## Research Findings

### Tool ownership (critical)

`backlogit` is consumed via its MCP server (`backlogit mcp` stdio transport). Its source is not in this repo. Item **A1B2C3D4** as originally written (tool-level fix) is therefore **not implementable in this repo** — it must be reframed as either an upstream-escalation deliverable or a workflow wrapper. The wrapper path overlaps with D4E5F6A7 (GI/GR audit), so the harness-side answer is to invest in the wrapper and file an upstream issue.

### Existing harness primitives

* `.github/agents/ship.agent.md` — Ship agent definition; owns Step 6 (post-merge closure including `backlogit_ship_shipment`)
* `.github/agents/stage.agent.md` — Stage agent definition; owns Step 5.5 (shipment assembly) where over-broad harvests originate
* `.github/skills/` — pattern for reusable workflow primitives. A new `shipment-reconcile` skill fits naturally here.
* Existing repo memory captures the P-007 archive-deletion workaround (`git restore .backlogit/archive/` post-ship) which addresses a *different* aspect of the same fragile area.

### Defense-in-depth layers available

| Layer | Owner | Mechanism |
|---|---|---|
| **Intake** | Stage harvest | Limit shipment scope to one executable phase |
| **Pre-ship** | Ship workflow | Reconcile manifest vs actually-done items; prune unbuilt entries |
| **Post-ship** | Ship workflow | Verify every manifest item exists in archive; no orphans |
| **Tool** | backlogit (external) | Per-item validation (upstream fix; can only escalate) |

The harness controls the first three. The fourth requires upstream collaboration but is no longer load-bearing once the harness layers are in place.

## Options Evaluated

### Option A — Harness-only defense (RECOMMENDED)

Build all integrity guarantees at the agent + skill layer. File an upstream issue documenting the tool bug for awareness, but do not block on it. Treat the `backlogit` tool as fixed-or-broken from our perspective; our wrapper makes the difference irrelevant.

**Components**:
1. New `shipment-reconcile` skill (~3 tasks) with `mode: pre` and `mode: post` invocations
2. Stage agent edit (~1 task): cap harvest scope at one executable phase per shipment, with explicit guard
3. Ship agent edit (~2 tasks): invoke `shipment-reconcile mode: pre` immediately before `backlogit_ship_shipment`; invoke `mode: post` immediately after
4. Upstream documentation deliverable (~1 task): write GitHub issue text for backlogit maintainers
5. Test fixtures + harness-level integration test (~2 tasks)
6. Update `.github/instructions/backlogit.instructions.md` to reference the new gate (~1 task)

**Pros**: self-contained, ships independently of upstream, ~10 atomic tasks, can be merged in one shipment
**Cons**: duplicates work the tool *should* be doing; wrapper overhead per ship cycle (acceptable: <1s)
**Effort**: medium

### Option B — Wait for upstream fix

File the bug to backlogit maintainers and rely on them. Document a manual reconciliation checklist in the meantime.

**Pros**: no harness code to maintain
**Cons**: zero control over timing; another 003-S could happen at any time; manual checklists are skipped under pressure; the operator already lost work once
**Effort**: low (but high risk)

### Option C — Replace backlogit

Switch to a different backlog tool we control or vendor backlogit's source.

**Pros**: full control over manifest semantics
**Cons**: massive scope; orthogonal to the actual problem; backlogit otherwise works well
**Effort**: very high

## Trade-off Comparison

| Criterion                       | A: Harness defense | B: Wait for upstream | C: Replace tool |
|---------------------------------|--------------------|----------------------|-----------------|
| Prevents 003-S recurrence       | Yes                | No                   | Yes             |
| Time to value                   | One shipment       | Unknown              | Many shipments  |
| Maintenance burden              | Low (skill + ~3 instruction edits) | Zero | High |
| Couples to upstream timing      | No                 | Yes                  | No              |
| Risk of regression elsewhere    | Low                | Unchanged            | High            |
| Operator confidence improvement | Strong             | None                 | Eventually      |

## Decision

**Option A** — harness-only defense.

### Covering feature scope (refined)

**Title**: "Harness-side shipment manifest integrity (GI/GR reconciliation)"
**Type**: chore (internal harness improvement, not a user-facing capability)

### Refined item mapping

| Original stash | Treatment |
|---|---|
| **C3D4E5F6** (Stage scoping) | IN — implemented as Stage agent edit; do FIRST so prevention is in place even if other items slip |
| **B2C3D4E5** (Ship pre-archive gate) | IN — implemented as Ship agent edit invoking `shipment-reconcile mode: pre` |
| **D4E5F6A7** (GI/GR audit) | IN — implemented as new `shipment-reconcile` skill providing both `mode: pre` and `mode: post` |
| **A1B2C3D4** (tool fix) | REFRAMED — not implementable in this repo; becomes an upstream-escalation deliverable (documented GitHub issue text + reference in instruction file). The wrapper path obviates the need to wait. |

### Implementation order (dependency-aware)

1. **Stage scoping** (C3D4E5F6) — prevent future over-broad manifests
2. **shipment-reconcile skill** (D4E5F6A7) — the wrapper itself
3. **Ship integration** (B2C3D4E5) — wire skill into Ship workflow
4. **Upstream escalation** (A1B2C3D4-reframed) — file issue, update instruction file

### Done criteria

* Stage agent rejects multi-phase harvests by default
* `shipment-reconcile` skill returns `[matched|missing|orphan|status-mismatch]` per item
* Ship halts on reconciliation failure with operator-actionable report
* Integration test simulates the 003-S scenario and confirms it now halts pre-ship
* Compound learning links to the new skill and instruction updates

## Rejected Alternatives

* **Option B** (wait for upstream) — operator already lost work once; unacceptable to leave the same bug exposed
* **Option C** (replace tool) — disproportionate to the scope of the actual problem; backlogit is otherwise sound
* **Tool-level only via vendoring** — would couple our release cadence to backlogit upstream and add a non-trivial maintenance surface

## Unresolved Questions

* **Atomicity of reconcile + ship**: between `shipment-reconcile mode: pre` PASS and the actual `backlogit_ship_shipment` call, can manifest state drift? In single-agent workflow, no. Multi-agent intercom mode may need a short critical section. Resolve in plan-harden if hardening is required.
* **Manifest mutation by reconcile**: should `mode: pre` mutate the manifest to prune unbuilt items, or only report and refuse to proceed? Lean toward report-only with an explicit operator-approved prune step, to avoid silent scope changes.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Reconcile skill becomes a rubber-stamp if always passing | Include a test fixture that injects a known mismatch and asserts the skill flags it |
| Stage one-phase rule blocks legitimate multi-task shipments | Allow opt-in flag for genuinely-coupled multi-phase work, with explicit operator confirmation |
| Wrapper overhead slows ship cycle | Performance budget: <1s for typical shipment of <50 items; skill must be O(n) on item count |
| Upstream backlogit fix arrives later and conflicts with our wrapper | Wrapper checks remain useful as belt-and-suspenders; not a conflict |
| Solo operator forgets the new gate during emergency ships | Gate is in Ship agent definition, not optional; bypass requires explicit `force_no_reconcile` flag like other emergency overrides |

## Promotion

**Promote to**: plan
**Next step**: invoke impl-plan skill on this artifact to produce `docs/exec-plans/2026-04-20-shipment-integrity-plan.md`.
