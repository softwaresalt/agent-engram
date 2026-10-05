---
title: "142-F P0 worktree classification (W1/W2)"
date: 2026-10-04
step: P0
plan: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
plan_revision: 18
---

# 142-F P0 worktree classification

Source: `git --no-pager status --short --untracked-files=all` (read-only), classified against plan section 7.2. W3 (`f6f3171f`) and W4 (deleted) are retired; no W3/W4-shaped path was found. "Judgement placement" rows are not enumerated in 7.2 and are placed by the class heading; the operator may move them before H1/H3.

| Path | Status | Class | Reason |
|---|---|---|---|
| `.backlogit/archive/stash.jsonl` | M | W1 | stash file |
| `.backlogit/queue/035-D.md` | ?? | W1 | decision item `035-D`-`038-D` (enumerated) |
| `.backlogit/queue/036-D.md` | ?? | W1 | decision item `035-D`-`038-D` (enumerated) |
| `.backlogit/queue/037-D.md` | ?? | W1 | decision item `035-D`-`038-D` (enumerated) |
| `.backlogit/queue/038-D.md` | ?? | W1 | decision item `035-D`-`038-D` (enumerated) |
| `.backlogit/queue/142.060-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.061-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.062-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.063-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.064-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.064.001-ST.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.064.002-ST.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.064.003-ST.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.065-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.066-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.067-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.068-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.069-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.070-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.071-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.072-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.073-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/queue/142.074-T.md` | ?? | W1 | `142.060-T`-`142.074-T` / `142.064.00x-ST` queue file (enumerated) |
| `.backlogit/stash.jsonl` | M | W1 | stash file |
| `docs/decisions/2026-09-24-cli-parity-all-user-facing-tools-deliberation.md` | ?? | W1 | decision artifact for `035-D` |
| `docs/decisions/2026-09-24-docs-only-task-harness-disposition-deliberation.md` | ?? | W1 | decision artifact for `036-D` |
| `docs/decisions/2026-09-25-archive-verifier-read-before-close-deliberation.md` | ?? | W1 | 142-F related decision (plan frontmatter; `038-D`) |
| `docs/decisions/2026-09-25-backlog-tool-surface-selection-deliberation.md` | ?? | W1 | decision artifact for `037-D` |
| `docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md` | ?? | W1 | 142-F source decision (plan frontmatter) |
| `docs/decisions/2026-09-27-pa5-read-handler-conversion-deliberation.md` | ?? | W1 | 142-F related decision (PA5-C) |
| `docs/decisions/2026-09-28-142-f-escalation-review.md` | ?? | W1 | 142-F related decision (plan frontmatter) |
| `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md` | M | W1 | 142-F authoritative plan (listed in `142-F.md`); Stage amendment 2026-09-24 (E06BABAD) |
| `docs/exec-plans/2026-09-24-docs-only-verification-gated-harness-disposition-plan.md` | ?? | W1 | Stage plan for decision item `036-D` |
| `docs/exec-plans/2026-09-24-full-cli-mcp-parity-plan.md` | ?? | W1 | Stage plan for decision item `035-D` |
| `docs/exec-plans/2026-09-25-archive-verifier-read-before-close-plan.md` | ?? | W1 | 142-F plan (parent_feature 142-F; `038-D`) |
| `docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md` | ?? | W1 | 142-F plan (old plan, Revs 6-14) |
| `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` | ?? | W1 | 142-F plan (this plan, Rev 18) |
| `docs/memory/2026-09-24-stage-036d-option2-plan-circuit-open.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-24-stage-cli-parity-and-docs-harness-intake.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-24-stage-cli-parity-plan-checkpoint.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-24-stage-path-s-handoff.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-25-hook-cli-bootstrap-upstream-handoff.md` | ?? | W1 | memory for `037-D` follow-up (operator-requested; states 142-S manifest unchanged); not a 142-S execution record. Judgement placement |
| `docs/memory/2026-09-25-stage-be626470-archive-verifier-unblock.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-25-stage-dda2506f-checkpoint-resume-disposition.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-25-stage-dda2506f-tool-surface-intake.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-attempt3-fail-escalation.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-attempt3-result-checkpoint.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-attempt4-result.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-harvest.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-plan-review-attempt2-checkpoint.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-preflight-command-checkpoint.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26-stage-142-f-rev4-attempt3-checkpoint.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-26/circuit-break-stage-142-f-final-review.md` | ?? | W1 | Stage 142-F planning circuit-break memory |
| `docs/memory/2026-09-27-stage-142-f-rev6.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-27-stage-142-f-rev7.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-27-stage-142-f-shipment-split.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-27-stage-pa5-deliberation.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-28-stage-142-f-rev8.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-29-stage-142-f-rev10.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-29-stage-142-f-rev11.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-29-stage-142-f-rev12-memory.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-29-stage-142-f-rev13-memory.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-29-stage-142-f-rev9.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-30-stage-142-f-rev14-memory.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-30-stage-142-f-rev15-decomposition-memory.md` | ?? | W1 | Stage memory (142-F / W1 decision-item planning) |
| `docs/memory/2026-09-30/circuit-break-142-f-decomposition-review.md` | ?? | W1 | 142-F decomposition-review circuit-break memory (Orchestrator, planning) |
| `docs/operator-glossary.md` | ?? | W1 | `docs/operator-glossary.md` (enumerated) |
| `docs/scratch/2026-09-24-docs-only-harness-gate-autoharness-handoff.md` | ?? | W1 | Stage scratch handoff for `036-D` |
| `docs/scratch/2026-09-25-backlog-cli-first-class-autoharness-handoff.md` | ?? | W1 | scratch handoff for `037-D` |
| `docs/scratch/2026-09-30-142-f-plan-review-attempt-14-findings.md` | ?? | W1 | 142-F plan-review scratch (attempt 14) |
| `docs/scratch/2026-10-04-142-f-p0-worktree-classification.md` | ?? | W1 | this P0 classification (Stage scratch; created by P0) |
| `docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md` | ?? | W1 | 142-F plan-review scratch (attempt 15) |
| `.backlogit/archive/142.058.001-ST.md` | ?? | W2 | `142.058.001-ST` archive move (enumerated) |
| `.backlogit/checkpoints/checkpoint-20260924-010034.json` | M | W2 | checkpoint; the C1 colliding path (carry per 7.1 C1) |
| `.backlogit/checkpoints/checkpoint-20260924-055708.json` | ?? | W2 | checkpoint (enumerated) |
| `.backlogit/checkpoints/checkpoint-20260924-185101.json` | ?? | W2 | checkpoint (enumerated) |
| `.backlogit/checkpoints/checkpoint-20260925-040601.json` | ?? | W2 | checkpoint (enumerated) |
| `.backlogit/checkpoints/checkpoint-20260925-210801.json` | ?? | W2 | checkpoint (enumerated) |
| `.backlogit/checkpoints/checkpoint-20260926-010145.json` | ?? | W2 | checkpoint (enumerated) |
| `.backlogit/checkpoints/checkpoint-20260926-062644.json` | ?? | W2 | checkpoint (enumerated) |
| `.backlogit/memories.json` | M | W2 | `memories.json` (enumerated) |
| `.backlogit/queue/142-F.md` | M | W2 | `142-F.md` (enumerated) |
| `.backlogit/queue/142-S.md` | M | W2 | `142-S.md` (enumerated) |
| `.backlogit/queue/142.054-T.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.054.001-ST.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.054.002-ST.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.054.003-ST.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.055-T.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.056-T.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.057-T.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.058-T.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.058.001-ST.md` | D | W2 | `142.058.001-ST` archive move, queue-side deletion (enumerated) |
| `.backlogit/queue/142.058.002-ST.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.058.003-ST.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/queue/142.059-T.md` | M | W2 | `142.054-T`-`142.059-T` task/subtask file (enumerated) |
| `.backlogit/reconcile/142-S-pre-20260923-2250.md` | ?? | W2 | Ship pre-claim reconciliation of shipment 142-S; matches W2 heading, not enumerated. Judgement placement |
| `.backlogit/reconcile/142-S-pre-20260924-2008.md` | ?? | W2 | Ship pre-claim reconciliation of shipment 142-S; matches W2 heading, not enumerated. Judgement placement |
| `docs/memory/2026-09-23-ship-142-s-claimed-handoff.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-142-s-f50-harness-blocker.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-142-s-f50-lock-block.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-142-s-f50-red-green.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-142-s-harness-follow-up.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-142-s-harness-gate.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-142-s-option2-policy-reachability.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-24-ship-operator-directed-diagnostics.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-25-ship-142-s-checkpoint-recovery.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-25-ship-142-s-cli-hook-gate-handoff.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-25-ship-142-s-current-f50-gate.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-25-ship-142-s-harness-continuation.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-25-ship-142-s-harness-repair-stall.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-25-ship-142-s-resumed-dogfood.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-26-orchestrator-142-s-preflight-block.md` | ?? | W2 | 142-S execution-state record (Orchestrator-authored; matches W2 heading, not enumerated). Judgement placement |
| `docs/memory/2026-09-26-ship-142-s-001-preflight-typed-stages-checkpoint.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-26-ship-142-s-f50-concrete-verifier-api-block.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-26-ship-142-s-f50-f51-fixture-phase-correction.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-26-ship-142-s-f54-red-f50-gate-blocked.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/memory/2026-09-27-ship-142-058-001-st-structural-completion.md` | ?? | W2 | Ship memory file (enumerated) |
| `docs/scratch/2026-09-26-142-s-restart-handoff.md` | ?? | W2 | 142-S execution-state snapshot (implemented/RED status); matches W2 heading, not enumerated. Judgement placement |

## Counts

| Class | Paths |
|---|---|
| W1 | 72 |
| W2 | 46 |
| W3 | 0 |
| W4 | 0 |
| UNCLASSIFIED | 0 |
| Total | 118 |

Reconciliation with plan section 2 (116 paths at 12:40): +1 `docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md` (created 13:03), +1 this file.

Judgement placements (not enumerated in 7.2): `.backlogit/reconcile/142-S-pre-20260923-2250.md` (W2), `.backlogit/reconcile/142-S-pre-20260924-2008.md` (W2), `docs/memory/2026-09-25-hook-cli-bootstrap-upstream-handoff.md` (W1), `docs/memory/2026-09-26-orchestrator-142-s-preflight-block.md` (W2), `docs/scratch/2026-09-26-142-s-restart-handoff.md` (W2)

P0 RESULT: PASS
