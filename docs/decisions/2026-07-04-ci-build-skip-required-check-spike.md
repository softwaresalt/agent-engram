---
title: "CI build-skip on non-code PRs — required-check reality spike"
type: spike
date: 2026-07-04
slug: ci-build-skip-required-check
status: decided
stash_ref: FC881353
umbrella_feature: 071-F
informs_plan: docs/exec-plans/2026-07-04-ci-build-skip-non-code-prs-plan.md
scope: .github/workflows/ci.yml
---

## Problem frame

Stash `FC881353` asks to stop wasting GitHub Actions minutes: every PR this
session — including backlog/docs/planning closure PRs (067-S..070-S closures,
spikes, unblocks) — triggered a full ~3-3.5 min Rust build on `ubuntu-latest`.
The stash proposes `paths-ignore` and/or a PR-title `if:` conditional, but flags
a **GOTCHA**: it *assumes* the `build` check is a **required status check**
(observed `mergeStateStatus: BLOCKED` / `reviewDecision: REVIEW_REQUIRED`,
merges done with `--admin`). If that assumption held, a naive `paths-ignore` /
job-`if:` skip would leave the required check **pending forever** and block
non-admin merges.

**The whole design hinges on one question: is `build` actually a required
status check on `main`, or is only review required?** This spike answers it
definitively before any impl-plan commits to a mechanism.

## Investigation (grounded against the live repo, 2026-07-04)

### 1. Branch protection is a ruleset, not classic protection

```
gh api repos/softwaresalt/agent-engram/branches/main/protection      → 404 "Branch not protected"
gh api .../branches/main/protection/required_status_checks           → 404 "Branch not protected"
```

Classic branch-protection is absent. The controls live in a **repository
ruleset**.

### 2. The one active ruleset and its effective rules on `main`

```
gh api repos/softwaresalt/agent-engram/rulesets
  → [{ id: 12812291, name: "PR-Required", enforcement: "active", target: "branch" }]

gh api repos/softwaresalt/agent-engram/rulesets/12812291 --jq '.rules[].type'
gh api repos/softwaresalt/agent-engram/rules/branches/main
  → rule types applying to main:
      deletion
      non_fast_forward
      pull_request            (required_approving_review_count: 1,
                               require_code_owner_review: true,
                               require_last_push_approval: true,
                               required_review_thread_resolution: true,
                               dismiss_stale_reviews_on_push: true,
                               allowed_merge_methods: ["merge"])
      copilot_code_review     (review_on_push: true, review_draft_pull_requests: true)
      update
```

**There is NO `required_status_checks` rule.** The merge gate is composed
entirely of **review** requirements (human approval + code-owner + last-push
approval + thread resolution + Copilot review). No CI status check is enforced.

### 3. Corroboration against a real non-code PR

PR **#200** (`chore(backlog): archive shipment 070-S post-merge`) — a pure
backlog/docs PR:

```
gh pr view 200 --json mergeStateStatus,reviewDecision,statusCheckRollup
  reviewDecision:  REVIEW_REQUIRED
  statusCheckRollup: [ { name: "build", workflowName: "CI",
                         status: COMPLETED, conclusion: SUCCESS,
                         startedAt 20:25:44Z → completedAt 20:28:58Z } ]   # ~3m14s
```

The backlog-only PR ran a full 3m14s Rust build for nothing, then merged with
`REVIEW_REQUIRED` (i.e. via `--admin`, bypassing the **review** gate). The
`build` CheckRun is present in the rollup but is **not enforced** by any ruleset
rule — its SUCCESS/absence is irrelevant to mergeability.

### 4. No CODEOWNERS file

```
.github/CODEOWNERS / CODEOWNERS → absent
```

`require_code_owner_review: true` with no CODEOWNERS means code-owner review can
never be satisfied conventionally — which is exactly why every merge is an
`--admin` merge. This reinforces that the observed `BLOCKED` is **review-driven,
not status-check-driven**.

## Finding / decision

**`build` is NOT a required status check for `main`.** The stash's central
"required status check → pending forever" gotcha **does not apply to this
repository**. The observed `BLOCKED` / `REVIEW_REQUIRED` is produced by the
`pull_request` + `copilot_code_review` review rules, not by a pending CI check.

Consequences for the design:

- A workflow-level **`paths-ignore`** that skips the entire CI run on
  doc/backlog-only PRs is **safe**: because no status check is required, a
  skipped run cannot leave a required check pending and cannot block merges
  (which are review-gated and admin-merged regardless).
- The elaborate required-check-safe patterns (companion always-passing job with
  the same check name; branch-protection path exclusions) are **not needed** at
  the current ruleset. They remain the correct answer **only if** `build` is
  ever promoted to a required status check.

## Guardrail (future coupling — carry into the plan)

This design is coupled to the ruleset. **If anyone later adds `build` (or any CI
job) to the ruleset as a required status check**, `paths-ignore` will cause
doc-only PRs to hang on *"Expected — Waiting for status to be reported"* forever
and block non-admin merges. At that point the mitigation is the **companion
always-passing job** carrying the identical required check name (`build`) gated
on the inverse path filter. The impl-plan records this contingency (and a
snippet) but does **not** implement it now, to keep the change minimal and
correct for the reality on the ground.

## Spike vs. plan-directly call

The determination is **conclusive from the ruleset + rules API + a corroborating
live PR** — no hands-on experimental branch was required to answer the crux, so
this is a grounding spike (evidence + decision) rather than a code experiment.
It unblocks a direct impl-plan. Findings landed here per the Stage contract.

## Provenance

- Stash: `FC881353` (kind task, priority medium).
- Evidence: `gh api .../rulesets`, `.../rulesets/12812291`,
  `.../rules/branches/main`, `.../branches/main/protection` (404),
  `gh pr view 200`.
- Informs plan:
  `docs/exec-plans/2026-07-04-ci-build-skip-non-code-prs-plan.md`.
- Umbrella feature: `071-F`; shipment `071-S`.
