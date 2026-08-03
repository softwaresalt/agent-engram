---
title: "PR #316 P1 plan remediation memory"
type: session-memory
date: 2026-08-02
agent: .Stage
model_provider: anthropic
model_family: claude-opus-4.8
model_override: none
status: complete-awaiting-ship-handoff
feature: "109-F"
shipment: "104-S"
pr: 316
head: "2f267d9c617243dd70cbaac9837826a4fd0358e9"
review_fix_cycle: "3/3"
---

## Session scope

Final Stage-only review-fix cycle 3/3 for Copilot P1 `discussion_r3701238147` / `PRRT_kwDORJEduc6V25-i` on PR #316 at exact HEAD `2f267d9c617243dd70cbaac9837826a4fd0358e9`. The sole core worktree, requested branch, exact HEAD, and clean start were verified. No source, tests, Cargo, config, `.engram`, build/lint/test, branch/worktree, commit, push, PR reply/resolution, checkpoint, stash, new task, new memory file, shipment claim/closure, or status requeue occurred.

## Validity assessment

The comment is a valid P1. A current empty Hydration/Startup/Watcher request can return `Queued` behind a `Running` owner and wait on `Notify`. The prior exact no-pending success path cleared the owner and selected `Released` without notification. With no pending bits, no later transition was guaranteed, so all empty waiters could remain stranded.

All accepted contracts remain unchanged: same-binding `0b101 OR 0b010 = 0b111` lives only in `RetirementBarrier.deferred` until quiescence acknowledgment; distinct binding carries zero old bits; every OwnerKind observes cancellation and acknowledges DB/file-capable exit before a successor; armed Drop and retirement acknowledgment notify once after unlock; success disarms; stale terminals are no-ops; process abort uses restart reconciliation/full rollback.

## Exact release and baton decision

1. Every empty Hydration/Startup/Watcher wait loop creates and enables `Notified` before its final `request`/recheck. `Queued` awaits that registration; after a wake, a fresh registration is enabled before rechecking.
2. Exact `Running` completion with no pending work clears owner, disarms and timestamps once, selects `Released`, drops the mutex, invokes `notify_one` exactly once, then returns the selected outcome.
3. One notification permits at most one mutex-authorized acquisition. It is not ownership authority. If a producer wins, the empty waiter remains queued and blocks; no polling occurs.
4. With multiple empty waiters, each call resumes at most one; one registered waiter eventually acquires an empty permit through the mutex recheck. Remaining waiters stay registered. That empty owner completes once as `Released` and emits the next post-unlock notification, passing a one-owner baton until all rows progress.
5. Exactness is the number of `notify_one` calls, not resumed tasks: Tokio may wake at most one registered waiter or retain/coalesce one permit. Tests assert exact calls, at-most-one acquisition per call, finite progress, no busy loop, no duplicated work/queue state, and no concurrent drivers.
6. Running Drop and retirement acknowledgment keep their one-call post-unlock rule. Stale terminals notify zero times.

## Updated artifacts

Updated only the existing Stage remediation artifacts directly required: the hardened plan/review, spike findings, this memory, `109-F`, and blocked tasks `109.014-T`, `109.015-T`, `109.018-T`, `109.019-T`, `109.020-T`, `109.021-T`, `109.024-T`, `109.025-T`, and `109.031-T`. No status, dependency, parent, owner, assignment, shipment, time, file, function, or scenario cap changed.

`106-S` and `109.013-T` remain active. `104-S`, `109-F`, old `109.001-T`-`109.012-T`, and replacements `109.014-T`-`109.031-T` remain blocked and unclaimed.

## Verification and review

- Exact GitHub comment/thread read through the GitHub API; `PRRT_kwDORJEduc6V25-i` remains unresolved.
- `.github/agents/stage.agent.md` verified `.Stage` Tier 3/frontier, high reasoning, `anthropic/claude-opus-4.8`, with no override.
- Contained frontmatter/heading/reference/cap/forbidden-scope, backlog doctor, status/assignment/dependency-chain, and `git diff --check` validations passed.
- Plan hardening was rerun for successful release, pre-registration, multi-waiter baton, notification counting, monitoring, rollback, and deterministic fixture coverage.
- Fresh configured final Stage review-fix cycle 3/3: PASS; open P0/P1/P2/P3 = `0/0/0/0`.

## Suggested reply for Ship

> Accepted and addressed. Exact no-pending `Running` completion now clears owner, selects `Released`, drops the mutex, and invokes `notify_one` once. Empty Hydration/Startup/Watcher waiters enable notification before their final request/recheck. One wake permits at most one mutex-authorized empty acquisition; remaining waiters stay registered, and each acquired empty owner completes `Released` to pass the baton. Deterministic single- and multi-waiter fixtures prove progress, no spin, no duplicate queue/work state, and no concurrent drivers within existing scenario caps. Exactness applies to the notification call, not task resumption. All prior binding, cancellation/quiescence, RAII, stale-terminal, and process-abort contracts remain unchanged.

Leave all changes uncommitted for Ship. Do not close `106-S`/`109.013-T`, requeue `104-S`/`109-F`, claim/close a shipment, post the reply, or resolve the thread in Stage.
