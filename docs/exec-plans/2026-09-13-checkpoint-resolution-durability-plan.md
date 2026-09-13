---
doc_type: exec-plan
date: 2026-09-13
revision: 8
scope: defect-2-only
status: halted-review-circuit-open
review_verdict: FAIL
review_verdict_revision: 8
review_attempts: 3
escalation: P-013.6 fired; route gpt-5.6-sol/openai/xhigh
harvest_authorized: false
source_document: docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md
supersedes: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md
stash_ids: [4EF24729]
policies: [P-001, P-003, P-005, P-006, P-008, P-009, P-010, P-012, P-014, P-015, P-016, P-017, P-018, P-020, P-022]
requires_plan_hardening: yes
hardening_document: docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md
task_count: 8
dependency_edge_count: 11
sub_epic_count: 2
---

# Checkpoint Resolution Durability — Implementation Plan (revision 8, Defect 2 only)

**Source document**: `docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md`
**Requires plan hardening**: **yes** — this plan changes merge-adjacent ordering
governed by P-014/P-018 and adds a startup route that could, if mis-specified,
become an unsupervised merge path.

**Machine-readable status.** The frontmatter is authoritative. `revision: 8`,
`scope: defect-2-only`, `harvest_authorized: false`, `review_verdict: none`.
Revision 8 claims **no** review verdict; a fresh independent full-plan review is
the gate that authorizes harvest.

**What changed in revision 8.** Revision 7 was reviewed by the same independent
four-persona panel and returned **FAIL** again — Scope, Correctness and Parity
FAIL; Constitution ADVISORY. The panel confirmed that every revision-6 finding
was genuinely closed, and that the design still holds; the new findings were
deeper specification defects that only became visible once the earlier layer was
fixed. Revision 8 remediates them, and they converge on five root corrections:

1. **`RESOLUTION_ORDER` is split into `RESOLUTION_PREFIX` (pre-merge) and
   `RESOLUTION_POSTCONDITION` (a Step 6 metadata write).** A single blob spanning
   both sides of the merge could not be invoked from Step 5 without either
   re-merging or stopping mid-sequence at an undefined boundary. The existing
   approval/re-fetch/merge items are now explicitly *not* moved or duplicated.
2. **The circular entry condition is gone.** The canonical block said
   `work complete AND PR merge-ready`, but readiness is established *by* this
   sequence. Revision 7 fixed this only in U4's prose and left the canonical text
   — which U2 installs verbatim — still circular. A zero-checkpoint bypass is now
   stated too.
3. **Orchestrator discovery is owner-scoped.** Revision 7 keyed it on *global*
   zero-candidate, so a legitimately-active **Stage** checkpoint would mask the
   whole obligation, Ship would never be routed, and the shipment would then be
   skipped at Step 2 for being `active` — nobody discharging it. It is now keyed
   on the absence of a **ship-owned** checkpoint, matching Ship's own scoping.
4. **Recovery is executable from a fresh checkout.** Re-entry has to commit, but
   startup is normally on `main`, where committing is forbidden. A mandatory
   working-tree placement step (fetch `refs/pull/<n>/head`, check out, halt on
   failure) is now specified.
5. **The Stage side is completed.** U8 now covers *both* Stage resolve sites,
   gives an executable merged-PR predicate instead of a prose condition, and
   retires the undischargeable best-effort checkpoint.

Also: the **P-003 sub-epic tier is restored** (revision 7 justified a flat
decomposition by precedent, but P-003 item 4 is explicit and its violation action
is Halt); the `pr_role: closure` ghost specification is **removed** (no unit ever
produced a closure-PR locator); U1 gains the amendment-log and gate-point
mechanics every other policy carries; U2's self-contradictory heading locus is
fixed; U4 stops restating the sequence it is supposed to reference; and the
Constitution Check gains P-008, P-012 and P-017. The full disposition table is in
`## Review record`.

**What changed in revision 7.** Revision 6 was reviewed by an independent
four-persona panel (Scope Boundary Auditor `gpt-5.6-sol`, Constitution Reviewer
`claude-opus-4.8`, Correctness Reviewer `gemini-3.8-flash`, Agent-Native Parity
Reviewer `grok-4.6`) and returned **FAIL** — three FAIL, one ADVISORY. Every
finding was a specification defect, not a falsified design. Revision 7 remediates
all of them; the disposition table is in `## Review record` below. The
substantive changes: `RESOLUTION_ORDER` now has one named canonical owner
(the instructions file) that every other surface references rather than restates;
U4 is retargeted at Ship **Step 5**, which is where the executable merge path
actually lives, and at Session end item 2, which is where the only happy-path
resolution actually is; the residual-window checkpoint-creation directive is
explicitly retired; locator discovery is now a real executable command with a
completeness test; `LAST_MILE_RECOVERY` gains the missing `RESOLUTION_PENDING`
rows and the missing ancestry assertion on the advanced-HEAD row; a new unit U8
closes the Stage-side procedural gap that revision 6 recorded only as a residual
risk; and `RECONCILED` is tied to the full P-001 closure set.

**What changed from revision 5.** Revision 6 was a **scope reduction**, not a
remediation round. Revision 5 planned Defect 1 and Defect 2 together across 14
tasks. Defect 1 has been removed from implementation and returned to an open
deliberation
(`docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md`)
because its central assumption was falsified: safe auto-routing needs durable,
concurrency-safe executable persistence that prose cannot supply. Removed with
it: the continuation predicate and its precedence table, the P-017 amendment, the
Orchestrator auto-route branch, the Stage/Ship owner-side continuation paths, the
activation record store, cursor typing, handoff evidence, owner-side
revalidation, mis-evaluation directionality, the drift-checker script pair with
its fixture corpus and hook shim, and the task↔plan parity gate. Eight tasks
remain, all Defect 2 (seven at revision 6, plus U8 added in revision 7).

**Prior review history is retained as evidence**, not erased — see
`## Retained review history` at the end of this document. Revision 5's FAIL
stands on the record against revision 5's scope.

## Primary objective

Make a unit of work's checkpoint resolution reach `main` in the **same merge**
that carries the work, and make any residual outstanding closure obligation
discoverable and safely recoverable without conferring merge authority.

## Constraints

* Documentation-only. No `src/`, no `crates/`, no scripts, no build system.
* Installed harness surfaces only: `.github/policies/`, `.github/instructions/`,
  `.github/agents/`, plus one `docs/compound/` learning.
* No backlogit tool change. No shipment status outside `queued`/`active`.
* Every unit is single-domain and under two hours of human-equivalent effort.
* **No implementation unit introduces or depends on a Defect-1 construct.**
  Defect-1 names appear in this document only in the revision summary, the
  out-of-scope list, and the retained review history — as documentary exclusions,
  never as implementation targets. V7 enforces the distinction by scanning the
  *changed files*, not this plan.

## Constitution Check

| Principle | Check |
|---|---|
| P-001 Single-release-unit completion | P-001 requires that no previously merged release unit is still awaiting required post-merge closure. `CLOSURE_LOCATOR` discovery at zero-candidate startup (U6, U5) is the mechanism that makes that precondition *checkable* rather than assumed. `RECONCILED` is defined to require the full P-001 closure set. Composed with, not weakened. |
| P-003 Decomposition chain | P-003 precondition item 4 requires that **every task references its parent sub-epic**. Revision 7 asserted a flat feature-direct decomposition on the strength of workspace precedent; precedent is not policy text, and the Violation Action is Halt at pre-harvest. Revision 8 therefore restores the tier: source document → this plan → `143-F` → **two sub-epics** → 8 tasks. `143-E1 Ordering contract` covers the prevention half (U1, U2, U4, U8); `143-E2 Discovery and recovery` covers the recovery half (U3, U5, U6, U7). The split is the plan's own prevention/recovery structure, not an artificial tier. Each sub-epic references this plan and `143-F`; each task references its sub-epic and carries acceptance criteria. |
| P-005 Policy telemetry | No gate is bypassed; the review gate is explicitly open. U1 requires P-022's Violation Action to record P-005 telemetry, matching neighbouring policies. |
| P-006 Plan hardening | `requires_plan_hardening: yes`; hardening document exists and is reduced in lockstep. |
| P-008 Markdown conformance | Engaged by every unit: each carries a `markdownlint passes` acceptance criterion and V1 runs it across all changed files. Satisfied by construction. |
| P-009 Merge-commit-only | Untouched. U4 does not alter Step 5's P-009 guardrail (item 16); U3/U6 confer no merge authority and therefore cannot select a merge strategy. |
| P-010 Role boundary | Stage plans; Ship executes. No source mutation planned by Stage. **U8 adds only a prohibition to Stage** — do not resolve into a merged staging PR, do not create an undischargeable checkpoint — and confers no merge authority, no locator obligation, and no `RESOLUTION_PREFIX` execution. U6 has the Orchestrator *route* to Ship rather than perform recovery. |
| P-012 Tool availability | Locator discovery adds a required `gh api` capability to the startup critical path. It is probed per P-012, and its failure mode is the fail-closed halt already required by RQ-11 — never a silent fall-through to ad hoc filesystem scanning. |
| P-014 Local review readiness | **This plan introduces a §1.9 hazard and then closes it.** Today §1.9 is trivially satisfiable because resolution happens post-merge and the reviewed HEAD is stable. Moving resolution pre-merge advances HEAD past the reviewed HEAD, which — left unmitigated — would defeat §1.9 while appearing to satisfy it (hardening D4) or cause the gate to be quietly skipped (D5). The mitigation is specific and mandatory: re-run the actual review at the final pushed HEAD, write the PR-body record, *then* gate. P-014 is **preserved by construction**, not strengthened. |
| P-015 Single-artifact shipment closure | Untouched. The locator records shipment identity but does not alter shipment-closure sequencing and introduces no cascade-close behaviour. |
| P-016 No parallel branches | The recovery path routes one owner to one PR; it never opens a second branch or worktree. |
| P-017 Dark factory | `LAST_MILE_RECOVERY` is auto-entered at startup and walks to a merge bar, so dark mode could otherwise supply approval for it. A recovered obligation belongs to a **prior** unit and is outside the current run's declared dark scope: U6 AC7 states that a dark approval for the current scope does **not** satisfy the merge bar for a prior unit's recovered obligation. |
| P-018 Copilot review gate | Re-run at the final HEAD is explicit in `RESOLUTION_PREFIX`. The resolution push naturally re-arms P-018, and Step 5's existing unconditional last-mile re-check (item 15) is retained unmodified. |
| P-020 Post-merge context compaction | U4 edits Ship Step 5 and Session end — **not** Step 6's `compact-context` invocation. U4 carries an explicit acceptance criterion that the P-020 invocation remains present and unmodified, and `RECONCILED` is defined to require the P-020 compaction record, so the locator cannot discharge an obligation P-020 has not yet met. |

