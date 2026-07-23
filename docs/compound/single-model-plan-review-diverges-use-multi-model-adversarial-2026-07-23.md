---
title: "Single-model incremental bot review of a PLAN/design doc does not converge monotonically or cheaply — it oscillates and terminates only under an explicit stop rule; certify contracts once with a multi-model adversarial review"
description: "PR #281 (a docs+backlog-only implementation plan, no code) ran 10 Copilot review cycles whose findings-per-cycle oscillated instead of trending down (10→3→2→2→7→6→4→2→9→0). Each fix edited the plan, and the next single-model cycle read the NEW text and surfaced NEW issues in a different category (contract ambiguity → task granularity → stale disposition-log history → decomposition seams). The count DID reach 0 at cycle 10 — but only after an explicit stop rule halted doc edits; left to run, single-model incremental review does not self-terminate. A single-pass multi-model adversarial review certified the architecture SOUND and reframed the divergence as contract ambiguity, not design flaws. The stop rule: fix genuine contradictions, document limitations + rollback triggers instead of expanding scope, split over-granular tasks before harvest (never defer a 2-hour-gate violation to Ship), and RESOLVE remaining low-confidence nits as backlog deferrals rather than fixing them (fixing spawns fresh text for the next cycle to flag)."
problem_type: "review_convergence + process_hazard + plan_doc_review"
category: "workflow-issues"
component: ".github/skills/plan-review; Adversarial Review agent; ship/stage review-fix loop; docs/exec-plans/*-plan.md under Copilot review"
root_cause: "Single-model incremental review of a natural-language PLAN/design document is a moving target, not a converging one. Unlike code (where a fix removes a defect and shrinks the surface), every plan fix ADDS or rewrites prose. The next review reads the changed prose and finds new, legitimately different-category issues (contract wording, task granularity, append-only disposition-log 'staleness', decomposition seams). Contract-convergence itself introduces the text that the following cycle flags, so findings-per-cycle oscillates instead of trending to zero."
resolution_type: "design_change (review strategy) + explicit stop-and-merge discipline"
severity: "medium"
message: "n/a (process)"
file_path: "docs/exec-plans/2026-07-22-spark-notebook-data-lineage-plan.md"
date: "2026-07-23"
shipment: "090-S (07BFA98E Spark notebook data-lineage plan; docs+backlog only)"
feature: "095-F"
pr: 281
related_pr: [239, 240, 243, 280]
citations:
  - "PR #281 review cycles 1-10 (HEAD progression a9ec72ed→d79f1b16→22fe3dce→1b1d5b4e→dfb99579)"
  - "docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md"
  - "docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md"
  - ".github/skills/plan-review/SKILL.md"
tags:
  - "plan-review"
  - "adversarial-review"
  - "copilot-review"
  - "review-convergence"
  - "stop-condition"
  - "process-hazard"
---

## Problem

PR #281 carried a **docs + backlog only** implementation plan (a Spark
notebook data-lineage design: one `*-plan.md` plus 16 backlog artifacts, **no
`.rs` code**). It was put through GitHub Copilot's incremental PR review. The
findings-per-cycle sequence over ten cycles was:

```text
cycle:      1   2   3   4   5   6   7   8   9   10
findings:  10   3   2   2   7   6   4   2   9    0
```

The count **oscillated** instead of trending monotonically down: each cycle we
fixed every finding, pushed, and re-requested review — and the next cycle came
back with a *new* batch, often larger than the previous one (note the 2→7 and
2→9 jumps). It *did* reach 0 at cycle 10, but only after we stopped editing the
doc (see Resolution). The thesis is therefore not "review never reaches zero" —
it did — but that single-model incremental review of a plan **does not converge
monotonically or cheaply and does not self-terminate**: left to run, fix-and-push
keeps re-arming the cycle and burns enormous review-cycle budget on a document
with no executable code.

## Root Cause

**A plan/design document under single-model incremental review behaves like a
moving target rather than a monotonically converging one — it can reach zero, but
only when edits stop, not on its own.**

- For *code*, a review finding maps to a defect; fixing it removes the defect and
  shrinks the reviewable surface, so findings trend down.
- For a *plan*, every fix **adds or rewrites prose**. The next review reads the
  changed prose and surfaces new, legitimately different-category issues. On
  #281 the categories rotated through:
  1. **Contract ambiguity** (what type does unit U2b emit; what is the seam
     between U2a/U2c/U2b) — the early cycles.
  2. **Task granularity** (a task enumerating 6 test scenarios vs the
     3-scenario 2-hour gate) — restated across several later cycles.
  3. **Append-only disposition-log "staleness"** — the per-cycle disposition log
     is intentionally append-only history, so it *always* contains superseded
     rows (a cycle-7 decision AND its cycle-9 correction). A bot reads the older
     row in isolation and flags it as stale.
  4. **Decomposition seams** — splitting a unit's spec in one cycle created a
     seam the next cycle read as an internal contradiction.
