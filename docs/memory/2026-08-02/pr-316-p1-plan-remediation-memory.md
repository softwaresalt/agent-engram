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
head: "436436587d7383bf4f97a2699b8aa473703d37df"
---

## Session scope

Stage-only planning remediation of six new Copilot threads on PR #316 at exact head `436436587d7383bf4f97a2699b8aa473703d37df`. No source, tests, Cargo, config, `.engram`, build/lint/test, branch/worktree, Git commit/push/PR reply/thread resolution, checkpoint, stash, shipment claim/closure, or status requeue occurred. The sole core worktree, branch, exact head, and clean starting tree were verified before edits.

## Validity assessment

All six comments are valid and represent one P1 contract contradiction:

- `discussion_r3700910169` and `discussion_r3700910181`: the plan and findings promoted old owner/pending masks unconditionally even though `src/tools/lifecycle.rs:850-878` requires newer-binding publication to replace old companion flags.
- `discussion_r3700910197`: `109-F` repeated the unsafe rule.
- `discussion_r3700910205`: `109.016-T` hard-coded the same-binding `0b111` result without a distinct-binding row.
- `discussion_r3700910211`: `109.017-T` omitted the binding-equality branch and concrete new-workspace reconciliation.
- `discussion_r3700910215`: this memory recorded the same unsafe unconditional union.

The same-binding union proof remains required; the flaw was applying it across a distinct binding.

## Decisions

1. Keep PIVOT strategy A and crate-private `GenerationToken`, exact private `BindingIdentity`, full `WorkMask`, `OwnerKind`, and RAII `OwnerPermit`; add no public semver/wire/schema/config/persistence surface.
2. `BindingIdentity` is derived from the fully prepared stable workspace UUID plus path/branch workspace ID. Generation may advance while this exact binding remains equal.
3. One prepared, no-await coordinator publication retires old identity and branches under the lock:
   - same binding: publish authoritative old owner mask OR old pending under the new generation, specifically `0b101 OR 0b010 = 0b111`, leave owner empty, and notify at most one after unlock;
   - distinct binding: signal old cancellation and publish none of the old routine/revalidate/backfill mask, with no coordinator wake for discarded work.
4. For a distinct binding, startup bind/hydration/offline-change detection reconciles durable file state. Non-durable revalidate/backfill intent is reissued only through a new-token-qualified request after a producer determines it applies to the new binding; otherwise old intent is discarded.
5. Exact completion transfers/releases once, writes `last_indexed_at` once, and disarms old Drop. Current same-binding cancellation/panic Drop republishes owner mask OR pending. After replacement, stale explicit finish and stale Drop mutate nothing in either binding relation.
6. Process abort does not run Drop. Restart bind/hydration plus offline-change detection reconciles durable files; applicable non-durable intent is reissued against the current binding. Full-unit revert/restart is rollback.

## Backlog and document changes

Updated only the six paths named by the threads: the findings, hardened plan/review, `109-F`, `109.016-T`, `109.017-T`, and this existing remediation memory. The `109.016/017` coverage remains within four scenarios by using a binding-relation fixture matrix and a stale-terminal fixture matrix rather than adding scenarios.

No status, dependency, parent, ownership, assignment, or shipment mutation: `106-S` and `109.013-T` remain active; `104-S`, `109-F`, old `109.001-T`-`109.012-T`, and replacements `109.014-T`-`109.031-T` remain blocked and unclaimed.

## Verification and review

- Exact PR threads and exact head read through the GitHub API.
- Existing lifecycle contract verified at `src/tools/lifecycle.rs:850-878`; current pending-state comments also specify newer-generation replacement and same-generation sticky OR.
- `.github/agents/stage.agent.md` verified `.Stage` Tier 3/high reasoning routing to `anthropic/claude-opus-4.8`; no override supplied.
- Contained frontmatter, path/reference, dependency/status, task-cap, and forbidden-scope checks passed.
- Plan hardening rerun for binding identity, cross-workspace reconciliation, monitoring, and rollback.
- Fresh configured plan review cycle 2: PASS; open P0/P1/P2/P3 = `0/0/0/0`. Cycle 1 is marked superseded.

## Suggested replies for Ship

**`discussion_r3700910169` (plan)**

> Accepted and addressed. The plan now separates generation from exact binding identity. Same-binding retirement preserves the authoritative union (`0b101 OR 0b010 = 0b111`); a distinct workspace binding transfers zero old routine/revalidate/backfill bits, cancels old work, and uses new-binding startup bind/hydration/offline-change detection plus qualified non-durable reissue. `109.016` adds the cross-workspace row without exceeding four scenarios.

**`discussion_r3700910181` (findings)**

> Accepted and addressed in the decision. Binding equality is now explicit. Only a same-binding generation refresh promotes owner mask OR pending; a distinct binding discards the old-workspace mask, signals cancellation, reconciles durable files through the new lifecycle, and accepts companion intent only through a new-token request.

**`discussion_r3700910197` (`109-F`)**

> Addressed. The feature summary now states both outcomes: same-binding `0b101 OR 0b010 = 0b111`, and distinct-binding zero carryover with startup bind/hydration/offline-change reconciliation and new-token-qualified reissue. Status and dependency fields remain unchanged.

**`discussion_r3700910205` (`109.016-T`)**

> Addressed with two parameterized matrices inside the existing four-scenario cap. The binding-relation matrix proves same-binding `0b111` preservation and distinct-binding zero old-bit transfer/reconciliation; the stale-terminal matrix proves old finish and Drop cannot mutate either replacement. No scope expansion or live DB race proof was added.

**`discussion_r3700910211` (`109.017-T`)**

> Addressed. The GREEN task now derives exact binding identity and branches atomically: promote the full union only for the same binding; for a distinct binding, cancel/retire old work, publish no inherited mask, and let the new startup/hydration/offline-change path reconcile durable state. Non-durable intent requires a new-token request.

**`discussion_r3700910215` (memory)**

> Addressed. The remediation memory no longer records unconditional union promotion. It now preserves the required same-binding `0b111` proof while recording zero cross-binding carryover, explicit durable reconciliation, qualified non-durable reissue, and stale finish/Drop isolation.

Leave these changes uncommitted for Ship. Do not close `106-S`/`109.013-T`, requeue `104-S`/`109-F`, claim/close a shipment, post replies, or resolve threads in this Stage handoff.
