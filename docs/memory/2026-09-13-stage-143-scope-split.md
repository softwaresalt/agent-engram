# Stage session — 143 scope split (Defect 2 reduced, Defect 1 returned)

**Date**: 2026-09-13
**Branch**: `chore/143-s-stage-checkpoint-lifecycle-continuity`
**Commit**: `48e4f49cbb5bc37ec3d1b117e8552c66efaf39c8`
**Outcome**: HALTED — plan-review circuit open, no harvest

## What was done

Executed the operator-approved split of the 143 release unit.

- **Defect 2** (non-orphaning checkpoint resolution ordering + last-mile
  recovery) reduced to its own decision, plan (rev 8) and hardening (rev 8).
- **Defect 1** (dark-mode continuation auto-routing) removed from implementation
  scope, returned to a fresh **open** deliberation.
- Three prior combined artifacts marked `status: superseded` with supersession
  links; review history retained as evidence.
- Archived stash provenance corrected: `4EF24729` → `reason: harvested`,
  `harvested_artifact_id: 143-F`; `A1D95672` → `reason: returned-to-deliberation`
  with a pointer to the new deliberation.
- `143-F` and `143-S` annotated truthfully; shipment marked DO NOT CLAIM.
- PR #396 body updated (metadata only): outcome stays `BLOCKED`, Reviewed HEAD
  refreshed to the pushed SHA.

## Why it halted

Independent four-persona panels (Scope `gpt-5.6-sol`, Constitution
`claude-opus-4.8`, Correctness `gemini-3.8-flash`, Parity `grok-4.6`) reviewed
revisions 6, 7 and 8. All three returned FAIL. Attempt counter reached 3 →
P-013.6 escalation fired, route `gpt-5.6-sol`/`openai`/`xhigh`, same-route guard
not triggered. Per the protocol the failing operation was **not** re-executed.

Each round genuinely closed the prior round's findings — nothing regressed, and
no panel falsified the design. The recurring failure mode is that the plan
specifies edits to `_ship.agent.md` **Step 5** against a *prose description* of
that step, while the real item ordering (readiness items 7b/7c sitting before
runtime verification, closure-artifact generation, follow-up writes and the push
in items 7–10) is more entangled than the description admits.

## Outstanding P1s carried to escalation

1. **RQ-6 has no executable enforcement path.** Real Step 5 item 15 re-runs the
   P-018 gate and re-queries `headRefOid` only — it never re-fetches required CI
   and does not refresh all threads when P-018 is disabled. U4 AC9 nonetheless
   requires item 15 to stay unmodified.
2. **Zero-checkpoint bypass claim is false.** U4 AC3 promises the pre-existing
   path runs unchanged; AC2/AC4 necessarily reorder the common path.
3. **D14 not folded into a task criterion.** Working-tree placement (checkout +
   verification) is absent from U5's ACs and from V1–V11, so U5 could pass while
   recovery is still on `main`.

## Recommended next step

Decompose U4 against a **verbatim extract of the real Step 5 item list** rather
than a prose description of it, then re-review. This is the escalation question.

## Do not repeat

- `backlogit shipment` has no archive/withdraw verb; `ship` only closes a
  *released* shipment. A queued shipment is made non-claimable by annotation,
  which is the existing convention in this repo.
- `backlogit archive <id>` hangs on the remote update check; pass
  `--no-update-check`.
- `gh pr list` is **not** an exhaustive discovery command: no `--paginate`,
  `--limit` defaults to 30, and no `body` unless requested.
