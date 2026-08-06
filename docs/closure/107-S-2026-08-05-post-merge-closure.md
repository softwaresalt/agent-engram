---
title: "107-S daemon index and IPC boundary post-merge closure"
doc_type: closure
shipment_id: "107-S"
feature_id: "111-F"
mode: post-merge
date: 2026-08-05
author: ship
pr: 321
approved_head: "f4bb34dcb782bc26b4f5784b558854a98438e628"
merge_commit: "ebc7f699bbee669009f2557246021d10f7084adc"
merged_at: "2026-08-05T22:44:37Z"
decision: PARTIAL
releasability: READY_WITH_CONDITIONS
closure_status: READY
compaction_status: done
---

# 107-S Daemon Index and IPC Boundary Post-Merge Closure

## Readiness

**READY WITH CONDITIONS.** PR #321 merged by merge commit
`ebc7f699bbee669009f2557246021d10f7084adc`, and that commit is reachable
from `origin/main`. Shipment `107-S` and its explicit manifest are archived;
the shipment archive records the merge SHA for release traceability.

The shipped release unit is investigation and characterization coverage, not a
production fix. Its durable decision remains **PARTIAL**: the controlled
persistence symptom did not reproduce, while cold CLI timeout and end-to-end
request/frame correlation remains a named blocker.

## Invariants to Preserve

- Do not reinterpret the validated persistence non-reproduction as a global
  proof that no daemon persistence defect can exist.
- Keep the persistence and IPC findings separate; evidence for one does not
  close the other.
- Keep the retained live characterization ignored and opt-in unless a fresh,
  reviewed investigation explicitly authorizes another bounded run.
- Every future live probe uses an owned temporary workspace, one owned daemon,
  one endpoint, and a five-minute aggregate cap with at most two equivalent
  attempts.
- The repository daemon, binding, endpoint, and persisted graph remain
  observation-only.
- No production protocol, schema, persistence, startup, or timeout behavior is
  claimed to have changed in this shipment.

## Validator Evidence

| Evidence | Result |
|---|---|
| Controlled persistence runs | PASS — two owned-daemon runs retained one `calls_resolved_singleton` before flush, after flush, and after graceful shutdown |
| Persistence classification | `no current defect` for the validated bare-call corpus |
| IPC classification | `startup-outside-deadline` by static contract inspection |
| IPC runtime evidence | BLOCKED — cold CLI end-to-end request-ID/frame correlation was not retained before the two-run cap |
| Local verification | formatting, strict pedantic clippy, `cargo dev-test` (538 library tests), focused characterization and stale-PID suites passed |
| Hosted CI | PASS — `build` completed successfully at approved HEAD |
| Pinned topology gate | PASS at autoharness source `6a791dbe6d47d044595000fe894c94f051df6ba6`, without force |
| Pinned Copilot gate | `SATISFIED` for approved HEAD; zero unresolved threads |
| Decision verification | PASS — `engram verify` recorded before merge |

The structured validator verdict is `PASS_WITH_FOLLOW_UP`: all evidence promised
by the bounded investigation is preserved, and the missing runtime correlation
is explicit rather than inferred.

## Pre-Deploy Audit

| Check | Result |
|---|---|
| Approved/current PR HEAD | PASS — `f4bb34dcb782bc26b4f5784b558854a98438e628` |
| Merge strategy | PASS — merge commits enabled; squash and rebase disabled |
| Mergeability immediately before merge | PASS — `MERGEABLE` / `CLEAN` |
| Copilot review at exact HEAD | PASS |
| Copilot absent from requested reviewers | PASS |
| Unresolved review threads | PASS — zero |
| CI | PASS — one successful `build` check |
| Merge confirmation | PASS — PR state `MERGED`, merge SHA in `origin/main` |
| Shipment reconciliation | PASS — pre, safe-close, and post reports completed |
| Archive deletion guard | PASS — no archive deletions |
| Migration, schema, config, or data action | Not applicable |

