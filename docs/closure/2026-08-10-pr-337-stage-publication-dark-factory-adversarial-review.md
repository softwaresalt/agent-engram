---
title: "PR 337 Stage publication dark-factory adversarial review"
doc_type: adversarial-review
status: remediated-stage-pass-final-rerun-pending
date: 2026-08-11
references:
  - docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md
  - docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md
  - .backlogit/archive/119.001-R-rustsec-2026-0041-spike-plan-security-review.md
  - .backlogit/archive/120.001-R-circuit-breaker-diagnostic-policy-plan-review.md
  - .backlogit/queue/115-S.md
  - .backlogit/queue/116-S.md
  - docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md
---

# PR 337 Stage Publication Dark-Factory Adversarial Review

## Review History

The first adversarial rerun at revision `331bdb858629d4c05ee1f1d1758d5bdd26944749` failed with no HIGH consensus, one MEDIUM finding (forbidden second worktree), and LOW retry/containment/approval/dependency/cleanup findings. Stage applied that set locally.

The second adversarial rerun also returned **FAIL**. Three valid reviewers found no HIGH consensus, four MEDIUM findings, and actionable LOW P1/P2/P3 details. This document records the second remediation set and the final local Stage review; it does not claim that a post-remediation adversarial rerun has passed.

## Second Rerun Findings and Resolutions

| Finding | Confidence / severity | Resolution |
|---|---|---|
| M1 generated policy durability | MEDIUM / P2 | Feature 120, both tasks, plan, review, and 116-S now make `templates/instructions/circuit-breaker.instructions.md.tmpl` authoritative; require template-first edit, supported tune-harness synchronization, clean staged hash comparison, and second-render no-diff proof. |
| M2 dark-factory admission/cleanup | MEDIUM / P1 | Security artifacts now require clean claim baseline, sole core worktree, process/handle quiescence, exact disk threshold, empty workspace-local target, prior-artifact inventory, synthetic-only data, owners, child exit, byte/path/fingerprint closure, and separately approved exact cleanup. 116-S also requires cleanup success/verification. |
| M3 path-scoped publication | MEDIUM / P2 | Handoff and PR acceptance use an explicit allowlist; prohibit `git add -A`, `git add .`, and `git commit -a`; require cached-name and cached-content verification excluding config and unrelated files. |
| M4 Windows/XDG accuracy | MEDIUM / P3 | Windows containment is workspace-local CARGO_HOME, explicit CARGO_TARGET_DIR, supported cargo-audit `--db`/data paths, tool flags, TMP/TEMP, and unchanged external fingerprints. XDG/TMPDIR are Unix-only supplements. |
| LOW exact-candidate approval | LOW / P1 | Discovery/static inspection is read-only. Any candidate build script, proc macro, test, or binary requires explicit operator approval after immutable identity/hash/license/delta/inventory/containment evidence. No generic, inferred, or auto-check approval. |
| LOW dirty baseline | LOW / P1 | 115-S remains queued/unclaimable until the unrelated tracked config receives separate operator disposition and tracked status is clean. Stage publication may remain path-scoped. |
| LOW breaker record | LOW / P2 | The verifier memory now records two distinct invalid usages and one completed validation with pre-existing blockers; the same-error breaker did not trip. |
| LOW completeness | LOW / P3 | Added `data/` inventory, workflow security, dedicated policy target, accurate doctor wording, and machine-readable references. |

## Stage Plan-Harden and Plan-Review Gate

Both revised plans retain explicit `ProposedAction`, `ActionRisk`, approval, rollback, and closure contracts. Constitution, scope, architecture, security/supply-chain, Windows containment, prompt/instruction, template durability, test cohesion, CI security, rollback/cleanup, and operational-sequencing lenses found P0 0, P1 0, P2 0, P3 0 after this remediation.

**Stage gate: PASS. Final adversarial rerun: pending.** Focused re-review evidence is in `119.001-R` and `120.001-R` and appended to both plans.

## Ordered Batch Contract

- `115-S`: `operator_batch=dark-factory-2026-08-10`, order `1`, predecessors `[]`, queued and unclaimable while the tracked config baseline is dirty.
- `116-S`: same batch, order `2`, predecessors `[115-S]`, hard edge `116-S -> 115-S`, plus successful exact cleanup approval/result/verification.
- A cleanup that is not approved, unavailable, partial, or failed keeps `116-S` blocked even when `115-S` has shipment/archive/merge evidence.

## Publication and PR Acceptance

Only the following local Stage batch paths may be staged by an authorized publisher:

```text
.backlogit/archive/119.001-R-rustsec-2026-0041-spike-plan-security-review.md
.backlogit/archive/120.001-R-circuit-breaker-diagnostic-policy-plan-review.md
.backlogit/checkpoints/checkpoint-20260811-190814.json
.backlogit/memories.json
.backlogit/queue/115-S.md
.backlogit/queue/116-S.md
.backlogit/queue/119-F.md
.backlogit/queue/119.001-T.md
.backlogit/queue/119.002-T.md
.backlogit/queue/119.003-T.md
.backlogit/queue/120-F.md
.backlogit/queue/120.001-T.md
.backlogit/queue/120.002-T.md
docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md
docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md
docs/memory/2026-08-10/stage-next-security-diagnostics-policy-memory.md
docs/memory/2026-08-11/circuit-break-autoharness-verify-workspace.md
docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md
docs/memory/compacted/2026-08-11-pr-337-stage-remediation-compaction-report.md
```

`git add -A`, `git add .`, and `git commit -a` are prohibited. Stage only the named paths. Before commit or push, run `git diff --cached --name-only` and reject any path outside the allowlist. Inspect `git diff --cached --` content and prove `.autoharness/config.yaml` plus every unrelated working-tree path is absent. No plan or acceptance criterion may depend on the config's values. If cached verification fails, unstage the offending path and stop; do not widen the allowlist.

## Validation Contract

Do not rerun the already diagnosed broad `autoharness verify-workspace`. Use targeted artifact schema/frontmatter/reference validation, backlog sync and doctor, DAG/topology wording, one-worktree/admission/cleanup contract scans, template durability and retry-semantic scans, workflow security inspection, and `git diff --check`. Report the 25 pre-existing strict-schema blockers separately. Backlog doctor language is `no new actionable findings`; preserve the known historical 43 `archived_from_self_ref` advisory baseline. Markdownlint unavailability is advisory.