## Canonical definitions

These four definitions are the single source of truth. Task cards quote them;
they are not restated differently anywhere.

**Canonical ownership — one installed surface per definition.** Each definition
has exactly **one** installed home that carries it verbatim. Every other surface
**references** it by name and must not restate the sequence, so the surfaces
cannot drift into two different orders.

| Definition | Canonical installed home | Referencing surfaces |
|---|---|---|
| `HEAD_EVIDENCE_RULE` | `github-pr-automation.instructions.md` (U2) | — |
| `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION` | `github-pr-automation.instructions.md` (U2) | `workflow-policies.md` P-022 (U1); `_ship.agent.md` Step 5 and Step 6 (U4); `_stage.agent.md` (U8) |
| `CLOSURE_LOCATOR` | `github-pr-automation.instructions.md` (U2, U3) | `_ship.agent.md` (U4, U5); `_orchestrator.agent.md` (U6) |
| `LAST_MILE_RECOVERY` | `github-pr-automation.instructions.md` (U3) | `_ship.agent.md` (U5); `_orchestrator.agent.md` (U6) |

### `HEAD_EVIDENCE_RULE`

Any artifact whose own commit advances HEAD must not restate a HEAD-pinned
verdict. HEAD-pinned evidence belongs in PR metadata (PR body `Reviewed HEAD`);
committed documents use point-in-time wording or defer to the PR body.

Observed on PR #395, threads `PRRT_kwDORJEduc6h2uOQ` and `PRRT_kwDORJEduc6h2viu`.

### `RESOLUTION_PREFIX` and `RESOLUTION_POSTCONDITION` *(RQ-1 … RQ-6)*

The sequence is defined in **two named parts** with the existing merge step
between them. This split is deliberate: a single blob spanning both sides of the
merge cannot be invoked from one step without either re-merging or stopping
mid-sequence with no defined boundary.

**Entry condition.** `RESOLUTION_PREFIX` runs when the unit owns **one or more
active checkpoints** and all branch-mutating work is complete except the
resolution-dependent final gates. It is explicitly **not** gated on "PR
merge-ready" — readiness is established *by* this sequence, so gating entry on it
would be circular. When the unit owns **zero** active checkpoints, no locator is
published and the sequence is skipped entirely; the pre-existing readiness path
runs unchanged.

```text
RESOLUTION_PREFIX  (invoked by Ship Step 5, BEFORE the readiness gates)
  entry: unit owns >=1 active checkpoint
         AND all branch-mutating work complete
         EXCEPT the resolution-dependent final gates below
  → publish CLOSURE_LOCATOR to the PR body, status RESOLUTION_PENDING
        (metadata write; MUST precede the first resolution commit, so the
         checkpoint-free window is never uncovered)
  → resolve every checkpoint owned by this unit (commits; advance HEAD;
        these are the LAST commits on the branch)
  → PUSH, so the resolution commits exist on the remote PR head
        (nothing downstream may derive evidence from an unpushed commit)
  → RE-RUN THE ACTUAL LOCAL REVIEW at that final pushed HEAD
        (a real review pass over the final diff — never a restatement
         of an earlier verdict)
  → update CLOSURE_LOCATOR in the PR body: status RESOLUTION_PUBLISHED,
        resolution commit SHAs, final_head
  → record Reviewed HEAD in the PR BODY at that final HEAD
        (metadata write; does NOT advance headRefOid)
  → re-run ALL current-HEAD gates:
        P-014 §1.9 local readiness (reads the PR body recorded above);
        required CI green-or-non-applicable;
        P-018 copilot-review PASS when engaged
  exit: readiness established at final_head; NO further branch mutation
        is permitted from here until merge

        ── existing approval / re-fetch / merge steps run UNCHANGED ──
        (obtain approval at final_head; re-fetch live HEAD/threads/CI;
         merge only if unchanged. These already exist in Ship Step 5 and
         are NOT restated or duplicated by this definition.)

RESOLUTION_POSTCONDITION  (a Step 6 closure postcondition)
  → after the FULL required post-merge closure set is complete AND verified
  → set CLOSURE_LOCATOR to RECONCILED
```

**No step of either part may be performed on a merged branch.** The prefix ends
before merge; the postcondition is a PR-body metadata write only and commits
nothing.

Four ordering invariants are load-bearing:

1. **No checkpoint resolution may occur after merge** *(RQ-1)*. Checkpoint JSONs
   are Git-tracked, so a post-merge resolution commit sits on an already-merged
   branch with no unmerged PR able to carry it to `main` — precisely the 139-S
   defect (commit `43e70430`, repaired only by the extra PR #395). A "resolve
   after merge" step is never correct for Git-tracked state.
2. **Push precedes evidence** *(RQ-3)*. The re-run review, the locator SHAs and
   the PR-body record all describe a remote state. Deriving any of them from an
   unpushed commit publishes a claim about SHAs no remote has, and a crash before
   push silently loses the resolution.
3. **The PR-body `Reviewed HEAD` record precedes the §1.9 gate** *(RQ-5)*. §1.9
   reads the PR body and requires `Reviewed HEAD == headRefOid`. Running the gate
   first is unsatisfiable, because the resolution commits already advanced HEAD
   past whatever the body recorded. Recording the body first is safe and
   terminating precisely because a PR-body edit is metadata and does not advance
   `headRefOid` (`HEAD_EVIDENCE_RULE`).
4. **The recorded evidence must be produced, not relabelled** *(RQ-4)*. Moving
   the SHA without re-running the review lets a stale verdict be re-pointed at a
   HEAD it never examined — a HEAD that by construction contains commits the
   earlier review never saw. Updating only the SHA is explicitly insufficient.

**Gate-failure remediation loop.** "These are the LAST commits on the branch"
describes the *intended* terminal state, not a prohibition on remediation. If a
downstream gate fails after resolution — CI red on the resolution commit, a new
Copilot thread, a §1.9 finding — the correct response is:

* push a remediation commit **on top of** the resolution commits, which remain
  ancestors of the new HEAD;
* do **not** re-touch, re-create, or re-resolve any checkpoint — they are already
  resolved and their resolution already rides this branch;
* re-run the actual local review at the new HEAD, then update the locator
  `final_head` and the PR-body `Reviewed HEAD` to that new HEAD;
* re-enter the gate set from the top.

The loop terminates because each iteration ends in either a merge or a halt, and
because remediation never creates new checkpoint state to resolve.

**Residual-window checkpoint prohibition.** Once a unit enters
`RESOLUTION_PREFIX`, no new Git-tracked checkpoint may be created for that unit —
not on yield, not for merge approval, not for closure work. Such a checkpoint
could only be resolved by a further commit, which needs a further PR once this
one merges: the original defect, one level down, recursively. The window is
covered by `CLOSURE_LOCATOR` and `LAST_MILE_RECOVERY` instead.

### `CLOSURE_LOCATOR` *(RQ-7 … RQ-9)*

**Surface: the pull-request body**, in an HTML-comment-fenced block. The PR body
is correct on four independent counts: it is not on any branch, so a fresh
checkout of `main` can read it; it is not a commit, so recording a SHA in it is
not self-referential and does not advance `headRefOid`; it survives branch
deletion after merge; and it is queryable by `gh`, which the protocols already
use.

```text
<!-- autoharness:closure-locator v1
shipment: <id>
feature: <id>
pr: <number>
branch: <branch>
status: RESOLUTION_PENDING | RESOLUTION_PUBLISHED | RECONCILED
checkpoints: <filename>[, <filename>...]
resolution_commits: <sha>[, <sha>...]   # empty while RESOLUTION_PENDING
final_head: <sha>                       # empty while RESOLUTION_PENDING
updated_at: <RFC3339 UTC>
-->
```

**Three-phase publication protocol.** The protocol has exactly three phases and
is referred to as three-phase everywhere.

| Phase | When | Content | Why |
|---|---|---|---|
| 1 — `RESOLUTION_PENDING` | **Before** the first resolution commit | shipment, feature, PR number, branch, the checkpoint filenames about to be resolved | A locator appearing only *after* the resolutions would leave the checkpoint-free window uncovered for the interval it exists to cover. Phase 1 requires no SHA, so it has no self-reference problem. |
| 2 — `RESOLUTION_PUBLISHED` | After the resolution commits exist **and are pushed**, before the §1.9 gate | adds `resolution_commits` and `final_head` | The SHAs now exist on the remote and are recorded in **metadata**, never inside a commit *(RQ-8)*. |
| 3 — `RECONCILED` | After merge **and** the **full P-001 post-merge closure set** has completed and been verified — including the P-020 compaction record | terminal | Prevents a completed unit from being rediscovered forever. Setting it at merge, or after partial closure, would discharge the obligation while closure work is still outstanding — the exact failure the locator exists to prevent, one door over *(RQ-7)*. |

**Read protocol — exhaustive and trusted** *(RQ-9)*. Discovery is over pull
requests, not branches and not shipment status. The command is given exactly,
because the obvious formulation is silently wrong:

```text
gh api --paginate \
   -H "Accept: application/vnd.github+json" \
   "repos/{owner}/{repo}/pulls?state=all&per_page=100" \
   --jq '.[] | {number, state, body}'
  → select entries whose body contains `autoharness:closure-locator`
  → parse each block; keep those whose status is NOT RECONCILED
```

**Why not `gh pr list`.** `gh pr list --state all` does **not** return PR bodies
unless `--json ... ,body` is supplied, and it has **no** `--paginate` flag — it
takes a bounded `--limit` that defaults to **30**. The naive formulation
therefore returns thirty bodiless records, finds zero locators, and reports a
clean startup. That is a fail-*open* discovery path masquerading as a fail-closed
one, and it is precisely the failure this requirement exists to prevent. `gh api
--paginate` follows `Link` headers to exhaustion and is the only form permitted.

* Enumeration MUST be **complete**. If the command exits non-zero, or any page
  is truncated, rate-limited, or unparseable, that is an **error**, and the
  session **halts**. An incomplete scan is never evidence of "nothing
  outstanding".
* A shipment-status filter MUST NOT be applied. 139-S's shipment was **archived**
  while its closure obligation was outstanding, so status-keyed discovery would
  have missed exactly the case the mechanism exists for.
* Only the **implementation PR** carries a locator. The post-merge closure PR does
  not publish one: the implementation locator already survives its own merge and
  branch deletion (the PR body is immutable to branch state), and it stays
  discoverable until phase 3 sets `RECONCILED`. A second locator would add a
  reconciliation case with no producer.
* A locator missing any field required for its declared status, or whose block is
  unparseable, is **incomplete** — a **halt-to-operator** signal, never a licence
  to proceed on partial evidence.
* **More than one distinct shipment carrying a non-`RECONCILED` locator is a
  P-001 violation and halts immediately.** P-001 permits exactly one release unit
  in flight; two outstanding closure obligations mean an earlier unit was
  abandoned mid-closure. Recovery must not silently pick one and proceed. Two
  locators for the *same* shipment are **not** this
  case and are handled normally.

### `LAST_MILE_RECOVERY` *(RQ-10, RQ-11)*

Resolving every checkpoint before merge necessarily leaves a **checkpoint-free
residual window** between the last resolution and verified closure. This window
is unavoidable: any Git-tracked checkpoint intended to cover a post-merge window
is provably unresolvable without a further PR, which is the orphaning defect
itself, one level down. The window is therefore covered by **live state
reconstruction anchored on the `CLOSURE_LOCATOR`**, not by a checkpoint.

Recovery in this window MUST NOT rely on checkpoint enumeration, which will
correctly report zero active candidates. **Entry is wired into the zero-candidate
branch itself**: a session finding zero active checkpoints MUST run locator
discovery **before** concluding "clean startup" and before selecting new queue
work. When no non-`RECONCILED` locator exists, normal startup continues
unchanged, byte for byte.

**Step 1 — classify the locator status, then the live PR state; the ancestry
target follows from both.** Asserting `origin/main` ancestry unconditionally is
wrong: while the PR is open its resolution commits are *correctly* not on `main`,
so the assertion would fire a false alarm on the normal path and train the
operator to ignore it.

**Step 1a — locator status gate (runs first).** A `RESOLUTION_PENDING` locator
means the crash happened *between* locator publication and the resolution
commits, so `resolution_commits` and `final_head` are empty **by design**.
Feeding that state into the live-PR table below would compare against an empty
`final_head`, mis-classify it as "HEAD advanced", and — following that row — mark
a branch with **no resolution commits at all** as `RESOLUTION_PUBLISHED` and
present it for merge. Treat it separately and first:

| Locator status | Live PR state | Required action |
|---|---|---|
| `RESOLUTION_PENDING` | **Open** | Resolution has not been published. Do **not** update the locator, do **not** re-establish readiness, do **not** approach the merge bar. Place the working tree on the PR branch (see *Working-tree placement* below), then verify whether resolution commits exist on the fetched PR head. **None exist** → re-enter `RESOLUTION_PREFIX` at the resolve step. **Some exist** (crash after push, before the phase-2 locator update) → do **not** re-resolve; **halt to operator**, because the locator cannot be trusted to enumerate them. If the working tree cannot be placed on the branch, **halt**. |
| `RESOLUTION_PENDING` | **Merged** | **Unrecoverable orphan — halt to operator immediately.** The PR merged carrying unresolved checkpoints. Never run closure, never mark `RECONCILED`. An empty `resolution_commits` list makes every ancestry assertion vacuously pass, so the merged row below must never be reached in this state. |
| `RESOLUTION_PENDING` | any other | **Halt to operator**, per the rows below. |
| `RESOLUTION_PUBLISHED` | — | Proceed to Step 1b. |
| `RECONCILED` | — | Not discovered; terminal. |

**Working-tree placement (required before any re-entry that commits).** Recovery
is entered at session start, when the working tree is normally on `main` and may
be a fresh checkout. Committing a resolution on `main` is forbidden (P-010), and
staying on `main` makes the re-entry undischargeable. Before re-entering
`RESOLUTION_PREFIX` at the resolve step, the agent MUST: read `branch` and `pr`
from the locator; `git fetch origin refs/pull/<pr>/head`; check out a local branch
at that tip; and confirm the checkout succeeded. If any step fails, **halt to the
operator** rather than proceeding on the wrong branch. Steps that only *read*
state (every ancestry assertion in Step 1b) need a fetch but no checkout.

**Step 1b — live PR state classification** (reached only for a complete
`RESOLUTION_PUBLISHED` locator):

| Live PR state | Ancestry target | Required action |
|---|---|---|
| **Open**, HEAD == locator `final_head` | fetched `refs/pull/<n>/head` | Assert each resolution commit is an ancestor of the fetched PR head; **halt to operator on any failure**. Then re-run the local review, §1.9, CI and P-018 at that HEAD; then the merge-authority bar below. `origin/main` ancestry is **not** expected and its absence is **not** an error. |
| **Open**, HEAD ≠ locator `final_head` | fetched `refs/pull/<n>/head` | **Assert first that each recorded resolution commit is still an ancestor of the fetched PR head** (`git merge-base --is-ancestor <sha> FETCH_HEAD`). A force-push, rebase, or branch reset can drop the resolution commits from the new HEAD; merging that HEAD would re-orphan them. If any assertion fails, **halt to operator**. Only if all pass: treat HEAD-pinned evidence as stale, re-establish readiness at the live HEAD, update the locator `final_head`, then the merge-authority bar. |
| **Merged** | `origin/main` | Assert each resolution commit is an ancestor of `origin/main` (`git merge-base --is-ancestor <sha> origin/main`). A non-ancestor here **is** the 139-S orphan and halts. Otherwise continue the full P-001 closure set; set `RECONCILED` only once that whole set is verified. |
| **Closed, not merged** | — | **Halt to operator.** Never report completion, never re-open, never merge. |
| **PR state unavailable / lookup failed** (including a 404 that cannot be distinguished from a deleted PR) | — | **Halt to operator.** Merge status is never inferred. This row absorbs every failed or ambiguous lookup, so no response code is unhandled. |
| **Merge requested, response lost** | — | Not a GitHub-reported state; it is a client-side execution ambiguity. Re-fetch the PR and `origin/main`, then **re-enter this table** with the freshly observed state. **Never issue a blind second merge.** |
| **Locator incomplete / unparseable** | — | **Halt to operator** with the locator contents surfaced. |

**Boundary with the Merge Confirmation Gate.** The "open PR whose resolution
commits are absent from `origin/main` is not an error" rule belongs to **this
recovery protocol only**. It does **not** relax Ship's Merge Confirmation Gate,
which remains NON-NEGOTIABLE: post-merge closure may not begin while the PR
state is anything other than `MERGED`. Recovery may *reconstruct readiness* for
an open PR; it may never *enter closure* for one.

**Step 2 — merge authority is NOT conferred by this path** *(RQ-10)*. This
protocol restores *readiness evidence*; it is not an alternate merge route and
must never become one. A merge may proceed from this path only when **all** hold,
and otherwise the session **halts**:

1. A **complete** locator (status `RESOLUTION_PUBLISHED`, all resolution SHAs
   present, `final_head` present, block parseable) was the trigger.
2. Merge approval is **proven at the live HEAD** — a fresh explicit operator
   approval at that HEAD. An approval read from a memory file, a checkpoint, or
   the locator itself does **not** count.
3. The full current-HEAD gate set (re-run local review, P-014 §1.9, required CI,
   P-018 when engaged) passes at that live HEAD.
4. Resumption authority never implies merge, admin-fallback, or destructive
   approval.

Any doubt at any of the four — halt. The correct failure mode for the last mile
is an unmerged PR awaiting an operator, never an unsupervised merge.

## Implementation units

Eight units. Each edits **one file**, is **documentation-domain only**, and is
sized for well under two hours. Acceptance criteria below are the *exact* text
carried into the task cards.

### U1 — State the resolution-durability policy *(root)*

**File**: `.github/policies/workflow-policies.md` — one new policy section,
`P-022: Checkpoint Resolution Durability`, placed after P-021.

Add an agent-agnostic policy in the existing policy table format (Policy ID,
Applies To, Gate Point, Statement, Precondition, Postcondition, Violation
Action). It states RQ-1 and RQ-2 normatively and names `RESOLUTION_PREFIX` as the
procedure that satisfies them. It does **not** restate the full ordering — the
instruction file owns that — so the two surfaces cannot drift into two different
orders.

**Acceptance criteria**

1. A new `## P-022: Checkpoint Resolution Durability` section exists, using the
   same field table shape as the surrounding policies.
2. `Applies To` names every agent that resolves Git-tracked checkpoints
   (`ship`, `stage`), not Ship alone.
3. `Gate Point` is stated concretely as *Ship Step 5 pre-merge; Stage and Ship
   session end and crash-resumption resolution*, matching the convention that
   every existing policy names a gate point.
4. The Statement prohibits resolving a Git-tracked checkpoint after its carrying
   PR has merged, and requires that resolution state reach the default branch in
   the same merge as the work it belongs to.
5. The Statement names the 139-S incident as the evidence: commit `43e70430`,
   PR #394, repaired by PR #395.
6. Violation Action is halt plus a P-005 telemetry record, matching the
   surrounding policies' convention.
7. The section **cross-references** `RESOLUTION_PREFIX` in
   `github-pr-automation.instructions.md` by name and does **not** restate the
   sequence, so the two surfaces cannot drift into two different orders.
8. The file's **Amendment Log** gains a row for this addition
   (`1.25.0 — Added P-022: Checkpoint Resolution Durability`), matching the
   convention that every prior policy addition carries one, and the header
   `**Version**` field is reconciled to the log's latest entry. Adding a policy
   without an amendment row would break the file's own governance convention.
9. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

### U2 — Record the HEAD-evidence rule, the resolution order, and the closure locator *(root)*

**File**: `.github/instructions/github-pr-automation.instructions.md` — one new
subsection at **heading level 3**, inserted **after the end of `### 1.9`** (that
is, after its last `####` child) and before the next `###` section. It is a
sibling of `### 1.9`, never spliced inside it — a level-3 heading placed among
1.9's children would silently terminate the readiness-gate section and orphan
1.9.3 onward. No existing section is renumbered.

This file is the **canonical home** for `HEAD_EVIDENCE_RULE`, `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION`
and `CLOSURE_LOCATOR`. It is the correct home because the locator is PR-body
metadata — the surface this file already governs — and because siting the text
here keeps it out of any commit, the property that makes it non-self-referential.

**Acceptance criteria**

1. States that PR-body updates do not advance `headRefOid`, and gives
   point-in-time wording as the alternative for committed documents.
2. Cites the observed instances: PR #395 threads `PRRT_kwDORJEduc6h2uOQ` and
   `PRRT_kwDORJEduc6h2viu`.
3. **`RESOLUTION_PREFIX` and `RESOLUTION_POSTCONDITION` appear here verbatim**, exactly as given in this plan's
   canonical definition, including its four ordering invariants, the
   gate-failure remediation loop, and the residual-window checkpoint
   prohibition. This is the single canonical copy.
4. The `CLOSURE_LOCATOR` block is given verbatim with every field named.
5. The publication protocol is described as **three-phase** and its three phases
   are `RESOLUTION_PENDING`, `RESOLUTION_PUBLISHED`, `RECONCILED`. No other
   phase count appears anywhere in the file.
6. Phase 1 is stated to be published **before the first resolution commit** and
   to require no SHA.
7. Phase 2 is stated to follow the **push** of the resolution commits, and to
   carry the SHAs and `final_head`.
8. Phase 3 is stated to require merge **plus the full P-001 post-merge closure
   set, including the P-020 compaction record** — not merge alone, and not
   partial closure.
9. States explicitly that no commit is ever required to record its own SHA, and
   that a branch-only artifact is not fresh-checkout discoverable.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U3 — Record exhaustive discovery and last-mile recovery

**Depends on**: U2 (extends the subsection U2 creates).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

Add the locator read protocol and `LAST_MILE_RECOVERY` immediately after U2's
subsection.

**Acceptance criteria**

1. The read protocol gives the **exact executable command**
   `gh api --paginate -H "Accept: application/vnd.github+json"
   "repos/{owner}/{repo}/pulls?state=all&per_page=100" --jq '.[] | {number, state, body}'`,
   and explicitly records **why `gh pr list` is forbidden**: it returns no body
   without `--json ...,body` and has no `--paginate` flag, only a bounded
   `--limit` defaulting to 30, so the naive form silently reports a clean
   startup.
2. A non-zero exit, a truncated or rate-limited page, or an unparseable response
   is stated to be an **error that halts**, never evidence that nothing is
   outstanding.
3. Filtering discovery by shipment status is **explicitly prohibited**, with the
   139-S archived-shipment case given as the reason.
4. It is stated that only the implementation PR publishes a locator, and why: the
   PR body survives merge and branch deletion, so no closure-PR locator is needed.
5. Non-`RECONCILED` locators for **more than one distinct shipment** are stated
   to be a P-001 violation that halts immediately.
6. An incomplete or unparseable locator is stated to be a halt-to-operator
   signal.
7. The **Step 1a locator-status gate** appears with all five rows, and states
   that a `RESOLUTION_PENDING` locator is evaluated **before** any live-PR
   classification — including that a merged PR with a `RESOLUTION_PENDING`
   locator is an unrecoverable orphan that halts, and must never reach an
   ancestry assertion that an empty commit list would vacuously pass.
8. The **Step 1b live-PR classification** table appears with all seven rows and
   their stated ancestry targets and actions.
9. The `Open, HEAD ≠ final_head` row requires the ancestry assertion against the
   fetched PR head **before** re-establishing readiness, and states that a
   force-push or rebase that dropped the resolution commits halts.
10. It is stated that an open PR whose resolution commits are absent from
    `origin/main` is **not** an error **for this recovery protocol**, and that
    this does **not** relax Ship's Merge Confirmation Gate.
11. The four-part merge-authority bar appears in full, with halt as the default
    for any doubt, and it is stated that this path confers **no** merge authority
    and cannot supply the approval signal P-014 requires.
12. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U4 — Move Ship's checkpoint resolution into the pre-merge path

**Depends on**: U1, U2.
**File**: `.github/agents/_ship.agent.md` — **Step 5 (PR Lifecycle)** and
**Session end item 2**.

The current state, verified against the file: the **only** happy-path resolution
is Session end item 2 (`resolve any still-active checkpoints from the current
session`), which runs after all Required Steps and therefore after Step 6
post-merge closure. Step 6.0 contains **no** resolution step — it only creates
the `post-merge/{feature_slug}` branch. The executable merge path is **Step 5**.
Placing `RESOLUTION_PREFIX` anywhere other than Step 5 would leave Step 5 merging
exactly as it does today.

Wire **`RESOLUTION_PREFIX`** — the pre-merge part only — into Step 5, positioned
**before** the existing readiness/approval/merge items so that the existing
last-mile re-check (item 15), P-009 guardrail (item 16) and merge execution all
operate on the final post-resolution HEAD. The existing approval, re-fetch and
merge items are **not moved, restated or duplicated**. Reference the canonical
text in `github-pr-automation.instructions.md`; do not restate the sequence.
Then amend Session end item 2 so it no longer resolves after merge and no longer
creates a residual-window checkpoint.

**Acceptance criteria**

1. Step 5 invokes **`RESOLUTION_PREFIX`** *by name*, referencing
   `github-pr-automation.instructions.md` as its canonical definition, and does
   **not** restate any of its steps, orderings or rationale. A reader must follow
   the reference to learn the sequence.
2. The invocation is positioned in Step 5 **before** the readiness gate, the
   approval item, the last-mile re-check (item 15), and merge execution — so
   every one of those operates on the final post-resolution HEAD.
3. The entry condition is stated as *the unit owns at least one active checkpoint
   and all branch-mutating work is complete except the resolution-dependent final
   gates*. The circular "PR merge-ready" wording is **not** used. When the unit
   owns zero active checkpoints the prefix is skipped and the pre-existing path
   runs unchanged.
4. **No branch-mutating step remains between the prefix's exit and merge.** Any
   existing Step 5 item that mutates the branch (runtime verification,
   operational-closure artifact generation, follow-up stash writes, the push)
   sits **before** the invocation. The old-to-new item order is recorded in the
   task card so the reordering is auditable rather than implicit.
5. Session end item 2 no longer resolves checkpoints after merge. No
   checkpoint-resolution step remains anywhere after merge in this file.
6. Session end item 2's directive to *"leave at most one final best-effort
   checkpoint"* for merge approval or closure work is **retired for units inside
   `RESOLUTION_PREFIX`**, with the recursion reason stated: such a checkpoint
   could only be resolved by a further commit needing a further PR. The window is
   covered by `CLOSURE_LOCATOR` / `LAST_MILE_RECOVERY` instead.
7. Step 6 gains **`RESOLUTION_POSTCONDITION`** by name — set the locator to
   `RECONCILED` after the full required closure set is verified. It commits
   nothing, so Step 6.0's post-merge branch rule is untouched.
8. Step 6.0's post-merge branch rule is unchanged for every other closure
   artifact, and **Step 6's P-020 `compact-context` invocation is present and
   unmodified**.
9. Step 5's existing P-018 last-mile re-check (item 15) and P-009 guardrail
   (item 16) are retained unmodified.
10. The section cites P-022.
11. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U5 — Ship orphan detection, startup discovery, and locator reconciliation

**Depends on**: U3, U4.
**File**: `.github/agents/_ship.agent.md` — Merge Confirmation Gate and the
`ZERO-CANDIDATE NORMAL STARTUP` block.

Two assertions plus a startup entry point. The durable input is the
`CLOSURE_LOCATOR` in the PR body — not a commit, not a branch-only artifact.

**Acceptance criteria**

1. The locator status is classified **before** the live PR state, per Step 1a, and
   the live PR state is classified **before** any ancestry assertion, per
   Step 1b.
2. The exact commands are given:
   `gh pr view <n> --json state,mergedAt,headRefOid`,
   `git fetch origin refs/pull/<n>/head`, and
   `git merge-base --is-ancestor <sha> <target>`.
3. An open PR whose resolution commits are not on `origin/main` is stated **not**
   to be an error **for the recovery path**, together with an explicit statement
   that this does **not** relax the Merge Confirmation Gate: post-merge closure
   still may not begin while the PR state is anything other than `MERGED`.
4. A merged PR with a non-ancestor resolution commit is stated to be the 139-S
   orphan and **halts**. A merged PR whose locator is still `RESOLUTION_PENDING`
   is stated to be an unrecoverable orphan and **halts** without any ancestry
   assertion.
5. Closed-unmerged, missing PR, failed lookup, and incomplete locator all halt to
   the operator; failure is surfaced, never logged silently.
6. Ship's `ZERO-CANDIDATE NORMAL STARTUP` item 4 runs `CLOSURE_LOCATOR`
   discovery **before** continuing to normal shipment validation, with the same
   halt-on-incomplete-enumeration rule. Ship is directly invokable, so relying on
   the Orchestrator's route alone would leave direct-Ship startup uncovered.
7. The locator is set to `RECONCILED` **only after the full P-001 post-merge
   closure set is complete and verified, including the P-020 compaction record**,
   and it is stated that setting it at merge or after partial closure would lose
   the obligation.
8. No-op when no locator was published; every recorded resolution commit is
   asserted when several were recorded.
9. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U6 — Orchestrator locator reconciliation route

**Depends on**: U3, U5.
**File**: `.github/agents/_orchestrator.agent.md` — Step 0.0b, applied to the
whole step rather than only its zero-candidate arm.

Wire `LAST_MILE_RECOVERY` routing into Step 0.0b. Without this the protocol has
no Orchestrator entry point: an Orchestrator finding no checkpoints proceeds to
state assessment and then to queue selection, starting new work while an unmerged
PR carrying resolution commits sits open.

**Scoping correction (revision 8).** Revision 7 wired discovery only into the
**global** zero-candidate arm — "no checkpoints at all". That leaves the exact
window this plan exists to close still open: Orchestrator permits planning
overlap, so a **Stage-owned** checkpoint can legitimately be active while Ship's
last-mile obligation is outstanding. Under the revision-7 wiring the Orchestrator
would see "a checkpoint exists", skip discovery entirely, never route Ship, and
then skip the shipment at Step 2 because it is still `active` — so nobody
discharges the obligation. Discovery must therefore be scoped the same way Ship's
is (U5 AC6): keyed on the absence of a **ship-owned** active checkpoint, not on
global emptiness.

**Acceptance criteria**

1. Step 0.0b runs locator discovery whenever there is **no `ship`-owned active
   checkpoint**, regardless of whether `stage`-owned checkpoints exist, and always
   **before** `Continue directly to Step 0 State Assessment` — therefore before
   any queue selection.
2. When a `ship`-owned active checkpoint **does** exist, the pre-existing
   owner-routing path is unchanged and discovery is skipped, because that
   checkpoint already carries the obligation.
3. A discovered non-`RECONCILED` locator is routed to **Ship** for
   `LAST_MILE_RECOVERY` **before** any Stage routing and before queue selection.
   If a `stage`-owned checkpoint is also present, both are presented to the
   operator and no new queue work is auto-selected.
4. Discovery is not filtered by shipment status, so an archived shipment with an
   outstanding non-`RECONCILED` locator is found (the 139-S shape).
5. Incomplete enumeration halts rather than falling through to state assessment.
6. Non-`RECONCILED` locators for more than one distinct shipment halt as a P-001
   violation.
7. Routing into `LAST_MILE_RECOVERY` conveys **no merge authority** and no
   implicit approval; the four-part merge bar applies unchanged and any doubt
   halts. In dark-factory mode (P-017), a dark approval for the *current* declared
   scope does **not** satisfy the merge bar for a *prior* unit's recovered
   obligation.
8. Owner exclusivity is preserved: the Orchestrator **routes**; Ship **performs**
   the reconciliation. The Orchestrator never performs recovery itself.
9. When no locator is found, the pre-existing fall-through is unchanged: the same
   `Continue directly to Step 0 State Assessment` instruction, the same
   non-failure classification, and no new operator interaction.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U8 — Close the Stage-side resolution gap

**Depends on**: U1.
**File**: `.github/agents/_stage.agent.md` — **both** resolve sites: Session end
item 2 **and** the Crash-Resumption `OWNER-SCOPED RESOLUTION` block.

Stage's Session end item 2 carries the same *"resolve any still-active
checkpoints"* and *"leave at most one final best-effort checkpoint"* sentences as
Ship's, and its `OWNER-SCOPED RESOLUTION` block resolves after a confirmed
resume. P-022 (U1) is agent-agnostic, so after U1 lands, Stage would hold a
procedure that unconditionally resolves against a policy that forbids resolving
after the carrying PR merged. Revision 6 recorded this as residual risk RR-2; the
independent review correctly held that a universal requirement with a procedural
gap in one of its two named agents is not realized. This unit closes it with a
narrow qualifier rather than by narrowing the policy.

Stage cannot open, approve, or merge pull requests (P-010), so Stage does **not**
execute `RESOLUTION_PREFIX` and is given no locator obligation. The qualifier is
only: do not resolve into an already-merged staging PR.

**Acceptance criteria**

1. **Both** resolve sites — Session end item 2 and `OWNER-SCOPED RESOLUTION` —
   state that a checkpoint MUST NOT be resolved once the staging PR carrying its
   resolution has merged, citing P-022.
2. An **executable predicate** is given for that test, not a prose condition:
   determine the current branch, then
   `gh pr list --state merged --head <branch> --json number,mergedAt`; a non-empty
   result means the carrying PR has merged. A lookup failure **halts** rather than
   assuming "not merged".
3. The correct action in the merged case is stated: leave the checkpoint active,
   surface it, and hand off to the operator — never resolve into a merged branch.
4. The *"leave at most one final best-effort checkpoint"* directive is qualified:
   a Git-tracked checkpoint MUST NOT be created when no open staging PR can carry
   its eventual resolution, because Stage cannot open a PR (P-010) and so could
   never discharge it. Hand off to the operator instead.
5. It is stated that Stage does **not** execute `RESOLUTION_PREFIX` and publishes
   no locator, because Stage holds no merge authority (P-010); the reference is to
   P-022's prohibition only.
6. No other Stage behaviour is modified; the existing checkpoint payload contract
   and normal (unmerged-PR) resolution semantics are unchanged.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U7 — Capture the compound learning *(closure deliverable)*

**Depends on**: U5, U6.
**File**: `docs/compound/workflow-issues/` — one new learning.

U7 is a **closure deliverable**, not a requirement-realizing unit. It implements
none of RQ-1 … RQ-11 and is deliberately absent from the requirement trace table
below. It is retained because capturing hard-won solutions is standing workspace
practice, and this defect class cost two extra pull requests to discover.

**Acceptance criteria**

1. The structural post-merge ordering root cause is documented with its observed
   evidence: 139-S, PR #394, commit `43e70430`, repair PR #395.
2. `HEAD_EVIDENCE_RULE` is stated with the PR #395 thread citations
   (`PRRT_kwDORJEduc6h2uOQ`, `PRRT_kwDORJEduc6h2viu`).
3. The **self-referential locator trap** is recorded: a commit cannot contain its
   own SHA, and a branch-only record is invisible to the fresh checkout that must
   perform the recovery.
4. The **status-filtered discovery trap** is recorded: 139-S's shipment was
   archived while its obligation was outstanding.
5. Frontmatter matches the prevailing convention of
   `docs/compound/workflow-issues/` and is internally consistent.
6. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

## Requirement → unit traceability

Every requirement has exactly **one owning unit** — the unit that installs the
normative text — plus zero or more **enforcing units** that wire that text into
an execution path. U7 is a closure deliverable and realizes no requirement; it is
therefore absent by design.

| Requirement | Owning unit | Enforcing units |
|---|---|---|
| RQ-1 no post-merge resolution | U1 (policy) | U4 (Ship), U8 (Stage) |
| RQ-2 resolution rides the same merge | U2 (`RESOLUTION_PREFIX`) | U4 (Ship). **Not U8** — U8 enforces the RQ-1 prohibition on the Stage path; Stage's resolution reaches `main` through the ordinary staging-PR merge, which Stage does not control, so U8 cannot *guarantee* RQ-2 and is no longer credited with it. |
| RQ-3 push before evidence | U2 | U4 |
| RQ-4 re-run the actual review | U2 | U4 |
| RQ-5 PR-body record precedes §1.9 | U2 | U4 |
| RQ-6 approval after gate, live re-fetch before merge | U2 | U4 |
| RQ-7 locator discoverable through closure | U2 (`CLOSURE_LOCATOR`) | U5 |
| RQ-8 non-self-referential locator | U2 | — |
| RQ-9 exhaustive, trusted, status-independent discovery | U3 (read protocol) | U5, U6 |
| RQ-10 no merge authority conferred | U3 (`LAST_MILE_RECOVERY` Step 2) | U5, U6 |
| RQ-11 every failure halts | U3 | U5, U6 |

## Dependencies

```text
U2 ──→ U1 ──┬──→ U4 ──→ U5 ──→ U6 ──→ U7
  │         └──→ U8            ↑
  └──→ U3 ──────────────────┬──┘
                            └──→ U5
```

Eleven edges: **U2→U1**, U1→U4, U1→U8, U2→U4, U2→U3, U3→U5, U3→U6, U4→U5,
U5→U6, U5→U7, U6→U7. The graph is acyclic.

The single **root is U2**. Revision 7 listed U1 and U2 as co-roots, but U1
installs a by-name reference to `RESOLUTION_PREFIX` whose definition only U2
creates; running U1 first would leave a dangling reference that cannot be
verified. **U2→U1** is therefore a real edge, and it transitively orders U8 after
the canonical definition too.

The two units that share `github-pr-automation.instructions.md` (U2, U3) are
strictly sequential, as are the two that share `_ship.agent.md` (U4, U5). U8 is
the only unit touching `_stage.agent.md`, U6 the only unit touching
`_orchestrator.agent.md`, and U1 the only unit touching `workflow-policies.md`.
No two concurrently-eligible units edit the same file.

## Verification

| # | Check | Expected result |
|---|---|---|
| V1 | `markdownlint` over every changed file | passes |
| V2 | Canonical-copy check, in two parts. **(a)** The `RESOLUTION_PREFIX` code block appears verbatim exactly once, in `github-pr-automation.instructions.md`. **(b)** In `workflow-policies.md`, `_ship.agent.md` and `_stage.agent.md`, grep for each of the eight step verbs of the sequence (`publish`, `resolve`, `push`, `re-run`, `update`, `record`, `re-run ALL`, `obtain`) within the changed regions. | (a) exactly one verbatim copy; (b) **zero** step-verb restatements — only the bare names `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION` and a file reference |
| V3 | Grep every changed file for `two-phase` and for `pr_role` | zero occurrences of each; only "three-phase" appears |
| V4 | **Ordered-step inspection** of `_ship.agent.md` (not a grep), recording the item numbers: read Step 5 top-to-bottom and confirm (a) the `RESOLUTION_PREFIX` invocation precedes the readiness gate, approval, last-mile re-check and merge execution, and (b) **no branch-mutating item appears between the invocation and merge**; then read Step 6 and Session end and confirm neither contains a resolution or checkpoint-creation step for the merged unit | (a) ordering holds; (b) the set of items between invocation and merge is empty of mutations; (c) no post-merge resolution or creation |
| V5 | Backlog structure equals `143-F` → 2 sub-epics → 8 tasks, and the shipment manifest membership equals those 11 IDs | exact **membership** match; order is not asserted, because Ship treats manifest order as non-executable and sorts by unfinished dependencies |
| V6 | Every task card's acceptance criteria are textually identical to its unit's criteria in this plan | identical |
| V7 | Grep the **changed files** (not this plan) for this closed list of Defect-1 construct names: `DARK_CONTINUATION_PREDICATE`, `ACTIVATION_RECORD_STORE`, `CURSOR_TYPING_RULES`, `CONTINUATION_HANDOFF_EVIDENCE`, `OWNER_SIDE_REVALIDATION`, `PREDICATE_PRECEDENCE`, `MIS_EVALUATION_DIRECTIONALITY`, `SCOPE_MATCH_RULES`, `check-continuation-predicate-drift`, `canonical-phrases.json`, `parity-gate` | zero occurrences of all eleven |
| V8 | Execute the U3 discovery command against this repository with `per_page=2`, count returned records, and compare against `gh api repos/{owner}/{repo}/pulls?state=all --jq 'length'` run per-page | record count exceeds 2, proving `Link`-header pagination actually followed; every record carries a non-empty `body` field; exit code 0 |
| V9 | Confirm Step 6's P-020 `compact-context` invocation in `_ship.agent.md` is byte-identical to its pre-change text | identical |
| V10 | Read `_stage.agent.md` Session end item 2 **and** the `OWNER-SCOPED RESOLUTION` block; confirm both carry the P-022 merged-PR prohibition and the executable `gh pr list --state merged --head <branch>` predicate, and that the best-effort-checkpoint directive is qualified | both sites carry both; directive qualified |
| V11 | Read `_orchestrator.agent.md` Step 0.0b; confirm discovery is keyed on the absence of a **ship-owned** active checkpoint, not on global zero-candidate | keyed on ship-owned absence |

## Residual risks

| Ref | Risk | Disposition |
|---|---|---|
| RR-1 | These are prose protocols executed by an LLM. Correct wording does not prove correct execution. | **Accepted and recorded.** The prior plan's answer — a static wording checker — could only prove the words had not changed, not that the protocol ran. U5's ancestry assertion is the real defence: a concrete command with a pass/fail outcome that makes a miss loud. V8 additionally proves the one discovery command that was silently wrong in revision 6. |
| RR-2 | *(Closed in revision 7.)* Stage's session-end resolution was previously bound by P-022 with no procedural rewiring. | **Closed by U8.** The independent review held that a universal requirement with a procedural gap in one of its two named agents is not realized. U8 adds the narrow Stage qualifier without granting Stage merge authority. |
| RR-3 | The locator lives in the PR body, which a human or bot can edit or delete. | **Accepted.** The mitigation is the halt-on-incomplete-locator rule: a damaged locator stops the session rather than being silently ignored. A *deleted* locator is indistinguishable from one that was never published, and would fall back to pre-change behaviour — no worse than today. |
| RR-4 | `gh api --paginate` over all PRs grows with repository history. | **Accepted.** Cost is bounded by PR count and runs once per zero-candidate startup. Correctness was chosen over speed deliberately (RQ-9). |
| RR-5 | A crash between locator phase 1 and the resolution commits leaves a `RESOLUTION_PENDING` locator with no commits. | **Handled, not merely accepted** — `LAST_MILE_RECOVERY` Step 1a re-enters `RESOLUTION_ORDER` for the open case and halts for the merged case. Listed here because the handling is a recovery path, not a prevention. |

## Out of scope

* All Defect-1 work — see
  `docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md`
  (status `open`). Harvest of Defect 1 is **not** authorized.
* The drift-checker script pair, fixture corpus, parity runner, hook shim, and
  the `.gitignore` entry they required.
* The task↔plan acceptance parity gate.
* backlogit tool changes; `src/`; `crates/`.
* Shipments 140-S, 141-S, 142-S; feature 142-F.
* Upstream autoharness template propagation.

## Retained review history

This section is **evidence, not authority**. It records what previous revisions
were reviewed against. None of it authorizes harvest of revision 7.

| Round | Reviewers | Verdict | Scope reviewed |
|---|---|---|---|
| 1 | Scope Boundary Auditor (`gpt-5.6-sol`), Constitution Reviewer (`claude-opus-4.8`) | FAIL (6×P1, 4×P2) / ADVISORY | revision 1 (both defects) |
| 2 | Scope Boundary Auditor | FAIL (5 blocking, 1 new P2) | revision 2 (both defects) |
| 3 | Scope Boundary Auditor | FAIL (5 blocking, mechanical cross-reference contradictions) | revision 3 (both defects) |
| 3-confirm | Scope Boundary Auditor | PASS — **superseded and withdrawn** | Same-reviewer, scoped to its own five findings, obtained without the mandatory escalation. Never a valid harvest gate. |
| 4 — P-013.6 escalation | Independent escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh`; same-route guard NOT triggered | **ESCALATION_BLOCKS** | 8 blocking corrections. **Seven were incorporated into revision 4; blocker 8 — the requirement for a fresh independent full-plan review — remained outstanding at that point.** |
| 5 | Independent seven-persona panel: Constitution, Rust/feasibility, Scope Boundary Auditor, Learnings, Architecture, Agent-Native Parity, Security | **FAIL** (13 blocking P1 plus P2/advisory) | revision 4 (both defects). Blocker 8 discharged **as process** — the review was performed — but its verdict was FAIL, so the gate it guarded stayed closed. |
| 6 | Independent four-persona panel: Scope Boundary Auditor (`gpt-5.6-sol`), Constitution (`claude-opus-4.8`), Correctness (`gemini-3.8-flash`), Agent-Native Parity (`grok-4.6`) | **FAIL** (3 FAIL, 1 ADVISORY) | revision 6 (Defect 2 only). All findings were specification defects; none falsified the design. Remediated into revision 7 — see the disposition table below. |
| 7 | Independent four-persona panel, same personas and models as round 6 | **FAIL** (Scope, Correctness, Parity FAIL; Constitution ADVISORY) | revision 7. Panel confirmed **every** revision-6 finding genuinely closed and the design still sound; new findings were deeper specification defects exposed by the earlier fixes. Remediated into revision 8 — see below. |
| 8 | Independent Scope Boundary Auditor (`gpt-5.6-sol`, xhigh) | **FAIL** (3 P1, 5 P2) | revision 8 (this document). **Third consecutive FAIL. Review circuit OPEN — attempt counter 3.** P-013.6 escalation fired; Stage halted without harvesting. |

### Round 8 — outstanding findings (NOT remediated; circuit open)

Revision 8 closed every round-7 P1 the auditor could verify: the P-003 sub-epic
tier is restored, U4 AC1 references without restating, the RQ-2 trace explicitly
excludes U8, U4 AC4 exposes the Step-5 reorder, hardening D6 defers to D10, U2's
locus is fixed, `pr_role` is gone, and U6 is owner-scoped. No Defect-1 construct
is reintroduced. All eight units remain single-file documentation units.

These three P1s remain open and are carried to escalation:

| # | Finding | Required fix |
|---|---|---|
| 1 | **RQ-6 has no executable enforcement path.** The canonical prefix asserts that a live re-fetch of HEAD, threads and CI already exists in Ship Step 5 and is unchanged. It does not: real item 15 re-runs the P-018 gate and re-queries `headRefOid` only — it never re-fetches required CI, and it does not refresh all review threads when P-018 is disabled. U4 AC9 then *requires* item 15 to stay unmodified, so RQ-6 is credited to a unit that is forbidden from implementing it. | Expand U4 to amend the last-mile item with an explicit fail-closed post-approval query of live HEAD, full thread state and required checks; drop the "item 15 unmodified" criterion. |
| 2 | **The zero-checkpoint bypass claim is false.** U4 AC3 promises a zero-checkpoint unit runs "the pre-existing path unchanged", while AC2/AC4 require all mutating items to move before the prefix and the readiness gate to move after it. Real Ship Step 5 has readiness items 7b/7c *before* runtime verification, closure-artifact generation, follow-up writes and the push (items 7–10), so the reorder changes the common path for every unit, including zero-checkpoint ones. | State that zero-checkpoint units skip locator publication and resolution but use the newly ordered common readiness path. Do not claim their step order is unchanged. |
| 3 | **D14 is not actually folded into a task criterion.** D14 requires reading `branch`/`pr`, fetching the PR head, checking out a local branch, confirming the checkout and halting on failure. Its cited U5 AC2 lists only `gh pr view`, `git fetch` and `git merge-base` — no checkout, no verification — and no V-check covers it. Because task-card criteria are declared exact, U5 could pass while recovery is still sitting on `main`. This re-opens the very hazard D14 was written to close. | Add a U5 acceptance criterion requiring the by-name working-tree placement, checkout and verification before any committing re-entry, plus a matching verification check; then repoint D14's fold reference. |

Open P2s: V2 is unsatisfiable as written (it forbids verbs U1/U8 are required to
use); V8's reference command lacks `--paginate` so its comparison is invalid;
several hardening `Folds into` AC references are still stale; and the decision
document's in-scope list omits `_stage.agent.md`, says "7-task plan", and still
names the retired `RESOLUTION_ORDER`.

## Escalation record (P-013.6)

**Trigger**: plan-review attempt counter reached **3** with three consecutive FAIL
verdicts (revisions 6, 7, 8).

**Resolved escalation route**: `gpt-5.6-sol` / `openai` / `xhigh`, read fresh from
`.autoharness/config.yaml` `model_routing.stage.escalation` at session start. The
legacy flat `model_routing.escalation` key is empty, so there is no both-present
ambiguity.

**Same-route guard**: NOT triggered. Stage's own role route is
`claude-opus-5` / `anthropic` / `high`, which differs from the escalation route in
all three fields. `ESCALATION_DEGRADED` therefore does **not** apply and the
escalation is live rather than a no-op.

**Disposition**: the failing operation is **not** re-executed. Stage has halted.
No harvest, no shipment assembly, no successor ID allocation. The escalation is a
reasoning escalation only and confers no authority to promote this plan.

**Assessment carried to escalation**: the architecture has not been falsified.
Three independent panels have each confirmed the design sound and each closed
finding has stayed closed. The failure mode is that this plan specifies *edits to
agent prompt files* against a target (`_ship.agent.md` Step 5) whose real item
ordering is more entangled than a documentation-domain unit can restate safely —
every round has surfaced a further mismatch between what the plan asserts Step 5
contains and what it actually contains. The escalation question is therefore
whether U4 should be decomposed against a **verbatim extract of the real Step 5
item list** rather than against a prose description of it.

### Round 7 — disposition of the independent revision-7 review

| Finding | Source | Revision-8 disposition |
|---|---|---|
| Circular precondition `work complete AND PR merge-ready` still in the **canonical block** (line 135) and U4, despite the disposition table claiming it fixed; U2 installs that block verbatim, so the circularity would ship | Correctness P1 | **Fixed.** Canonical block rewritten with an explicit `entry:` clause — at least one active checkpoint plus all branch-mutating work complete except the resolution-dependent gates. The phrase "PR merge-ready" is gone from the definition and from U4. |
| `RESOLUTION_ORDER` spans both sides of the merge, so U4's "invoke by name, do not restate" is unsatisfiable — an agent either re-merges or stops at an undefined boundary | Parity P2, Scope P1 | **Fixed.** Split into `RESOLUTION_PREFIX` (ends at readiness, before approval/merge) and `RESOLUTION_POSTCONDITION` (a Step 6 metadata write). The existing approval/re-fetch/merge items are explicitly not moved or duplicated. |
| U4 AC6–AC9 restated ordering the canonical owner owns, so the surfaces could drift while V2 still passed | Scope P1 | **Fixed.** Those ACs removed; U4 now carries only invocation, placement and non-restatement criteria. V2 gained part (b): grep for the sequence's step verbs in the referencing files and require **zero**. |
| U4 hid a materially larger Step-5 reorder than its S/<2h estimate admitted — AC2 and AC3 together require moving several existing mutating items | Scope P1 | **Fixed.** New U4 AC4 requires all branch-mutating items to sit before the invocation and the old-to-new order to be recorded in the task card. V4 gained part (b): assert the set of items between invocation and merge contains no mutation. |
| Orchestrator discovery wired only into the **global** zero-candidate arm — a legitimately active Stage checkpoint masks the obligation, Ship is never routed, and Step 2 then skips the shipment for being `active` | Parity P1 | **Fixed.** U6 rescoped to fire whenever there is no **ship-owned** active checkpoint, matching Ship's own scoping (U5 AC6), with explicit precedence over Stage routing and queue selection. |
| Recovery re-entry is not executable from a fresh checkout — it must commit, but startup is on `main`, where committing is P-010-forbidden | Parity P2 | **Fixed.** New **Working-tree placement** rule: read `branch`/`pr` from the locator, fetch `refs/pull/<pr>/head`, check out, halt on failure. Read-only ancestry assertions need a fetch but no checkout. |
| U8 patched only Session end, gave no executable predicate, and left the undischargeable best-effort checkpoint in place | Parity P2, Scope P1 | **Fixed.** U8 now covers **both** Stage resolve sites, supplies `gh pr list --state merged --head <branch>` as the test (halt on lookup failure), and qualifies the checkpoint-creation directive. New V10. |
| U8 credited with enforcing RQ-2 although it only prohibits; Stage cannot guarantee its resolution reaches `main` | Scope P1 | **Fixed.** Trace now credits U8 with RQ-1 only, and states why it is not credited with RQ-2. |
| P-003 item 4 requires every task to reference a parent **sub-epic**; the flat decomposition was justified by precedent, not policy text, and P-003's violation action is Halt | Scope P1, Constitution P2 | **Fixed.** Two sub-epics restored — `143-E1` (ordering contract: U1, U2, U4, U8) and `143-E2` (discovery and recovery: U3, U5, U6, U7) — mirroring the plan's own prevention/recovery split. V5 updated. |
| `pr_role: implementation \| closure` is a ghost specification — no unit ever publishes a closure-PR locator, and recovery never branches on it | Correctness P2, Scope P2 | **Fixed.** Field removed everywhere. The locator is stated to be implementation-PR-only, with the reason: the PR body survives merge and branch deletion. |
| No bypass specified for a unit owning zero checkpoints — the prefix would publish an empty locator | Correctness P2 | **Fixed.** Entry condition requires ≥1 active checkpoint; zero-checkpoint units skip the sequence and run the pre-existing path unchanged. |
| U1 lacked the Amendment Log row and version bump every prior policy addition carries; `Gate Point` value never specified | Constitution P2, P3 | **Fixed.** New U1 AC3 (concrete gate point) and AC8 (amendment row `1.25.0` plus header version reconciliation). |
| U2's locus was self-contradictory — "sibling of `### 1.9`" but "placed after `#### 1.9.2`", which would orphan 1.9.3 onward | Scope P2, Correctness P3 | **Fixed.** Locus is now a level-3 section after the **end** of `### 1.9`, with the orphaning hazard stated as the reason. |
| Constitution Check omitted P-017 (recovery auto-enters and walks to a merge bar; dark mode could supply approval for a prior unit's obligation), P-012 and P-008; P-010 row not updated for U8 | Constitution P2, P3 ×3 | **Fixed.** All three rows added, P-010 extended, and U6 AC7 states that a dark approval for the current scope does not satisfy the merge bar for a prior unit's recovered obligation. |
| U1 and U2 listed as co-roots although U1 references a definition only U2 creates | Scope P2 | **Fixed.** Edge **U2→U1** added; U2 is the single root; edge count 10 → 11. |
| Step 1a's "resolution never happened" prose ignored the crash-after-push-before-phase-2 window | Correctness P3 | **Fixed.** The row now branches: no commits → re-enter; commits present → halt, because the locator cannot be trusted to enumerate them. |
| V2/V3/V4/V7/V8 not uniformly falsifiable | Scope P2 | **Fixed.** V2 split into two parts, V3 given concrete tokens, V4 given an explicit mutation check, V7 closed to eleven literal names, V8 rewritten to force a page boundary with `per_page=2`. V10 and V11 added. |
| Hardening D6 ("classify live PR state first") contradicted D10 (locator-status gate first) | Scope P1 | **Fixed** in the hardening document: D6 now applies only after Step 1a admits a complete `RESOLUTION_PUBLISHED` locator. Stale `Folds into` AC references refreshed throughout. |

### Round 6 — disposition of the independent revision-6 review

| Finding | Source | Revision-7 disposition |
|---|---|---|
| `RESOLUTION_ORDER` ownership contradiction — U1 said the instructions file owned it, but U2/U3 never installed it there while U4 required it verbatim in Ship | Scope P1 | **Fixed.** New `Canonical ownership` table names one installed home per definition. U2 AC3 installs `RESOLUTION_ORDER` verbatim; U1, U4 and U8 reference it by name only. V2 rewritten to assert exactly one verbatim copy. |
| `gh pr list --state all` is not an exhaustive-discovery command — no `--paginate`, bounded `--limit 30`, and returns no body | Scope P1, Correctness P3, Parity P2 | **Fixed.** Read protocol now gives the exact `gh api --paginate … /pulls?state=all&per_page=100 --jq …` command, with an explicit note on *why* `gh pr list` is forbidden. New V8 executes it. |
| U4 targeted the wrong loci — resolution is only at Session end item 2; Step 6.0 has no resolve step; the executable merge path is Step 5 | Scope P1, Correctness P2, Parity P1 | **Fixed.** U4 retargeted at **Step 5** and Session end item 2, with an AC requiring the invocation to precede the readiness gate, approval, last-mile re-check and merge. Circular "PR merge-ready" precondition replaced. |
| Session end still directs creating a best-effort checkpoint on yield — hardening D1 recursion, one level down | Correctness P1, Parity P2 | **Fixed.** New `Residual-window checkpoint prohibition` in `RESOLUTION_ORDER`; U4 AC5 retires the directive explicitly; V4 inspects for it. |
| `RESOLUTION_PENDING` locator unhandled — an open PR would be marked `RESOLUTION_PUBLISHED` with no resolution commits; a merged one would vacuously pass ancestry and be marked `RECONCILED`, orphaning the checkpoints | Correctness P1 | **Fixed.** New `Step 1a` locator-status gate runs before any live-PR classification, with explicit open (re-enter `RESOLUTION_ORDER`) and merged (unrecoverable orphan, halt) rows. |
| `Open, HEAD ≠ final_head` row omitted the ancestry assertion — a force-push could drop the resolution commits and the row would merge anyway | Correctness P1 | **Fixed.** Row now asserts ancestry against the fetched PR head **first** and halts on failure. |
| RQ-1/RQ-2 universal but only Ship procedurally rewired; RR-2 does not realize a requirement | Scope P1, Parity P3 | **Fixed.** New unit **U8** adds the narrow Stage qualifier. RR-2 closed rather than carried. |
| Decision DoD required "exactly one implementation unit" per RQ while the trace was many-to-many | Scope P1 | **Fixed.** Trace table now names one **owning** unit plus **enforcing** units per RQ; the decision's DoD wording is corrected to match. |
| U6 routed to an undefined "owning agent"; Ship's own zero-candidate path had no discovery | Scope P2, Parity P2 | **Fixed.** U6 AC6 routes explicitly to **Ship**; U5→U6 edge added; U5 AC6 adds discovery to Ship's `ZERO-CANDIDATE NORMAL STARTUP`. |
| U5's "open PR not an error" could be read as weakening the NON-NEGOTIABLE Merge Confirmation Gate | Parity P2 | **Fixed.** New `Boundary with the Merge Confirmation Gate` paragraph; U3 AC10 and U5 AC3 both state the boundary. |
| No remediation loop specified for a gate failure after resolution | Correctness P2 | **Fixed.** New `Gate-failure remediation loop` in `RESOLUTION_ORDER`; U4 AC9. |
| Constitution Check omitted P-001, P-020, P-015; P-014 row overclaimed "strengthened"; P-003 sub-epic tier unaddressed | Constitution P2/P3 ×4 | **Fixed.** All added; the P-014 row now names the hazard this plan *introduces* and the specific mitigation; P-003 states the flat feature-direct shape explicitly. |
| `RECONCILED` could be set before the P-020 compaction record completes | Constitution P2 | **Fixed.** Phase 3, U2 AC8 and U5 AC7 all require the full P-001 closure set including the P-020 record. U4 AC10 and V9 protect the P-020 invocation itself. |
| U7 was listed as realizing a requirement but implements none | Scope P2 | **Fixed.** U7 reclassified as a closure deliverable and deliberately excluded from the trace table. |
| Verification was weak — V2 token-presence only, V4 asked grep to infer ordering, V7 list incomplete, "byte-for-byte" unfalsifiable | Scope P2 | **Fixed.** V2 is a canonical-copy check; V4 is an ordered-step inspection; V7 expanded to eleven names; U6 AC7 replaced with a concrete fall-through criterion; V8 and V9 added. |
| Multiple non-`RECONCILED` locators across different shipments undefined | Correctness P3 | **Fixed.** Read protocol states this is a P-001 violation that halts; U3 AC5, U6 AC4. |
| Constraint "nothing in this plan references Defect 1" factually false | Scope P3 | **Fixed.** Narrowed to "no implementation unit introduces or depends on a Defect-1 construct"; V7 scans changed files, not this plan. |

**Why revision 6 is a reduction rather than a revision-5 remediation.** The
revision-5 findings R3, R4, R5, R11 and R12 were each attempts to specify
executable persistence in prose. They were remediated by *adding more prose*. The
halted revision-6 attempt recognized that this could not converge and classified
Defect-1 safety as requiring a new executable component or upstream support — a
conclusion the operator has approved. Revision 6 therefore removes Defect 1
rather than attempting an eighth specification of it. The revision-5 findings
that apply to Defect 2 — R7 (self-referential locator), R8 (live-PR-state-first),
R9 (status-independent discovery), R10 (re-run the review), R13 (merge-authority
bar) — are all carried forward and are realized by U2–U6.

<!-- plan-review-attempt: 3 -->