## Deployment and Post-Deploy Checks

This release unit is merge-only. It has no deployment, migration, feature flag,
daemon restart, reindex, or operator-workspace action.

The closure branch must pass Markdown/frontmatter/reference gates and merge
through its own reviewed PR. No additional live daemon run is required to close
this investigation shipment.

## Healthy and Failure Signals

Healthy closure signals are:

- the merge SHA remains reachable from `origin/main`;
- `107-S` has `archived_status: shipped`;
- all five explicit manifest members remain archived and terminal;
- the PARTIAL decision and its four active follow-ups remain discoverable;
- unrelated blocked work remains unchanged.

Intervention is required if the decision is represented as FIX-READY, the
ignored live probe becomes default, any follow-up is silently removed, archive
scope expands beyond the manifest, or a closure artifact claims production
timeout/persistence behavior changed.

## Monitoring Plan and Validation Window

Ship owns the repository-closure window through closure-PR readiness. During
that window, verify backlog reconciliation, documentation integrity, backlog
index synchronization, and Engram synchronization. The next fresh runtime
investigation owns any live daemon observation; it must use the bounded controls
in the decided plan.

An initial default-timeout `engram sync` expired, and `--direct` correctly
refused to run while the daemon remained live. A bounded CLI retry with
`--timeout 300` then completed in 6.542 seconds with 340 unchanged files and no
errors. This closure observation is not new evidence about the historical
bounded characterization.

## Rollback Procedure

If the merged characterization causes a repository regression, create a
dedicated rollback branch from current `main` and run:

```text
git revert --no-edit -m 1 ebc7f699bbee669009f2557246021d10f7084adc
```

Push the complete-unit revert through a separately reviewed PR. Do not
partially revert the test harness and decision evidence. No schema or data
rollback is required.

## Risky Action Record

- **Approved merge:** explicit operator approval was recorded at
  `2026-08-05T15:33:15-07:00`. The normal `gh pr merge 321 --merge` path
  succeeded; no admin, force, auto-merge, squash, rebase, or branch deletion was
  used.
- **Shipment close:** the manifest-scoped non-cascading safe-close archived only
  `107-S`; all manifest members were already archived. The cascade shipment
  command was not used.
- **Runtime state:** no daemon was stopped, killed, rebound, flushed, or mutated
  during closure.

## Follow-Ups and Source Cleanup

No new follow-up was created; the existing active entries remain authoritative:

- `62046B37` — complete cold CLI timeout and end-to-end request-ID/frame
  correlation.
- `12418607` — stabilize the unrelated S072 smoke fixture.
- `9A4D18E9` — refactor the oversized retained characterization without widening
  live coverage.
- `017-D` — decide the unrelated `lz4_flex` advisory upgrade.

Source stash `5765BAAB` is already absent from the live stash index, and source
deliberation `015-D` is already archived/done. Closure-repair stash `4CD6335D`
was already archived. No source or follow-up artifact required mutation.

## Reconciliation and Compaction

- Pre: `.backlogit/reconcile/107-S-pre-20260805T154800.md`
- Safe-close: `.backlogit/reconcile/107-S-safe-close-20260805T155200.md`
- Post: `.backlogit/reconcile/107-S-post-20260805T155200.md`
- Compaction: `docs/closure/2026-08-05-107-s-compact-context.md` — `done`
- Checkpoints: two completed-work handoffs resolved; zero active shipment
  checkpoints preserved

## Knowledge Graduation

The reviewed implementation plan was reduced to the durable decided plan at
`docs/exec-plans/2026-08-04-daemon-index-runtime-root-cause-decided-plan.md`;
the verbose source remains archived. Shipment memories were consolidated into
`docs/memory/compacted/2026-08-05-107-s-111-f-compacted.md`.

No compound refresh is required: the shipment did not supersede an existing
compound learning, and its reusable controls and rejected alternatives are
already preserved in the decision and decided-plan artifacts.
