---
doc_type: memory
date: 2026-09-13
agent: stage
session: 143-defect-2-revision-10-rr3-ruleset-prerequisite
pr: 396
branch: chore/143-s-stage-checkpoint-lifecycle-continuity
start_head: e7194c2b8adcc61b8fb9072ef4ca020a109f3672
revision_head: 23731cba156b8aee3334d65b7ed58ff91837aa55
review_verdict: FAIL
review_attempts: 5
harvest_authorized: false
outcome: BLOCKED
supersedes_status_of: docs/memory/2026-09-13/stage-143-defect-2-revision-9-round-9-review.md
---

# Stage session — Defect-2 revision 10 and the RR-3 ruleset prerequisite

## Authorization boundary

Invoked by Orchestrator for the **second** bounded remediation/review attempt on
blocked staging PR #396. The operator authorized exactly **one coherent
revision**, **one push**, and **one fresh full independent plan-review**.

Explicitly **not** authorized and **not** performed: harvest, shipment
claim/assembly, activation, implementation, merge, GitHub settings mutation,
product-code edits, workflow edits, installed-agent edits, task-card edits, and
any reply to or resolution of the open Copilot review threads.

`143-F`, `143-S` and `143.001-T … 143.014-T` remain **abandoned historical
evidence only**. No backlog IDs were allocated.

## The operator's RR-3 decision

RR-3 — "the locator lives in the mutable PR body" — is closed by an
**approval-gated GitHub branch-ruleset prerequisite**, *not* by a CI-check
persistence substrate and *not* by weakening RQ-7. Three parts:

1. **A repository ruleset** covering every permitted resolution-bearing Ship
   branch pattern, with `deletion` + `non_fast_forward`, `enforcement: active`,
   no bypass actors, `current_user_can_bypass: never`. It is an external GitHub
   settings change, so it is carried by a distinct unit **U0** classified
   `ProposedAction` / `ActionRisk: high` / `approval_required: true`. **No
   planning PR applies it.**
2. **A run-time proof** (`BRANCH_PROTECTION_PREREQUISITE`, new segment **S3.5**,
   new requirement **RQ-13**) against the **live `headRefName` read from the PR
   API** — never the ambient checked-out branch — before any obligation is
   published. Unavailable API, uncovered branch, ambiguous rules, or any bypass
   capability halts **before** publication.
3. **Channel B made independent of the PR body**: candidates derived from the
   exhaustive trusted-PR enumeration rather than from marker-bearing bodies;
   each fetched to a **unique retained ref** `refs/autoharness/scan/pr-<n>`;
   full reachable history scanned with **explicit revision roots**; every
   transition bound to record identity and commit ancestry; ref cleanup on all
   paths including halts.

## Measured GitHub evidence (read-only; no mutation)

| Probe | Result |
|---|---|
| `GET /repos/softwaresalt/agent-engram/rulesets` | one ruleset: `PR-Required`, id `12812291`, `enforcement: active`, `target: branch` |
| Ruleset `12812291` conditions | `ref_name.include: ["~DEFAULT_BRANCH"]` only |
| Ruleset `12812291` rules | `deletion`, `non_fast_forward`, `pull_request` (merge-only), `copilot_code_review`; **no bypass actors**; `current_user_can_bypass: never` |
| `GET /rules/branches/main` | `["deletion","non_fast_forward","pull_request","copilot_code_review"]` |
| `GET /rules/branches/chore%2F143-s-stage-checkpoint-lifecycle-continuity` | `[]` — **uncovered** |
| Legacy branch-protection endpoint, `main` and source branch | `404` on both |

**Conclusion carried into the plan**: Ship source branches are **unprotected
today**. No part of the plan may assume source-branch history immutability until
U0 is applied and S3.5 proves it per branch at run time.

## Ship branch patterns (derived, not guessed)

Read from `.github/agents/_ship.agent.md`: `feat/{feature-slug}`,
`chore/{slug}`, `post-merge/{feature_slug}`. `post-merge/**` is **not
optional** — POST-B's `CLOSED` transition commits there, so leaving it
unprotected would leave the discharge itself erasable.

## Round-9 P0/P1 remediation map

