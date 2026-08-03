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
head: "d6321504137445a94b4134718355b87cceb75db6"
review_fix_cycle: "2/3"
---

## Session scope

Stage-only review-fix cycle 2/3 for Copilot P1 `discussion_r3701136926` / `PRRT_kwDORJEduc6V2olJ` on PR #316 at exact base HEAD `d6321504137445a94b4134718355b87cceb75db6`. The sole core worktree and requested branch/HEAD were verified clean before edits. No source, tests, Cargo, config, `.engram`, build/lint/test, branch/worktree, commit, push, PR reply/resolution, checkpoint, stash, shipment claim/closure, or status requeue occurred.

## Validity assessment

The comment is a valid P1. Current `begin_scan_generation` cancellation reaches only `background_db_hydration`. `index_workspace`, `sync_workspace`, and both watcher loops can keep using their captured database/workspace context after owner clearing. The previous binding-aware plan therefore allowed a successor to acquire while a retired driver was still active, including two same-binding drivers against one database.

Prior accepted rules remain unchanged: same-binding refresh preserves `0b101 OR 0b010 = 0b111`; distinct binding transfers zero old bits and uses new-binding reconciliation plus latest-token reissue; running RAII Drop republishes the authoritative union; success disarms; process abort uses restart reconciliation.

## Decision and exact transition

1. Every Index, Sync, Hydration, Startup, and Watcher permit receives the current generation cancellation receiver and an immutable binding snapshot for its complete DB/file-capable future.
2. Active rebind atomically advances binding/floor and changes `Running(old)` to `Retiring(barrier)`. It signals old cancellation after unlock but does not notify or expose a successor.
3. Same binding moves owner `0b101 OR` pending `0b010` into the barrier's current-generation `deferred = 0b111`. Distinct binding initializes deferred to zero. Ordinary pending is empty while retiring.
4. Current-token requests cannot acquire and OR only into deferred; empty waiters publish no work. A later rebind retargets the same barrier: equal target preserves deferred, distinct target discards superseded-target work.
5. Exact explicit terminal or armed Drop is the acknowledgment only after all DB/file/workspace mutation-capable work exits. It moves deferred to ordinary pending, clears the barrier, writes no timestamp, and performs exactly one unconditional post-unlock wake. It returns no successor permit to the retired driver.
6. The successor then competes through normal request. Later finish/Drop is stale and changes nothing. A non-quiescent driver leaves the barrier closed; no timeout permits overlap.

## Updated artifacts

Updated only the existing Stage remediation artifacts directly required: the hardened plan/review, spike findings, this memory, `109-F`, and blocked tasks `109.014-T`-`109.019-T`, `109.022-T`-`109.025-T`, and `109.031-T`. No status, dependency, parent, owner, assignment, shipment, time, file, function, or scenario cap changed.

`106-S` and `109.013-T` remain active. `104-S`, `109-F`, old `109.001-T`-`109.012-T`, and replacements `109.014-T`-`109.031-T` remain blocked and unclaimed.

## Verification and review

- Exact GitHub thread, path, line, and body read through the GitHub API; the thread remains unresolved.
- Current source paths were read only and confirm the cancellation gap; no Cargo/npm/npx command or build/test/lint ran.
- `.github/agents/stage.agent.md` verified `.Stage` Tier 3/high, `anthropic/claude-opus-4.8`, with no override.
- Targeted backlog doctor checks passed for every modified backlog artifact. Status/assignment and dependency-chain SQL checks passed.
- Frontmatter/heading/reference/cap/forbidden-scope checks and `git diff --check` passed.
- Plan hardening was rerun for cancellation delivery, quiescent acknowledgment, stuck-barrier rollback, monitoring, and all-owner deterministic matrices.
- Fresh configured Stage review-fix cycle 2/3: PASS; open P0/P1/P2/P3 = `0/0/0/0`.

## Suggested reply for Ship

> Accepted and addressed. The plan no longer clears ownership and starts a successor on cancellation alone. Every Index, Sync, Hydration, Startup, and Watcher permit now carries the generation cancellation receiver. Active rebind installs a fail-closed RetirementBarrier: same-binding `0b101 OR 0b010 = 0b111` lives only in its deferred slot, distinct binding carries zero old bits, and new requests coalesce there without acquiring. Only exact terminal/Drop after the retired driver's DB/file-capable work exits publishes deferred, clears the barrier, and wakes once; stale terminals are no-ops. Deterministic OwnerKind × binding × terminal matrices prove no successor-before-ack, max one DB driver, and no post-ack old work within the existing four-scenario caps.

Leave all changes uncommitted for Ship. Do not close `106-S`/`109.013-T`, requeue `104-S`/`109-F`, claim/close a shipment, post the reply, or resolve the thread in Stage.
