---
title: "PR 337 second adversarial remediation Stage memory"
doc_type: memory
date: 2026-08-11
agent: stage
status: staged-local-uncommitted
branch: chore/stage-publication-dark-factory
shipments: [115-S, 116-S]
references:
  - docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
  - docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md
  - docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md
  - .backlogit/queue/115-S.md
  - .backlogit/queue/116-S.md
---

# PR 337 second adversarial remediation Stage memory

## Session Scope

Applied all four second-rerun MEDIUM findings and every actionable LOW finding to Stage-owned plans, focused reviews, features/tasks, shipment manifests, adversarial/continuity/compaction artifacts, and references. No Rust source/test implementation, template/generated-instruction edit, workflow edit, build/test/lint, config mutation, branch operation, PR/remote action, claim, Ship execution, cleanup, merge, or shipment transition occurred.

## Final Contracts

- `115-S` cannot be claimed until the unrelated tracked `.autoharness/config.yaml` change receives separate operator disposition and the tracked core baseline is clean. This batch never modifies, stages, commits, or infers intent from that file.
- Spike admission requires the sole current core worktree; no cargo/rustc/rust-analyzer/daemon or handle collision; free bytes >= `max(20 GiB, ceil(1.5 * B) + 2 GiB)`; absent/empty workspace-local target; prior-artifact inventory including `tmp/rustsec-2026-0041/data/`; synthetic-only data; and exact owners.
- Windows containment uses workspace-local CARGO_HOME, explicit CARGO_TARGET_DIR, supported cargo-audit `--db`/data controls, tool flags, TMP/TEMP, and unchanged external-directory fingerprints. XDG variables and TMPDIR are Unix-specific supplements.
- Candidate discovery/static inspection is read-only. Candidate build scripts, proc macros, tests, or binaries require explicit operator approval after immutable identity/hash/license/source-delta/inventory/containment evidence. Generic, inferred, auto-check, shipment, and dark-factory approval are invalid; unavailable approval pauses execution.
- Post-spike closure requires process exit, no protected handles, byte restoration, admitted status, canonical path proof, external fingerprints, then separately approved exact targeted cleanup. Cleanup refusal/failure keeps 116-S blocked.
- Feature 120 now treats `templates/instructions/circuit-breaker.instructions.md.tmpl` as source of truth and generated `.github/instructions/circuit-breaker.instructions.md` as synchronized output. Required order is dedicated RED -> template edit -> supported tune-harness synchronization -> GREEN -> clean staged hash match -> second-render no diff.
- 120.002-T uses dedicated `contract_circuit_breaker_policy`, not CLI `contract_verify`. Its workflow acceptance requires least privilege, full SHA pins under current policy, and `persist-credentials: false`.
- Retry semantics remain next-counted-attempt, no reset/pause/parallel counter/fourth run, provisional-to-concrete identity without recounting, genuine observable-error exception, and immediate de-escalation.

## Review Gate

Plan-hardening and focused plan-review were rerun against both final contracts. Constitution, scope, architecture, security/supply-chain, Windows containment, prompt/instruction, template durability, test cohesion, CI security, rollback/cleanup, and operational sequencing found P0 0, P1 0, P2 0, P3 0. Stage gate PASS; final adversarial rerun remains pending.

## Shipment State

- `115-S`: queued; batch `dark-factory-2026-08-10`; order 1; predecessors `[]`; unclaimable on the current dirty tracked baseline.
- `116-S`: queued; same batch; order 2; predecessors `[115-S]`; hard dependency on 115-S plus cleanup-result gate.
- If cleanup is not approved, unavailable, partial, or failed, 116-S remains blocked even after 115-S shipment/archive/merge evidence.
- No shipment was claimed, shipped, closed, returned, or transitioned.

## Publication Allowlist and PR Acceptance

Authorized publication must stage only these exact paths:

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

Never use `git add -A`, `git add .`, or `git commit -a`. Before commit/push, verify `git diff --cached --name-only` is an exact subset of this list and inspect cached content. `.autoharness/config.yaml` and every unrelated working-tree path must be absent. Stage role policy permits planning commits only on default/admin branches, so this Stage session leaves the feature-branch changes unstaged and uncommitted for an authorized publisher.

## Validation and Blockers

Do not rerun broad `autoharness verify-workspace`. Its record shows two distinct invalid usages and one completed validation with 25 pre-existing strict-schema blockers; the universal same-error breaker did not trip. Targeted validations cover schema/frontmatter/references, backlog sync/doctor, DAG, one-worktree wording, admission/cleanup, generated-template durability, retry semantics, and `git diff --check`.

Backlog doctor result must be stated as no new actionable findings while preserving the known historical 43 `archived_from_self_ref` advisory baseline. Markdownlint unavailable is advisory. Current execution blockers are the config disposition/clean baseline for 115-S, exact candidate approval before execution, exact cleanup approval/result before 116-S, the 25 pre-existing strict-schema blockers for broad verification, and final adversarial rerun/publication by an authorized role.
