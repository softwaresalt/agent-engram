---
title: "Stage next: advisory spike and diagnostic policy batch"
doc_type: memory
date: 2026-08-10
agent: stage
status: staged-remediated
operator_batch: dark-factory-2026-08-10
shipments: [115-S, 116-S]
references:
  - docs/exec-plans/2026-08-10-rustsec-2026-0041-remediation-spike-plan.md
  - docs/exec-plans/2026-08-10-circuit-breaker-diagnostic-escalation-plan.md
  - docs/closure/2026-08-10-pr-337-stage-publication-dark-factory-adversarial-review.md
  - docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md
---

# Stage next: advisory spike and diagnostic policy batch

## Session Scope

Originally harvested stash `27F691AE`, `241B503F`, and deliberation `017-D` into reviewed plans, features/tasks, and queued shipments. The 2026-08-11 adversarial remediation supersedes execution wording in this memory; the governing details are the referenced final plans, focused reviews, manifests, and final Stage handoff. No source/test/config implementation, claim, Ship execution, cleanup, PR, or merge occurred in Stage.

## Reviewed Hierarchies

### 115-S — RUSTSEC-2026-0041 feasibility spike

- `119-F`
- `119.001-T`: <=110m admission, immutable evidence, read-only candidate discovery, exact approval stop
- `119.002-T`: <=110m one explicitly approved prototype in the sole current core worktree
- `119.003-T`: <=110m synthetic compatibility, restoration, process/path/fingerprint closure, and cleanup disposition
- `119.001-R`: second focused Stage re-review PASS; final adversarial rerun pending

### 116-S — Circuit-breaker diagnostic policy

- `120-F`
- `120.002-T`: <=110m dedicated `contract_circuit_breaker_policy` RED target plus narrow secure workflow
- `120.001-T`: <=110m template-first policy edit, supported generated synchronization, clean staging/hash and second-render proof
- `120.001-R`: second focused Stage re-review PASS; final adversarial rerun pending

## Ordered Batch

- `115-S`: queued; `operator_batch=dark-factory-2026-08-10`; order `1`; predecessors `[]`. It is unclaimable until the unrelated tracked `.autoharness/config.yaml` change receives separate operator disposition and tracked status is clean.
- `116-S`: queued; same batch; order `2`; predecessors `[115-S]`; hard dependency `116-S -> 115-S`. It requires 115-S shipment/item archive/merge evidence **and** separately approved successful cleanup with passing absence/path/external-fingerprint verification. Cleanup unavailable/not-approved/failed keeps 116-S blocked.

Exact ordered metadata is unchanged. Neither shipment has ever been claimed.

## Final Admission and Durability Decisions

- Spike execution requires the sole current core worktree, quiescent cargo/rustc/rust-analyzer/daemon/handles, free bytes >= `max(20 GiB, ceil(1.5 * B) + 2 GiB)`, an absent/empty workspace-local target, prior-artifact inventory including `tmp/rustsec-2026-0041/data/`, synthetic-only data, and named owners.
- Windows uses workspace-local CARGO_HOME, explicit CARGO_TARGET_DIR, supported cargo-audit/tool paths, TMP/TEMP, and unchanged external fingerprints. XDG/TMPDIR are Unix supplements only.
- Candidate discovery/static inspection is read-only. Candidate build scripts, proc macros, tests, or binaries require explicit operator approval after exact identity/hash/license/delta/inventory/containment evidence. No generic, inferred, or auto-check approval.
- Cleanup is a separately approved exact destructive action after byte restoration, child-process exit, no handles, path/fingerprint verification, and artifact inventory.
- `templates/instructions/circuit-breaker.instructions.md.tmpl` is the policy source of truth; `.github/instructions/circuit-breaker.instructions.md` is generated. Use template-first edit -> supported tune-harness sync -> targeted GREEN -> clean staged hash -> second-render no diff.
- The policy plan does not depend on `.autoharness/config.yaml` values.

## Evidence and Validation Wording

Backlog doctor is recorded as **no new actionable findings**, preserving the known historical 43 `archived_from_self_ref` advisory baseline. The broad verifier record contains two distinct invalid usages and one completed validation exposing 25 pre-existing strict-schema blockers; the universal same-error breaker did not trip, and the operator prohibited rerunning that broad command. Use targeted validation only. Markdownlint unavailability is advisory.

## Publication Continuity

The original local-main publication attempt (`156c85ee`) and GH013 rejection are historical. Current remediation is on `chore/stage-publication-dark-factory`. Authorized publication must use the exact allowlist in `docs/memory/2026-08-11/pr-337-adversarial-remediation-memory.md`; never use `git add -A`, `git add .`, or `git commit -a`. Verify cached names/content exclude `.autoharness/config.yaml` and all unrelated files. Stage role policy leaves this feature-branch batch uncommitted for an authorized publisher.
