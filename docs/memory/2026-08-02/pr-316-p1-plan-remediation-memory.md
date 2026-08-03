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
head: "897406cc79896eef90d3a44645804d691c3aff96"
---

## Session scope

Stage-only remediation of the two valid unresolved P1 plan comments on PR #316. No source, tests, Cargo, build/lint/test, branch/worktree, Git commit/push/PR/thread resolution, shipment claim/closure, or status requeue occurred. The sole worktree and exact PR head were verified before edits.

## Accepted P1 findings

- `discussion_r3700752674`: generation advance did not retire a prior-generation owner or preserve its owned/pending work.
- `discussion_r3700752695`: a non-cloneable permit without mandatory cleanup could be dropped by cancellation/panic and strand ownership.

## Decisions

1. Keep PIVOT strategy A and crate-private `GenerationToken`, full `WorkMask`, `OwnerKind`, and `OwnerPermit`; add no public semver/wire/schema/config/persistence surface.
2. `AppState` owns a private `Arc<CoordinatorCell>` containing one recoverable synchronous coordinator mutex and `Notify`; each non-cloneable permit owns the cell plus an exact identity and armed cleanup bit.
3. Generation/binding advance prepares and checks first, acquires binding guards in fixed order, then performs one no-await coordinator transition: retire old identity; compute `old owner mask OR old pending`; publish that union under the new binding/floor; swap cancellation ownership; signal old cancellation; leave owner empty; notify at most one after unlock. It never installs an owner without returning a permit.
4. Exact completion transfers/releases once, writes `last_indexed_at` once in the same successful transition, and disarms old Drop. Stale explicit completion returns `Stale` and mutates nothing.
5. Armed Drop is mandatory cancellation/panic cleanup: exact identity republishes authoritative owner mask OR pending, clears owner once, unlocks, and notifies once. It never awaits, spawns, timestamps, or panics. Stale Drop is no-op and cannot affect replacement.
6. Pre-acquisition hydration cancellation is a zero-permit/no-mutation path; post-acquisition cancellation/panic relies on RAII. Normal and handled failure explicitly complete/disarm.
7. Process abort does not run Drop. Restart bind/hydration plus offline-change detection reconciles durable files; revalidate/backfill intent is reissued. Full-unit revert/restart is rollback. No exactly-once external-side-effect claim crosses process death or binding identity.

## Backlog changes

Updated blocked feature `109-F` and blocked replacement tasks: `109.014-T`–`109.021-T`, `109.023-T`–`109.025-T`, `109.027-T`, `109.030-T`, and `109.031-T`. Dependencies remain the original chain `109.013-T -> 109.014-T -> ... -> 109.031-T`; every replacement remains blocked, parented by `109-F`, within two files/four scenarios/110 minutes.

No status or shipment mutation: `106-S` and `109.013-T` remain active; `104-S` and `109-F` remain blocked.

## Verification and review

- Exact PR threads and PR head read through GitHub API.
- `.github/agents/stage.agent.md` verified `.Stage` routes to Anthropic `claude-opus-4.8`, Tier 3/high reasoning; no override supplied.
- Backlog doctor: no duplicate/orphan findings.
- Structural check: required lifecycle obligations present; all replacement statuses blocked; parent/dependency chain exact; scenario caps <=4.
- Plan hardening rerun for RAII, generation retirement, process-abort boundary, monitoring, and rollback.
- Fresh plan review cycle 1: PASS, open P0/P1/P2/P3 = 0/0/0/0. Earlier PASS is marked superseded.

## Ship handoff and PR thread guidance

Ship should commit/push only the uncommitted planning/backlog/docs changes, then reply without resolving on Stage's behalf until GitHub recognizes the pushed diff.

**Thread `discussion_r3700752674` suggested response:**

> Addressed in the remediated findings, plan, and tasks 109.016/109.017. Generation/binding advance now has one no-await coordinator-lock transition that retires the prior identity, promotes the authoritative retiring-owner WorkMask OR pending WorkMask to the new binding/floor, leaves no inaccessible owner, and wakes at most one after unlock. Separate deterministic fixtures cover active owner 0b101 + pending 0b010 -> one 0b111 current-generation pending mask and prove stale old explicit finish/Drop cannot mutate the replacement.

**Thread `discussion_r3700752695` suggested response:**

> Addressed with a crate-private RAII OwnerPermit backed by Arc<CoordinatorCell>. Successful exact completion disarms Drop and timestamps once. Cancellation/panic Drop synchronously and poison-safely republishes the authoritative owned mask OR pending, clears the exact owner once, and notifies one after unlock; stale Drop is a no-op. Deterministic tasks 109.014/109.015 and 109.020/109.021 cover completion disarm, dropped-owner successor recovery, and no second drain. Process abort is explicitly restart reconciliation plus non-durable intent reissue, not a false Drop guarantee.

Do not close `106-S`/`109.013-T`, requeue `104-S`/`109-F`, claim/close a shipment, or resolve either PR thread in this Stage handoff.

## Compact-context assessment

Invoked after memory persistence. `docs/memory/` contains 92 files / 441,937 bytes and 109-F has only two checkpoints (one active, one resolved). Active 109-F artifacts must be preserved; compacting unrelated completed work would violate this session's frozen scope. Files compacted: 0.
