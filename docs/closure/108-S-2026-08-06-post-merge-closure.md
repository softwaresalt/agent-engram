---
title: "108-S cold CLI correlation post-merge closure"
doc_type: closure
shipment_id: "108-S"
feature_id: "112-F"
mode: post-merge
date: 2026-08-06
author: ship
pr: 323
approved_head: "88668b85fc42baf186f2e1666d59cc04ccea2896"
merge_commit: "8e46559d1ed9a85cecd14e55e41c95bc6e473d50"
merged_at: "2026-08-06T16:10:03Z"
decision: BLOCKED
releasability: READY_WITH_CONDITIONS
closure_status: READY
compaction_status: done
---

# 108-S Cold CLI Correlation Post-Merge Closure

## Readiness

**READY WITH CONDITIONS.** PR #323 merged by merge commit
`8e46559d1ed9a85cecd14e55e41c95bc6e473d50`, and that commit is reachable
from `origin/main` with exactly two parents. Shipment `108-S` and its explicit
manifest are archived; the shipment archive records the merge SHA.

This release unit adds bounded characterization and debug-only observability,
not a production timeout fix. Its durable runtime classification remains
**BLOCKED** because shipment `108-S` exhausted its two-run cap before the final
JSON capture remediation could be validated live.

## Invariants to Preserve

- Release builds do not enable test capture.
- Debug capture remains fixed beneath the owned workspace `.engram`; it cannot
  select an arbitrary path or inherit `ENGRAM_DATA_DIR`.
- Response wire bytes and exact JSON-RPC ID echo remain unchanged.
- Shipment `108-S` owns exactly two live attempts; no third run may be
  attributed to it.
- Any fresh live validation or timeout-contract change begins in a new Stage
  cycle.
- Repository daemon state remains observation-only.

## Validator Evidence

| Evidence | Result |
|---|---|
| Runtime report | `docs/closure/108-S-2026-08-05-runtime-verification.md` |
| Live attempts | Two bounded Windows runs completed at the exact `2/2` cap |
| Request/correlation IDs | `62046B37-cold-1` / `62046B37` |
| Cleanup | Both owned PIDs dead and both named pipes unreachable |
| Final JSON remediation | Deterministic non-live coverage only |
| Runtime verdict | `BLOCKED` pending a fresh reviewed live run |
| Hosted CI | PASS — `build` completed successfully at the approved HEAD |
| Pinned topology gate | PASS at autoharness `6a791dbe6d47d044595000fe894c94f051df6ba6` |
| Pinned Copilot gate | `SATISFIED` for the exact approved HEAD |

The structured validator handoff remains `BLOCKED`, while shipment
releasability is `READY_WITH_CONDITIONS`: the reviewed definition of done
explicitly allowed one concrete blocker in place of the missing retained frame
record.

## Pre-Deploy Audit

| Check | Result |
|---|---|
| Approved/current PR HEAD | PASS — `88668b85fc42baf186f2e1666d59cc04ccea2896` |
| Merge strategy | PASS — merge commits enabled; squash and rebase disabled |
| Active ruleset | PASS — only `merge` is allowed |
| Mergeability immediately before merge | PASS — `MERGEABLE` / `CLEAN` |
| Copilot exact-HEAD review | PASS |
| Copilot requested reviewer | PASS — absent |
| Review threads | PASS — five total, zero unresolved |
| CI | PASS — one successful `build` check |
| Merge confirmation | PASS — PR state `MERGED`; merge SHA in `origin/main` |
| Merge topology | PASS — exactly two parents |
| Shipment reconciliation | PASS — pre, safe-close, and post reports completed |
| Archive deletion guard | PASS — no archive deletions |
| Migration, schema, config, or data action | Not applicable |

## Deployment and Post-Deploy Checks

This release unit is merge-only. It has no deployment, migration, feature
flag, daemon restart, reindex, or operator-workspace action.

The post-merge closure branch must pass Markdown, frontmatter, reference, and
backlog integrity checks and merge through a separate reviewed PR. No
additional live daemon run is permitted during this closure.

## Healthy and Failure Signals

Healthy closure signals are:

- the merge SHA remains reachable from `origin/main`;
- `108-S` has `archived_status: shipped`;
- all four explicit manifest members remain archived and terminal;
- the runtime blocker and fresh-intake boundary remain discoverable;
- stashes `9D943A6F` and `12418607`, and deliberation `017-D`, remain active.

Intervention is required if the final JSON remediation is represented as live
proof, the ignored test is run a third time under `108-S`, capture escapes the
owned workspace, release behavior changes, or unrelated work is archived.

## Monitoring Plan and Validation Window

Ship owns repository closure through closure-PR readiness. During that window,
verify backlog reconciliation, documentation integrity, backlog index
synchronization, and Engram synchronization.

Fresh live observation belongs to a new reviewed intake. It must preserve the
fixed IDs, owned workspace/PID/pipe boundary, aggregate deadline, and cleanup
proof recorded by `108-S`.

## Rollback Procedure

If the merged observability change causes a repository regression, create a
dedicated rollback branch from current `main` and reverse only the capture/frame
implementation and focused test registration:

```text
git diff 8e46559d1ed9a85cecd14e55e41c95bc6e473d50^1 8e46559d1ed9a85cecd14e55e41c95bc6e473d50 -- Cargo.toml src/bin/engram.rs src/daemon/ipc_server.rs src/shim/lifecycle.rs tests/integration/cold_cli_request_frame_correlation_test.rs | git apply --reverse
git add -A -- Cargo.toml src/bin/engram.rs src/daemon/ipc_server.rs src/shim/lifecycle.rs tests/integration/cold_cli_request_frame_correlation_test.rs
git commit -m "revert: remove 108-S observability seam"
```

Push the path-scoped revert through a separately reviewed PR. Preserve the
decision, runtime, closure, memory, and backlog evidence from PR #323. No schema
or data rollback is required.

## Risky Action Record

- **Approved merge:** explicit operator approval was recorded at
  `2026-08-06T09:00:47-07:00`. The normal
  `gh pr merge 323 --merge --match-head-commit
  88668b85fc42baf186f2e1666d59cc04ccea2896` path succeeded.
- **Prohibited merge paths:** no admin, force, bypass, auto-merge, squash,
  rebase, or branch deletion was used.
- **Shipment close:** the manifest-scoped non-cascading safe-close archived
  only `108-S`; all manifest members were already archived.
- **Runtime state:** no daemon was stopped, killed, rebound, flushed, or
  mutated during closure.

## Follow-Ups

- `9D943A6F` — fresh Stage intake for one newly authorized, bounded validation
  of the final JSON capture.
- `12418607` — stabilize the unrelated S072 zero-function fixture.
- `017-D` — decide the unrelated `lz4_flex` advisory upgrade.

No follow-up was harvested or modified during closure.

## Reconciliation and Compaction

- Pre: `.backlogit/reconcile/108-S-pre-20260806T091949.md`
- Safe-close: `.backlogit/reconcile/108-S-safe-close-20260806T092313.md`
- Post: `.backlogit/reconcile/108-S-post-20260806T092313.md`
- Compaction: `docs/closure/2026-08-06-108-s-compact-context.md` — `done`

## Knowledge Graduation

The durable decision remains
`docs/decisions/2026-08-05-cold-cli-request-frame-correlation-follow-up.md`.
The reviewed implementation plan and shipment memories are compacted during
this closure. No separate compound learning is required because the reusable
constraints and fresh-intake boundary are already explicit in the decision
record.