| Finding | Remediation |
|---|---|
| **F-11 (P0)** | Stage's merged-carrier predicate now keys off the **selected checkpoint's validated `context.pr` / `context.branch`**, which become mandatory fields. `gh pr list --head`, `git branch --show-current` and `rev-parse --abbrev-ref` are explicitly prohibited as predicate inputs; V10 greps for all three. |
| F-01 | `RESOLUTION_POSTCONDITION` split into **POST-A** (PR-body write, no commit) and **POST-B** (`OPEN` → `CLOSED` commit on `post-merge/{feature_slug}`, which must itself merge). The "commits nothing" sentence is deleted. |
| F-02 | Every "item 15 retained unmodified" restatement removed; whole document set re-grepped. |
| F-03 | V4(g) now permits S4's own conditional resolution commit and asserts nothing mutates after S4. |
| F-04 / F-05 | S8's locator and ancestry terms are **explicitly conditional**; POST-A/POST-B are no-ops on the zero-checkpoint path; `none` → `CLOSED` is forbidden; the record is left intact at `none`. |
| F-06 | Amended item 15 **re-enumerates** checkpoints; a nonzero result returns the unit to S3, voids the S6 approval, and is bounded at two re-entries. |
| F-07 | P-020 compaction ordered strictly after POST-B's `CLOSED` write; path-stability invariant added in three places (record definition, U10, U12); archive-following rule so compaction cannot trip OB-1. |
| F-08 | S1…S8 defined by **role** with no live item numbers in the instructions file; the item binding lives only in U4 beside the items; new **V20** standing role-order drift check. |
| F-09 | U3 → U3 + **U11**; U4 → U4 + **U12**; U9 → U9 + **U13**. Fourteen units total. |
| F-10 | New normative `S3_ENUMERATION_ALGORITHM` with the no-API-filter rule, anomalies-before-partition, the identity predicate, and per-record proof; bulk resolution prohibited. |
| F-16 | Explicit **P-012 availability contract** (U11 AC13); a registry exposing no checkpoint operations is a **halt**, never an implicit zero. |
| F-17 | **PV-8** binds a candidate to the shipment's backlog-recorded implementation/closure PR; unbindable candidates are **held for operator review**; V24 tests the decoy case. |
| F-18 | The merge call **pins the S8-observed head SHA** so GitHub refuses server-side on a race. |
| F-21 | Worktree-topology reconciliation in U11, citing the recorded post-merge worktree prior art; new **WP-0** P-011/P-016 gate; V12g. |

P2/P3 sweep: F-20 (nine invariants), F-22 (U1 not a root), F-23 (U5 invokes WP
by name rather than restating), F-24 (V19 parses YAML), F-25 (`U10→U4`,
create-only init), F-26 (U3 Channel-B criterion), F-27 (Principles I–XI table),
F-28 (P-011), F-29 (shallow/partial-clone guard at B0), F-30 (API-side merge
verification + two-parent assertion), F-31 (recorded as **RR-6**), F-32…F-37.

V2's verb prohibition and V8's `--paginate` were already correct from revision 9
and were re-verified, not re-fixed.

## Copilot thread disposition (18 unresolved, read as review input only)

Substantively addressed in this revision: the schema `shipment`/`shipment_id`
mismatch; resolution-commit recovery without a SHA field; merged-PR
rediscovery / state-aware routing; `ls-tree` cannot yield content; missing
revision roots on `git log`; the continuity-plan supersession notice; the stale
scope-split memory; the decision's B4 disposition and its revision-pinned
Definition of Done; hardening D15's fold; and the plan's live-status wording.

**Intentionally not addressed — out of scope**: two threads on
`.backlogit/queue/143-F.md` and `.backlogit/queue/143-S.md`. Those are task
cards for the abandoned `143.*` hierarchy; editing them is outside this
session's authorization and would touch abandoned machine state.

No thread was replied to or resolved. That is deferred by operator direction.

## Do not repeat

- `gh api graphql -f query=@file` passes the **literal string**; use
  `-F query=@file`. The query file must be **BOM-free** — Windows PowerShell
  5.1's `Out-File -Encoding utf8` adds a BOM and breaks GraphQL parsing. Write
  it with `[System.IO.File]::WriteAllText`.
- These plan artifacts are **LF in the git index, CRLF in the worktree**
  (`core.autocrlf=true`, `.gitattributes` pins only `*.sh`). String-replacement
  edit tooling cannot match CRLF `old_str`. Normalize to LF first and confirm
  `git diff --stat` is empty before editing.
- A grep-based verification over a document that also contains the prose
  *forbidding* the grepped token can never fail. V19 was exactly this shape and
  is now a YAML key-set parse.
- `FETCH_HEAD` is **global**. Fetching a second PR silently rebinds every
  assertion written against the first. Always fetch to a unique named ref.
- `git log` without a revision argument defaults to `HEAD` — the ambient
  branch — which is the same class of defect as keying a safety predicate off
  the working tree.

## Round-10 review outcome — FAIL / BLOCKED

Revision 10 was pushed as `23731cba156b8aee3334d65b7ed58ff91837aa55` and reviewed
by a fresh independent four-persona cross-model panel at that HEAD:

| Persona | Model | Verdict |
|---|---|---|
| Correctness | `gpt-5.6-sol` (xhigh) | FAIL |
| Constitution and policy | `claude-opus-4.8` (high) | FAIL |
| Scope boundary and maintainability | `grok-4.6` (high) | FAIL |
| Security and ops risk | `gemini-3.8-flash` (high) | FAIL |