- Crucially, **contract-convergence itself introduces the text the next cycle
  flags.** Clarifying U2's contract (cycles 5-7) is what produced the granularity
  and seam wording that cycles 8-9 then flagged. The reviewer is not wrong each
  time; the document genuinely changed. But the process does not self-terminate.

## Resolution

Converge the **contracts** in one pass with a multi-model adversarial review,
then **cap** the single-model cycles with an explicit stop-and-merge rule.

1. **One-pass multi-model adversarial review to certify the architecture.** A
   parallel multi-reviewer pass (reviewers across different model tiers, findings
   assembled by consensus) certified #281's architecture **SOUND** and reframed
   the ongoing Copilot divergence as **contract ambiguity, not design flaws**.
   This is the load-bearing move: it gives you a defensible "the design is done"
   signal that a single-model incremental review never emits. **Caveat — the
   verdict only covers the revision the reviewers actually saw.** On #281 the
   adversarial pass ran on an early revision (`a9ec72ed`); the plan then changed
   materially in later cycles, and cycle 9 exposed a genuine U2 seam
   contradiction introduced *after* certification — a contract edit that never
   got multi-model review. So a SOUND verdict is not a durable, open-ended
   "design is done" stamp. After certification you MUST either **freeze the
   certified contract text** (treat further edits to it as scope changes) or
   **re-run certification when a material contract change lands**. Purely
   editorial or limitation-documenting edits do not require re-certification;
   changes to type contracts, unit seams, or dependency direction do.

2. **Apply an explicit STOP rule after adversarial certification.** For each
   remaining single-model finding, classify and route — do **not** reflexively
   fix:
   - **Correctness / contract contradiction** → **fix** (these are real; e.g.
     the cycle-9 U2a→U2c→U2b seam contradiction was a genuine internal error).
   - **Known limitation** → **document** it in a "v1 Limitations & Deferred
     Items" section with a rollback trigger; do **not** expand scope to make the
     finding disappear.
   - **Task granularity / decomposition** → **split the task before
     harvest/staging** — do **not** defer it to Ship. The 2-hour /
     3-test-scenario limit is a NON-NEGOTIABLE gate
     (`constitution.instructions.md`, Task Granularity), and `harness-architect`
     only scaffolds *ready* tasks — it "prepares the red phase and stops there"
     and does not re-decompose backlog work
     (`harness-architect/SKILL.md`, Purpose + Step 1). Deferring an over-granular
     task to the Ship harness-architect institutionalizes a gate violation. On
     #281 this deferral shipped in `095.004-T` (6 scenarios vs the 3-scenario
     ceiling) — that was a **deviation to correct, not a pattern to copy**. Split
     it during planning/harvest so every harvested task is gate-compliant.
   - **Low-confidence nit / append-only-history "staleness"** → **RESOLVE the
     thread with a backlog-deferral rationale; do NOT edit the doc.** Editing to
     silence a nit only spawns fresh text for the next cycle to flag.

3. **Merge on the gate, not on "zero findings."** Once contracts are certified
   and real contradictions are fixed, run the 4-point merge gate
   (`commit_id == HEAD` review present, Copilot off `requested_reviewers`, 0
   unresolved threads, `mergeable_state == clean`) and merge. Observed on #281:
   cycle 10 ran on `dfb99579` (the final reconciliation commit — the doc was not
   edited again afterward to chase nits) and returned **"reviewed 17 of 18 files
   and generated no new comments."** Note this is zero *public* comments, not a
   fully clean review: one low-confidence granularity finding remained,
   **suppressed** rather than posted. The observed facts are that no new public
   threads appeared once edits stopped, the 4-point gate was clean, and the PR
   merged — not a proven causal law that stopping edits guarantees zero findings.

## Prevention

- **For plan/design docs, do not chase "0 findings" via single-model
  incremental bot review.** Reaching zero is achieved by *stopping edits* once
  contracts are certified, not by exhaustively fixing — each fix feeds the next
  cycle. Budget the review as bounded, not exhaustive.
- **Escalate to a one-pass multi-model adversarial review to certify
  architecture/contracts.** Treat its SOUND verdict — not the bot's finding
  count — as the "design is done" signal. Note: persistent non-convergence is
  **not** currently an automatic escalation trigger in
  `adversarial-review.instructions.md` — the listed triggers are 3+ P0/P1
  findings, security-sensitive scope touching auth/crypto/data/PII, or an
  explicit operator request. On #281 the adversarial review was
  **operator-triggered**. If you want non-convergence to auto-escalate, propose
  it as a new trigger in that instruction file rather than assuming it applies.
- **Classify every residual finding before touching the doc.** Only
  correctness/contract contradictions warrant an edit. Granularity → split before
  harvest; limitations → document; nits/append-only-history → resolve as backlog.
- **Expect append-only disposition logs to look "stale" to bots.** They record
  superseded decisions *by design*. Resolve such flags as intended history; do
  **not** rewrite the log — rewriting destroys the audit trail and adds new text
  to flag.
- **Every doc edit re-arms the async review clock.** Combined with the merge-gate
  race (see `copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`), each
  nit-fix push costs a full ~4-5 min review round-trip. The stop rule is both a
  correctness discipline and a large time saver.
