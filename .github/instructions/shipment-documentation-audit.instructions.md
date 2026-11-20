---
description: "Normative, technology-agnostic documentation consistency audit applied on every shipment: ten audit categories with checkable predicates, anti-self-referential readiness, finding routing, recorded outcome, and PR-body concision"
applyTo: '**'
---

# Shipment Documentation Consistency Audit

This instruction is the **single source of truth** for the documentation
consistency audit that runs on **every shipment**, in every workspace. Other
surfaces (the ship agent, the operational-closure skill, the pr-lifecycle skill,
the pull-request instruction, and the Evidence Consistency Reviewer persona)
point here and never restate the categories below. A surface that copies the
category list creates the cross-surface drift this audit exists to catch.

The audit examines the documentation and evidence artifacts a shipment creates
or touches: the configured docs root (`docs`) and its subdirectories, the
configured backlog directory, instruction/agent/skill text, and any file whose
frontmatter keys are read by the workspace's own gates. It is a consistency
check between statements, not a prose-quality review.

## Audit Categories

Apply every category to each documentation or evidence artifact in scope. Each
predicate is checkable: it names two statements or sources that must agree.

| # | Category | Checkable predicate |
|---|---|---|
| 1 | **Lifecycle/temporal consistency** | Every before/after, "merged", "shipped" or "completed" claim agrees with real commit order and timestamps, and no artifact asserts a state as current that its own record time shows was not yet true; historical observations explicitly stated as such are valid. |
| 2 | **Status consistency across frontmatter/headings/conditions/summaries/follow-ups/verdicts** | A status or verdict stated in frontmatter, headings, condition lists, summaries, follow-up lists and verdict sentences is the same value everywhere in the artifact. |
| 3 | **Immutable evidence semantics** | Committed evidence names only an immutable observed state (the `last_code_affecting_head` or an explicitly named observed revision), never the current PR HEAD, never its own commit SHA, and never live worktree hashes. |
| 4 | **PR/commit/branch/task/shipment identity consistency** | Every PR number, commit, branch, task ID and shipment ID cited resolves to the same entity everywhere it appears. |
| 5 | **Quantitative consistency** | Every count (tests, findings, items, cycles, rounds) agrees with the underlying record it summarizes and with every other place it is repeated; committed Markdown artifacts are never required to cite volatile self-referential file-count denominators (such as total repository or allowlist file counts). |
| 6 | **Ownership/role attribution consistency** | Every statement of who did, owns or approved something names the role the governing policy assigns, and agrees across artifacts. |
| 7 | **Path/cross-reference integrity** | Every referenced file, ID or heading exists on the committed branch tree at the stated path, including moved, archived or compacted files and their relocation stubs. |
| 8 | **Scope/distribution wording consistency** | Wording about what is in scope, what ships, and where it is distributed (this workspace versus installed workspaces) agrees with what the change actually does. |
| 9 | **Frontmatter schema consistency with canonical neighbouring artifacts** | Frontmatter keys, value shapes and enumerations match the canonical neighbouring artifacts of the same kind, not only the artifact's own prose. |
| 10 | **Stale TODO/pending/conditional language** | No TODO, pending, "will be", or unmet-condition wording remains once the condition it describes has been satisfied. |

## Recorded Outcome

The audit obligation is **unconditional** per shipment. Invocation may
fast-path, but an outcome is always recorded in the evidence record the shipment
already keeps. The outcome is one of:

* `AUDIT_CLEAN` — every category was applied and no inconsistency remains.
* `AUDIT_FINDINGS_RESOLVED` — in-scope findings were fixed and re-verified.
* `AUDIT_FOLLOWUPS` — out-of-scope findings were captured as deferred entries.
* `AUDIT_NOT_APPLICABLE` — **no documentation surface touched** by the shipment,
  stated explicitly with the observed change set it was judged against.

An unrecorded audit must be distinguishable from one that never ran, so a silent
omission is never an acceptable outcome. A fast-path still states which
documentation surfaces were checked and why the rest were not.

## Anti-Self-Referential Readiness

Mutable surfaces own current-HEAD readiness: the PR body and external gates
state readiness for the current head. Committed evidence records only an
immutable `last_code_affecting_head` or an explicitly named observed revision.

A rule that requires a committed file to name its own commit is MALFORMED: the
value cannot exist inside the commit that contains it, and substituting a new
value after every push never converges. Likewise, committed Markdown files
(plans, reviews, compound learnings, closure records, and backlog items) MUST
NOT be required to cite volatile self-referential file counts (such as total
repository or allowlist file counts), their own commit SHAs, or live worktree
hashes. Such a rule is restructured so the
mutable surface carries the current-HEAD claim, never re-substituted round after
round.

## Finding Routing

Route every audit finding through the existing scope-expansion contract (P-021);
this audit adds no new deferral mechanism.

* **In-scope findings** (the fix only completes the change already authorized)
  are fixed before readiness is claimed, then the affected category is re-applied.
* **Out-of-scope findings** are captured as deferred entries before disposition,
  and the captured entry IDs are the only follow-up reference carried forward.

The inverse also holds: deferring an in-scope inconsistency without a captured
entry is itself a violation of the same contract.

## Delta Re-Audit

The recorded outcome covers the documentation and evidence that existed when the audit ran. Any documentation or
evidence created or changed afterwards (during PR preparation, review fixes, post-merge closure or any later phase)
is covered by a **bounded delta re-audit** of only those new or changed artifacts, run against the candidate being
committed (the branch tree, or a candidate tree built from a temporary index when it is not yet committed) before it
is committed. The delta re-audit applies the same categories, recorded outcomes and finding routing, and records
which candidate was checked as a named observed revision or candidate tree, never as the commit that contains the
record itself.

## PR-Body Concision Contract

A pull request body is a mutable surface. For the audit it carries only:

* the audit verdict (the recorded outcome above),
* a pointer to the committed audit evidence record, and
* any deferred follow-up IDs.

The category list is never reproduced in a PR body. The body MUST NOT paste or
paraphrase the checklist; it points here instead.