**Consensus: FAIL — 4 P0, 17 P1, 9 P2, 5 P3.** Circuit OPEN at attempt counter 5.
Full findings appended verbatim to the plan as `## Plan Review — round 10`.

### What revision 10 genuinely closed

Recording this so round 11 does not relitigate it:

* **The round-9 P0 (F-11) is closed.** All four reviewers independently confirmed
  U8 AC2 now keys off the checkpoint's validated `context.pr`/`context.branch`, with
  `gh pr list --head`, `git branch --show-current`, and `rev-parse --abbrev-ref`
  prohibited as predicate inputs. The P-010 ambient-branch probe **passed 4 of 4**.
* The Ship Step 5 structural extraction was independently re-derived and confirmed:
  the duplicated item `7` is real, and 7b/7c do precede the second 7 and 8/9/10.
* The zero-checkpoint S8 path is now genuinely satisfiable (round-9 F-04/F-05 closed).
* The F-09 unit splits are real seams, not relabelling.
* API-surface fail-closed behaviour is correct; `[]` is correctly read as *uncovered*.
* Abandoned-ID hygiene is clean; frontmatter counts reconcile.

### The four new P0s

1. **Canonical Channel B and U13 install different protocols**, and U13 AC5
   reinstates the "embedded `pr` equals the PR it was read from" rule that the
   canonical PV-B4/PV-B6 split exists to prevent — so POST-B can never pass
   provenance. Round-9 F-15 re-opened inside the unit created to fix it.
2. **U13 AC6 halts on merged + `OPEN`**, which is the protocol's own happy-path
   interval (merge succeeded, POST-B pending), not the 139-S orphan shape.
3. **The `FETCH_HEAD` contract is unsatisfiable.** Canonical LMR, WP-3 and U11 AC9
   use bare `FETCH_HEAD` with no destination refspec; U5 AC2, D11, D14 and V22
   require unique retained refs. U11 cannot both install the canonical block and
   pass V22. D14's discharge claim is consequently false.
4. **U0's ruleset breaks the repository.** `deletion` with no bypass actors blocks
   all merged-branch cleanup including `gh pr merge --delete-branch`, contradicting
   Ship's own branch-management rule. `non_fast_forward` on `feat/**` and `chore/**`
   makes every working branch append-only from first push, breaking rebase, squash,
   and amend. The plan never enumerates these workflows and has no real rollback.

### The structural judgement — the important part

Two reviewers reached this independently. The load-bearing fix for Defect-2 is
small and **is already fully specified**: resolve before merge, push, re-review at
that HEAD, assert ancestry at merge, forbid post-merge resolution in Ship and Stage.

Everything layered above it — the obligation record, Channel B B0–B7, PV-1…PV-8,
OB-1…OB-8, WP-0…WP-7, the seven-part S8 bar on every merge, and now U0's
repository-wide ruleset — *detects* a residual window rather than *preventing* the
defect, and by the plan's own V17 the detection path terminates in a halt. Ten
revisions of escalating residual-window machinery around a one-step ordering bug is
itself the finding.

**Panel recommendation to the operator** (advisory; no decision taken here):

1. Keep the ordering fix, locator, merge-authority bar, and ancestry assertion.
2. Replace the branch ruleset with an **immutable marker ref or annotated tag**
   pushed at S4 and protected in its own namespace — same reachability property,
   near-zero blast radius, no P-019 conflict, no admin-token requirement.
3. Demote Channel B and the OB/PV invariant sets to an explicitly optional
   hardening layer scoped to PRs bound to active shipments, or drop them and record
   the residual honestly.

### Do not repeat in round 11

* Do not attempt a third in-place remediation of thirty findings. Two consecutive
  bounded remediations have each closed the prior round's findings and surfaced a
  deeper layer. The design question, not the prose, is what is failing.
* Do not re-fix the round-9 P0 — it is closed and confirmed by four reviewers.
* Do not invent a constitution mapping. The workspace Principles I–XI are
  Safety-First Rust, Test-First Development, Workspace Isolation, CLI Workspace
  Containment, Structured Observability, Single Responsibility, Destructive Command
  Approval, Explicit Safety Modes, Git-Friendly Persistence, Agent Context
  Efficiency, Merge Commit History Preservation. Revision 10 mapped a fabricated set.
* P-019 states the harness never edits rulesets. Any ruleset design must reconcile
  with it explicitly, and no agent should hold `Administration: write`.

### Boundary confirmation

No harvest, no shipment claim, no activation, no implementation, no merge, and
**no GitHub settings mutation** occurred. All GitHub reads were read-only. The 18
unresolved Copilot threads on PR #396 were used as review input and were **not**
replied to or resolved, per instruction. No product code, workflow, installed agent,
or task card was modified.