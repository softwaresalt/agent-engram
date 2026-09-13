---
doc_type: exec-plan
date: 2026-09-13
revision: 10
scope: defect-2-only
status: blocked
review_verdict: FAIL
review_verdict_revision: 10
review_attempts: 5
escalation: P-013.6 fired at revision 8 (route gpt-5.6-sol/openai/xhigh). Rounds 6-10 all returned FAIL. Revision 10 was a second bounded remediation revision explicitly authorized by the operator, carrying the operator's RR-3 decision (an approval-gated GitHub branch-ruleset prerequisite), plus ONE fresh full independent review. Round 10 returned FAIL (4 P0, 17 P1) unanimously across four cross-model reviewers; the circuit is OPEN at attempt counter 5 and the panel recommends returning a narrower design to the operator rather than a further in-place remediation.
harvest_authorized: false
source_document: docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md
supersedes: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md
stash_ids: [4EF24729]
policies: [P-001, P-003, P-005, P-006, P-008, P-009, P-010, P-011, P-012, P-014, P-015, P-016, P-017, P-018, P-020, P-022]
requires_plan_hardening: yes
hardening_document: docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md
task_count: 14
dependency_edge_count: 22
sub_epic_count: 3
contains_proposed_action: true
proposed_action_units: [U0]
---

# Checkpoint Resolution Durability — Implementation Plan (revision 10, Defect 2 only)

**Source document**: `docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md`
**Requires plan hardening**: **yes** — this plan changes merge-adjacent ordering
governed by P-014/P-018, adds a startup route that could, if mis-specified,
become an unsupervised merge path, and now carries one **`ProposedAction`** unit
(U0) that mutates GitHub repository settings.

**Machine-readable status.** The frontmatter is authoritative. `revision: 10`,
`scope: defect-2-only`, `harvest_authorized: false`,
`review_verdict: FAIL`, `review_verdict_revision: 10`, `review_attempts: 5`,
`status: blocked`, `contains_proposed_action: true`.

**Round-10 outcome — FAIL. This revision is BLOCKED and not harvestable.**
Revision 10 was reviewed at HEAD `23731cba` by a fresh independent four-persona
cross-model panel (correctness, constitution/policy, scope and maintainability,
security and ops risk). All four returned **FAIL**: **4 P0**, **17 P1**, 9 P2,
5 P3. The complete finding set is retained verbatim in `## Plan Review — round 10`
at the end of this document. The round-9 P0 (F-11, the ambient-branch predicate)
was independently confirmed **closed** by all four reviewers, and the Ship Step 5
structural extraction was independently re-derived and confirmed accurate. But four
new P0s were found: the canonical Channel B block and U13 install different
protocols and U13 reinstates the discharge-rejection rule the canonical split
exists to prevent; U13 AC6 halts on the protocol's own happy-path interval; the
`FETCH_HEAD` versus retained-ref contract is unsatisfiable across the canonical
block, U11, D14 and V22; and U0's `deletion` + `non_fast_forward` rules would break
branch cleanup and rebase repository-wide for every contributor. The panel's
recommendation — advisory to the operator, not a decision taken here — is to keep
the ordering fix, replace the branch-ruleset prerequisite with a protected
immutable marker ref, and demote the Channel B obligation machinery to an optional
hardening layer, rather than attempt a third in-place remediation. **Read the
sections below with that verdict in view: they record what revision 10 specifies,
not what has been accepted.**

**Round-9 outcome — FAIL, and what followed.** Revision 9 was reviewed by a fresh
full independent seven-persona panel at HEAD `9b15fd43` and returned **FAIL**
with **1 P0** and **15 P1** findings after dedupe, with **RR-3 re-opened**. That
complete finding set is retained verbatim in `## Plan Review — round 9`. The
operator then made the **RR-3 decision** the round-9 review returned as a
blocker, and authorized a **second bounded remediation revision plus one fresh
full independent review**. Revision 10 is that revision. Its own review has
**not yet returned**, so nothing below may be read as approved and
`harvest_authorized` stays **false**.

**The operator's RR-3 decision (the load-bearing change in revision 10).** RR-3
is closed by an **approval-gated GitHub branch-ruleset prerequisite** — *not* by
a CI-check persistence substrate, and *not* by weakening RQ-7. The decision has
three parts:

1. **A repository ruleset must cover every permitted resolution-bearing Ship
   branch pattern** with both `deletion` and `non_fast_forward` rules, no bypass
   actors, and no current-user bypass. Configuring it is an **external GitHub
   settings change**, so it is carried by a distinct unit **U0**, classified
   `ProposedAction` / `ActionRisk: high` / `approval_required: true`. **This
   planning PR does not apply the setting**; U0 requires explicit operator or
   admin approval at implementation time.
2. **Ship must prove the protection is live before it publishes any obligation**,
   by querying the effective-rules API for the **actual live `headRefName`**.
   API unavailable, ambiguous rules, uncovered branch, or any relevant bypass
   capability **halts before obligation publication** (new segment **S3.5**,
   new requirement **RQ-13**).
3. **Channel B is made independent of the PR body** and is rebuilt on the
   protected head: it enumerates trusted PRs exhaustively from API fields alone,
   fetches each to a **unique retained local ref**, and scans the **full
   reachable history** of that ref by canonical path and record identity. The
   ruleset's `non_fast_forward` + `deletion` rules are what make an
   ordinary later deletion commit still detectable, because the introducing
   commit stays reachable and the branch cannot vanish.

**Measured evidence behind that decision (read-only, gathered at revision 10).**
Repository ruleset `PR-Required` (id `12812291`) is `active` but targets
`~DEFAULT_BRANCH` only. `GET /repos/{owner}/{repo}/rules/branches/main` returns
`["deletion","non_fast_forward","pull_request","copilot_code_review"]`; the same
endpoint for this plan's own source branch
`chore/143-s-stage-checkpoint-lifecycle-continuity` returns `[]`. **Ship source
branches are therefore unprotected today**, and no part of this plan may assume
source-branch history immutability until U0 has been applied and S3.5 has proven
it per branch at run time.

**Honest framing of revision 10's size.** This is not a small revision, and
calling it "bounded" would be generous without saying what bounds it. It is
bounded by *authorization*, not by diff size: the operator authorized one
revision carrying the RR-3 decision plus the round-9 P0/P1 remediation, and
revision 10 does exactly that and nothing else. It adds four units (U0 plus the
three mandated granularity splits), one requirement (RQ-13), and one hardening
(D16), and it re-scopes two existing units. It allocates **no** backlog IDs,
implements nothing, and changes **no** GitHub settings.

**What changed in revision 10.** Every round-9 P0 and P1, the P2s that block a
clean PASS, and the RR-3 decision:

1. **RR-3 closed by the ruleset prerequisite** (F-13, F-14, F-15, F-19) — new
   unit **U0**, new requirement **RQ-13**, new segment **S3.5**, rebuilt
   Channel B in **U13**, new hardening **D16**.
2. **F-11 (P0) fixed** — the Stage safeguard now keys off the **selected
   checkpoint's carrying PR**, recorded in the checkpoint payload, never the
   ambient working-tree branch.
3. **F-01 fixed** — `RESOLUTION_POSTCONDITION` is split into two explicitly
   named mutations: a PR-body metadata write that commits nothing, and an
   `OPEN` → `CLOSED` commit on the **named `post-merge/{feature_slug}` closure
   branch** Ship already creates. The "commits nothing" sentence is deleted.
4. **F-02 fixed** — the Constitution Check P-018 row no longer asserts "item 15
   retained unmodified"; the whole document was grepped for every restatement.
5. **F-03, F-04, F-05, F-06 fixed** — V4(g) now permits the S4 mutation; S8's
   locator terms and the postcondition are explicitly **conditional**, giving
   zero-checkpoint units a satisfiable common path; S7 re-enumerates checkpoints
   and the race response is defined.
6. **F-07 fixed** — P-020 compaction is ordered after the `CLOSED` transition,
   with a path-stability invariant and an archive-following rule so legitimate
   compaction can never trip OB-1.
7. **F-08 fixed** — S1…S8 are defined by **role**, with no live item numbers in
   the instructions file, plus a standing executable drift check (**V20**).
8. **F-09 fixed** — U3, U4 and U9 are split along the seams the auditor named,
   restoring the 2-hour / single-domain rule.
9. **F-10, F-16, F-17, F-18, F-21 fixed** — a normative S3 algorithm and
   ownership predicate; an explicit P-012 availability contract with the
   no-checkpoint-operations registry defined as a **halt**; PV-8/OB-8 binding
   discovery to the shipment's designated implementation PR; an expected-head
   SHA pinned on the merge call; and WP reconciled with the recorded post-merge
   worktree-isolation prior art.
10. **P2 sweep** — F-20 (**nine** invariants), F-22 (U1 is not a root), F-23,
    F-24 (V19 parses YAML), F-25 (`U10→U4` edge), F-26, F-27 (Principles I–XI),
    F-28 (P-011), F-29 (shallow-clone check), F-30 (API-side merge
    verification), F-31 (recorded as **RR-6**); plus the P3s F-32…F-37. V2's verb
    prohibition and V8's missing `--paginate`, both fixed in revision 9, are
    re-verified unchanged.

**Abandoned-ID notice.** `143-F`, `143-S` and `143.001-T` … `143.014-T` are
machine-state **abandoned**. They appear in this document **only** as historical
evidence of what was previously attempted. They are **never** to be revived,
re-parented, or reused, and no future-tense statement in this plan depends on
them. Replacement IDs stay **unassigned** until a later authorized harvest.
Where a section below still needs to name the release-unit shape, it does so
structurally (top-level release unit → three sub-epics → fourteen tasks) rather
than by citing an abandoned ID as a live target.

**What changed in revision 9** *(historical record — superseded in part by
revision 10; counts stated here are revision-9 counts and are **not** the live
contract. Where this list and the sections above disagree, the sections above
govern: the merge bar is now **seven**-part, the invariants **nine**, the
zero-checkpoint omission set **five**, and the unit count **fourteen**.)* Five
corrections, all from the P-013.6 escalation:

1. **RQ-6 gained an executable enforcement path against the *real* Step 5.**
   Every prior round described Step 5 in prose and mismatched it. U4 now carries
   a **verbatim extract of the live item list** — including the fact that the
   item number `7` is **duplicated** in the file, that readiness items 7b/7c
   currently precede the mutating items 7(second)/8/9/10 and the push, and that
   item 15 re-fetches only the P-018 verdict and `headRefOid`. The canonical
   `RESOLUTION_PREFIX` is restructured into eight named segments S1…S8 that
   define the exact safe order; items 7b/7c **move** after the push, item 14
   records an `approved_head`, item 15 is **amended** (revision 8's "item 15
   unmodified" criterion forbade the unit from implementing the requirement it
   was credited with, and is withdrawn), and a **six-part merge bar** plus
   explicit no-stale-approval refresh rules are stated.
2. **The zero-checkpoint bypass claim is removed as false.** The checkpoint count
   now selects **only** segment S4. Every unit runs the reordered common
   finalization tail; a complete zero enumeration omits exactly four things
   (locator publication, resolution, resolution commit/push, phase-2 locator) and
   nothing else. Enumeration failure, malformed or quarantined records, and
   ambiguity are **not zero** and halt; a checkpoint appearing after enumeration
   forces re-evaluation before merge; no empty locator is ever published; Stage's
   startup recovery stays a separate protocol.
3. **D14 is given real executable coverage.** The working-tree placement
   paragraph — which revision 8 left as an **unnamed fragment with its heading
   lost**, so no criterion could cite it — is now the named procedure
   `Working-tree placement — committing re-entry only` with steps WP-1…WP-7,
   defined in **U3** (AC12), executed by name in **U5** (AC9) before the only
   committing recovery branch, and verified by **V12/V12a–V12f**. The existing
   U3→U5 dependency is unchanged and **no new unit was needed** for it.
4. **RR-3 is closed rather than weakened or silently deferred.** New unit **U9**
   installs `RESOLUTION_OBLIGATION_RECORD`: a SHA-free, Git-tracked record
   written into the unit's existing `docs/closure/` pre-merge closure artifact,
   in the **same commit** as the resolutions, discovered from exhaustive trusted
   PR/commit/tree history independently of the mutable PR body, with deletion
   detected from commit history. New unit **U10** declares the field in the
   owning `operational-closure` skill so the record is not an unowned squatter —
   an openly recorded scope addition of one file. RQ-7 is made true; **RQ-8 is
   not traded**, because the record carries no SHA.
5. **Consistency cleanup.** V2's unsatisfiable verb prohibition is replaced with
   a sequence-restatement check that permits ordinary prose verbs; V8's reference
   command gains the `--paginate` it was missing (without it the comparison could
   only ever prove the two commands disagreed); the abandoned `143.*` IDs are
   retained as historical evidence only and no longer appear as implementation
   targets; and the revision counter is incremented with
   `harvest_authorized: false` held.

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

* Documentation-only **except U0**. No `src/`, no `crates/`, no scripts, no
  build system. **U0 is the single exception**: it changes GitHub repository
  settings, changes no file, and is gated on explicit operator/admin approval.
* Installed harness surfaces only: `.github/policies/`, `.github/instructions/`,
  `.github/agents/`, **one field declaration in
  `.github/skills/operational-closure/SKILL.md`** (added in revision 9 by U10, so
  the RQ-12 obligation record lives on an *owned* schema rather than squatting on
  someone else's artifact), plus one `docs/compound/` learning.
* No backlogit tool change. No shipment status outside `queued`/`active`.
* Every unit is single-domain and under two hours of human-equivalent effort.
  Revision 10 splits U3, U4 and U9 to restore this after round-9 finding F-09.
* **No implementation unit introduces or depends on a Defect-1 construct.**
  Defect-1 names appear in this document only in the revision summary, the
  out-of-scope list, and the retained review history — as documentary exclusions,
  never as implementation targets. V7 enforces the distinction by scanning the
  *changed files*, not this plan.
* **No unit in this plan mutates GitHub settings at plan time.** U0 *specifies*
  a settings change and requires approval before it is applied; producing this
  plan applies nothing.

## Constitution Check

### Workflow policies

| Principle | Check |
|---|---|
| P-001 Single-release-unit completion | P-001 requires that no previously merged release unit is still awaiting required post-merge closure. Two-channel discovery at zero-candidate startup (U5, U6, U13) is the mechanism that makes that precondition *checkable* rather than assumed. `RECONCILED` is defined to require the full P-001 closure set. Composed with, not weakened. |
| P-003 Decomposition chain | P-003 precondition item 4 requires that **every task references its parent sub-epic**. Stated **structurally**, because the former IDs are abandoned: source document → this plan → **one top-level release unit** → **three sub-epics** → **fourteen tasks**. Sub-epic **E1 — Ordering contract** covers the prevention half (U1, U2, U4, U12, U8); **E2 — Discovery and recovery** covers the recovery half (U3, U11, U9, U13, U10, U5, U6); **E3 — Prerequisite and closure** covers the ops prerequisite and the closure deliverable (U0, U7). The split follows the plan's own structure, not an artificial tier. Each sub-epic references this plan and the top-level release unit; each task references its sub-epic and carries acceptance criteria. **No ID is assigned here** — `143-F`, `143-S` and `143.001-T` … `143.014-T` are abandoned and must not be revived or reused; replacement IDs are allocated only at a later authorized harvest. |
| P-005 Policy telemetry | No gate is bypassed; the review gate is explicitly open. U1 requires P-022's Violation Action to record P-005 telemetry, matching neighbouring policies. |
| P-006 Plan hardening | `requires_plan_hardening: yes`; hardening document exists and is revised in lockstep (D1–D16). |
| P-008 Markdown conformance | Engaged by every file-changing unit: each carries a `markdownlint passes` acceptance criterion and V1 runs it across all changed files. U0 changes no file and is exempt, stated explicitly rather than silently. |
| P-009 Merge-commit-only | Untouched as a *guardrail*. U4 does not weaken Step 5's P-009 requirement; revision 10 adds an **API-side** verification path beside the existing rendered-UI confirmation (F-30) so the guardrail is agent-checkable, and requires the merge call to be made in merge-commit mode with two parents asserted afterwards. U3/U6/U11/U13 confer no merge authority and therefore cannot select a merge strategy. |
| P-010 Role boundary | Stage plans; Ship executes. No source mutation planned by Stage. **U8 adds only a prohibition to Stage** — do not resolve into a merged carrying PR, do not create an undischargeable checkpoint — and confers no merge authority, no locator obligation, and no `RESOLUTION_PREFIX` execution. U6 has the Orchestrator *route* to Ship rather than perform recovery. **U0 is an operator/admin action**, not a Stage action: the plan specifies it and requires approval; Stage neither applies nor can apply it. |
| P-011 Worktree topology | **Added in revision 10 (round-9 finding F-28).** `Working-tree placement` performs fetch, branch create/switch, fast-forward and commit, so it crosses the P-011 topology boundary. WP-0 now runs the P-011/P-016 worktree-topology gate **before** WP-1, and WP reconciles with the recorded post-merge worktree-isolation prior art (F-21): the resolution commit is made in the **implementation worktree** on the PR head, and post-merge closure uses the separate `post-merge/{feature_slug}` branch Ship already creates. |
| P-012 Tool availability | **Wired, not merely asserted (round-9 finding F-16).** U11 AC13 installs an explicit availability contract covering every operation this plan puts on the critical path: `backlogit_list_checkpoints`, `backlogit_get_checkpoint`, `backlogit_resolve_checkpoint`, `backlogit_create_checkpoint`, `gh api` (including the effective-rules endpoint), review-thread enumeration, and Git history access. Each is probed before the path that needs it; only declared official CLI fallbacks may be used; anything else **halts**. A registry exposing **no** checkpoint operations is defined as a **halt**, never as an implicit zero enumeration — that silent fail-open into the merge path is the dangerous case. |
| P-014 Local review readiness | **This plan introduces a §1.9 hazard and then closes it.** Today §1.9 is trivially satisfiable because resolution happens post-merge and the reviewed HEAD is stable. Moving resolution pre-merge advances HEAD past the reviewed HEAD, which — left unmitigated — would defeat §1.9 while appearing to satisfy it (hardening D4) or cause the gate to be quietly skipped (D5). The mitigation is specific and mandatory: re-run the actual review at the final pushed HEAD, write the PR-body record, *then* gate. P-014 is **preserved by construction**, not strengthened. |
| P-015 Single-artifact shipment closure | Untouched. The locator and the obligation record record shipment identity but do not alter shipment-closure sequencing and introduce no cascade-close behaviour. The standing `backlogit shipment ship` non-termination risk and its P-015 safe-close fallback are recorded as a coupling point in RR-6 (F-31). |
| P-016 No parallel branches | The recovery path routes one owner to one PR; it never opens a second implementation branch. The one additional working space this plan touches is the `post-merge/{feature_slug}` closure branch Ship's Step 6.0 **already** creates — reused, not invented. |
| P-017 Dark factory | `LAST_MILE_RECOVERY` is auto-entered at startup and walks to a merge bar, so dark mode could otherwise supply approval for it. A recovered obligation belongs to a **prior** unit and is outside the current run's declared dark scope: U6 AC7 states that a dark approval for the current scope does **not** satisfy the merge bar for a prior unit's recovered obligation. **U0 is never dark-approvable**: a `ProposedAction` at `ActionRisk: high` requires an explicit human approval and P-017 dark mode must not supply it. |
| P-018 Copilot review gate | Re-run at the final HEAD is explicit in `RESOLUTION_PREFIX`. The resolution push naturally re-arms P-018. **Step 5's last-mile re-check is AMENDED, not retained unmodified** — it currently re-runs only the P-018 verdict and `headRefOid`, which is exactly why RQ-6 had no executable enforcement path. P-018 is evaluated at S5 and **re-evaluated** at S7; the merge-commit guardrail item is retained unmodified. *(Revision 10 corrects the withdrawn "item 15 retained unmodified" claim that survived in this row through revision 9 — round-9 finding F-02.)* |
| P-020 Post-merge context compaction | U4 edits Step 5; **U12** edits Step 6 and Session end — neither removes Step 6's `compact-context` invocation, and U12 carries an explicit acceptance criterion that it remains present and unmodified. **Revision 10 fixes the ordering hazard (round-9 finding F-07)**: the `OPEN` → `CLOSED` transition is written **before** any compaction that can touch the closure artifact, a path-stability invariant forbids renaming/compacting/archiving an artifact whose record is `none` or `OPEN`, and Channel B additionally follows `docs/archive/closure/` so an archive move is never misread as the OB-1 deletion attack. |
| P-022 Checkpoint resolution durability | Introduced by this plan (U1). Self-consistently applied: every surface references the canonical definition by name rather than restating it, and V2/V20 enforce that. |

### Workspace constitution — Principles I–XI

*Added in revision 10 (round-9 finding F-27): the Governance clause requires the
workspace constitution's own principles to be mapped, not only the workflow
policies.*

| Principle | Check |
|---|---|
| I — Specification before implementation | Satisfied: decision → plan → hardening → review gate, with no implementation authorized. |
| II — Single source of truth | The `Canonical ownership` table names exactly one installed home per definition; V2 and V20 enforce non-restatement. |
| III — Traceability | Every requirement RQ-1…RQ-13 maps to one owning unit plus enforcing units; U7 is labelled a closure deliverable rather than given an invented requirement. |
| IV — Incremental delivery | Fourteen single-domain units, each under two hours, with an acyclic dependency graph. |
| V — Test/verification first | V1–V20 are specified with expected results, and revision 10 makes V4(g), V16, V18 and V19 actually satisfiable. |
| VI — Reversibility | Every documentation unit is revertible by revert. **U0 is the one partially irreversible action** — a ruleset is re-configurable but its absence window cannot be retro-actively closed — which is precisely why it is `ActionRisk: high` and approval-gated. |
| VII — Destructive-command approval | Engaged by WP (fetch, branch create/switch, fast-forward, commit) and by U0. WP **forbids** `git reset --hard`, force checkout and rebase outright; U0 requires explicit approval. No destructive command is auto-approved. |
| VIII — Explicit safety modes | U0 is declared `ProposedAction` with `ActionRisk: high` and `approval_required: true`; P-017 dark mode is explicitly barred from satisfying it. |
| IX — Fail closed | Every discovery, provenance, ruleset-proof and ancestry failure halts. OB-1…OB-8, PV-1…PV-8 and S3.5 are all fail-closed, and "absence" is never read as "discharged". |
| X — Least privilege | The recovery path confers **no** merge authority (RQ-10). U0 grants no bypass actors and no current-user bypass. Stage receives a prohibition only. |
| XI — Auditability | The obligation record's whole lifecycle is Git-tracked and ancestry-checkable; the ruleset makes the introducing commit provably reachable; review history is appended, never rewritten. |

## Canonical definitions

These six definitions are the single source of truth. Task cards quote them;
they are not restated differently anywhere.

**Canonical ownership — one installed surface per definition.** Each definition
has exactly **one** installed home that carries it verbatim. Every other surface
**references** it by name and must not restate the sequence, so the surfaces
cannot drift into two different orders.

| Definition | Canonical installed home | Referencing surfaces |
|---|---|---|
| `HEAD_EVIDENCE_RULE` | `github-pr-automation.instructions.md` (U2) | — |
| `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION` | `github-pr-automation.instructions.md` (U2) | `workflow-policies.md` P-022 (U1); `_ship.agent.md` Step 5 (U4) and Step 6 (U12); `_stage.agent.md` (U8) |
| `BRANCH_PROTECTION_PREREQUISITE` | `github-pr-automation.instructions.md` (U2) | `_ship.agent.md` (U4); repository settings (U0) |
| `CLOSURE_LOCATOR` | `github-pr-automation.instructions.md` (U2, U3) | `_ship.agent.md` (U4, U5); `_orchestrator.agent.md` (U6) |
| `RESOLUTION_OBLIGATION_RECORD` | `github-pr-automation.instructions.md` (U9 schema/lifecycle, U13 discovery) | `_ship.agent.md` (U4, U12, U5); `_orchestrator.agent.md` (U6); `operational-closure/SKILL.md` (U10, schema field only) |
| `LAST_MILE_RECOVERY` | `github-pr-automation.instructions.md` (U11) | `_ship.agent.md` (U5); `_orchestrator.agent.md` (U6) |

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

**Entry condition.** `RESOLUTION_PREFIX` is the **whole finalization tail** of
Ship Step 5 and it runs for **every** unit. It is explicitly **not** gated on "PR
merge-ready" — readiness is established *by* this sequence, so gating entry on it
would be circular.

**The checkpoint count selects one conditional segment pair, not the whole
sequence.** The unit's active-checkpoint count selects **only** segments **S3.5**
and **S4** — the branch-protection proof, locator publication, checkpoint
resolution, the resolution commit, its push, and the phase-2 locator
publication. **Every other segment — S1, S2, S3, S5, S6, S7, S8 — runs
identically for every unit, zero-checkpoint units included.** A unit with a
**complete** zero enumeration omits S3.5 and S4 and nothing else. There is no
"pre-existing path unchanged" bypass and no unit escapes the reordered common
finalization tail.

**Zero is a proven result, never a default.** S3 must *prove* the enumeration
complete. An enumeration that fails, returns a malformed or quarantined record,
or is ambiguous in any way is **not zero** — it **halts**. A registry exposing no
checkpoint operations at all is **not zero** — it **halts** (P-012). A checkpoint
that appears after S3 completed forces re-evaluation from S3 before merge, and S7
re-enumerates specifically to detect that. No empty locator is ever published:
when S4 is omitted there is no locator, not a locator with an empty checkpoint
list.

This is scoped to the **Ship unit's own** pre-merge obligation. Stage's
crash-resumption startup recovery is a **separate** protocol and is not altered,
subsumed, or gated by anything in this definition.

**Segments are defined by ROLE, not by live item number** *(revision 10, round-9
finding F-08)*. The canonical text installed in the instructions file names each
segment by the **role** it plays. It contains **no** `_ship.agent.md` item
numbers at all. The item-to-segment binding lives **solely** in `_ship.agent.md`,
where the items actually are, and a standing invariant (**V20**) asserts that the
executable order in that file still matches these roles. This is deliberate: the
previous form hard-coded live item numbers — including the live file's
**duplicated** `7` — into a second file, which is the same coupling that caused
revisions 6, 7 and 8 to describe a Step 5 that did not exist.

```text
RESOLUTION_PREFIX  (the finalization tail of Ship's PR-lifecycle step)
  entry: all in-scope task work complete
         (NOT gated on "PR merge-ready" — readiness is produced here)

  S1  ROLE: the CI / review fix loop
        the existing automated CI-fix and optional shadow-review loops.
        These MAY commit and push. They run to completion FIRST.

  S2  ROLE: the remaining branch-mutating tail
        runtime verification, operational-closure artifact generation,
        follow-up stash writes, and the ordinary branch push.
        After S2 no ordinary work remains that can mutate the branch.

  S3  ROLE: prove current-unit checkpoint enumeration COMPLETE
        (normative algorithm below — it is not a prose instruction)
        complete + count == 0        → skip S3.5 and S4; continue at S5
        complete + count >= 1        → run S3.5, then S4
        failed / malformed / quarantined / ambiguous / tooling absent
                                     → HALT (this is NOT zero)

  S3.5 CONDITIONAL — ROLE: prove the branch protection is LIVE   (RQ-13)
      runs only on a complete NONZERO enumeration, and ONLY BEFORE any
      obligation is published. See BRANCH_PROTECTION_PREREQUISITE below.
      → query the effective-rules API for the ACTUAL LIVE headRefName
      → prove `deletion` AND `non_fast_forward` both apply
      → prove no relevant bypass exists for the acting identity
      → API unavailable / ambiguous / branch uncovered / bypass present
            → HALT BEFORE obligation publication. Never publish first
              and verify afterwards.

  S4  CONDITIONAL — runs only after S3.5 has PASSED
      → publish CLOSURE_LOCATOR to the PR body, status RESOLUTION_PENDING
            (metadata write; MUST precede the first resolution commit, so
             the checkpoint-free window is never uncovered)
      → resolve every checkpoint owned by this unit, and write the
            RESOLUTION_OBLIGATION_RECORD at OPEN, in ONE commit,
            in the IMPLEMENTATION worktree on the PR head
            (the Git-tracked durable obligation record; see its canonical
             definition below — it rides the SAME commit and carries NO SHA)
      → PUSH that commit
      → prove the remote PR head now equals the local HEAD
            (nothing downstream may derive evidence from an unpushed commit)
      → update CLOSURE_LOCATOR in the PR body: status RESOLUTION_PUBLISHED,
            resolution commit SHAs, final_head

  S5  at the resulting pushed HEAD — RUNS FOR EVERY UNIT
      → RE-RUN THE ACTUAL LOCAL REVIEW at that pushed HEAD
            (a real review pass over the final diff — never a restatement
             of an earlier verdict)
      → publish the final locator resolution state, IF a locator exists
      → record Reviewed HEAD in the PR BODY at that HEAD
            (metadata write; does NOT advance headRefOid)
      → run, in this order:
            P-014 §1.9 local readiness       (MOVED here, after the push)
            EXPLICIT required-check evaluation — each required check
              enumerated and evaluated green, or explicitly PROVEN
              non-applicable; "no red" is not an evaluation
            P-018 copilot-review gate        (MOVED here, after the push)

  S6  ROLE: record approval, pinned
      → obtain explicit operator approval and record `approved_head`
            equal to the S5 HEAD. An approval without a recorded
            approved_head is not an approval.

  S7  ROLE: strengthened last-mile re-check   (the existing last-mile
      re-check item, AMENDED — it currently re-runs only the P-018
      verdict and headRefOid)
      → re-fetch and re-evaluate, all of them, unconditionally:
            headRefOid; the PR body; reviewDecision;
            review requests and reviews; EVERY review-thread page to
            exhaustion; required checks; the ancestry of every recorded
            resolution commit; AND a RE-ENUMERATION of active checkpoints
            owned by this unit.
      → RACE RESPONSE: if the re-enumeration returns a NONZERO count, or
            is incomplete/ambiguous by the S3 algorithm, the unit RETURNS
            TO S3 and re-runs S3 → S3.5 → S4 → S5 → S6 → S7. The prior
            S6 approval is VOID and a fresh approved_head is required.
            This loop is bounded: re-entry is permitted at most twice,
            after which the session HALTS to the operator.
      → if the unit is under S3.5 protection, also RE-PROVE the ruleset
            still applies; ruleset drift since S3.5 HALTS.

  S8  merge bar — merge ONLY when ALL hold
      1. HEAD AGREEMENT. The four ALWAYS-PRESENT values agree:
             live headRefOid == local HEAD
                             == PR-body Reviewed HEAD
                             == approved_head
         AND, CONDITIONALLY, when a locator was published (that is, when
         S4 ran), locator final_head joins them and must agree too.
         On a zero-checkpoint unit no locator exists, so the locator term
         is ABSENT rather than empty, and its absence is NOT a failure.
      2. every recorded resolution commit, IF ANY WERE RECORDED, is an
         ancestor of that HEAD. On a zero-checkpoint unit the set is
         empty and this condition is VACUOUSLY SATISFIED, which is
         correct here precisely because condition 1 pins the HEAD by
         four independent always-present values.
      3. review-thread pagination COMPLETED to exhaustion
      4. no blocking review thread and no blocking review exists
      5. required checks pass, or are explicitly PROVEN non-applicable
      6. P-018 passes
      7. the S7 checkpoint re-enumeration returned a COMPLETE ZERO
      Any doubt at any of the seven — HALT.
      → THE MERGE CALL ITSELF MUST PIN THE OBSERVED HEAD. Pass the
        S8-observed headRefOid to the merge API as the expected head SHA
        so GitHub refuses server-side if the branch advanced between the
        S8 read and the call. An unpinned merge call is a TOCTOU hole,
        not a guarantee.
      → merge in MERGE-COMMIT mode (P-009), verified by API rather than
        only by the rendered UI, and assert the resulting commit has TWO
        parents afterwards.
      Then the P-009 guardrail item and the P-017 dark-mode item apply
      unchanged.

  REFRESH RULES (no stale approval may ever be reused)
    * Any HEAD change at any point requires a FRESH cycle:
      push → re-run the actual review → PR-body metadata → S5 gates →
      S6 approval. The prior approval is void.
    * A thread or check change WITHOUT a HEAD change requires the
      affected gates to be re-run and the S6 approval to be refreshed.

RESOLUTION_POSTCONDITION  (post-merge; TWO separately named mutations)
  entry: the FULL required post-merge closure set is complete AND verified

  POST-A  the PR-body RECONCILED write          — METADATA, NO COMMIT
      → set CLOSURE_LOCATOR status to RECONCILED in the PR body.
      → this is a metadata write. It creates no commit and advances no
        headRefOid.
      → CONDITIONAL: performed only if a locator was published. When no
        locator exists, POST-A is a NO-OP.

  POST-B  the OPEN -> CLOSED transition          — A COMMIT, ON A NAMED
                                                   CLOSURE BRANCH
      → CONDITIONAL: performed only when resolution_obligation is OPEN.
        When the field is `none` (the zero-checkpoint case), POST-B is an
        explicit NO-OP: `none` is LEFT INTACT and is NEVER transitioned.
        `none -> CLOSED` is not a permitted transition and must not be
        attempted.
      → When OPEN: set status CLOSED and write closed_at, as a COMMIT on
        the `post-merge/{feature_slug}` closure branch that Ship's
        post-merge closure step ALREADY creates. Ship MUST NOT commit
        this to the default branch (P-010).
      → That closure branch's PR MUST BE MERGED before the obligation is
        discharged. An unmerged CLOSED transition discharges nothing.
      → ORDERING AGAINST P-020: POST-B MUST complete BEFORE any
        compaction or archival that can move the closure artifact. See
        the path-stability invariant in RESOLUTION_OBLIGATION_RECORD.
```

**No step of `RESOLUTION_PREFIX` may be performed on a merged branch.** The
prefix ends before merge. `RESOLUTION_POSTCONDITION` runs after merge and is the
only part that may: POST-A writes PR metadata, and POST-B commits to the separate
`post-merge/{feature_slug}` closure branch — never to the merged implementation
branch and never to the default branch.

Nine ordering invariants are load-bearing:

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
5. **Every mutating step precedes S3** *(RQ-6)*. S1 and S2 exhaust the branch's
   ordinary mutation — CI/review fixes, runtime verification, closure-artifact
   generation, follow-up stash writes, and the ordinary push. The readiness
   gates (§1.9, required-check evaluation, P-018) **move** from their current
   position *before* those mutating items to **after** that push, in S5. This is
   the reorder, and it applies to **every** unit, not only checkpoint-owning
   ones.
6. **Approval is pinned to a HEAD** *(RQ-6)*. S6 records `approved_head`. An
   approval with no recorded `approved_head` cannot be checked against anything
   at S8 and is therefore not an approval. **No stale approval is ever reused**:
   any HEAD change voids it, and a thread or check change without a HEAD change
   requires the affected gates plus a refreshed approval.
7. **Required checks are evaluated, never assumed** *(RQ-6)*. S5 and S8 require
   each required check to be enumerated and evaluated green or **proven**
   non-applicable. The live last-mile re-check item re-runs P-018 and re-queries
   `headRefOid` only; "no red observed" is not an evaluation, and S7 is the
   amendment that closes that gap.
8. **The durable obligation record rides the resolution commit** *(RQ-12)*. The
   SHA-free `RESOLUTION_OBLIGATION_RECORD` is written in the **same** commit as
   the checkpoint resolutions and pushed with them, so the obligation is
   recorded in Git history — not only in mutable PR-body metadata. See its
   canonical definition below.
9. **Protection is proven before the obligation is published** *(RQ-13, added in
   revision 10)*. S3.5 runs **before** S4. Publishing an obligation record onto
   a branch whose history can still be rewritten, or which can still be deleted,
   produces a record that its own author can erase without trace — which is
   exactly the RR-3 gap. Proving the protection **after** publication would be
   useless, because the unprotected window is the window the attack occupies.

### `BRANCH_PROTECTION_PREREQUISITE` *(RQ-13)*

**This definition is revision 10's answer to RR-3**, and it implements the
operator's decision directly. It replaces the revision-9 mechanism that the
round-9 review invalidated (F-13, F-14, F-19). It is **not** a CI-check
persistence substrate, and it does **not** weaken RQ-7.

**The problem, stated exactly.** The `RESOLUTION_OBLIGATION_RECORD` is Git-tracked
and therefore durable *against ordinary edits* — a later commit that deletes it
is itself a detectable history event. But durability against *history rewriting*
is not a property of Git; it is a property of the **forge's branch protection**.
If the PR author can force-push the branch back past the introducing commit, or
delete the branch entirely, then both the record and the evidence that it ever
existed vanish together, and no SHA-free record can distinguish "never existed"
from "erased" (F-19). The fix therefore has to come from outside the branch
author's erasure surface — and the narrowest such mechanism that GitHub already
provides is a **repository ruleset**.

**Measured current state (read-only, at revision 10).**

| Probe | Result |
|---|---|
| `GET /repos/{owner}/{repo}/rulesets` | one ruleset, `PR-Required`, id `12812291`, `enforcement: active`, `target: branch` |
| That ruleset's `conditions.ref_name.include` | `~DEFAULT_BRANCH` only |
| `GET /repos/{owner}/{repo}/rules/branches/main` | `["deletion","non_fast_forward","pull_request","copilot_code_review"]` |
| `GET /repos/{owner}/{repo}/rules/branches/chore%2F143-s-stage-checkpoint-lifecycle-continuity` | `[]` — **uncovered** |
| Legacy branch-protection endpoint for `main` and for the source branch | `404` on both |

**Conclusion, recorded so no later reader re-assumes it**: Ship source branches
carry **no** `deletion` or `non_fast_forward` protection today. Every claim in
this plan that depends on source-branch history immutability is therefore
conditional on U0 having been applied, and must be **re-proven per branch at run
time** by S3.5. The plan must not assume it.

**Part 1 — the ops prerequisite (unit U0, `ProposedAction`, approval-gated).**
A repository ruleset must cover **every permitted resolution-bearing Ship branch
pattern**. Those patterns are derived from the installed Ship agent's own branch
naming, not guessed:

| Pattern | Source in `_ship.agent.md` | Why it is resolution-bearing |
|---|---|---|
| `feat/*` | shipment branch for features — `git checkout -b feat/{feature-slug}` | carries the S4 resolution commit and the `OPEN` record |
| `chore/*` | shipment branch for chores — the `chore/` variant of the same step | same |
| `post-merge/*` | `git checkout -b post-merge/{feature_slug}` for all Step 6 closure work | carries the POST-B `OPEN` → `CLOSED` transition |

The ruleset must declare both `deletion` and `non_fast_forward` rules, **no
bypass actors**, and must resolve to `current_user_can_bypass: never`. It must be
`enforcement: active`. It does **not** add a `pull_request` rule to these
patterns: requiring PR review on every Ship working branch would break Ship's own
push loop, and the obligation record needs only immutability, not review.

**Part 2 — the per-run proof (segment S3.5).** A configured ruleset is not a
proof that *this* branch is covered *now*: patterns can drift, the ruleset can be
disabled, and a bypass actor can be added later. Ship therefore proves it per
run, against the **actual live `headRefName`** — never against a locally assumed
branch name:

```text
# Preferred form — the effective-rules endpoint, which resolves ALL rulesets
# that actually apply to the branch, rather than enumerating configuration.
gh api "repos/{owner}/{repo}/rules/branches/{live_headRefName}"

  → REQUIRE an entry with .type == "deletion"
  → REQUIRE an entry with .type == "non_fast_forward"
  → For each matched rule, resolve its ruleset via
        .ruleset_id  →  gh api "repos/{owner}/{repo}/rulesets/{id}"
    and REQUIRE:
        .enforcement == "active"
        .bypass_actors is empty OR contains no actor the acting identity
          holds, at any bypass_mode
        .current_user_can_bypass == "never"

  HALT BEFORE OBLIGATION PUBLICATION on ANY of:
    - the endpoint is unavailable, errors, rate-limits, or is not
      supported by this repository/plan tier
    - either required rule type is absent
    - the branch is not covered by any ruleset (an empty [] response)
    - enforcement is anything other than "active"
    - any bypass actor is present that the acting identity could hold
    - current_user_can_bypass is anything other than "never"
    - the response is ambiguous or cannot be parsed
```

**`{live_headRefName}` MUST be read from the PR API** (`gh pr view <n> --json
headRefName`) immediately before the probe, and MUST be URL-encoded — Ship branch
names contain `/`. Using the locally checked-out branch name is forbidden: it is
the same ambient-state error as round-9 finding F-11.

**Repository-supported equivalent.** If this repository or plan tier does not
expose `GET /rules/branches/{branch}`, the permitted equivalent is to enumerate
`GET /rulesets` with `includes_parents=true`, resolve each ruleset's
`conditions.ref_name` against the live `headRefName`, and apply the same four
requirements. If **neither** form is available, that is the "API unavailable"
row: **halt**. It is never an assumption of coverage.

**What this buys, stated without overclaiming.** With `non_fast_forward` and
`deletion` in force and no bypass:

* an ordinary later commit that **deletes** the obligation record is still
  detectable, because the introducing commit remains **reachable** from the
  branch head and the branch cannot disappear (this is what makes OB-1 real);
* a **force-push** that would unreach the introducing commit is refused by the
  forge, so OB-2's "unverifiable ancestry" case becomes rare rather than routine;
* the residual is honestly bounded below in **RR-3**: an actor who can *change
  the ruleset itself* (a repository admin) is outside this mechanism's reach.
  That is a strictly smaller and better-audited surface than "anyone who can push
  to their own branch", which was the revision-9 residual, and the reduction is
  the entire point of the change.

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
* do **not** re-write, duplicate, or re-open the `RESOLUTION_OBLIGATION_RECORD`
  — it is already in history on this branch and a remediation commit does not
  create a second obligation;
* re-enter the gate set from the top **at S5**, and obtain a **fresh** S6
  approval with the new `approved_head`. The prior approval is void.

The loop terminates because each iteration ends in either a merge or a halt, and
because remediation never creates new checkpoint state to resolve.

**Residual-window checkpoint prohibition.** Once a unit enters
`RESOLUTION_PREFIX`, no new Git-tracked checkpoint may be created for that unit —
not on yield, not for merge approval, not for closure work. Such a checkpoint
could only be resolved by a further commit, which needs a further PR once this
one merges: the original defect, one level down, recursively. The window is
covered by `CLOSURE_LOCATOR`, the `RESOLUTION_OBLIGATION_RECORD`, and
`LAST_MILE_RECOVERY` instead.

### `S3_ENUMERATION_ALGORITHM` — normative, not prose *(round-9 finding F-10)*

Revision 9 said only "enumerate every checkpoint owned by THIS unit" and "resolve
every checkpoint owned by this unit", supplying neither an invocation nor an
ownership predicate. That is weaker than the recovery protocols **already
installed** in Ship and Stage, and it permits three concrete failures: filtering
at the API call and hiding quarantined records; sweeping in another unit's
records; and bulk-resolving without a per-record handling proof. The algorithm is
therefore stated executably, and mirrors the installed protocols rather than
inventing a second dialect.

```text
S3  ENUMERATION  (proof obligation: COMPLETE, and correctly SCOPED)

 1. AVAILABILITY (P-012, before anything else)
      probe backlogit_list_checkpoints / _get_checkpoint / _resolve_checkpoint.
      registry declares NO checkpoint operations   → HALT
        (this is NOT an implicit zero — that is the fail-open into merge)
      operation present but failing, no declared official CLI fallback → HALT
      declared official CLI fallback present       → use ONLY that fallback

 2. ENUMERATE WITHOUT FILTERING
      backlogit_list_checkpoints  with consumer_id ONLY.
      Apply NO `status` filter and NO `agent` filter at the API call.
        Rationale, identical to the installed protocols: a parse-failure or
        schema-invalid record is returned as a QUARANTINED summary with an
        empty agent/status, and an API-side filter silently drops exactly
        those records. The registry exposes no `agent` list parameter anyway.
      non-zero exit / truncated / unparseable response  → HALT (not zero)

 3. ANOMALIES BEFORE PARTITION  (order is load-bearing)
      Inspect EVERY enumerated summary for a validation error, a quarantine
      flag, or a missing/malformed required field — regardless of its
      (possibly empty) agent/status.
      ANY anomaly → HALT. Do not partition, do not count, do not proceed.

 4. PARTITION BY CURRENT-UNIT IDENTITY  (only after step 3 is clean)
      A record belongs to THIS unit when ALL hold:
        validated CheckpointV1
        AND  agent == "ship"
        AND  status == "active"
        AND  context.shipment_id == the current shipment_id
      Records failing the identity predicate belong to ANOTHER unit and are
      NEITHER counted NOR resolved. Cross-unit resolution is PROHIBITED.
      A record that is ship-owned and active but whose context.shipment_id
      is absent or unparseable is an ANOMALY → HALT (step 3's rule, applied
      to a field only reachable after validation).

 5. PROVE THE COUNT
      count == 0  → the enumeration is a COMPLETE ZERO. Skip S3.5 and S4.
      count >= 1  → proceed to S3.5, then S4.

S4  RESOLUTION  (per record, never in bulk)
      For EACH record admitted by step 4, IN TURN:
        a. backlogit_get_checkpoint  — re-read and re-validate it
        b. handle it (write its resolution into the resolution commit)
        c. PROVE the handling succeeded for THAT record
        d. only then backlogit_resolve_checkpoint for THAT record
      BULK resolution is PROHIBITED. Resolving without a per-record
      successful-handling proof is PROHIBITED. Resolving any record not
      admitted by step 4 is PROHIBITED.
```

### `RESOLUTION_OBLIGATION_RECORD` *(RQ-12)*

**This definition is revision 9's answer to RR-3.** It is the narrowest
correction that makes RQ-7 true rather than weakening it. The problem it solves
is stated exactly: because `RESOLUTION_PREFIX` resolves **every** checkpoint
before merge, the PR body becomes the **sole** record of an outstanding closure
obligation, and a PR body is mutable — a human or bot can delete it and no
mechanism in revisions 6–8 would notice. Startup would then see zero
checkpoints **and** zero locators and conclude "clean".

**Surface: the unit's existing pre-merge operational-closure artifact in
`docs/closure/`.** This is an **existing owned repo-local state surface**, not a
new ad-hoc tracker. `docs/closure/` is created and owned by the
`operational-closure` skill, which Ship already invokes at real Step 5 item 8 —
inside **S2**, therefore already committed on the PR branch before S4 runs. The
surface already carries exactly this placeholder→finalize lifecycle shape for
two other fields (`compaction_status`, initialized `pending` by the skill and
finalized by Ship's post-merge closure; and the source-artifact cleanup
placeholder). The obligation record is a third field of the same shape, so it
inherits an owner, a schema, a path convention and a lifecycle rather than
inventing any of them.

**Why this and not the alternatives.** A record in its own new file under
`docs/closure/` would be an unowned ad-hoc tracker. A record in the checkpoint
store is self-defeating: resolving the checkpoints is precisely what removes the
signal. A record in a commit **message** is not addressable by path and is lost
to any history rewrite that preserves trees. A new executable persistence
substrate is Defect-1 scope and is forbidden here.

```text
# canonical schema — a frontmatter field of the pre-merge closure artifact
# canonical path     docs/closure/{pre-merge closure artifact for this shipment}
# canonical identity (shipment_id, pr) — exactly one OPEN record per shipment

resolution_obligation:
  status: none | OPEN | CLOSED   # 'none' is a real lifecycle state, not an
                                 # absence: the skill initializes it at S2.
                                 # An OMITTED field after S2 is OB-5, and is
                                 # NEVER read as "no obligation".
  shipment_id: <id>              # canonical field name; matches the identity
  feature: <id>
  pr: <number>
  branch: <branch>
  checkpoints: [<filename>, ...]
  opened_at: <RFC3339 UTC>       # present only when status is OPEN or CLOSED
  closed_at: <RFC3339 UTC>       # present only when status is CLOSED
```

*Revision 10 corrects two schema defects: the field was declared `shipment` while
the canonical identity and U9 called it `shipment_id` (PR #396 Copilot thread
`PRRT_kwDORJEduc6h79c6`), and `none` was used as a lifecycle state while the
status union admitted only `OPEN | CLOSED` (round-9 finding F-34).*

**It carries NO SHA.** `resolution_commits` and `final_head` stay in the
`CLOSURE_LOCATOR` and appear nowhere in this record. The record is written
*inside* the resolution commit, so a SHA field would be self-referential — RQ-8
is preserved intact, not traded away. Recovery re-derives the SHAs from Git
history — see *Recovering resolution commits* below, which replaces revision 9's
claim that the resolution-state classification could supply them.

**Lifecycle.**

| Transition | When | Commit that carries it |
|---|---|---|
| absent → `none` | S2, when `operational-closure` **creates** the artifact (create-only; it must never overwrite an existing `OPEN` or `CLOSED`) | the ordinary S2 closure commit |
| `none` → `OPEN` | S4, in the **same commit** as the checkpoint resolutions, on the protected implementation branch | the resolution commit, pushed in S4 |
| `OPEN` → `CLOSED` | `RESOLUTION_POSTCONDITION` **POST-B**, after the full verified P-001 closure set | a commit on the `post-merge/{feature_slug}` closure branch, which must itself be **merged** |
| `none` → *(unchanged)* | `RESOLUTION_POSTCONDITION` POST-B is an explicit **no-op** on a zero-checkpoint unit | no commit |

`OPEN` → `CLOSED` is the **only** permitted way to discharge the record, it
happens in a **later** commit whose ancestry stays auditable, and that commit
must itself be **merged**. Deleting the record is never a discharge.
`none` → `CLOSED` is **not** a permitted transition and must never be attempted
*(round-9 finding F-05)*.

**Path-stability invariant — the P-020 interaction** *(round-9 finding F-07)*.
`compact-context` compacts `docs/closure/` and moves originals to
`docs/archive/closure/` for records older than `threshold_days` (default 14). A
pre-merge artifact written at S2 can easily exceed that by merge, and `CLOSED` is
written only *after* the full closure set — which includes P-020 itself. Left
unhandled, routine compaction archives a still-`OPEN` record and every subsequent
startup fail-closes permanently on **correct** behaviour. Three rules, together,
close it:

1. **Ordering.** POST-B writes `CLOSED` **before** any compaction or archival
   that can touch the artifact. This is stated in `RESOLUTION_POSTCONDITION` and
   carried by U12.
2. **Stability.** An artifact that has ever carried `resolution_obligation` MUST
   NOT be renamed, compacted, or archived while its status is `none` or `OPEN`.
   `OPEN` and `none` records are **excluded from `compact-context` candidate
   selection**, stated on the P-020 surface and in U10.
3. **Tolerance.** Channel B additionally follows `docs/archive/closure/` as well
   as `docs/closure/`, and an archive **move** — the same record identity present
   at the archive path, with its history intact — is explicitly **not** OB-1.
   Only a genuine disappearance with no `OPEN` → `CLOSED` transition is.

**Recovering resolution commits without a SHA field** *(PR #396 Copilot thread
`PRRT_kwDORJEduc6h79dC`)*. S7 and S8 assert ancestry for "every recorded
resolution commit". When the locator is intact those SHAs come from the locator.
When the PR body has been deleted, they must come from Git, and revision 9's
resolution-state classification cannot supply them — it returns only
`RESOLVED`/`UNRESOLVED` per file and derives no commit. The executable recovery
is:

```text
# for the retained scan ref of the PR that carries the OPEN record:
#   the resolution commit is the commit that introduced status: OPEN
git log --format=%H --reverse <scan_ref> -- <closure_artifact_path>
  → for each candidate commit C, in order:
        git show C:<closure_artifact_path>   → parse YAML
        git show C^:<closure_artifact_path>  → parse YAML (absent ⇒ treat none)
        the FIRST C where parent.status != OPEN and child.status == OPEN
          is the resolution commit for this record identity.
  → additionally, the checkpoint resolutions ride the SAME commit, so
    assert that C also changes at least one path in the record's
    `checkpoints` list:
        git show --name-only --format= C
  → ZERO such C, or MORE THAN ONE such C for one record identity  → HALT
  → any git or parse failure                                      → HALT
```

**Two-channel discovery — Channel B does not read the PR body at all.**
*(Rebuilt in revision 10 from round-9 findings F-13, F-14, F-15 and PR #396
Copilot threads `PRRT_kwDORJEduc6h79dQ`, `…dW`, `…dn`.)* Discovery runs **both**
channels and takes the union; neither may be skipped because the other returned
nothing.

**The candidate sets are derived independently from ONE paginated enumeration.**
This is the specific defect that made revision 9's Channel B dependent on the
very surface it was meant to bypass: Channel A selected "entries whose body
contains the marker", and Channel B then re-used *that* set, so deleting the body
removed the PR from Channel B too.

```text
ONE exhaustive paginated enumeration  (the U3 read-protocol command)
        │
        ├── apply PV-1, PV-2, PV-3, PV-8 using API FIELDS ALONE
        │     → the TRUSTED set. Body text is never consulted here.
        │
        ├── CHANNEL A  (mutable, fast)
        │     from the TRUSTED set, inspect ONLY bodies carrying the
        │     `autoharness:closure-locator` marker; then PV-4…PV-7.
        │
        └── CHANNEL B  (durable, history-backed)
              from the TRUSTED set, take EVERY open or closed-unmerged PR
              — REGARDLESS OF BODY CONTENTS, marker or no marker, empty
              body or no body — plus the default branch.
```

**Channel B procedure — executable, with explicit refs.** Revision 9's commands
were not executable: they carried no commit-ish, so `git log` scanned the current
checkout; `FETCH_HEAD` was overwritten by each subsequent fetch; and a fully
deleted artifact had no current-tree path for `ls-tree` to supply.

```text
B0. SHALLOW-HISTORY GUARD  (round-9 finding F-29)
      git rev-parse --is-shallow-repository   → must be "false"
      test -e "$(git rev-parse --git-dir)/info/grafts"  → must be absent
      A shallow or grafted clone makes `git log` exit 0 while silently
      omitting commits past the horizon, so a deletion beyond it scans
      clean. Either unshallow first (`git fetch --unshallow`) or HALT.
      Exiting 0 is NOT evidence of completeness here.

B1. STATE-AWARE ROUTING  (round-9 finding F-15; Copilot thread …dQ)
      For each TRUSTED PR, route by its API state:
        state == MERGED            → scan `origin/main` ONLY.
              Its head branch may be deleted, and the record's correct
              final state lives on main. Re-scanning a merged PR's stale
              head would rediscover the OPEN record forever, even after a
              valid CLOSED transition landed on main.
        state == OPEN              → scan its live protected head (B2).
        state == CLOSED, unmerged  → scan its live head (B2) if it still
              exists; if the branch is gone, that is OB-2, not "clean".

B2. FETCH TO A UNIQUE RETAINED REF — never FETCH_HEAD
      git fetch origin \
        "+refs/pull/<n>/head:refs/autoharness/scan/pr-<n>"
      Each candidate gets its OWN ref. `FETCH_HEAD` is overwritten by the
      next fetch, so a loop over candidates that reads FETCH_HEAD reads
      the LAST one every time. Using it across candidates is PROHIBITED.
      fetch non-zero exit → HALT (OB-7).
      These refs are cleaned up per the lifecycle rule below.

B3. READ THE TREE, THEN READ THE FILE
      git ls-tree -r --name-only <ref> -- docs/closure/ docs/archive/closure/
      `ls-tree --name-only` returns PATHS ONLY and never parses anything
      (Copilot thread …dW), so for EVERY returned path:
        git show <ref>:<path>   → parse YAML → read resolution_obligation
      keep records whose status is OPEN.
      any read, YAML, or schema error → HALT (OB-5).

B4. SCAN THE FULL REACHABLE HISTORY — with an explicit root
      Both probes take the explicit <ref> (or origin/main) as their
      revision root. Omitting it (revision 9's form) scans the CURRENT
      CHECKOUT instead, so a record deleted on a PR head is invisible
      (Copilot thread …dn).
        git log --format=%H <ref> --diff-filter=D --name-only \
            -- docs/closure/ docs/archive/closure/
        git log --format=%H <ref> -S'resolution_obligation' \
            -- docs/closure/ docs/archive/closure/
      This scans the FULL reachable history from the protected head,
      independently of whether the current tree still contains the path
      and independently of the PR body.

B5. BIND EVERY TRANSITION TO A RECORD IDENTITY
      A commit list is not an obligation. For each commit C the probes
      return, parse the parent and child blobs:
        git show C^:<path>   and   git show C:<path>
      and classify the transition for the (shipment_id, pr) identity the
      blob declares:
        absent/none → OPEN     = INTRODUCTION
        OPEN        → CLOSED   = DISCHARGE
        OPEN/CLOSED → absent   = DELETION
        path A      → path B, same identity, both readable = MOVE
                       (an archive move; NOT a deletion)
      Unparseable parent or child, or an identity that changes across a
      move → HALT (OB-5).

B6. ANCESTRY + RECONCILIATION
      Every INTRODUCTION commit MUST be an ancestor of the ref it was
      found on (or of origin/main for a merged PR):
        git merge-base --is-ancestor <C> <ref>
      Unverifiable ancestry → HALT (OB-2).
      A record retains status OPEN unless a DISCHARGE commit exists that
      is (a) bound to the SAME identity and (b) a DESCENDANT of the
      INTRODUCTION commit. A valid descendant DISCHARGE on origin/main
      reconciles an older OPEN found elsewhere; it is NOT a conflict.

B7. REF LIFECYCLE
      On completion — success OR halt — delete every
      refs/autoharness/scan/* ref created by this scan. The scan leaves
      no fetched branch, worktree, or ref state on disk. Failure to clean
      up is reported but does not itself discharge anything.
```

**Provenance for Channel B is established entirely from API response fields and
Git ancestry**, never from PR-body text — that is what makes it independent of
the surface under suspicion. **PV-1** (same repository), **PV-2** (base is the
default branch), **PV-3** (trusted `author_association`) and **PV-8** (the
implementation-PR binding, below) are the admission filters. PV-4 and PV-5, which
compare against locator *body* fields, are **not applicable** to Channel B.

**Provenance of the CLOSED transition is separate from provenance of the
INTRODUCTION** *(round-9 finding F-15)*. Revision 9 applied one "the record's
embedded `pr` equals the PR it was read from" rule to both, which structurally
rejected every correct discharge: the record is opened on the implementation PR
and closed on a **different** `post-merge/*` closure PR, where its embedded `pr`
and `branch` correctly still name the implementation PR.

| Check | Applies to | Rule |
|---|---|---|
| PV-B4 | the **INTRODUCTION** commit only | the record's embedded `pr` equals the PR whose head the introduction was found on, and `branch` equals that PR's head ref |
| PV-B6 | the **DISCHARGE** commit | the record's embedded `pr` and `branch` still name the **implementation** PR (they are immutable identity, not a location), **and** the discharge commit is a Git **descendant** of the introduction commit, **and** it is reachable from `origin/main` or from a trusted `post-merge/*` PR head whose relationship to the implementation PR is recorded in the closure artifact and checked |

PV-6 and PV-7 apply unchanged to both.

**PV-8 — bind discovery to the shipment's designated implementation PR**
*(round-9 finding F-17)*. Both channels previously admitted **any** trusted
same-repository PR as a carrier for an **arbitrary** shipment ID, so a
trusted-but-careless or compromised collaborator could open an unrelated decoy PR
carrying a fabricated record naming a real in-flight shipment; any field
difference then tripped OB-3 and halted that shipment's every future startup. No
fork and no admin access were needed.

> **PV-8.** A discovered locator or obligation record naming shipment `X` is
> admitted **only** if the PR it was found on **is** shipment `X`'s
> backlog-recorded implementation PR (or, for a discharge, that shipment's
> `post-merge/*` closure PR). A candidate whose PR number does not match that
> binding is **DISCARDED silently** — never halted on. Discarding is essential:
> halting would hand the denial-of-service back to the decoy author. If the
> backlog records no implementation PR for the shipment, that specific candidate
> cannot be bound and is **held for operator review** rather than either trusted
> or silently dropped.

**Fail-closed rules.** Each of these **halts to the operator**:

| # | Condition | Why it cannot be tolerated |
|---|---|---|
| OB-1 | An artifact that carried a record in history is absent from **both** `docs/closure/` and `docs/archive/closure/` with no bound `OPEN` → `CLOSED` transition | This is the deletion attack. Absence is never evidence of discharge. An archive **move** with intact history is explicitly *not* this. |
| OB-2 | Force-push, rebase, branch disappearance, or shallow/grafted history makes the record's introducing commit unreachable or its ancestry unverifiable | An unverifiable record cannot be trusted either way. S3.5's ruleset makes this rare; it does not make it impossible. |
| OB-3 | Two or more **conflicting** `OPEN` records for the same shipment, after PV-8 binding and after B6 reconciliation | No field can rank them. Identical duplicates collapse to one and are logged; **any** field difference halts. `opened_at` MUST NOT be a tie-breaker. |
| OB-4 | `OPEN` records for more than one distinct shipment | A P-001 violation, exactly as for the locator. |
| OB-5 | A record is unparseable, missing a field its declared status requires, **or omitted entirely from an artifact created after S2** | Partial evidence is not evidence. An omitted field is OB-5, never "no obligation". |
| OB-6 | Channel A and Channel B **disagree** — a locator says `RECONCILED` while an `OPEN` record exists, or a locator is absent while an `OPEN` record exists | The disagreement is the signal. Never prefer the mutable channel, and never let a missing locator discharge a live record. |
| OB-7 | Channel B enumeration is incomplete — a fetch, `ls-tree`, `show`, or `log` exits non-zero, or PR pagination is truncated | Identical to RQ-9's rule for Channel A. |
| OB-8 | **Ruleset drift**: a unit that published an `OPEN` record under S3.5 protection is later found on a branch that no longer satisfies `BRANCH_PROTECTION_PREREQUISITE` | The durability guarantee the record was published under has been withdrawn beneath it. Continuing would treat a now-erasable record as durable. |

**What this deliberately does not do.** It adds **no** executable persistence
substrate, **no** lock or compare-and-swap, and **no** cross-run cursor. It
implements **no** part of Defect 1 — no continuation predicate, no activation
record store, no auto-routing, and no lineage or cursor auto-routing of any kind.
Its only authority is to **halt**; it confers no merge authority (RQ-10 is
untouched).

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
   --jq '.[] | {number, state, body, author_association,
                head_repo: .head.repo.full_name, head_ref: .head.ref,
                base_repo: .base.repo.full_name, base_ref: .base.ref}'
  → select entries whose body contains `autoharness:closure-locator`
  → apply PV-1 … PV-7 provenance validation (below); discard untrusted
  → parse each TRUSTED block; keep those whose status is NOT RECONCILED
```

**Provenance validation — required before a locator participates in recovery.**
Enumeration is exhaustive over `state=all`, which necessarily includes **fork
PRs opened by untrusted authors**. A PR body is attacker-controlled text, so
"contains the marker" is an insufficient admission test: without the checks
below, any account able to open a PR can forge a locator and either halt every
startup (denial of service) or steer Ship's recovery onto an attacker-chosen
branch, PR number, and checkpoint set. Marker presence establishes only that a
*candidate* block exists — never that it is authentic.

A parsed candidate is **TRUSTED** only when **all** of the following hold. They
are evaluated on API response fields, never on the body text, because the body
is the very thing under suspicion:

| # | Check | Field | Fail action |
|---|---|---|---|
| PV-1 | The PR is **same-repository**: `head.repo.full_name == base.repo.full_name == {owner}/{repo}`. Any fork-originated PR is rejected outright. | `head.repo.full_name`, `base.repo.full_name` | **Discard** as untrusted |
| PV-2 | The PR **base** is the repository's default branch. | `base.ref` | **Discard** as untrusted |
| PV-3 | The PR author is trusted: `author_association` is `OWNER`, `MEMBER`, or `COLLABORATOR`. | `author_association` | **Discard** as untrusted |
| PV-4 | The locator's embedded `pr` equals the PR number the block was read from. | block `pr` vs. API `number` | **Halt to operator** — a self-inconsistent locator on a trusted PR is corruption, not noise |
| PV-5 | The locator's embedded `branch` equals the PR's own head ref. | block `branch` vs. `head.ref` | **Halt to operator** |
| PV-6 | The locator's `shipment` and `feature` exist in this workspace's backlog and the shipment owns that feature. | backlog lookup | **Halt to operator** |
| PV-7 | Every filename in the locator's checkpoint list is a checkpoint this workspace could own (resolvable path shape, `agent` field `ship` or `stage`). | checkpoint lookup | **Halt to operator** |
| PV-8 | The PR **is** the named shipment's backlog-recorded implementation PR (or, for a discharge, that shipment's `post-merge/*` closure PR). *(Added in revision 10, round-9 finding F-17.)* | backlog lookup vs. API `number` | **Discard** as not-ours — never halt, or a decoy author gains a denial-of-service. If the backlog records no implementation PR, **hold for operator review**. |

**Discard versus halt is deliberate and must not be collapsed.** PV-1…PV-3 and
**PV-8** are *authenticity and binding* filters: a failing candidate is simply
**not ours**, is discarded **silently**, and MUST NOT halt the session —
otherwise any outsider could permanently deny startup by opening a fork PR
containing the marker, and any trusted-but-careless collaborator could do the
same with a decoy PR naming a real shipment. PV-4…PV-7 run only on candidates
that already passed PV-1…PV-3 and PV-8, so a failure there means a **trusted,
correctly bound** locator is internally inconsistent, which is exactly the
corruption the halt-on-incomplete rule exists to catch.

Discarded (untrusted) candidates MUST NOT be counted toward the
multiple-locator P-001 check below, MUST NOT be acted on, and MUST NOT be
treated as evidence of an outstanding obligation. They SHOULD be logged with
their PR number and the failing check so a genuine misconfiguration is
diagnosable. Every subsequent rule in this section — the P-001 multiplicity
check, `LAST_MILE_RECOVERY` entry, and every action table row — operates
**exclusively over the TRUSTED set**.

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
  abandoned mid-closure. Recovery must not silently pick one and proceed. This
  check is evaluated over the **TRUSTED** set only (see *Provenance validation*),
  so a discarded fork-authored forgery can neither manufacture a P-001 halt nor
  mask a genuine one.
* **Two or more TRUSTED non-`RECONCILED` locators naming the *same* shipment
  halt to the operator unless they are byte-identical.** The previous wording —
  "handled normally" — was undefined and unsafe. Same-shipment duplicates are
  *not* a P-001 violation, but they are not automatically benign either: they may
  disagree on `branch`, `pr`, the checkpoint filename set, `status`, or
  `final_head`, and no rule can rank them, because both are equally authentic
  under PV-1…PV-7 and the locator carries no revision, sequence, or signature
  field to canonicalize on. `updated_at` MUST NOT be used as a tie-breaker: it is
  body text, so a stale duplicate can carry the later timestamp. The rule is
  therefore mechanical and fail-closed:

  1. Normalize each block (strip trailing whitespace; compare fields, not layout).
  2. If every duplicate is **field-for-field identical**, they are one logical
     locator recorded twice. Proceed on that single record; log the duplication.
  3. If any field differs — *including* `status`, and *including* the case where
     one is `RESOLUTION_PENDING` and another `RESOLUTION_PUBLISHED` — **halt to
     the operator** and present every conflicting copy. Do not merge, do not
     prefer the more advanced status, and do not re-resolve.

  Rationale: an ambiguous same-shipment pair means an earlier session published
  a locator the current one cannot account for. Guessing between them risks
  merging a branch whose resolution commits were never enumerated — the exact
  orphaning failure the locator exists to prevent *(RQ-7, RQ-11)*.

### `LAST_MILE_RECOVERY` *(RQ-10, RQ-11)*

Resolving every checkpoint before merge necessarily leaves a **checkpoint-free
residual window** between the last resolution and verified closure. This window
is unavoidable: any Git-tracked checkpoint intended to cover a post-merge window
is provably unresolvable without a further PR, which is the orphaning defect
itself, one level down. The window is therefore covered by **live state
reconstruction anchored on the `CLOSURE_LOCATOR`**, not by a checkpoint.

Recovery in this window MUST NOT rely on checkpoint enumeration, which will
correctly report zero active candidates. **Entry is wired into the zero-candidate
branch itself**: a session finding zero active checkpoints MUST run
**both discovery channels** — the `CLOSURE_LOCATOR` read protocol (Channel A)
**and** the `RESOLUTION_OBLIGATION_RECORD` Git-history scan (Channel B) —
**before** concluding "clean startup" and before selecting new queue work.
Neither channel may be skipped because the other returned nothing, and a
Channel-A/Channel-B disagreement halts (OB-6). When **both** channels complete
and neither yields a non-`RECONCILED` locator nor an `OPEN` obligation record,
normal startup continues unchanged, byte for byte.

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
| `RESOLUTION_PENDING` | **Open** | Resolution has not been published. Do **not** update the locator, do **not** re-establish readiness, do **not** approach the merge bar. Place the working tree on the PR branch (see *Working-tree placement* below), then run the **resolution-state classification** (below) over the locator's checkpoint list at the fetched PR head. **`NONE`** → re-enter `RESOLUTION_PREFIX` at the resolve step. **`PARTIAL`, `ALL`, or `INDETERMINATE`** → do **not** re-resolve; **halt to operator**, because the locator cannot be trusted to enumerate them. If the working tree cannot be placed on the branch, **halt**. |
| `RESOLUTION_PENDING` | **Merged** | **Unrecoverable orphan — halt to operator immediately.** The PR merged carrying unresolved checkpoints. Never run closure, never mark `RECONCILED`. An empty `resolution_commits` list makes every ancestry assertion vacuously pass, so the merged row below must never be reached in this state. |
| `RESOLUTION_PENDING` | any other | **Halt to operator**, per the rows below. |
| `RESOLUTION_PUBLISHED` | — | Proceed to Step 1b. |
| `RECONCILED` | — | Not discovered; terminal. |

**Resolution-state classification (executable; required by the
`RESOLUTION_PENDING` / Open row).** A `RESOLUTION_PENDING` locator deliberately
carries **no** `resolution_commits` and an empty `final_head` *(RQ-8 — the
locator is never self-referential)*, so "have the resolution commits been made?"
cannot be answered from the locator. It MUST be answered from **checkpoint
state at the fetched PR head**, over the locator's own `checkpoints` list, which
is the only enumeration that exists in this phase.

Run after working-tree placement, at the fetched `refs/pull/<pr>/head`, for
**every** filename in the locator's `checkpoints` list:

```text
git fetch origin refs/pull/<pr>/head          # already done by placement
for each <file> in locator.checkpoints:
    git cat-file -e FETCH_HEAD:<file> 2>/dev/null   # does it exist at PR head?
      → absent            ⇒ state(<file>) = MISSING
      → present: read it; parse CheckpointV1
          → parse failure / schema-invalid    ⇒ state(<file>) = UNPARSEABLE
          → status == "resolved"              ⇒ state(<file>) = RESOLVED
          → status == "active"                ⇒ state(<file>) = UNRESOLVED
          → any other status value            ⇒ state(<file>) = UNPARSEABLE
```

Classify the **whole list**, never a sample, then reduce:

| Reduced state | Condition | Required action |
|---|---|---|
| `NONE` | **every** listed checkpoint is `UNRESOLVED` | Resolution never started. **Re-enter `RESOLUTION_PREFIX` at the resolve step.** This is the only branch that resumes. |
| `PARTIAL` | at least one `RESOLVED` **and** at least one `UNRESOLVED` | Crash mid-resolution. **Halt to operator.** Re-resolving would re-commit already-resolved records and the locator cannot enumerate what was done. |
| `ALL` | **every** listed checkpoint is `RESOLVED` | Crash after resolution, before the phase-2 locator update. **Halt to operator** — the locator under-reports the branch's true state and must be reconciled by hand. |
| `INDETERMINATE` | any `MISSING` or `UNPARSEABLE`, **or** the `checkpoints` list is empty, **or** any `git`/read command exits non-zero | **Halt to operator.** Missing or unreadable evidence is never "nothing was done". |

**Precedence is strict**: evaluate `INDETERMINATE` **first**, then `PARTIAL`,
then `ALL`, then `NONE`. A single unreadable file therefore halts rather than
being silently skipped into a `NONE` verdict — the failure mode this
classification exists to close, in which a partially-resolved branch is
re-classified as untouched and resolved a second time.

**Working-tree placement — committing re-entry only.** *(The canonical name;
referenced by that exact phrase from U11 and V12.)*

`LAST_MILE_RECOVERY` is entered at session start, when the working tree is
normally on `main` and may be a fresh checkout. Committing a resolution on
`main` is forbidden (P-010), and staying on `main` makes the re-entry
undischargeable. This procedure is **required before any committing re-entry**
and is **not** required — and must not be performed — for read-only steps: every
ancestry assertion in Step 1b needs the fetch but **no** switch.

**Worktree topology — reconciled with the recorded prior art** *(round-9 findings
F-21 and F-28)*. The compound library already solved this shape in
`docs/compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`,
which uses **two distinct worktree paths** — an implementation worktree and a
clean post-merge worktree spawned at the merge SHA — with an explicit rule not to
mutate the primary working tree. This plan reuses that pattern rather than
inventing a third:

* the **S4 resolution commit** is made in the **implementation worktree**, on the
  PR head — it is pre-merge work on the implementation branch;
* the **POST-B `CLOSED` transition** is made on the `post-merge/{feature_slug}`
  branch that Ship's post-merge closure step already creates — it is post-merge
  work and must not touch the implementation branch;
* `LAST_MILE_RECOVERY`'s committing re-entry re-creates the **implementation**
  placement only, because the only re-entry that commits is the S4 resolve step;
* no *additional* parallel implementation branch or worktree is created by
  anything in this plan (P-016).

| # | Step | Fail action |
|---|---|---|
| WP-0 | Run the **P-011 / P-016 worktree-topology gate** for this shipment before any branch mutation. *(Added in revision 10: WP creates/switches branches and commits, so it crosses the topology boundary and must not bypass the gate.)* | **Halt** on gate failure or on a detected parallel implementation worktree. |
| WP-1 | Assert the working tree is **clean** (no staged, unstaged, or untracked changes that a switch would carry or clobber) | **Halt.** Never stash, never discard. |
| WP-2 | Read `branch` and `pr` from a locator that already passed PV-1…PV-8, or from an obligation record that already passed PV-1…PV-3, PV-8 and PV-B4. An untrusted or unvalidated source is never used | **Halt.** |
| WP-3 | `git fetch origin refs/pull/<pr>/head` | **Halt** on non-zero exit. |
| WP-4 | Place the tree on that tip: if no local branch of that name exists, create it at the fetched tip and switch; if one exists, switch and **fast-forward only** to it | **Halt** on switch failure, and **halt** on any non-fast-forward/divergence. Never `git reset --hard`, never force checkout, never rebase. |
| WP-5 | Assert the **current branch name** equals the locator/record `branch` | **Halt** on mismatch. |
| WP-6 | Assert `HEAD` equals the fetched tip | **Halt** on mismatch. |
| WP-7 | Assert HEAD is **not** the default branch and **not** detached | **Halt.** |

**Argument safety** *(round-9 finding F-35)*. `branch` and `pr` are read from a
locator or record and passed to Git. Every such invocation MUST use **argv-array
execution** (no shell string interpolation) and MUST place a `--` separator
before any ref or path operand. Git's ref-naming rules limit exploitability, but
the plan forecloses shell interpolation explicitly rather than relying on them.

**Side effects and cleanup** *(round-9 finding F-37)*. A committing re-entry can
leave side effects in `.backlogit/stash.jsonl` and `docs/memory/**`. These MUST
be either committed with the resolution commit or explicitly carried forward, so
the next session does not classify the tree as dirty at WP-1. On completion or
halt, WP leaves **no** fetched scan ref and **no** extra worktree state on disk
(the same lifecycle rule as Channel B's B7).

**After any WP failure the session performs no resolution and no commit.** It
does not retry with a reset, a force checkout, or a discard; it halts to the
operator with the failing step named. Proceeding on the wrong branch, on a
diverged branch, or on `main` is precisely the hazard this procedure exists to
close, and a "best effort" placement is worse than none because the subsequent
commit would look legitimate.

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

Fourteen units. Each edits **one file** — except **U0**, which edits **no** file
and changes GitHub repository settings instead. Every unit is **single-domain**
and sized for well under two hours. Acceptance criteria below are the *exact*
text carried into the task cards.

*Revision 10 raised the count from ten to fourteen: **U0** carries the operator's
RR-3 ops prerequisite, and round-9 finding **F-09** required U3, U4 and U9 to be
split along their natural seams (producing **U11**, **U12** and **U13**) because
each exceeded the NON-NEGOTIABLE 2-hour / single-domain granularity rule.*

### U0 — Configure the resolution-branch protection ruleset *(ops prerequisite; `ProposedAction`)*

**Depends on**: nothing. U0 is a **root** alongside U2.
**File**: **none.** This unit changes **GitHub repository settings**, not
repository content.

> ### ⚠ ProposedAction — approval required before execution
>
> | Field | Value |
> |---|---|
> | `action_class` | `ProposedAction` |
> | `ActionRisk` | **high** |
> | `approval_required` | **true** |
> | `approver` | repository **operator or admin** — never an agent, never dark-mode (P-017) |
> | `surface` | external GitHub repository settings (a repository ruleset) |
> | `applied_by_this_PR` | **NO.** This planning PR specifies the change and applies nothing. |
> | `reversibility` | the ruleset is re-configurable, but the unprotected window before it exists cannot be closed retro-actively — hence `high` |
>
> **The implementing agent MUST obtain explicit operator/admin approval
> immediately before applying this change, and MUST halt if approval is absent,
> ambiguous, or supplied by dark-factory mode.** An approval recorded for any
> other unit does not carry to U0.

**Why this unit exists.** It implements the operator's RR-3 decision. The
obligation record's durability against *history rewriting* is a property of forge
branch protection, not of Git. Measured today, Ship source branches have none:
`GET /rules/branches/main` returns
`["deletion","non_fast_forward","pull_request","copilot_code_review"]`, while the
same endpoint for a live Ship branch returns `[]`. Without U0, S3.5 would halt
every checkpoint-owning unit, so U0 is a genuine prerequisite rather than a
hardening.

**Branch patterns are derived from the installed Ship agent, not guessed.**
`_ship.agent.md` creates `feat/{feature-slug}` for features and the `chore/`
variant for chores as the shipment branch, and `post-merge/{feature_slug}` for
all Step 6 closure work. Those three patterns are exactly the resolution-bearing
set: the first two carry the `OPEN` record, the third carries the `CLOSED`
transition.

**Acceptance criteria**

1. A repository ruleset exists whose `conditions.ref_name.include` covers
   **exactly** `refs/heads/feat/**`, `refs/heads/chore/**` and
   `refs/heads/post-merge/**`, derived from the installed `_ship.agent.md`
   branch-creation steps and **recorded in the task card with the line
   references** they were derived from. It does **not** cover `~DEFAULT_BRANCH`;
   the existing `PR-Required` ruleset (id `12812291`) already does, and U0 must
   not modify, replace, or weaken it.
2. The ruleset declares both a `deletion` rule and a `non_fast_forward` rule.
3. The ruleset declares **no bypass actors** (`bypass_actors` is empty), and
   `current_user_can_bypass` resolves to `never` for the acting identity.
4. `enforcement` is `active`.
5. The ruleset adds **no** `pull_request` rule and **no** required-status-check
   rule to these patterns. Requiring review on every Ship working branch would
   break Ship's own push loop; the obligation record needs immutability, not
   review. *(This is stated as an explicit non-goal so a later editor does not
   "helpfully" add one.)*
6. Verification is by the **effective-rules API**, not by reading back the
   configuration: `GET /repos/{owner}/{repo}/rules/branches/{branch}` for a
   representative branch of **each** of the three patterns returns entries of
   type `deletion` **and** `non_fast_forward`.
7. The task card records the **approval evidence** — who approved, when, and the
   resulting ruleset id — and records that no other repository setting was
   changed.
8. `markdownlint` is **not applicable** (no file changes). This exemption is
   stated explicitly rather than silently omitted.

**Posture**: ops prerequisite, approval-gated. **Size**: XS. **Complexity**: low.
*(Low complexity, high risk: the action is small and well specified; the risk is
that it mutates a shared external surface.)*

### U1 — State the resolution-durability policy

**Depends on**: U2 (U1 installs a by-name reference to `RESOLUTION_PREFIX`, whose
definition only U2 creates; running U1 first would leave a dangling reference).
*(Revision 10 removes the stale `*(root)*` label this heading still carried,
which contradicted the dependency section's `U2→U1` edge — round-9 finding
F-22.)*
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

**Depends on**: nothing. U2 is a **root** alongside U0.
**File**: `.github/instructions/github-pr-automation.instructions.md` — one new
subsection at **heading level 3**, inserted **after the end of `### 1.9`** (that
is, after its last `####` child) and before the next `###` section. It is a
sibling of `### 1.9`, never spliced inside it — a level-3 heading placed among
1.9's children would silently terminate the readiness-gate section and orphan
1.9.3 onward. No existing section is renumbered.

This file is the **canonical home** for `HEAD_EVIDENCE_RULE`, `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION`,
`BRANCH_PROTECTION_PREREQUISITE` and `CLOSURE_LOCATOR`. It is the correct home
because the locator is PR-body metadata — the surface this file already governs —
and because siting the text here keeps it out of any commit, the property that
makes it non-self-referential.

**Acceptance criteria**

1. States that PR-body updates do not advance `headRefOid`, and gives
   point-in-time wording as the alternative for committed documents.
2. Cites the observed instances: PR #395 threads `PRRT_kwDORJEduc6h2uOQ` and
   `PRRT_kwDORJEduc6h2viu`.
3. **`RESOLUTION_PREFIX` and `RESOLUTION_POSTCONDITION` appear here verbatim**,
   exactly as given in this plan's canonical definition, including **all nine**
   of its ordering invariants, the gate-failure remediation loop, and the
   residual-window checkpoint prohibition. This is the single canonical copy.
   *(Revision 10 corrects "four" to **nine**: the canonical definition declares
   nine, U2 installs the block verbatim and V6 checks exactness, so the previous
   wording had no satisfiable reading and invited an implementer to drop
   invariants 5–9 — round-9 finding F-20.)*
4. **The installed text contains no `_ship.agent.md` item numbers.** Segments
   S1…S8 are stated by **role** only. *(Round-9 finding F-08: hard-coding live
   item numbers — including the live file's duplicated `7` — into a second file
   is the coupling that caused revisions 6–8 to describe a Step 5 that did not
   exist.)*
5. The `CLOSURE_LOCATOR` block is given verbatim with every field named.
6. The publication protocol is described as **three-phase** and its three phases
   are `RESOLUTION_PENDING`, `RESOLUTION_PUBLISHED`, `RECONCILED`. No other
   phase count appears anywhere in the file.
7. Phase 1 is stated to be published **before the first resolution commit** and
   to require no SHA.
8. Phase 2 is stated to follow the **push** of the resolution commits, and to
   carry the SHAs and `final_head`.
9. Phase 3 is stated to require merge **plus the full P-001 post-merge closure
   set, including the P-020 compaction record** — not merge alone, and not
   partial closure.
10. States explicitly that no commit is ever required to record its own SHA, and
    that a branch-only artifact is not fresh-checkout discoverable.
11. **`BRANCH_PROTECTION_PREREQUISITE` appears verbatim**, including segment
    **S3.5**, the effective-rules query against the **live `headRefName`** read
    from the PR API, the four requirements (`deletion` present,
    `non_fast_forward` present, `enforcement: active`, no usable bypass and
    `current_user_can_bypass: never`), the repository-supported equivalent, and
    the rule that API unavailability, ambiguity, an uncovered branch, or any
    bypass capability **halts before obligation publication**.
12. **`RESOLUTION_POSTCONDITION` is installed as two explicitly named
    mutations** — **POST-A**, a PR-body `RECONCILED` write that creates no commit,
    and **POST-B**, an `OPEN` → `CLOSED` **commit** on the named
    `post-merge/{feature_slug}` closure branch which must itself be merged. The
    withdrawn sentence "the postcondition is a PR-body metadata write only and
    commits nothing" **does not appear**, and neither does any equivalent
    restatement. *(Round-9 finding F-01.)*
13. Both POST-A and POST-B are stated to be **conditional**: POST-A is a no-op
    when no locator was published, and POST-B is a no-op when
    `resolution_obligation` is `none`, which is **left intact**. `none` →
    `CLOSED` is stated to be a forbidden transition. *(Round-9 finding F-05.)*
14. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U3 — Record exhaustive discovery and provenance validation

**Depends on**: U2 (extends the subsection U2 creates).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

*Split in revision 10 (round-9 finding F-09). U3 previously combined exhaustive
API discovery, seven provenance rules, duplicate handling, locator-status
classification, a four-state checkpoint classifier, seven live-PR outcomes, a
merge-authority bar and WP-1…WP-7 — at least seven behavioural scenarios against
a fewer-than-four limit. U3 now owns **discovery and provenance only**; **U11**
owns recovery, classification and working-tree placement.*

Add the locator read protocol and its provenance validation immediately after
U2's subsection.

**Acceptance criteria**

1. The read protocol gives the **exact executable command**, whose `--jq`
   projection carries **every field the provenance checks are evaluated on** —
   never the pre-provenance `{number, state, body}` form:
   `gh api --paginate -H "Accept: application/vnd.github+json"
   "repos/{owner}/{repo}/pulls?state=all&per_page=100" --jq '.[] | {number,
   state, body, author_association, head_repo: .head.repo.full_name, head_ref:
   .head.ref, base_repo: .base.repo.full_name, base_ref: .base.ref}'`. It
   explicitly records **why `gh pr list` is forbidden**: it returns no body
   without `--json ...,body` and has no `--paginate` flag, only a bounded
   `--limit` defaulting to 30, so the naive form silently reports a clean
   startup.
2. **`PV-1`…`PV-8` provenance validation runs on every candidate before that
   candidate participates in recovery**, evaluated on API response fields rather
   than body text, and the **discard-versus-halt split is reproduced without
   being collapsed**: `PV-1` (same-repository), `PV-2` (base is the default
   branch), `PV-3` (author `OWNER`/`MEMBER`/`COLLABORATOR`) and **`PV-8`** (the
   PR is the named shipment's backlog-recorded implementation or closure PR) are
   authenticity/binding filters whose failures are **discarded silently and MUST
   NOT halt**; `PV-4`…`PV-7` run only on candidates that already passed those and
   **halt to the operator**. Discarded candidates MUST NOT be counted toward the
   P-001 multiplicity check, acted on, or treated as evidence of an outstanding
   obligation, and every downstream rule operates over the **TRUSTED set only**.
3. **PV-8's rationale and its hold-for-review case are stated**: without the
   implementation-PR binding, any trusted collaborator can open a decoy PR
   naming a real in-flight shipment and halt that shipment's every future
   startup, with no fork and no admin access. A candidate that cannot be bound
   because the backlog records no implementation PR is **held for operator
   review**, neither trusted nor silently dropped. *(Round-9 finding F-17.)*
4. **The two candidate sets are stated to be derived independently from the one
   paginated enumeration**: Channel A may inspect only trusted bodies carrying
   the marker, while Channel B takes **every** trusted open or closed-unmerged
   PR **regardless of body contents**. It is stated explicitly that Channel B's
   candidate set is **never** filtered by marker presence, because that
   dependency is what made the previous design fail in the exact residual window
   it existed to cover. *(Round-9 finding F-13.)*
5. A non-zero exit, a truncated or rate-limited page, or an unparseable response
   is stated to be an **error that halts**, never evidence that nothing is
   outstanding.
6. Filtering discovery by shipment status is **explicitly prohibited**, with the
   139-S archived-shipment case given as the reason.
7. It is stated that only the implementation PR publishes a locator, and why: the
   PR body survives merge and branch deletion, so no closure-PR locator is needed.
8. Non-`RECONCILED` locators for **more than one distinct shipment** are stated
   to be a P-001 violation that halts immediately.
9. An incomplete or unparseable locator is stated to be a halt-to-operator
   signal, and the same-shipment duplicate rule is given in full: byte-identical
   duplicates collapse and are logged; **any** field difference halts;
   `updated_at` is barred as a tie-breaker because it is body text.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U11 — Record last-mile recovery and working-tree placement

**Depends on**: U3 (extends the same subsection; same file, strictly sequential).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

*Added in revision 10 as the recovery half of the U3 split (round-9 finding
F-09).* Add `LAST_MILE_RECOVERY` immediately after U3's material.

**Acceptance criteria**

1. The **Step 1a locator-status gate** appears with all five rows, and states
   that a `RESOLUTION_PENDING` locator is evaluated **before** any live-PR
   classification — including that a merged PR with a `RESOLUTION_PENDING`
   locator is an unrecoverable orphan that halts, and must never reach an
   ancestry assertion that an empty commit list would vacuously pass.
2. The **resolution-state classification** appears with its four reduced states
   (`NONE`, `PARTIAL`, `ALL`, `INDETERMINATE`), its strict precedence
   (`INDETERMINATE` first), and the rule that only `NONE` resumes.
3. It is stated explicitly that the resolution-state classification returns
   per-file states and **does not** derive commit SHAs, and that the
   *Recovering resolution commits without a SHA field* procedure is the executable
   source of `resolution_commits` whenever the locator is unavailable.
   *(PR #396 Copilot thread `PRRT_kwDORJEduc6h79dC`.)*
4. The **Step 1b live-PR classification** table appears with all seven rows and
   their stated ancestry targets and actions.
5. The `Open, HEAD ≠ final_head` row requires the ancestry assertion against the
   fetched PR head **before** re-establishing readiness, and states that a
   force-push or rebase that dropped the resolution commits halts.
6. It is stated that an open PR whose resolution commits are absent from
   `origin/main` is **not** an error **for this recovery protocol**, and that
   this does **not** relax Ship's Merge Confirmation Gate.
7. The four-part merge-authority bar appears in full, with halt as the default
   for any doubt, and it is stated that this path confers **no** merge authority
   and cannot supply the approval signal P-014 requires.
8. The entry rule requires **both** discovery channels before concluding "clean
   startup", names **Channel B** and the `RESOLUTION_OBLIGATION_RECORD`
   explicitly, and states that a Channel-A/Channel-B disagreement halts (OB-6)
   and that a missing locator never discharges an `OPEN` record. *(Round-9
   finding F-26: the entry rule previously required both channels while the unit
   owning it carried no criterion mentioning Channel B at all, so it could pass
   with a single mutable channel installed.)*
9. A procedure titled **exactly** `Working-tree placement — committing re-entry
   only` appears, required **before any committing re-entry** and explicitly
   **not** required for read-only ancestry assertions (fetch, no switch). It
   carries all eight steps with their halt actions: **WP-0** the P-011/P-016
   worktree-topology gate; **WP-1** clean working tree; **WP-2** branch/PR read
   only from a provenance-validated locator or obligation record; **WP-3**
   `git fetch origin refs/pull/<pr>/head`; **WP-4** safe create-and-switch for a
   new local branch and **fast-forward-only** switch for an existing one;
   **WP-5** current branch name equals the recorded `branch`; **WP-6** `HEAD`
   equals the fetched tip; **WP-7** HEAD is neither the default branch nor
   detached. It states that a dirty tree, a failed gate, a failed fetch, a failed
   switch, a divergence, or any assertion mismatch **fails closed**, and that
   after any failure the session performs **no** `git reset`, **no** force
   checkout, **no** resolution and **no** commit.
10. The **worktree-topology reconciliation** is stated: the S4 resolution commit
    is made in the **implementation** worktree on the PR head, the POST-B
    `CLOSED` transition is made on the `post-merge/{feature_slug}` branch Ship
    already creates, and no additional parallel implementation branch or worktree
    is created (P-016). The recorded prior art
    (`docs/compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`)
    is cited as the source of the pattern. *(Round-9 finding F-21.)*
11. **Argument safety** is stated: locator/record-sourced `branch` and `pr`
    values are passed to Git via **argv-array execution** with a `--` separator,
    never by shell string interpolation. *(Round-9 finding F-35.)*
12. **Side effects and cleanup** are stated: `.backlogit/stash.jsonl` and
    `docs/memory/**` side effects are committed or explicitly carried forward so
    the next session does not fail WP-1, and no fetched ref or worktree state is
    left on disk. *(Round-9 finding F-37.)*
13. A **P-012 availability contract** is stated covering every operation this
    plan places on the critical path — `backlogit_list_checkpoints`,
    `backlogit_get_checkpoint`, `backlogit_resolve_checkpoint`,
    `backlogit_create_checkpoint`, `gh api` including the effective-rules
    endpoint, review-thread enumeration, and Git history access. Each is probed
    **before** the path that needs it (pre-S3, pre-S3.5, pre-discovery,
    pre-recovery, pre-merge); only **declared official CLI fallbacks** may be
    used; anything else **halts**. A registry exposing **no checkpoint
    operations** is defined as a **halt**, explicitly **never** as an implicit
    zero enumeration. *(Round-9 finding F-16.)*
14. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U4 — Re-order Ship Step 5 into the `RESOLUTION_PREFIX` finalization tail

**Depends on**: U0, U1, U2, U9, U10.
**File**: `.github/agents/_ship.agent.md` — **Step 5 (PR Lifecycle)** only.

*Split in revision 10 (round-9 finding F-09). U4 previously declared its locus as
Step 5 and session end while AC7–AC8 also modified Step 6, carried 13 acceptance
criteria, moved two gates, amended one item, added a merge bar and refresh rules,
and was exercised by V4, V9 and V13–V16. Raising it to M/high was honest but did
not make it compliant. U4 now owns **Step 5 only**; **U12** owns Step 6 and
session end.*

**The real Step 5 item list, extracted verbatim rather than described.** The
escalation's assessment was that every prior round mismatched what this plan
*asserted* Step 5 contains against what it *actually* contains. The extraction
below is the authority for this unit; the task card carries it verbatim.
**Two independent round-9 personas re-derived this table from the live file and
confirmed it accurate; it is preserved unchanged in revision 10.**

| Real item | Content | Mutates branch? |
|---|---|---|
| 1, 1a | full quality gates; `pipeline-topology` lifecycle gate | no |
| 2 | session memory summary to `docs/memory/` | yes (commit) |
| 3 | full local build | no |
| 4, 5, 5a | confirm readiness covers HEAD; prepare §1.9 body block; topology gate | no |
| 6 | `pr-lifecycle` — create/update the PR | no |
| **7** *(first)* | `fix-ci` loop | **yes — may commit and push** |
| 7a | optional shadow-review loop | **yes — may commit and push** |
| 7b | **P-014 §1.9 readiness gate** | no |
| 7c | **P-018 copilot-review gate** | no |
| **7** *(second — the item number is duplicated in the live file)* | `runtime-verification` | yes |
| 8 | `operational-closure` (writes `docs/closure/`) | yes |
| 9 | follow-up stash writes | yes |
| 10 | push the branch | yes |
| 11, 12, 13 | broadcast; present PR; branch retention | no |
| 14 | P-014 operator approval gate | no |
| 15 | last-mile re-check — **P-018 re-run + `headRefOid` re-query only** | no |
| 16 | P-009 merge-commit guardrail | no |
| 17 | P-017 dark-mode fallback state machine | no |

Two facts follow directly and are the substance of this unit. First, **items 7b
and 7c currently run before items 7(second)/8/9/10**, so readiness is gated
*before* four mutating items and the push — which is why moving resolution into
Step 5 changes the common path for **every** unit, not only checkpoint-owning
ones. Second, **item 15 re-fetches only the P-018 verdict and `headRefOid`** — it
never evaluates required checks and never re-paginates review threads — so RQ-6
has no executable enforcement path in the live file.

**This extract lives HERE, in the agent file's own unit — not in the
instructions file.** Per round-9 finding F-08, the canonical `RESOLUTION_PREFIX`
text U2 installs names segments by **role** and carries no item numbers; the
item-to-segment binding belongs only where the items are.

**Acceptance criteria**

1. Step 5 invokes **`RESOLUTION_PREFIX`** *by name*, referencing
   `github-pr-automation.instructions.md` as its canonical definition, and does
   **not** restate any of its steps, orderings or rationale. A reader must follow
   the reference to learn the sequence.
2. The resulting Step 5 order matches the canonical segments exactly, and the
   task card records the **old-to-new item mapping** using the verbatim extract
   above so the reorder is auditable rather than implicit:
   **S1** = items 7 (first) and 7a, run to completion first, permitted to commit
   and push; **S2** = item 7 (second), item 8, item 9, item 10 (the ordinary
   push); **S3** = the checkpoint-enumeration proof; **S3.5** = the conditional
   branch-protection proof; **S4** = the conditional
   locator/resolution/push segment; **S5** = the re-run local review, the final
   locator resolution state, the PR-body `Reviewed HEAD` write, then item **7b
   (MOVED here)**, the explicit required-check evaluation, and item **7c (MOVED
   here)**; **S6** = item 14 with a recorded `approved_head`; **S7** = item 15,
   **amended**; **S8** = the seven-part merge bar, after which items 16 and 17
   apply.
3. **The checkpoint count selects segments S3.5 and S4 only.** The criterion
   states that a unit whose enumeration proves **zero** checkpoints omits
   **exactly five things** — the branch-protection proof, locator publication,
   checkpoint resolution, the resolution commit/push, and the phase-2 locator
   publication — and **runs every other segment identically**, including the
   whole reordered common finalization tail. The criterion MUST NOT claim that
   any unit runs "the pre-existing path unchanged"; that claim is false and is
   expressly prohibited here. It further states that a failed, malformed,
   quarantined, or ambiguous enumeration is **not zero** and **halts**, that a
   registry with no checkpoint operations is **not zero** and **halts**, that a
   checkpoint appearing after S3 forces re-evaluation from S3 before merge, that
   **no empty locator is ever published**, and that Stage's crash-resumption
   startup recovery is a separate protocol this does not alter. The circular
   "PR merge-ready" wording is **not** used.
4. **The `S3_ENUMERATION_ALGORITHM` is installed as an executable contract**, not
   a prose instruction: the exact `backlogit_list_checkpoints` invocation with
   `consumer_id` **only**; the **no `status`/`agent` API-filter** rule with its
   quarantine rationale; **anomaly inspection before partition**; the
   current-unit identity predicate (validated `CheckpointV1` **and**
   `agent == "ship"` **and** `status == "active"` **and**
   `context.shipment_id == current shipment_id`); and a **per-record
   successful-handling proof** before each `backlogit_resolve_checkpoint`. Bulk
   resolution and cross-unit resolution are **explicitly prohibited**. *(Round-9
   finding F-10.)*
5. **Segment S3.5 is invoked by name before S4**, and it is stated that the
   effective-rules probe runs against the **live `headRefName` read from the PR
   API** — never the ambient checked-out branch — and that API unavailability, an
   uncovered branch, ambiguous rules, or any bypass capability **halts before
   obligation publication**. *(RQ-13.)*
6. **No branch-mutating step remains after S4.** Every mutating item in the
   verbatim extract — item 2, item 7 (first), 7a, item 7 (second), 8, 9, 10 —
   sits in S1 or S2, before the enumeration proof. Nothing between S5 and merge
   mutates the branch.
7. Item **15 is amended**, not retained unmodified. The amended item re-fetches
   and re-evaluates, unconditionally and fail-closed, **all** of: `headRefOid`;
   the PR body; `reviewDecision`; review requests and reviews; **every
   review-thread page to exhaustion**; required checks; the **ancestry of every
   recorded resolution commit**; and a **re-enumeration of active checkpoints
   owned by this unit**. A nonzero or incomplete re-enumeration **returns the
   unit to S3**, voids the S6 approval, and requires a fresh `approved_head`;
   re-entry is bounded at two attempts, after which the session halts. If the
   unit published under S3.5, item 15 also **re-proves the ruleset still
   applies**, and ruleset drift halts (OB-8). *(Round-9 findings F-06 and F-08.)*
8. The merge step permits merge **only** when all seven merge-bar conditions
   hold, with the locator terms **explicitly conditional**: the four
   always-present values (live `headRefOid`, local HEAD, PR-body `Reviewed HEAD`,
   `approved_head`) must agree, and `locator final_head` joins them **only when a
   locator was published**; every recorded resolution commit **if any were
   recorded** is an ancestor of that HEAD, vacuously satisfied when the set is
   empty; review-thread pagination completed to exhaustion; no blocking thread or
   review; required checks pass or are **proven** non-applicable; P-018 passes;
   and the S7 re-enumeration returned a complete zero. It is stated that a
   zero-checkpoint unit has **no** locator term rather than an empty one, so the
   bar is satisfiable for it. *(Round-9 findings F-04 and F-05.)*
9. **The merge call pins the observed HEAD.** The S8-observed `headRefOid` is
   passed to the merge API as the expected head SHA so GitHub refuses
   server-side if the branch advanced between the S8 read and the call.
   *(Round-9 finding F-18.)*
10. **P-009 is verified by API, not only by the rendered UI.** Merge-commit
    capability and mergeability are checked through the API, the merge is invoked
    explicitly in merge-commit mode, and the resulting commit is asserted to have
    **two parents** afterwards. The existing rendered-UI confirmation is retained
    beside this, not replaced. *(Round-9 finding F-30.)*
11. The refresh rules are stated: any HEAD change voids the approval and requires
    a fresh push/review/metadata/gates/approval cycle, and a thread or check
    change without a HEAD change requires the affected gates plus a refreshed
    approval. **No stale approval may be reused.**
12. S4's resolution commit also writes the `RESOLUTION_OBLIGATION_RECORD` at
    `OPEN` **in the same commit**, in the **implementation worktree** on the PR
    head, and that commit is pushed before the S5 review re-run.
13. The section cites P-022.
14. markdownlint passes.

**Posture**: documentation-first. **Size**: M. **Complexity**: high.
*(Still M/high after the split, and honestly so: this unit carries the verbatim
extract, the segment mapping, two moved items, one amended item and the merge
bar. It is a single-file, single-section documentation change and stays inside
two hours because the extract removes the re-derivation work every prior round
repeated, and because Step 6 and session end have moved out to U12.)*

### U12 — Rewire Ship Step 6 closure and Session end

**Depends on**: U4 (same file, strictly sequential), U9, U10.
**File**: `.github/agents/_ship.agent.md` — **Step 6** and **Session end item 2**.

*Added in revision 10 as the post-merge half of the U4 split (round-9 finding
F-09).*

**Acceptance criteria**

1. Session end item 2 no longer resolves checkpoints after merge. No
   checkpoint-resolution step remains anywhere after merge in this file.
2. Session end item 2's directive to *"leave at most one final best-effort
   checkpoint"* is **retired for units inside `RESOLUTION_PREFIX`**, with the
   recursion reason stated: such a checkpoint could only be resolved by a further
   commit needing a further PR. The window is covered by `CLOSURE_LOCATOR`, the
   `RESOLUTION_OBLIGATION_RECORD` and `LAST_MILE_RECOVERY` instead.
3. Step 6 gains **`RESOLUTION_POSTCONDITION`** by name, installed as **two
   explicitly named mutations**: **POST-A**, the PR-body `RECONCILED` write,
   which is metadata and **creates no commit**; and **POST-B**, the
   `OPEN` → `CLOSED` transition, which **is a commit**, is made on the
   `post-merge/{feature_slug}` closure branch Step 6.0 already creates, and whose
   PR must itself be **merged** before the obligation is discharged. The
   withdrawn "commits nothing" characterization of the postcondition as a whole
   **does not appear**. *(Round-9 finding F-01.)*
4. **Both mutations are conditional.** POST-A is a no-op when no locator was
   published; POST-B is a no-op when `resolution_obligation` is `none`, which is
   **left intact** in the closure commit. `none` → `CLOSED` is stated to be
   forbidden. *(Round-9 finding F-05.)*
5. Both are stated to run **only after the full P-001 post-merge closure set is
   complete and verified, including the P-020 compaction record**, and it is
   stated that setting either at merge or after partial closure would lose the
   obligation, and that deleting the record is never a discharge.
6. **P-020 ordering is explicit**: POST-B's `CLOSED` write completes **before**
   any compaction or archival that can move the closure artifact, and the
   path-stability invariant is stated — an artifact that has ever carried
   `resolution_obligation` must not be renamed, compacted, or archived while its
   status is `none` or `OPEN`. *(Round-9 finding F-07.)*
7. Step 6.0's post-merge branch rule is unchanged for every other closure
   artifact, and **Step 6's P-020 `compact-context` invocation is present and
   unmodified**.
8. It is stated that Ship MUST NOT commit POST-B to the default branch (P-010),
   and that the closure branch is the one Step 6.0 already creates rather than a
   new parallel branch (P-016).
9. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U5 — Ship orphan detection, startup discovery, and locator reconciliation

**Depends on**: U11, U12, U13.
**File**: `.github/agents/_ship.agent.md` — Merge Confirmation Gate and the
`ZERO-CANDIDATE NORMAL STARTUP` block.

Two assertions plus a startup entry point. The durable inputs are the
`CLOSURE_LOCATOR` in the PR body **and** the Git-tracked
`RESOLUTION_OBLIGATION_RECORD` — the second of which is reachable without the
body.

**Acceptance criteria**

1. The locator status is classified **before** the live PR state, per Step 1a, and
   the live PR state is classified **before** any ancestry assertion, per
   Step 1b.
2. The exact commands are given:
   `gh pr view <n> --json state,mergedAt,headRefOid,headRefName`,
   `git fetch origin refs/pull/<n>/head:refs/autoharness/scan/pr-<n>`, and
   `git merge-base --is-ancestor <sha> <target>` with an explicit commit-ish
   target — never a bare reused `FETCH_HEAD`.
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
6. Ship's `ZERO-CANDIDATE NORMAL STARTUP` item 4 runs **both** discovery
   channels — the `CLOSURE_LOCATOR` read protocol and the
   `RESOLUTION_OBLIGATION_RECORD` Git-history scan — **before** continuing to
   normal shipment validation, with the same halt-on-incomplete-enumeration rule
   for each, and halts on a Channel-A/Channel-B disagreement (OB-6). Ship is
   directly invokable, so relying on the Orchestrator's route alone would leave
   direct-Ship startup uncovered. It is stated that **Channel B runs even when
   Channel A returns nothing**, because a deleted PR body is the case Channel B
   exists to cover.
7. `RESOLUTION_POSTCONDITION` is invoked **by name** for the discharge, with its
   POST-A / POST-B split owned by U12 and **not restated here**. It is stated
   that discharge requires the full P-001 post-merge closure set including the
   P-020 compaction record, that deleting the record is never a discharge, and
   that POST-B is a **no-op** when the record is `none`.
8. No-op when no locator was published **and** no obligation record is `OPEN`;
   every recorded resolution commit is asserted when several were recorded.
9. **Before any committing re-entry**, the `Working-tree placement — committing
   re-entry only` procedure is executed **by that exact name**, referencing
   `github-pr-automation.instructions.md` as its canonical definition. Its steps
   are **not restated here** — the canonical eight-step text (WP-0…WP-7) is
   installed once by U11, and duplicating it across files is the drift vector
   round-9 finding F-08 identifies. This criterion requires only that the
   invocation is by exact name, that the procedure is stated to be required
   before any committing re-entry and **not** required for read-only ancestry
   assertions that fetch without switching, and that any failure inside it
   **fails closed** with no `git reset`, no force checkout, no resolution and no
   commit. *(Round-9 finding F-23.)*
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U6 — Orchestrator locator reconciliation route

**Depends on**: U11, U5, U13.
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

1. Step 0.0b runs **both** discovery channels — the `CLOSURE_LOCATOR` read
   protocol and the `RESOLUTION_OBLIGATION_RECORD` Git-history scan — whenever
   there is **no `ship`-owned active checkpoint**, regardless of whether
   `stage`-owned checkpoints exist, and always **before** `Continue directly to
   Step 0 State Assessment` — therefore before any queue selection.
2. When a `ship`-owned active checkpoint **does** exist, the pre-existing
   owner-routing path is unchanged and discovery is skipped, because that
   checkpoint already carries the obligation.
3. A discovered non-`RECONCILED` locator **or** an `OPEN` obligation record is
   routed to **Ship** for `LAST_MILE_RECOVERY` **before** any Stage routing and
   before queue selection. If a `stage`-owned checkpoint is also present, both
   are presented to the operator and no new queue work is auto-selected.
4. Discovery is not filtered by shipment status, so an archived shipment with an
   outstanding non-`RECONCILED` locator or `OPEN` record is found (the 139-S
   shape).
5. Incomplete enumeration in **either** channel halts rather than falling
   through to state assessment, and a Channel-A/Channel-B disagreement halts
   (OB-6). A missing locator never discharges an `OPEN` record.
6. Non-`RECONCILED` locators for more than one distinct shipment halt as a P-001
   violation.
7. Routing into `LAST_MILE_RECOVERY` conveys **no merge authority** and no
   implicit approval; the four-part merge bar applies unchanged and any doubt
   halts. In dark-factory mode (P-017), a dark approval for the *current* declared
   scope does **not** satisfy the merge bar for a *prior* unit's recovered
   obligation.
8. Owner exclusivity is preserved: the Orchestrator **routes**; Ship **performs**
   the reconciliation. The Orchestrator never performs recovery itself.
9. When **neither** channel finds anything, the pre-existing fall-through is
   unchanged: the same `Continue directly to Step 0 State Assessment`
   instruction, the same non-failure classification, and no new operator
   interaction.
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

**Why the predicate must not read the ambient branch (revision 10, round-9
finding F-11).** Revision 9's predicate began *"determine the current branch"* and
then queried `gh pr list --head <that branch>`. That is wrong at both of Stage's
resolve sites and produced a false "not merged" in the common case. At Session
end, Stage may legitimately be on the default branch or on an admin branch while
the checkpoint it is resolving was carried by a different, already-merged staging
PR — the query returns nothing, the guard passes, and Stage resolves into a
merged branch, which is precisely the P-022 violation this unit exists to
prevent. At the `OWNER-SCOPED RESOLUTION` site the ambient branch is even less
related: crash resumption routinely runs from a fresh checkout whose branch has
no connection to the checkpoint being restored. Worse, the ambient branch is
**attacker- and accident-controllable** — any checkout of an unmerged branch name
silently re-enables resolution. The predicate must therefore key off the
**checkpoint's own recorded carrying PR**, which is immutable state bound to the
checkpoint, not off working-tree position.

**Acceptance criteria**

1. **Both** resolve sites — Session end item 2 and `OWNER-SCOPED RESOLUTION` —
   state that a checkpoint MUST NOT be resolved once the staging PR carrying its
   resolution has merged, citing P-022.
2. **The merged-carrier predicate is keyed from the checkpoint, never from the
   ambient working tree.** The installed text states, in this order:
   **(a)** The carrying PR is read from the **selected checkpoint's own validated
   `CheckpointV1` context** — `context.pr` for the PR number and `context.branch`
   for its head ref. These two fields are declared **mandatory** for any
   Stage-created Git-tracked checkpoint, and Stage's checkpoint-creation step is
   amended to populate them.
   **(b)** The state query is `gh pr view <context.pr> --json
   number,state,mergedAt,headRefName,baseRefName`. `gh pr list --head <branch>`
   is **explicitly prohibited** at both sites, and reading the current branch —
   `git branch --show-current`, `git rev-parse --abbrev-ref HEAD`, or any
   equivalent — is **explicitly prohibited as an input to this predicate**.
   **(c)** Resolution proceeds **only** when the returned `state` is exactly
   `OPEN`. `MERGED` is the P-022 prohibition. `CLOSED` (unmerged) **also halts**,
   because a closed-unmerged carrier can never carry the resolution to `main`.
   **(d)** Every other outcome **halts to the operator**: the checkpoint carries
   no `context.pr` or no `context.branch`; the fields are present but the PR does
   not exist or is not in this repository; `headRefName` does not equal the
   recorded `context.branch` (the carrier was retargeted or the record is stale);
   a non-zero exit; or an unparseable response. It is stated explicitly that a
   lookup failure is **never** read as "not merged".
   It is stated that the ambient branch may be used for **logging context only**
   and never as a predicate input.
3. The correct action in the merged case is stated: leave the checkpoint active,
   surface it with its recorded `context.pr`, and hand off to the operator —
   never resolve into a merged branch.
4. The *"leave at most one final best-effort checkpoint"* directive is qualified:
   a Git-tracked checkpoint MUST NOT be created when no open staging PR can carry
   its eventual resolution, because Stage cannot open a PR (P-010) and so could
   never discharge it. The check for "an open staging PR exists to carry it" uses
   the **same** `gh pr view <pr> --json state` form against the PR Stage intends
   to record in `context.pr`, not a head-ref search. Hand off to the operator
   instead when none exists.
5. It is stated that Stage does **not** execute `RESOLUTION_PREFIX` and publishes
   no locator, because Stage holds no merge authority (P-010); the reference is to
   P-022's prohibition only.
6. No other Stage behaviour is modified; the existing checkpoint payload contract
   is extended **only** by the two mandatory `context` fields in AC2(a), and
   normal (open-carrier) resolution semantics are unchanged.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U9 — Record the durable resolution-obligation record schema and lifecycle

**Depends on**: U3 (extends the same subsection; same file, strictly sequential).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

This unit is the first half of revision 9's answer to **RR-3**, re-scoped in
revision 10. It installs the canonical `RESOLUTION_OBLIGATION_RECORD`
**schema, siting rationale and lifecycle** — the Git-tracked,
history-immutable second channel that makes RQ-7 true when the mutable PR body is
deleted. It does **not** weaken RQ-7, and it does **not** touch RQ-8: the record
carries no SHA. **U13** owns the Channel-B discovery protocol.

*Split in revision 10 (round-9 finding F-09): U9 previously carried the schema,
the siting argument, a lifecycle table, a two-channel discovery protocol, five
exact command forms, a six-rule provenance variant and seven fail-closed rules
across eleven criteria.*

**Acceptance criteria**

1. The `RESOLUTION_OBLIGATION_RECORD` schema block appears verbatim as given in
   this plan's canonical definition, with its canonical **path** (the unit's
   pre-merge operational-closure artifact under `docs/closure/`) and canonical
   **identity** (`shipment_id`, `pr`; exactly one `OPEN` record per shipment).
   The identity field is named **`shipment_id`** — matching the closure
   artifact's own field name and the S3 enumeration predicate — and the bare
   `shipment` spelling does not appear. *(PR #396 Copilot thread
   `PRRT_kwDORJEduc6h79cv`.)*
2. It is stated that the record **carries no SHA** and why: it is written inside
   the resolution commit, so any SHA field would be self-referential — RQ-8 is
   preserved, not traded.
3. The **four-transition** lifecycle table appears: absent → `none` (S2, the
   `operational-closure` artifact write); `none` → `OPEN` (S4, **in the same
   commit as the checkpoint resolutions**, pushed before the S5 review re-run);
   `OPEN` → `CLOSED` (**POST-B**, a commit on the `post-merge/{feature_slug}`
   branch, after the full verified P-001 closure set, and that closure PR must
   itself be merged); and `none` → `none` (**POST-B is a no-op** for a
   zero-checkpoint unit; the record is left intact at `none`). It is stated that
   `OPEN` → `CLOSED` is the **only** discharge, that `none` → `CLOSED` is
   **forbidden**, and that **deletion is never a discharge**. *(Round-9 findings
   F-01, F-05, F-34.)*
4. The **status value set is exactly `none | OPEN | CLOSED`**, and no fourth
   value, no boolean form, and no absent-means-`none` shorthand appears anywhere
   in the installed text.
5. It is stated that `docs/closure/` is chosen because it is an **existing owned
   repo-local state surface** — created by the `operational-closure` skill,
   already written at real Step 5 item 8, and already carrying the same
   placeholder→finalize field convention as `compaction_status` — and that a
   new standalone tracker file, a checkpoint-store record, and a commit-message
   record were each rejected, with the stated reason for each.
6. The **path-stability invariant** is installed with all three of its rules: an
   artifact that has ever carried `resolution_obligation` MUST NOT be renamed,
   moved, compacted or archived while its status is `none` or `OPEN`; P-020
   compaction MUST exclude such artifacts and MUST run only after POST-B has
   written `CLOSED`; and any tooling that relocates a closure artifact MUST
   record the move so `--follow` can reconstruct the chain. *(Round-9 finding
   F-07.)*
7. The **`Recovering resolution commits without a SHA field`** procedure appears
   as an executable named procedure, and is stated to be the source of
   `resolution_commits` whenever the locator is unavailable: fetch the PR head to
   a unique retained ref; run the resolution-state classification at that head;
   then bound the search with
   `git log --format=%H -S'"resolution_obligation": "OPEN"' <base>..<head> --
   <artifact path>` and take the introducing commit. It is stated that the
   classification returns per-file states and does **not** itself yield SHAs.
   *(PR #396 Copilot thread `PRRT_kwDORJEduc6h79dC`.)*
8. It is stated explicitly that this mechanism introduces **no** executable
   persistence substrate, **no** lock or compare-and-swap, **no** cross-run
   cursor persistence, and **no** part of Defect 1, and that it confers **no**
   merge authority — its only authority is to halt (RQ-10 untouched).
9. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U13 — Record the Channel-B history scan and reconciliation

**Depends on**: U0, U9 (extends the same subsection; same file, strictly
sequential).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

*Added in revision 10 as the discovery half of the U9 split (round-9 finding
F-09), and rewritten to be independent of the PR body per the RR-3 decision.*

Install the Channel-B protocol: the body-independent, history-immutable discovery
path that finds an `OPEN` obligation record even when the PR body carrying the
locator has been emptied.

**Acceptance criteria**

1. **Two-channel discovery** is specified: Channel A (the PR-body
   `CLOSURE_LOCATOR`) and Channel B (the Git-history scan). Both run, the union
   is taken, and **neither may be skipped because the other returned nothing**.
2. **Channel B's candidate derivation is stated to be independent of PR-body
   content.** Its candidate set is every **trusted open or closed-unmerged** PR
   from the single exhaustive paginated enumeration, admitted by PV-1, PV-2,
   PV-3 and PV-8 — **never** filtered by marker presence, locator presence, or
   any body text. The installed text states the reason in one sentence: the body
   is the surface under suspicion, so a body-derived candidate list cannot detect
   a deleted body. *(Round-9 finding F-13; RR-3 decision part 3.)*
3. The **B0…B7 steps** appear in order with their exact commands:
   **B0** — a **shallow/partial-clone guard**: `git rev-parse
   --is-shallow-repository` must be `false` and the repository must not be a
   blobless/treeless partial clone; otherwise **halt (OB-2)**, because a shallow
   history cannot distinguish "never introduced" from "truncated away".
   *(Round-9 finding F-29.)*
   **B1** — scan merged history: `git ls-tree -r --name-only origin/main --
   docs/closure/`, then `git show origin/main:<path>` for each hit to read the
   status field. It is stated that `ls-tree` lists names and **cannot** yield
   file content, so the read step is mandatory and not optional. *(PR #396
   Copilot thread `PRRT_kwDORJEduc6h79dm`.)*
   **B2** — for each trusted candidate PR, prove its live head is protected via
   the `BRANCH_PROTECTION_PREREQUISITE` effective-rules probe against that PR's
   **live `headRefName`**; an uncovered branch, ambiguous rules, or any bypass
   capability **halts (OB-8)**.
   **B3** — fetch that head to a **unique retained ref**:
   `git fetch origin refs/pull/<n>/head:refs/autoharness/scan/pr-<n>`. It is
   stated that a bare `git fetch origin refs/pull/<n>/head` followed by
   `FETCH_HEAD` is **prohibited**, because `FETCH_HEAD` is global and is
   overwritten by the next candidate's fetch, silently rebinding every subsequent
   assertion to the wrong PR. *(Round-9 finding F-14; RR-3 decision part 3.)*
   **B4** — enumerate closure artifacts at that retained ref
   (`git ls-tree -r --name-only refs/autoharness/scan/pr-<n> -- docs/closure/`)
   and read each with `git show refs/autoharness/scan/pr-<n>:<path>`.
   **B5** — scan the **full reachable history** from that retained ref for
   introduction, transition and deletion of the record by canonical path and
   record identity, with **explicit revision roots on every command**:
   `git log --follow --diff-filter=D --format=%H refs/autoharness/scan/pr-<n> --
   <artifact path>` and
   `git log --format=%H -S'resolution_obligation' refs/autoharness/scan/pr-<n>
   -- docs/closure/`. It is stated that omitting the revision argument makes
   `git log` default to `HEAD`, scanning the ambient working branch instead of
   the candidate — the same ambient-state defect as F-11. *(PR #396 Copilot
   thread `PRRT_kwDORJEduc6h79d5`.)*
   **B6** — **bind each observed transition to record identity and commit
   ancestry**: the record's `shipment_id` and `pr` must match the candidate, and
   the introducing commit must be an ancestor of the retained ref
   (`git merge-base --is-ancestor <sha> refs/autoharness/scan/pr-<n>`).
   Unverifiable ancestry **halts (OB-2)**.
   **B7** — **clean up** every `refs/autoharness/scan/*` ref created by the scan,
   including on the halt paths, so no scan state is left on disk.
4. It is stated that **ordinary later commits that delete the record remain
   detectable**, and why: the `deletion` and `non_fast_forward` rules proven at
   B2 keep the branch present and the introducing commit reachable, so B5's
   deletion probe sees the removal instead of an indistinguishable absence. It is
   stated that this guarantee holds **only** for branches B2 proved protected,
   and that an unprotected branch halts rather than being scanned optimistically.
5. Channel B's provenance is established **only** from API response fields and
   Git ancestry — PV-1, PV-2, PV-3 and PV-8, plus **PV-B4** (the record's `pr`
   field equals the PR number the record was read from) and **PV-B6** (the
   record's `branch` field equals that PR's **live `headRefName`** read from the
   API) — and **never** from PR-body text. **PV-B4 and PV-B6 are distinct checks
   and are listed separately**: PV-B4 binds the record to the PR, PV-B6 binds the
   PR to its live branch, and collapsing them into one rule loses the retarget
   case where the record's `pr` is correct but its `branch` is stale. PV-6 and
   PV-7 are unchanged. PV-4 and PV-5 are stated to be **inapplicable** to
   Channel B, with the reason given: both read locator fields, and Channel B has
   no locator. *(Round-9 finding F-15.)*
6. **State-aware routing** is specified for what Channel B finds: an `OPEN`
   record on an **open** PR routes to `LAST_MILE_RECOVERY`; an `OPEN` record on a
   **closed-unmerged** PR **halts** as an unrecoverable orphan; an `OPEN` record
   whose PR has since **merged** is the 139-S shape and **halts**; a `CLOSED`
   record is discharged and is **not** re-routed; a `none` record on an open PR
   is a zero-checkpoint unit and is **not** an obligation. *(Round-9 finding
   F-15; PR #396 Copilot thread `PRRT_kwDORJEduc6h79dW`.)*
7. **Deletion is detected via commit history, never inferred from absence.** A
   record which ever appeared in history and is now absent from the tree
   **without** a recorded `OPEN` → `CLOSED` transition is a deletion that
   **halts (OB-1)**, and absence from the current tree is never evidence that no
   obligation exists.
8. All **eight** fail-closed rules appear with their halt actions: **OB-1**
   deletion; **OB-2** force-push, rebase, shallow/partial history, or otherwise
   unverifiable ancestry; **OB-3** conflicting `OPEN` records for one shipment
   (byte-identical duplicates collapse and are logged; **any** field difference
   halts; `opened_at` is barred as a tie-breaker); **OB-4** `OPEN` records for
   more than one shipment (P-001); **OB-5** unparseable or field-incomplete
   record; **OB-6** Channel A/Channel B disagreement, with the explicit rule that
   the mutable channel is **never** preferred and a missing locator **never**
   discharges an `OPEN` record; **OB-7** incomplete Channel-B enumeration,
   including any pagination gap, fetch failure, or branch that has disappeared;
   **OB-8** ruleset drift — a candidate head that was protected at B2 and is not
   at re-check, or any bypass capability appearing.
9. It is stated that Channel B confers **no** merge authority and introduces no
   lock, no CAS, and no cross-run continuation state; its only authority is to
   halt or to route.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U10 — Declare the obligation field in the closure-artifact schema

**Depends on**: U9.
**File**: `.github/skills/operational-closure/SKILL.md` — the pre-merge closure
artifact's field list only.

**Why this unit exists, and why it is a scope addition made openly.** The
`operational-closure` skill **owns** the `docs/closure/` artifact schema. Adding
a field that the skill does not declare would leave the record an unowned
squatter on someone else's artifact — precisely the "ad-hoc tracker" the RR-3
correction is required to avoid — and would drift the moment the skill's field
list changed. One field declaration keeps the surface owned. This adds
`.github/skills/operational-closure/SKILL.md` to the plan's in-scope file set;
the decision's scope boundary is updated to match rather than the addition being
made silently.

**Acceptance criteria**

1. The pre-merge closure artifact's field list gains `resolution_obligation`,
   documented in the same style as the existing `compaction_status` and
   source-artifact-cleanup fields.
2. The skill is stated to **initialize** the field to `none` when it creates the
   pre-merge artifact, exactly as it initializes `compaction_status` to
   `pending`. **Initialization is create-only**: the criterion states that the
   skill MUST NOT write `none` over an existing `OPEN` or `CLOSED` value on a
   re-run or regeneration, and that doing so would silently discharge an
   outstanding obligation. A regeneration encountering an existing
   `resolution_obligation` **preserves it verbatim**. *(Round-9 finding F-25.)*
3. The allowed transitions are stated as `none` → `OPEN` (written by Ship in the
   S4 resolution commit) and `OPEN` → `CLOSED` (written by Ship's POST-B
   post-merge closure commit). `none` → `CLOSED` is **forbidden**, `none` →
   `none` is the legitimate zero-checkpoint terminal state, and no other
   transition is permitted. The skill does **not** itself write `OPEN` or
   `CLOSED`.
4. **The P-020 compaction exclusion is declared here as well as in the record's
   canonical definition**, because this skill owns the artifact: a closure
   artifact whose `resolution_obligation` is `none` or `OPEN` MUST NOT be
   compacted, renamed, moved, or archived, and any relocation must be recorded so
   `git log --follow` can reconstruct the chain. *(Round-9 findings F-07, F-25.)*
5. The field's canonical definition is **referenced by name** in
   `github-pr-automation.instructions.md`; the skill does **not** restate the
   schema, the discovery protocol, or the fail-closed rules.
6. No other skill behaviour is modified — `compaction_status` handling, the
   source-artifact-cleanup placeholder, and the releasability verdict set are
   untouched.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

### U7 — Capture the compound learning *(closure deliverable)*

**Depends on**: U5, U6.
**File**: `docs/compound/workflow-issues/` — one new learning.

U7 is a **closure deliverable**, not a requirement-realizing unit. It implements
none of RQ-1 … RQ-13 and is deliberately absent from the requirement trace table
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
5. The **sole-mutable-record trap** is recorded: resolving every checkpoint
   pre-merge makes the PR body the only obligation record unless a Git-tracked
   record rides the resolution commit, and absence of a record must be
   distinguished from deletion of one via commit history.
6. Frontmatter matches the prevailing convention of
   `docs/compound/workflow-issues/` and is internally consistent.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

## Requirement → unit traceability

Every requirement has exactly **one owning unit** — the unit that installs the
normative text — plus zero or more **enforcing units** that wire that text into
an execution path. U7 is a closure deliverable and realizes no requirement; it is
therefore absent by design.

| Requirement | Owning unit | Enforcing units |
|---|---|---|
| RQ-1 no post-merge resolution | U1 (policy) | U4 (Ship), U12 (Ship session end), U8 (Stage) |
| RQ-2 resolution rides the same merge | U2 (`RESOLUTION_PREFIX`) | U4 (Ship). **Not U8** — U8 enforces the RQ-1 prohibition on the Stage path; Stage's resolution reaches `main` through the ordinary staging-PR merge, which Stage does not control, so U8 cannot *guarantee* RQ-2 and is no longer credited with it. |
| RQ-3 push before evidence | U2 | U4 |
| RQ-4 re-run the actual review | U2 | U4 |
| RQ-5 PR-body record precedes §1.9 | U2 | U4 |
| RQ-6 approval after gate, live re-fetch before merge | U2 (`RESOLUTION_PREFIX` S5–S8) | U4 (Ship — moves items 7b/7c, **amends item 15** to re-enumerate and re-prove, installs the **seven-part** merge bar, the expected-head pin and the refresh rules) |
| RQ-7 locator discoverable through closure | U2 (`CLOSURE_LOCATOR`) | U5, **U9** (the durable Git channel's schema), **U13** (the body-independent discovery that makes RQ-7 hold when the PR body is deleted) |
| RQ-8 non-self-referential locator | U2 | U9 (the obligation record carries no SHA, by construction; the *Recovering resolution commits without a SHA field* procedure re-derives them) |
| RQ-9 exhaustive, trusted, status-independent discovery | U3 (read protocol) | U5, U6, U13 (Channel B) |
| RQ-10 no merge authority conferred | U11 (`LAST_MILE_RECOVERY` Step 2) | U5, U6, U13 |
| RQ-11 every failure halts | U3 | U5, U6, U11 (P-012 availability contract), U13 (OB-1 … OB-8) |
| RQ-12 durable Git-tracked obligation record | **U9** (`RESOLUTION_OBLIGATION_RECORD` schema and lifecycle) | U4 (writes it `OPEN` in the S4 resolution commit), U12 (POST-B closes it), U5 (discovers it), U6 (routes on it), U10 (schema ownership and create-only initialization), U13 (finds it without the PR body) |
| RQ-13 protected resolution-bearing branches | **U0** (the approval-gated ruleset itself) + U2 (`BRANCH_PROTECTION_PREREQUISITE` S3.5 text) | U4 (invokes S3.5 before S4 and re-proves at amended item 15), U13 (B2 per-candidate proof; OB-8 drift halt) |

## Dependencies

```text
U0 ──┬────────────────────────────────→ U4 ──→ U12 ──┐
     └──────────────────────→ U13 ──┐               │
                                    │               │
U2 ──┬──→ U1 ──┬──→ U4              ├──→ U5 ──→ U6 ──┴──→ U7
     │         └──→ U8              │    ↑         ↑
     ├──→ U3 ──┬──→ U11 ────────────┴────┘         │
     │         │      └───────────────────────────→┘
     │         └──→ U9 ──┬──→ U13
     │                   ├──→ U4
     │                   └──→ U10 ──→ U4
     └──→ U4
```

**Twenty-two edges**, listed explicitly because the diagram is a reading aid and
the list is the contract:

`U2→U1`, `U2→U3`, `U2→U4`, `U1→U4`, `U1→U8`, `U3→U11`, `U3→U9`, `U9→U13`,
`U9→U4`, `U9→U10`, `U10→U4`, `U0→U4`, `U0→U13`, `U4→U12`, `U11→U5`, `U12→U5`,
`U13→U5`, `U5→U6`, `U11→U6`, `U13→U6`, `U5→U7`, `U6→U7`.

The graph is **acyclic** and has **two roots**: **U0** and **U2**.

* **U0 is a root** because the ruleset is an external GitHub settings change with
  no repository-file prerequisite. It is an **ops prerequisite**: U4's S3.5
  invocation and U13's B2 per-candidate proof both describe a protection that
  must actually exist, so `U0→U4` and `U0→U13` are real edges rather than
  bookkeeping.
* **U2 is the other root.** Revision 7 listed U1 and U2 as co-roots, but U1
  installs a by-name reference to `RESOLUTION_PREFIX` whose definition only U2
  creates; running U1 first would leave a dangling reference that cannot be
  verified. **U2→U1** is therefore a real edge, and it transitively orders U8
  after the canonical definition too.
* **`U9→U4`** exists because U4's AC12 requires the S4 commit to write a record
  whose schema only U9 defines. **`U10→U4`** is new in revision 10: U4's S4
  transitions the field `none` → `OPEN`, and that transition is only well-defined
  once U10 has declared the field and its create-only initialization in the
  owning skill — writing `OPEN` into a field the skill does not know about is the
  unowned-squatter failure U10 exists to prevent. *(Round-9 finding F-25.)*

**File-serialization check.** Four units share
`github-pr-automation.instructions.md` (U2, U3, U11, U9, U13 — five) and are
strictly sequential by the chain `U2→U3→{U11, U9→U13}`; U11 and U13 are the only
pair in that file with no direct edge between them, so the harvest note records
that they must be sequenced in that order at execution time. Three units share
`_ship.agent.md` (U4, U12, U5) and are strictly sequential by `U4→U12→U5`. U8 is
the only unit touching `_stage.agent.md`, U6 the only unit touching
`_orchestrator.agent.md`, U1 the only unit touching `workflow-policies.md`, U10
the only unit touching `operational-closure/SKILL.md`, U7 the only unit touching
`docs/compound/`, and U0 touches **no repository file at all**.

## Verification

| # | Check | Expected result |
|---|---|---|
| V1 | `markdownlint` over every changed file | passes |
| V2 | **Canonical-copy check**, in two parts. **(a)** The `RESOLUTION_PREFIX` code block appears verbatim exactly once, in `github-pr-automation.instructions.md`. **(b)** *Sequence-restatement check (rewritten in revision 9; the revision-8 form was unsatisfiable — it banned the ordinary verbs `resolve`, `record` and `push`, which U1's policy Statement and U8's Stage prohibition are **required** to use).* In each referencing file's changed region — `workflow-policies.md`, `_ship.agent.md`, `_stage.agent.md`, `_orchestrator.agent.md`, `operational-closure/SKILL.md` — assert that **no ordered enumeration of three or more canonical segment steps appears**: that is, no list or arrow-chain reproducing three or more of the S1…S8 segment contents in their canonical order. Ordinary prose use of any individual verb is **permitted and expected**. | (a) exactly one verbatim copy; (b) no referencing file contains a three-or-more-step ordered restatement; each contains the bare name `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION` / `RESOLUTION_OBLIGATION_RECORD` plus a file reference |
| V3 | Grep every changed file for `two-phase` and for `pr_role` | zero occurrences of each; only "three-phase" appears |
| V4 | **Ordered-step inspection** of `_ship.agent.md` (not a grep), recording the item numbers against U4's verbatim extract: read Step 5 top-to-bottom and confirm (a) items 7 (first) and 7a complete first; (b) item 7 (second), 8, 9 and 10 (the push) all precede the checkpoint-enumeration proof; (c) items **7b and 7c now appear after that push**, together with the re-run review, the PR-body `Reviewed HEAD` write and an explicit required-check evaluation; (d) item 14 records `approved_head`; (e) item 15 is **amended** to re-fetch headRefOid, PR body, reviewDecision, review requests/reviews, every review-thread page, required checks, resolution-commit ancestry **and a re-enumeration of active checkpoints**; (f) item 16 (P-009) retains its rendered-UI confirmation and **gains** the API-side merge-commit mode check and the two-parent assertion; (g) **the only branch-mutating step after the enumeration proof is S4's own conditional resolution commit, and no branch-mutating item appears after S4**; then read Step 6 and Session end and confirm neither contains a resolution or checkpoint-creation step for the merged unit, and that Step 6 carries POST-A and POST-B | (a)–(f) each hold as stated; (g) the post-S4 mutation set is empty and the only post-proof mutation is S4's own commit; no post-merge resolution or creation; POST-A and POST-B both present |
| V5 | Backlog structure equals **one top-level release unit → 3 sub-epics → 14 tasks**, and the shipment manifest membership equals those **18** IDs. *(IDs are allocated at harvest; the abandoned `143.*` IDs must not be reused.)* | exact **membership** match; order is not asserted, because Ship treats manifest order as non-executable and sorts by unfinished dependencies |
| V6 | Every task card's acceptance criteria are textually identical to its unit's criteria in this plan | identical |
| V7 | Grep the **changed files** (not this plan) for this closed list of Defect-1 construct names: `DARK_CONTINUATION_PREDICATE`, `ACTIVATION_RECORD_STORE`, `CURSOR_TYPING_RULES`, `CONTINUATION_HANDOFF_EVIDENCE`, `OWNER_SIDE_REVALIDATION`, `PREDICATE_PRECEDENCE`, `MIS_EVALUATION_DIRECTIONALITY`, `SCOPE_MATCH_RULES`, `check-continuation-predicate-drift`, `canonical-phrases.json`, `parity-gate` | zero occurrences of all eleven |
| V8 | Execute the U3 discovery command against this repository with `per_page=2`, count returned records; compare against the **paginated** reference `gh api --paginate -H "Accept: application/vnd.github+json" "repos/{owner}/{repo}/pulls?state=all&per_page=100" --jq '.[].number' \| Measure-Object -Line`. *(Revision 9 fix: the revision-8 reference command omitted `--paginate`, so it was itself capped at one page and the comparison was invalid — it could only ever prove the two commands disagreed.)* | both counts are equal **and** exceed 2, proving `Link`-header pagination actually followed; every record carries a non-empty `body` field; exit code 0 on both |
| V9 | Confirm Step 6's P-020 `compact-context` invocation in `_ship.agent.md` is byte-identical to its pre-change text | identical |
| V10 | Read `_stage.agent.md` Session end item 2 **and** the `OWNER-SCOPED RESOLUTION` block; confirm both carry the P-022 merged-PR prohibition and the **checkpoint-keyed** predicate `gh pr view <context.pr> --json number,state,mergedAt,headRefName,baseRefName`; confirm the best-effort-checkpoint directive is qualified; and confirm the **negative** cases: grep both changed regions for `gh pr list --head`, `git branch --show-current` and `rev-parse --abbrev-ref` | both sites carry the prohibition and the checkpoint-keyed predicate; directive qualified; **zero occurrences** of all three prohibited ambient-branch forms in the changed regions |
| V11 | Read `_orchestrator.agent.md` Step 0.0b; confirm discovery is keyed on the absence of a **ship-owned** active checkpoint, not on global zero-candidate, and that it runs **both** channels | keyed on ship-owned absence; both channels present |
| V12 | **Working-tree placement coverage** *(the verification half of D14)*. Read **U11's** criteria and confirm the procedure named **exactly** `Working-tree placement — committing re-entry only` is defined with WP-0…WP-7 and their halt actions; read **U5's** criteria and confirm it is **invoked by that exact name** before the only committing recovery branch **and does not restate the steps**. Then run the positive and fail-closed cases below. | the name matches exactly in both units; U5 restates no step; every case below behaves as stated |
| V12a | *Positive*: clean tree on `main`, valid locator, PR head fetchable. Run the placement | WP-0…WP-7 all pass; HEAD equals the fetched tip; current branch == locator `branch`; not `main`, not detached; no extra worktree created |
| V12b | *Fail-closed, dirty tree*: uncommitted change present | halts at WP-1; **no** stash, **no** discard, **no** switch, **no** commit |
| V12c | *Fail-closed, fetch failure*: unreachable `refs/pull/<pr>/head` | halts at WP-3; no switch, no resolution, no commit |
| V12d | *Fail-closed, divergence*: local branch of that name exists and is not a fast-forward of the fetched tip | halts at WP-4; **no** `git reset --hard`, **no** force checkout, **no** rebase |
| V12e | *Fail-closed, mismatch*: switch succeeds but HEAD differs from the fetched tip, or branch name differs, or HEAD is detached | halts at WP-5/WP-6/WP-7 respectively; no resolution, no commit |
| V12f | *Read-only path*: a Step 1b ancestry assertion | fetch performed; **no** switch performed; the placement procedure is **not** run |
| V12g | *Fail-closed, topology*: a second implementation worktree is already present | halts at WP-0 (P-011/P-016); no switch, no commit |
| V13 | **Zero-checkpoint case**: a unit whose enumeration proves zero. Trace Step 5 | **S3.5 and S4 both omitted in full** — no protection probe, no locator published, no resolution commit, no phase-2 update, **no empty locator**; S1, S2, S3, S5, S6, S7, S8 all execute; item 7b/7c run **after** the push; the S8 merge bar is **satisfiable** with its locator term absent rather than empty and its ancestry term vacuously true; POST-A and POST-B are both no-ops and the record is left at `none` |
| V14 | **Nonzero-checkpoint case**: a unit owning ≥1 checkpoint. Trace Step 5 | S3.5 proves protection **before** S4; the locator, the resolutions and the `OPEN` obligation record ride **one** commit that is pushed before the S5 review re-run; remote head proven equal to local head; POST-B later writes `CLOSED` on the closure branch and that closure PR merges |
| V15 | **Enumeration-failure case**: enumeration errors, or returns a malformed or quarantined record, or is ambiguous, or the registry exposes no checkpoint operations | **halts**; it is **not** classified as zero; no locator, no resolution, no merge |
| V16 | **Race case**: a checkpoint appears after S3 completed but before merge | the **amended item 15 re-enumeration** detects it; the unit returns to S3; the prior approval is void and a fresh S6 approval with a new `approved_head` is required; re-entry is bounded at two attempts and then halts |
| V17 | **Obligation-record durability without a PR body**: publish a record, then **empty the PR body entirely** so no locator and no marker remain. Run both discovery channels | Channel A finds nothing **and contributes no candidate**; **Channel B still finds the `OPEN` record**, because its candidate set is derived from the trusted-PR enumeration rather than from body content; OB-6 (A/B disagreement) halts; the missing locator does **not** discharge the record |
| V18 | **Obligation-record deletion, executed end to end**: on a protected candidate branch, delete the closure artifact from the tree in an ordinary later commit with no `OPEN` → `CLOSED` transition, then **run the full B0…B7 workflow** rather than inspecting the command strings | B0 passes (non-shallow); B2 proves protection; B3 creates `refs/autoharness/scan/pr-<n>`; B5's `git log --diff-filter=D` with an explicit revision root detects the deletion; B6 binds identity and ancestry; **OB-1 halts**; absence is **not** read as "no obligation"; B7 removes every scan ref |
| V19 | **Parse** the `RESOLUTION_OBLIGATION_RECORD` schema block and its lifecycle text as YAML (not a substring grep) and enumerate its keys | the key set contains no SHA-bearing field — specifically no `resolution_commits`, no `final_head`, and no key whose value is a commit-ish — so RQ-8 is preserved. *(Revision 10: a plain grep matched the surrounding explanatory prose that names these fields in order to forbid them, so it could never fail — round-9 finding F-24.)* |
| V20 | **Standing role-order drift check** *(the executable half of round-9 finding F-08)*. Extract the ordered S1…S8 **role descriptors** from the canonical `RESOLUTION_PREFIX` block in `github-pr-automation.instructions.md`, and independently extract the ordered segment mapping recorded in `_ship.agent.md`'s Step 5. Compare the two role sequences element-wise. The check reads **roles**, never item numbers, so the live file's duplicated item `7` cannot break it | the two role sequences are identical and identically ordered; any divergence fails the check and is reported as protocol drift. The check is run as part of V1's lint pass over changed files and is re-runnable standalone |
| V21 | **Ruleset drift and bypass.** With U0 applied, query `GET /repos/{owner}/{repo}/rules/branches/{branch}` for a live Ship head ref and assert `deletion` and `non_fast_forward` are both present, `enforcement` is `active`, and no bypass applies. Then simulate drift by evaluating the same probe against a branch the ruleset does **not** cover | covered branch: all four requirements hold and S4 may proceed. Uncovered branch: the probe returns no matching rules and the protocol **halts before obligation publication**; at re-check it halts as **OB-8**. A response indicating any bypass capability halts in both cases |
| V22 | **`FETCH_HEAD` reuse.** Grep the changed files for `FETCH_HEAD` and for `git fetch origin refs/pull/` without a destination refspec | zero occurrences of bare `FETCH_HEAD` as an assertion target and zero fetches without a `:refs/autoharness/scan/pr-<n>` destination; every ancestry and `ls-tree`/`show`/`log` command carries an explicit commit-ish |
| V23 | **P-020 ordering.** Trace a nonzero-checkpoint unit through Step 6 | POST-B writes `CLOSED` **before** the P-020 compaction runs; the compaction excludes any artifact whose `resolution_obligation` is `none` or `OPEN`; no artifact that ever carried the field is renamed or archived while unclosed; no mutation occurs after the S5 review that is not part of POST-A/POST-B; OB-1 is **not** tripped by compaction |
| V24 | **PV-8 decoy binding.** Open a PR from a trusted collaborator naming a real in-flight shipment but not recorded in the backlog as its implementation or closure PR. Run discovery | the candidate fails PV-8 and is **discarded silently**, not halted on; startup is not denied. A candidate that cannot be bound because the backlog records no implementation PR is **held for operator review** rather than trusted or dropped |
| V25 | **P-012 availability contract.** For each of `backlogit_list_checkpoints`, `backlogit_get_checkpoint`, `backlogit_resolve_checkpoint`, `backlogit_create_checkpoint`, `gh api` (including the effective-rules endpoint), review-thread enumeration and Git history access, simulate unavailability at its probe point | each probe runs **before** the path that needs it; only declared official CLI fallbacks are used; anything else **halts**; a registry exposing no checkpoint operations **halts** and is never treated as an implicit zero enumeration |

## Residual risks

| Ref | Risk | Disposition |
|---|---|---|
| RR-1 | These are prose protocols executed by an LLM. Correct wording does not prove correct execution. | **Accepted and recorded.** The prior plan's answer — a static wording checker — could only prove the words had not changed, not that the protocol ran. U5's ancestry assertion is the real defence: a concrete command with a pass/fail outcome that makes a miss loud. V8 additionally proves the one discovery command that was silently wrong in revision 6. |
| RR-2 | *(Closed in revision 7.)* Stage's session-end resolution was previously bound by P-022 with no procedural rewiring. | **Closed by U8.** The independent review held that a universal requirement with a procedural gap in one of its two named agents is not realized. U8 adds the narrow Stage qualifier without granting Stage merge authority. |
| RR-3 | The locator lives in the PR body, which a human or bot can edit or delete. | **Closed at revision 10 by U0 + U13, under an explicit operator decision.** Revision 9's attempt failed on three grounds the round-9 review found and this revision accepts in full: **(a)** Channel B's candidate set was drawn from marker-bearing bodies, so deleting the body removed the PR from the scan (F-13); **(b)** its commands were not executable — no commit-ish, a reused global `FETCH_HEAD`, and no current-tree path for a fully deleted artifact (F-14); **(c)** with no SHA anywhere, an erased introducing commit was indistinguishable from "never existed", needing only the author's ordinary push rights (F-19). Revision 10 closes each: **(a)** U13 AC2 derives Channel B's candidates from the exhaustive **trusted-PR enumeration**, explicitly never filtered by body content, and V17 tests discovery against a **fully emptied** body; **(b)** U13 AC3 replaces every command with an explicit-revision-root form fetched to a **unique retained ref** `refs/autoharness/scan/pr-<n>`, adds a shallow/partial-clone guard at B0, and V18 **executes** the workflow rather than inspecting strings; **(c)** U0 installs an **approval-gated repository ruleset** carrying `deletion` and `non_fast_forward` with no bypass actors over the resolution-bearing Ship branch patterns, and S3.5 **proves** those rules apply to the live `headRefName` before any obligation is published. With force-push and branch deletion both barred, the introducing commit stays reachable, so an ordinary later deletion commit is **detectable** (OB-1) rather than indistinguishable from absence, and the F-19 gap is structurally removed rather than asserted away. **RQ-7 is not weakened.** **Honest residual**: the guarantee is exactly as strong as the ruleset. A repository **admin** can disable or edit the ruleset, and an admin who does so between S3.5 and a later erasure defeats the mechanism. That residual is (i) **bounded to admin privilege** rather than any collaborator's ordinary push rights, which is the material change; (ii) **detected on re-check** — amended item 15 re-proves the ruleset and U13's B2 re-proves per candidate, so drift halts as **OB-8**; and (iii) **not silently absorbed** — an uncovered branch, an ambiguous response, or an unavailable API halts before publication rather than proceeding optimistically. It is accepted at that scope. **Measured precondition**: as of this revision the repository ruleset `PR-Required` (id `12812291`) covers `~DEFAULT_BRANCH` only, and the current Ship source branch returns an **empty** effective-rules set, so the protection U0 must create **does not exist yet**. The plan therefore does **not** assume source-branch immutability today; S3.5 exists precisely because it must be proven per run. |
| RR-4 | `gh api --paginate` over all PRs grows with repository history. | **Accepted.** Cost is bounded by PR count and runs once per zero-candidate startup. Correctness was chosen over speed deliberately (RQ-9). Channel B adds one fetch and one history scan per **trusted** candidate, which is a small subset of that enumeration; B7's ref cleanup keeps the cost transient. |
| RR-5 | A crash between locator phase 1 and the resolution commits leaves a `RESOLUTION_PENDING` locator with no commits. | **Handled, not merely accepted** — `LAST_MILE_RECOVERY` Step 1a runs the *resolution-state classification* over the locator's checkpoint list at the fetched PR head and re-enters **`RESOLUTION_PREFIX`** (at the resolve step) **only** on a reduced state of `NONE`; `PARTIAL`, `ALL` and `INDETERMINATE` all halt, as does the merged case. `RESOLUTION_POSTCONDITION` is never re-entered here — POST-A is a metadata write and POST-B is a post-merge closure commit. Listed here because the handling is a recovery path, not a prevention. |
| RR-6 | **The protocol couples to three Ship-side behaviours it does not own**, and a change to any of them silently breaks it. *(Round-9 finding F-31, recorded rather than hidden.)* First, **Step 5 ↔ Step 6 backlogit state**: S4 writes `OPEN` into an artifact Step 5 item 8 created, and POST-B closes it in Step 6 — if `operational-closure` stops emitting a pre-merge artifact, or emits it at a different path, the record has no host. Second, the **post-merge worktree pattern**: POST-B commits on the `post-merge/{feature_slug}` branch Step 6.0 creates, so a change to that branch convention relocates the closure commit. Third, **`backlogit shipment ship` non-termination and the P-015 safe-close path**: a unit that cannot reach normal termination cannot run POST-B, leaving the record `OPEN` — which is the *correct* fail-closed outcome but will surface as a halt on the next startup rather than as a clean close. | **Accepted and recorded, with two mitigations rather than a claim of independence.** U10 makes the first coupling **owned** — the field is declared in the `operational-closure` skill that owns the artifact, so a schema change is a visible edit to a declared field rather than a silent orphaning — and adds the path-stability invariant so the artifact cannot be relocated while unclosed. U11's worktree-topology reconciliation names the Step 6.0 branch explicitly and cites the recorded prior art, so the second coupling is documented at the point of use. The third is **deliberately left fail-closed**: an `OPEN` record surviving a non-terminating run is the designed behaviour, and the resulting startup halt is the intended signal, not a defect. No attempt is made to auto-recover it, because that would require the cross-run continuation semantics this plan is expressly forbidden to add. |

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
were reviewed against, and it is **appended to, never rewritten**. Revision 8's
own Round 8 review genuinely returned **FAIL** with three open P1s and left the
circuit **OPEN** at attempt counter 3; that record stands unaltered below. The
round-9 review returned **FAIL** with one P0 and fifteen P1s and is appended
verbatim at the end of this document. Revision 10 remediated those findings under
explicit operator authorization, but **no row below, and no earlier revision's
verdict, may be cited as a harvest gate for revision 10**. Revision 10's own gate
was a fresh independent four-persona cross-model full-plan review, appended at the
end of this document as **round 10**. It returned **FAIL** with four P0s and
seventeen P1s. The circuit is **OPEN at attempt counter 5**, and revision 10 is
**not harvestable**.

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
| 8 | Independent Scope Boundary Auditor (`gpt-5.6-sol`, xhigh) | **FAIL** (3 P1, 5 P2) | revision 8. **Third consecutive FAIL. Review circuit OPEN — attempt counter 3.** P-013.6 escalation fired; Stage halted without harvesting. |
| 8-escalation | P-013.6 escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh`, against HEAD `bbb52b65` | **ESCALATION_BLOCKS** | revision 8. Reasoning-only. Produced findings A–E, remediated into revision 9 under explicit operator authorization for ONE bounded revision plus ONE fresh full review. |
| 9 | Independent five-persona panel, cross-model | **FAIL** (1 P0, 15 P1, 11 P2, 6 P3) | revision 9. Returned **RR-3** as the blocking design question for an operator decision. Findings appended verbatim below and remediated into revision 10; the operator's RR-3 decision — an approval-gated GitHub branch-ruleset prerequisite — is implemented as **U0** + the rebuilt Channel B in **U13**. |
| 10 | Independent four-persona panel, cross-model: Correctness (`gpt-5.6-sol`), Constitution/Policy (`claude-opus-4.8`), Scope Boundary and Maintainability (`grok-4.6`), Security and Ops Risk (`gemini-3.8-flash`) | **FAIL** (4 P0, 17 P1, 9 P2, 5 P3) — **unanimous** | revision 10, at HEAD `23731cba`. Confirmed the round-9 **P0 (F-11) genuinely closed** and the Ship Step 5 structural extraction accurate. Found four new P0s: the canonical/U13 Channel B divergence with the discharge-rejection bug reinstated; U13 AC6 halting on the protocol's own happy-path interval; an unsatisfiable `FETCH_HEAD` vs retained-ref contract; and repository-wide workflow breakage from U0's `deletion` + `non_fast_forward` rules. Panel recommended returning a narrower design — immutable marker ref instead of branch ruleset — to the operator. Findings appended verbatim below. **Circuit OPEN at attempt counter 5.** |

### Round 8 — outstanding findings (SUPERSEDED by revision 9; recorded as they stood)

**Status of this subsection.** The three P1s below were genuinely open when
revision 8 was reviewed, and the text is preserved as it stood rather than
rewritten. Revision 9 remediates each; the remediation column names where.
Marking them remediated here is **not** a verdict — only the pending revision-9
review can confirm the remediations are adequate.

Revision 8 closed every round-7 P1 the auditor could verify: the P-003 sub-epic
tier is restored, U4 AC1 references without restating, the RQ-2 trace explicitly
excludes U8, U4 AC4 exposes the Step-5 reorder, hardening D6 defers to D10, U2's
locus is fixed, `pr_role` is gone, and U6 is owner-scoped. No Defect-1 construct
is reintroduced. All eight units remain single-file documentation units.
*(Historical: revision 9 raised the count to ten; every unit is still a
single-file documentation unit.)*

These three P1s were open at revision 8 and were carried to escalation. The
revision-9 remediation for each is recorded in the fourth column:

| # | Finding | Required fix | Revision-9 remediation |
|---|---|---|---|
| 1 | **RQ-6 has no executable enforcement path.** The canonical prefix asserts that a live re-fetch of HEAD, threads and CI already exists in Ship Step 5 and is unchanged. It does not: real item 15 re-runs the P-018 gate and re-queries `headRefOid` only — it never re-fetches required CI, and it does not refresh all review threads when P-018 is disabled. U4 AC9 then *requires* item 15 to stay unmodified, so RQ-6 is credited to a unit that is forbidden from implementing it. | Expand U4 to amend the last-mile item with an explicit fail-closed post-approval query of live HEAD, full thread state and required checks; drop the "item 15 unmodified" criterion. | **Applied.** `RESOLUTION_PREFIX` restructured into S1…S8 with the exact safe order; U4 carries a verbatim Step-5 extract; the "item 15 unmodified" criterion is **withdrawn** and U4 AC9 now **amends** item 15 to re-fetch headRefOid, PR body, reviewDecision, review requests/reviews, every review-thread page, required checks and resolution-commit ancestry; U4 AC10 adds the six-part merge bar and the no-stale-approval refresh rules; item 16 (P-009) stays unmodified. Verified by V4. |
| 2 | **The zero-checkpoint bypass claim is false.** U4 AC3 promises a zero-checkpoint unit runs "the pre-existing path unchanged", while AC2/AC4 require all mutating items to move before the prefix and the readiness gate to move after it. Real Ship Step 5 has readiness items 7b/7c *before* runtime verification, closure-artifact generation, follow-up writes and the push (items 7–10), so the reorder changes the common path for every unit, including zero-checkpoint ones. | State that zero-checkpoint units skip locator publication and resolution but use the newly ordered common readiness path. Do not claim their step order is unchanged. | **Applied.** The checkpoint count now selects segment **S4 only**; the "pre-existing path unchanged" claim is removed everywhere and expressly prohibited by U4 AC3. Enumeration failure/malformed/quarantine/ambiguity is **not zero** and halts; a late-appearing checkpoint forces re-evaluation; no empty locator; Stage startup recovery preserved as separate. Verified by V13–V16. |
| 3 | **D14 is not actually folded into a task criterion.** D14 requires reading `branch`/`pr`, fetching the PR head, checking out a local branch, confirming the checkout and halting on failure. Its cited U5 AC2 lists only `gh pr view`, `git fetch` and `git merge-base` — no checkout, no verification — and no V-check covers it. Because task-card criteria are declared exact, U5 could pass while recovery is still sitting on `main`. This re-opens the very hazard D14 was written to close. | Add a U5 acceptance criterion requiring the by-name working-tree placement, checkout and verification before any committing re-entry, plus a matching verification check; then repoint D14's fold reference. | **Applied using existing units — no new unit.** The placement paragraph (whose heading revision 8 had lost, leaving it uncitable) is named `Working-tree placement — committing re-entry only` and defined in **U3 AC12** (WP-1…WP-7); **U5 AC9** executes it by that exact name before the only committing recovery branch; the U3→U5 dependency is unchanged. Verified by **V12** and cases **V12a–V12f**. D14's fold reference is repointed. |

Open P2s at revision 8, both now closed: V2 was unsatisfiable as written (it
forbade verbs U1/U8 are required to use) — **replaced in revision 9** by a
sequence-restatement check that permits ordinary prose verbs; and V8's reference
command lacked `--paginate` so its comparison was invalid — **fixed in revision
9** by paginating the reference command.

**Closed on 2026-09-13** (PR #396 Copilot review remediation pass, commit
recorded in the PR): the decision document's in-scope list now carries
`_stage.agent.md`, states the unit count explicitly instead of "7-task plan", and
no longer names the retired `RESOLUTION_ORDER`; hardening D14's stale
`Folds into` reference is withdrawn along with its incorrect "applied" status.
*(The unit count that sentence recorded was **eight (U1–U8)** at revision 8; it
is **ten (U1–U10)** at revision 9 following the RR-3 closure.)*

### Additional findings from the PR #396 Copilot review (remediated in the revision-8 pass)

These were raised on the published PR rather than by the four-persona panel.
They are **specification hardenings and honesty corrections**. The dispositions
below are recorded **as they stood at revision 8** and are not rewritten; at that
point none of them closed any of the three P1s and none changed the circuit
state. Where revision 9 has since advanced a disposition, that is noted inline.

| Thread | Finding | Disposition |
|---|---|---|
| `PRRT_kwDORJEduc6h6juE` | The "exhaustive and trusted" read protocol validated no provenance, so a fork PR could forge a locator and halt startup or steer recovery. | **Fixed.** New *Provenance validation* block: PV-1…PV-7, with untrusted candidates **discarded silently** (so an outsider cannot deny startup) and trusted-but-inconsistent ones **halting**. All downstream rules operate over the TRUSTED set only. |
| `PRRT_kwDORJEduc6h6juv` | RR-3's "no worse than today" disposition contradicts RQ-7. | **Fixed by honest reclassification** *(at revision 8)*. RR-3 was reclassified **OPEN**, not accepted: resolving every checkpoint pre-merge makes the deletable PR-body marker the *sole* obligation record, which is strictly worse than the pre-change still-active checkpoint. The durable publication record was **not yet designed** at that point. **Superseded at revision 9:** RQ-12, units U9/U10 and hardening D15 design and install it; RR-3 is now **CLOSED**, subject to the pending revision-9 review. |
| `PRRT_kwDORJEduc6h6jv7` | Body status said `review_verdict: none` while frontmatter said FAIL. | **Fixed.** The status paragraph now states the round-8 FAIL, the three open P1s, attempt counter 3 and the open circuit. |
| `PRRT_kwDORJEduc6h6jv-` | Retained-history note still said "revision 7". | **Fixed.** Now names revision 8 and its FAIL verdict. |
| `PRRT_kwDORJEduc6h6ldd` | "Handled normally" undefined for two locators naming the same shipment. | **Fixed.** Byte-identical duplicates collapse to one record; **any** field difference halts. `updated_at` explicitly barred as a tie-breaker (it is body text). |
| `PRRT_kwDORJEduc6h6ldr` | "Resolution commits exist" had no executable definition on the `RESOLUTION_PENDING` path. | **Fixed.** New *Resolution-state classification*: per-checkpoint state at the fetched PR head, reduced to `NONE` / `PARTIAL` / `ALL` / `INDETERMINATE`, with strict precedence. Only `NONE` resumes; the other three halt. |

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

**Disposition**: the failing operation was **not** re-executed at revision 8.
Stage halted: no harvest, no shipment assembly, no successor ID allocation. The
escalation is a reasoning escalation only and confers no authority to promote
this plan.

**Escalation execution and operator re-authorization (revision 9).** The
escalation ran under route `gpt-5.6-sol` / `openai` / `xhigh` against HEAD
`bbb52b65df1e63f9e8ebbd28b4ccd0fc61718cdd` and returned `ESCALATION_BLOCKS` with
findings A–E. The operator then explicitly authorized **one bounded planning
revision plus one fresh full independent plan-review gate** — and nothing
further. Revision 9 is that revision. It remains within the reasoning-escalation
boundary: **no** harvest, **no** shipment claim or assembly, **no** activation,
**no** implementation, **no** merge, and **no** revival of the abandoned `143.*`
artifacts. `harvest_authorized` stays `false` and moves only if the fresh review
of revision 9 returns PASS **and** the Orchestrator separately routes the next
step.

**Answer to the escalation question.** The escalation asked whether U4 should be
decomposed against a **verbatim extract of the real Step 5 item list** rather
than against a prose description of it. **Yes — and revision 9 does exactly
that.** U4 now carries the extract inline, including the two facts every prior
prose round missed: the item number `7` is **duplicated** in the live file, and
readiness items 7b/7c sit **before** the mutating items 7(second)/8/9/10 and the
push. Those two facts are the direct cause of open findings 1 and 2, which is
strong evidence the escalation's diagnosis was correct.

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

## Plan Review — round 9

**Plan reviewed**: `docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md` revision 9
**Reviewed at HEAD**: `9b15fd4347c49c8a5f6d2277a8eba5dcd4f07242`
**Attempt**: 4 (consecutive FAILs: revisions 6, 7, 8, 9)
**Gate decision**: **FAIL**

### Gate rationale

This was a fresh, full, independent review — not a confirmation of the
revision-8 round. Seven personas read the plan, the source decision (revision 3),
the hardening (revision 9) and the live `.github/agents/_ship.agent.md` from
scratch, and were told explicitly not to manufacture findings where the plan was
sound.

The gate fails on three independent grounds, any one of which is sufficient:

1. **One P0.** The Stage-side safeguard in U8 AC2 keys off the ambient
   working-tree branch instead of the selected checkpoint's carrying PR, which
   permits the exact post-merge resolution P-022 exists to forbid.
2. **Fifteen P1 findings after dedupe.** Four are cross-confirmed by three or
   more independent personas.
3. **RR-3 is re-opened, not closed.** The durable-publication mechanism that
   revision 9 introduced to close RR-3 was found not to hold. Per the governing
   directive, an unresolved RR-3 is itself a FAIL condition.

**Plan hardening**: required (`requires_plan_hardening: yes`) and present
(`docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md`
revision 9, D1–D15). The hardening document is not the cause of this FAIL; its
D15 fold is invalidated as a consequence of RR-3 re-opening, not the reverse.

### Panel

| Persona | Model | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Constitution Reviewer | `claude-opus-4.8` | 0 | 2 | 3 | 2 |
| Scope Boundary Auditor | `gpt-5.6-sol` (xhigh) | 0 | 5 | 4 | 0 |
| Correctness Reviewer | `gemini-3.8-flash` | 0 | 3 | 2 | 2 |
| Architecture Strategist | `grok-4.6` | 0 | 3 | 4 | 1 |
| Agent-Native Parity Reviewer | `gpt-5.6-terra` | 1 | 6 | 2 | 0 |
| Security Lens Reviewer | `claude-sonnet-5` | 0 | 3 | 1 | 1 |
| Learnings Researcher | `claude-haiku-4.5` | 0 | 1 | 3 | 2 |
| **Merged, deduplicated** | — | **1** | **15** | **11** | **6** |

Cross-model diversity was satisfied: six distinct models across four vendors.
Where personas disagreed on severity, the more conservative severity was taken.

### What the panel confirmed as genuinely sound

Recorded so the FAIL is not read as a wholesale rejection.

* **The verbatim Step 5 extract in U4 is accurate.** Two personas independently
  re-derived it from the live `.github/agents/_ship.agent.md`: the item number
  `7` really is duplicated, readiness items 7b/7c really do precede the mutating
  items and the push, and item 15 really does re-run only P-018 and `headRefOid`.
  This closes the escalation's core diagnosis and is the one structural advance
  revision 9 genuinely delivers.
* **Fork-PR forgery is blocked at the root.** PV-1 discards fork-originated
  candidates for both channels before any body content is trusted, and the
  silent-discard versus halt asymmetry is drawn correctly against outsider DoS.
* **P-009 and P-017 hold.** Item 16 is untouched; U6 AC7's dark-mode carve-out
  closes a real hole.
* **P-010 role separation holds.** U8 gives Stage only a prohibition and a halt,
  never `RESOLUTION_PREFIX` and never merge authority.
* **No Defect-1 leakage.** The obligation record is a passive, halt-only
  frontmatter field with no lock, CAS, cursor, or auto-routing. Three personas
  checked this specifically.
* **Abandoned-ID discipline holds.** No `143.*` ID is revived, re-parented, or
  reused; replacement IDs remain unassigned.
* **`docs/closure/` was the right surface to have chosen.** The architecture
  review agreed it is the only per-shipment Git-tracked artifact Ship already
  writes on the PR branch before resolution, and that the alternatives were
  correctly rejected. The mechanism fails on execution detail, not on venue.

### P0 findings

#### F-11 (P0) — U8 AC2 keys the Stage safeguard off the ambient branch, not the checkpoint's carrying PR

*Agent-Native Parity Reviewer. File: plan, U8 AC2.*

U8 AC2 has Stage `determine the current branch, then
gh pr list --state merged --head <branch>`. Stage is permitted to run on `main`,
and the Checkpoint Payload Contract does not require a carrying-PR number or
branch. A resumed Stage checkpoint created on a staging branch can therefore be
handled while Stage sits on `main`: the query inspects `main`, returns no
matching merged PR, and the agent proceeds to resolve a checkpoint whose
carrying PR has already merged. That is precisely the post-merge resolution
P-022 is written to forbid, reachable through the safeguard meant to prevent it.

**Required fix.** Take the carrying PR number and branch from the selected,
ownership-validated checkpoint context and make those fields mandatory whenever
a Git-tracked checkpoint may be resolved. Query that exact PR
(`gh pr view <pr> --json state,mergedAt,headRefName,baseRefName`), verify head
and base against the stored identity, require state `OPEN`, and halt on absent,
mismatched, closed, or merged. Never use the ambient working-tree branch as the
authority.

### P1 findings

Ordered by cross-persona confirmation count, then by severity of consequence.

#### F-01 (P1) — `RESOLUTION_POSTCONDITION` is self-contradictory: it both closes the record in a commit and "commits nothing"

*Confirmed independently by Constitution, Scope Boundary, Architecture and
Agent-Native Parity — four of seven personas.*

The canonical block requires Ship to "set `CLOSURE_LOCATOR` to `RECONCILED` and
close the `RESOLUTION_OBLIGATION_RECORD` in the same closure commit". The
sentence immediately following it still says the postcondition "is a PR-body
metadata write only and commits nothing", and RR-5 still calls it "a Step 6
metadata write". U2 installs this block **verbatim**, so the contradiction is
installed into the instructions file. Closing the record is a change to a
Git-tracked `docs/closure/` file and necessarily requires a commit.

The consequence is not cosmetic. If "commits nothing" governs, `OPEN` records
are never discharged and Channel B fail-closes every future startup forever. If
the commit governs, the canonical definition the plan installs is false. The
plan additionally never establishes which branch may legally carry that commit:
Ship must not commit to `main` (P-010), and the plan removed `pr_role: closure`,
so no closure-PR path is established — which risks reintroducing the very
"extra PR for a one-line status flip" pattern this work exists to eliminate.

**Required fix.** Split the postcondition into two explicitly named mutations —
the PR-body `RECONCILED` write (metadata, no commit) and the `OPEN` → `CLOSED`
transition (a commit on a named, P-010-compliant closure branch that must be
merged before the obligation is discharged). Delete the "commits nothing"
sentence and the RR-5 phrasing that depends on it. Name the branch.

#### F-02 (P1) — the Constitution Check still asserts the withdrawn "item 15 retained unmodified"

*Confirmed by Constitution (P1), Scope Boundary (P2) and Architecture (P2).*

The Constitution Check P-018 row reads: "Step 5's existing unconditional
last-mile re-check (item 15) is retained unmodified." That is the exact claim
escalation finding A identified as open finding 1 and that revision 9 withdraws
everywhere else — S7 says "AMENDED", U4 AC9 says "Item 15 is amended, not
retained unmodified", and hardening D5 agrees. The Constitution Check is a
second normative statement of Step 5's shape, so an implementer or later editor
reading it is told not to make the change U4 requires.

This finding matters beyond its own content: it is a **residual instance of the
very escalation finding revision 9 was authorized to remediate**, surviving in a
section the revision did not sweep. It is direct evidence that the remediation
was applied section by section rather than globally.

**Required fix.** Rewrite the P-018 row: item 15 is amended per S7, P-018 is
evaluated at S5 and re-evaluated at S7, item 16 remains unmodified. Then grep
the whole document for every other surviving assertion about Step 5's shape.

#### F-03 (P1) — V4(g) forbids the mutation that S4 requires, so every nonzero-checkpoint implementation fails its own verification

*Scope Boundary Auditor.*

V4(g) asserts that no branch-mutating item appears after the S3 enumeration
proof. Canonical S4 deliberately resolves checkpoints, writes the obligation
record, commits and pushes — all after S3. The expected-empty mutation set
therefore makes V4 unsatisfiable for exactly the case the plan exists to handle.

**Required fix.** Restate V4(g) as: no mutation after S3 **except** the
conditional S4 resolution commit and its push; and no branch mutation at all
after S4.

#### F-04 (P1) — S8's five-way HEAD equality is unsatisfiable for zero-checkpoint units

*Correctness Reviewer.*

S8 condition 1 requires `live headRefOid == local HEAD == locator final_head ==
PR-body Reviewed HEAD == approved_head`, "all five agree", under a rule that any
doubt halts. But the plan establishes that a zero-checkpoint unit omits S4
entirely and that "no empty locator is ever published", so `locator final_head`
does not exist. Every zero-checkpoint PR therefore halts unconditionally at the
merge bar and becomes unmergeable.

This is a **new zero-checkpoint truthfulness defect introduced by revision 9's
own finding-B remediation** — the remediation correctly made S5–S8 universal but
did not make the locator-dependent clauses conditional at the same time.

**Required fix.** Make the locator clause conditional: the four always-present
heads must agree, and `locator final_head` joins them when a locator was
published. Likewise condition 2 becomes "every recorded resolution commit, if
any, is an ancestor".

#### F-05 (P1) — `RESOLUTION_POSTCONDITION` demands an impossible transition on zero-checkpoint units

*Correctness Reviewer.*

On a zero-checkpoint unit no locator exists and `resolution_obligation` was
initialized `none` at S2, so `none → OPEN` never happened. Step 6 nonetheless
unconditionally orders Ship to set the locator `RECONCILED` and move the record
`OPEN → CLOSED`, while U10 AC3 states those are the only permitted transitions
and "no other transition is permitted". The agent is instructed to perform a
transition the schema forbids on a record that is not in the required state.

**Required fix.** Make `RESOLUTION_POSTCONDITION` an explicit no-op when no
locator was published and `resolution_obligation` is `none`, leaving `none`
intact in the closure commit. State it in the canonical definition, U4 AC7 and
U5 AC7.

#### F-06 (P1) — S7 never re-enumerates checkpoints, so the late-checkpoint race the plan claims to close stays open

*Correctness Reviewer.*

The plan asserts that "a checkpoint that appears after S3 completed forces
re-evaluation from S3 before merge", and V16 tests exactly that. But S7's
exhaustive re-fetch list — `headRefOid`, PR body, `reviewDecision`, review
requests and reviews, every review-thread page, required checks and
resolution-commit ancestry — does not include active checkpoints, and S8's merge
bar never asserts a zero active-checkpoint count. An agent executing S7 and S8 as
written will never observe a late-appearing checkpoint. The claim and V16 are
unbacked.

**Required fix.** Add "re-enumerate active checkpoints owned by this unit and
assert count == 0; any active checkpoint forces re-evaluation from S3" to S7's
list in both the canonical definition and U4 AC9.

#### F-07 (P1) — legitimate P-020 compaction relocates the obligation record and OB-1 reads it as the deletion attack

*Architecture Strategist (P1); Constitution Reviewer raised the same coupling at P2.*

Channel B reads `docs/closure/`, and OB-1 halts when a record that once appeared
in history is absent from the tree without a recorded `OPEN → CLOSED`
transition. But `compact-context` — whose invocation U4 AC8 leaves **unmodified**
— compacts `docs/closure/` and moves originals to `docs/archive/closure/` for
completed-feature records older than `threshold_days` (default 14). A pre-merge
artifact written at S2 can easily exceed 14 days by merge, and `CLOSED` is
defined to be written *after* the full closure set, which includes P-020. So
routine compaction archives a still-`OPEN` record, and every subsequent startup
fail-closes permanently on what is in fact correct behaviour.

**Required fix.** Add a path-stability invariant: an artifact that has ever
carried `resolution_obligation` must not be renamed, compacted or archived while
its status is `none` or `OPEN`. Either exclude `OPEN` records from
compact-context candidates (and say so in U10 and the P-020 surface) or extend
Channel B to follow `docs/archive/closure/` and treat an archive move as not
OB-1. `CLOSED` must be written before any compaction that can touch the file.

#### F-08 (P1) — S1–S8 hard-codes live item numbers with no standing drift check

*Architecture Strategist.*

The canonical prefix is expressed in terms of "real Step 5 items 7 (first)/7a,
second item 7, 8, 9, 10, 7b/7c moved, item 15 amended, item 16 unchanged",
including the live file's duplicated `7`. U4 then requires `_ship.agent.md` to
invoke the prefix by name and not restate it. The only consistency check, V4,
runs once at implementation time; the wording-drift checker was withdrawn as
Defect-1. This is the *same coupling* that caused revisions 6, 7 and 8 to
describe a Step 5 that did not exist. When Step 5 gains an item or is
renumbered, the instructions and the agent diverge silently, and
readiness-before-mutation can be restored without anything noticing.

**Required fix.** Define S1–S8 by **role** (fix-ci loop, mutating tail,
enumeration proof, conditional resolution, readiness gates, pinned approval,
last-mile re-check, merge bar) with no item numbers in the instructions file, and
keep the item-to-segment binding solely in `_ship.agent.md`. Add a standing
invariant asserting the executable order still matches those roles. V4 is not a
substitute for it.

#### F-09 (P1) — U3, U4 and U9 each exceed the NON-NEGOTIABLE 2-hour granularity rule

*Scope Boundary Auditor raised all three at P1; Constitution Reviewer raised U4
at P3.*

* **U3** combines exhaustive API discovery, seven provenance rules, duplicate
  handling, locator-status classification, a four-state checkpoint classifier,
  seven live-PR outcomes, a merge-authority bar and WP-1…WP-7. V8 plus
  V12a–V12f place at least seven behavioural scenarios on it against a
  fewer-than-four limit.
* **U4** declares its locus as Step 5 and session end, but AC7–AC8 also modify
  Step 6; it carries 13 acceptance criteria, moves two gates, amends item 15,
  adds a merge bar and refresh rules, and is exercised by V4, V9 and V13–V16.
  Raising it to M/high was honest but does not make it compliant.
* **U9** bundles schema and identity, lifecycle, rejected alternatives,
  two-channel enumeration, provenance, deletion archaeology and OB-1…OB-7, while
  labelled S/medium.

**Required fix.** Split each along the seams the auditor named: discovery and
provenance apart from recovery and working-tree placement (U3); the Step 5
reorder apart from Step 6 and session-end reconciliation (U4); record schema and
lifecycle apart from Channel B discovery and reconciliation (U9). Re-derive
`task_count`, the dependency graph and the verification mapping afterwards.

#### F-10 (P1) — S3/S4 is not a mechanically executable `backlogit` contract

*Agent-Native Parity Reviewer.*

S3 says "enumerate every checkpoint owned by THIS unit" and S4 says "resolve
every checkpoint owned by this unit", supplying neither the
`backlogit_list_checkpoints` invocation nor a deterministic ownership predicate.
The installed Ship and Stage recovery protocols are far stricter: call with
`consumer_id` only, apply no `status`/`agent` API filter, inspect quarantine and
validation anomalies *before* partitioning, and resolve only a validated
owner-selected record after a confirmed successful handling. The registry also
exposes no `agent` list parameter. As written, an agent could filter at the API
call and hide quarantined records, sweep in another unit's records, or
bulk-resolve without a per-record handling proof.

**Required fix.** Add a normative S3 algorithm and a matching U4 acceptance
criterion specifying the exact call, the no-filter rule, anomaly-before-partition
ordering, a precise current-unit identity predicate (for example validated
`context.shipment_id == current shipment_id` together with `agent == ship`) and a
per-checkpoint successful-handling proof before any
`backlogit_resolve_checkpoint`. Prohibit bulk and cross-unit resolution
explicitly.

#### F-13 (P1) — Channel B is not independent of the mutable PR body (RR-3 blocker 1)

*Agent-Native Parity Reviewer.*

Channel A selects "entries whose body contains `autoharness:closure-locator`".
Channel B then scans "every PR admitted by PV-1…PV-3 of the same exhaustive
paginated enumeration". If the body locator was deleted before merge, that PR is
no longer a marker-bearing candidate and no instruction unambiguously admits it
for Channel B. The claimed body-deletion recovery therefore fails in exactly the
open-PR residual window it was designed for — which is the whole of RR-3.

**Required fix.** Derive two independent candidate sets from the one paginated
response: apply PV-1…PV-3 to every enumerated PR using API fields alone; Channel
A may then inspect only trusted bodies carrying the marker, but Channel B must
fetch and inspect every trusted PR head **regardless of body contents**. Add an
executable test for an `OPEN` record on an unmerged PR whose body has been
deleted.

#### F-14 (P1) — Channel B's discovery and deletion commands are not executable (RR-3 blocker 2)

*Agent-Native Parity Reviewer.*

`git log --follow --diff-filter=D --format=%H -- <path>` carries no commit-ish,
so it examines only the currently checked-out history rather than each fetched PR
head; `FETCH_HEAD` is overwritten by every subsequent PR fetch; and a fully
deleted artifact has no current-tree path for the preceding `ls-tree` to supply.
`git log -S'resolution_obligation' -- docs/closure/` yields commits but the plan
defines no procedure mapping introduction, deletion and `OPEN → CLOSED`
transitions to a particular record identity. OB-1 and OB-2 therefore cannot be
evaluated exhaustively from a fresh checkout.

**Required fix.** Fetch each trusted PR to a unique retained local ref rather
than relying on `FETCH_HEAD`; scan that ref and `origin/main` explicitly;
enumerate historical artifact paths and field transitions from those explicit
refs; parse parent and child blobs to bind each record identity to its `OPEN` and
`CLOSED` transitions; require an ancestry check from the introducing commit to
the retained ref or `origin/main`; halt on fetch, history or parse failure.
Rewrite V18 to execute the whole workflow, including deletion from an open PR
head and deletion after merge.

#### F-15 (P1) — PV-B4/PV-B5 structurally reject the correct post-merge `CLOSED` transition

*Agent-Native Parity Reviewer.*

Channel B requires "the record's embedded `pr` equals the PR it was read from".
The record is opened in the implementation PR and closed in a *different*
post-merge closure PR, where its embedded `pr` and `branch` correctly still name
the implementation PR. A correct `CLOSED` transition therefore fails PV-B4/PV-B5,
and no alternate provenance relation for the closure PR is defined.

**Required fix.** Define provenance in terms of the immutable implementation-PR
identity, and define the authorized closure-transition carrier separately:
validate the introduction against the implementation PR's head and history, then
validate `CLOSED` as a descendant on a closure PR whose relationship to that
implementation PR is explicitly recorded and checked by Git or API ancestry. Do
not apply one "read from this PR" equality rule to both trees.

#### F-16 (P1) — P-012 degraded mode is asserted but never wired for the new critical-path tools

*Agent-Native Parity Reviewer.*

The Constitution Check says locator discovery "is probed per P-012", but U4, U5
and U6 add dependence on `backlogit_list_checkpoints`, `backlogit_get_checkpoint`,
`backlogit_resolve_checkpoint`, `backlogit_create_checkpoint`, `gh api`,
review-thread enumeration and Git history access without a single acceptance
criterion updating Ship's tool-availability gate, and without specifying
CLI-fallback versus halt behaviour when the registry lacks checkpoint operations.
The dangerous case is a registry with **no** checkpoint operations being treated
as an implicit zero-checkpoint result — a silent fail-open into the merge path.

**Required fix.** Add an explicit pre-S3 and pre-startup availability contract:
probe every required operation before entering the affected path, use only
declared official CLI fallbacks, and otherwise halt before S3, before either
discovery channel, before recovery and before merge. Define the
no-checkpoint-operations registry case explicitly as a halt, never as zero.

#### F-17 (P1) — a trusted decoy PR can indefinitely deny recovery for a real shipment

*Security Lens Reviewer (confidence 0.75).*

Both channels admit any same-repository, default-branch-targeting PR authored by
an `OWNER`/`MEMBER`/`COLLABORATOR` as a carrier for an arbitrary real shipment
ID. PV-6/PV-7 only check that the referenced shipment and feature exist and that
checkpoint filenames look plausible; PV-4/PV-5/PV-B4/PV-B5 only check internal
self-consistency. Nothing checks that the PR *is* the shipment's designated
implementation PR. A trusted-but-careless or compromised collaborator can
therefore open an unrelated decoy PR carrying a fabricated record or locator
naming a real in-flight shipment; any field difference then trips OB-3 or the
duplicate-locator rule and halts that shipment's every future startup until a
human disambiguates. No fork and no admin access is needed, and the risk is
disclosed nowhere in the residual-risks table.

**Required fix.** Add PV-8/OB-8 binding a discovered locator or record to the
shipment's backlog-recorded implementation PR number, and **discard** — not halt
on — candidates whose PR number does not match that binding. At minimum, record
it as an explicit accepted residual risk.

#### F-18 (P1) — S8 is check-then-act; the merge call does not pin the observed HEAD

*Security Lens Reviewer (confidence 0.65).*

S8 requires five values to agree at the moment of evaluation but never requires
the merge invocation itself to be atomic against that observation. Between the S8
read and the merge API call, a push by the author, a bot, or any app with write
access can advance `headRefOid`. GitHub's merge endpoint accepts an expected-head
`sha` parameter for exactly this purpose and the plan does not use it, so the
six-part bar is a TOCTOU pattern rather than a guarantee.

**Required fix.** Require S8 (and item 16 / P-009) to pass the S8-observed HEAD
as the expected head SHA to the merge call so GitHub refuses server-side on
mismatch.

#### F-19 (P1) — the RR-3 "detectable, halting" claim is unsubstantiated (RR-3 blocker 3)

*Security Lens Reviewer (confidence 0.68).*

Revision 9's accepted-residual sentence claims the force-push + history-rewrite +
body-edit combination is now "a detectable, halting condition (OB-2) rather than
a silent one". It is not. The record deliberately carries no SHA anywhere (RQ-8),
and Channel B's probes scan only currently reachable history. Force-push the
branch to before the resolution commit and edit the body, and both probes return
empty — indistinguishable from the obligation never having existed. No checksum,
prior-observed SHA or reflog check exists anywhere in the design to tell the two
apart. This needs only the PR author's own ordinary push and body-edit rights on
their own open branch: no admin, no fork.

**Required fix.** Correct the disposition to state honestly that this combination
is *not* detectable once both channels are erased by the same actor, and then
either require branch protection forbidding force-push once S4 has run (for
example through a required status check set at resolution time) or introduce a
corroboration signal outside that actor's erasure surface — a CI check run or
deployment status posted at S4 is not deletable by an account that can
force-push. **This is an operator decision, not a Stage decision.**

#### F-21 (P1) — the plan ignores established prior art on post-merge worktree isolation

*Learnings Researcher, confidence medium. Prior art:
`docs/compound/workflow-issues/post-merge-worktree-regenerate-ignored-task-gate-evidence-2026-08-02.md`.*

The established pattern uses two distinct worktree paths — an implementation
worktree and a clean post-merge worktree spawned at the merge SHA — with an
explicit rule not to mutate the primary working tree. WP-1…WP-7 describes
fetching `refs/pull/<n>/head` and switching, but never states whether Ship's
pre-merge resolution commit is made in the implementation worktree or a separate
space. The compound library already solved this and the plan does not reuse it.

**Required fix.** State explicitly which worktree carries the resolution commit
and reconcile WP-1…WP-7 with the recorded pattern, or record why it does not
apply here.

### P2 findings

| Ref | Finding | Raised by |
|---|---|---|
| F-20 | U2 AC3 requires the canonical text "including its four ordering invariants" while the canonical definition declares **eight**. U2 installs the block verbatim and V6 checks exactness, so the criterion has no satisfiable reading and an implementer could drop invariants 5–8 — which are the RQ-6 reorder, pinned approval, explicit check evaluation and the RQ-12 record. Fix: say eight, and have V6/V2 assert all eight headings. | Scope, Correctness, Architecture, Agent-Native (4×) |
| F-22 | U1's heading still carries `*(root)*` and has no `Depends on: U2`, contradicting the dependency section's `U2→U1` edge and single-root-U2 statement. An extractor would run U1 first and create the dangling `RESOLUTION_PREFIX` reference the edge exists to prevent. | Correctness |
| F-23 | U5 AC9 restates all seven WP steps although U3 owns the canonical procedure and U5 is meant to invoke it by name. The duplicate can drift undetected because V2 only guards S1–S8 restatements. | Scope |
| F-24 | V19 is not a runnable grep: the file legitimately contains `resolution_commits` and `final_head` in the adjacent locator definition, and a plain grep cannot honour the "within this subsection" boundary. Extract the fenced schema or parse its YAML keys instead. | Scope |
| F-25 | U10 is a behaviour change, not a schema-only change: initializing `none` is new skill behaviour, AC2/AC3 restate a lifecycle U9 owns, and a re-run of `operational-closure` in `pre-merge` may clobber `OPEN`. The graph also lets U4 and U10 run concurrently after U9 — the exact unowned-squatter window U10 exists to close. Add edge `U10→U4`; require create-only initialization; forbid clobbering `OPEN`/`CLOSED`. | Architecture |
| F-26 | The `LAST_MILE_RECOVERY` entry rule requires both channels, but U3 — which owns that rule — has no acceptance criterion mentioning Channel B, the record, or OB-6. U3 can pass with a single mutable channel still installed. | Architecture |
| F-27 | The Constitution Check maps only workflow policies P-001…P-020 and never the workspace constitution's own Principles I–XI, though the Governance clause requires it. Principles VII (destructive-command approval) and VIII (explicit safety modes) are unmapped despite WP performing fetch, branch create/switch, fast-forward and commit. | Constitution |
| F-28 | WP-1 asserts only a clean tree and omits the P-011/P-016 worktree-topology gate before a committing re-entry that creates or switches branches and commits. P-011 is absent from both the Constitution Check and the frontmatter policy list. | Constitution |
| F-29 | OB-7 defines incomplete enumeration only as a non-zero exit or a truncated page. A shallow or grafted clone makes `git log` exit 0 while silently omitting commits past the horizon, so a deletion beyond it scans clean. Require `git rev-parse --is-shallow-repository` and fail closed or unshallow first. | Security |
| F-30 | Step 5 item 16 requires confirming the GitHub merge button in the rendered UI, which U4 AC9 retains unmodified. There is no agent-runnable equivalent, so the new S8 path depends on a human-only check. Verify merge-commit capability and mergeability by API, invoke merge explicitly in merge-commit mode, and assert two parents afterwards. | Agent-Native |
| F-31 | Three coupling points flagged from prior art: Step 5 reordering versus Step 6's assumptions about backlogit record state; WP-1…WP-7 versus the linked-worktree bounded-startup pattern; and the standing `backlogit shipment ship` non-termination risk whose P-015 safe-close fallback the plan does not mention. | Learnings |

### P3 findings

| Ref | Finding | Raised by |
|---|---|---|
| F-32 | Hardening D5's fold cites `U2 acceptance criterion 6`, which concerns locator phase-1 timing (D2). The criterion installing S1–S8 is U2 AC3. | Correctness |
| F-33 | `*(corrected in revision 9; see B below)*` has no referent — the document labels that item Round 8 finding **2**, not B. | Correctness |
| F-34 | `none` is used as a lifecycle state but the schema union is only `OPEN \| CLOSED`. Make it `none \| OPEN \| CLOSED` and state that an omitted field after S2 is OB-5, not "no obligation". | Architecture |
| F-35 | WP-4 passes a locator-sourced branch name to git without requiring argv-array execution or a `--` separator. Git's ref-naming rules limit exploitability, but the plan should foreclose shell interpolation explicitly. | Security |
| F-36 | Describing revision 9 as "bounded" while it adds two units, a requirement, a hardening and an in-scope file is generous. It is sanctioned by escalation finding D and decision revision 3, but the plan should say so explicitly so the framing is auditable. | Constitution |
| F-37 | Prior-art advisories: resolution-commit side effects on `.backlogit/stash.jsonl` and `docs/memory/**` must be committed or carried forward to avoid a dirty-worktree classification next session; and WP re-entry must leave no fetched branch or worktree state on disk before the next claim. | Learnings |

### Runtime verification and operational closure

Explicitly assessed, as the skill requires.

* **Runtime verification** is *specified* (V1–V19 including V12a–V12f) but two
  checks are **unsatisfiable as written** — V4(g) contradicts S4 (F-03) and V19
  is not a runnable grep (F-24) — and one check, **V16, tests behaviour the
  specification does not contain** (F-06). V18 does not execute the workflow it
  claims to verify (F-14). The verification suite therefore cannot currently
  discharge the plan.
* **Operational closure** is the principal structural gap. The plan makes
  `docs/closure/` load-bearing for a merge-gating obligation without reconciling
  it against the two mechanisms that already own that directory's lifecycle:
  `operational-closure` re-invocation (F-25) and P-020 `compact-context` archival
  (F-07). Closure readiness is **not** demonstrated.

### Required disposition

Per the governing directive and the gate table:

1. **FAIL.** Do **not** harvest. Do **not** assemble or claim a shipment. Do
   **not** activate, implement or merge. `harvest_authorized` stays `false`.
2. **RR-3 is re-opened** and recorded as unresolved in `## Residual risks`. Its
   closure requires an operator decision between a branch-protection remedy and
   an out-of-band corroboration signal (F-19), because both options reach past
   the reduced Defect-2 boundary this plan is confined to.
3. **The circuit is open again** at `review_attempts: 4`. This is the fourth
   consecutive FAIL. Stage does **not** self-authorize a fifth attempt; the
   remediation of these findings requires fresh operator authorization, and the
   RR-3 decision must be made before another revision is attempted, because
   F-13, F-14 and F-15 are all consequences of the mechanism RR-3 selects.

### Honest assessment for the operator

Revision 9 is a real advance on revision 8 in one specific, important way: the
verbatim Step 5 extract ends the four-round pattern of planning against a Step 5
that did not exist, and both personas who checked it confirmed it is accurate.
Escalation findings A, C and E are substantially remediated.

It fails on the other two. Finding B's remediation was correct in principle —
making S5–S8 universal — but introduced two *new* zero-checkpoint defects (F-04,
F-05) because the locator-dependent clauses were not made conditional at the same
time. And finding D's remediation, the RR-3 closure, does not hold: three
independent reviewers found the second channel is neither independent of the
first (F-13) nor executable (F-14) nor honest about its residual (F-19).

The directive anticipated this outcome: *"If no existing owned state surface can
safely support this without expanding into Defect 1 or new executable
persistence, do not fabricate a solution."* The venue chosen — `docs/closure/` —
was endorsed by the architecture review as correct. The mechanism built on it was
not sufficient. The narrowest honest position is the one now recorded: RR-3 stays
open, RQ-7 stays unweakened, and the operator decides.

## Plan Review — round 10

- **Attempt**: 5
- **Reviewed revision**: 10 (plan), 10 (hardening), 4 (decision)
- **Reviewed HEAD**: `23731cba156b8aee3334d65b7ed58ff91837aa55`
- **Panel**: four independent reviewers, cross-model (GPT-5.6 Sol — correctness;
  Claude Opus 4.8 — constitution and policy; Grok 4.6 — scope boundary and
  maintainability; Gemini 3.8 Flash — security and ops risk). Each read the plan,
  hardening, decision, and the live agent/instruction files independently, with no
  shared context and no access to each other's reports.
- **Mandated probes**: Channel B with an emptied PR body; ruleset drift and bypass;
  zero-checkpoint versus nonzero paths; P-020 ordering; the verbatim Ship Step 5
  extraction; D14 discharge; P-010 ambient-branch independence. All seven were
  executed. Results are recorded per probe below.
- **Verdict**: **FAIL** — unanimous across all four reviewers.
- **Counts**: **P0 = 4**, **P1 = 17**, P2 = 9, P3 = 5.
- **Gate outcome**: **BLOCKED**. Not harvestable. Not mergeable.

### What revision 10 did close

Recording this first, because it is real and should not be relitigated in round 11.

- **The round-9 P0 (F-11) is closed.** All four reviewers independently confirmed it.
  U8 AC2 now derives merge-authority from the selected checkpoint's own validated
  `context.pr` / `context.branch`, explicitly prohibits `gh pr list --head`,
  `git branch --show-current`, and `git rev-parse --abbrev-ref` as safety-predicate
  inputs, and permits ambient branch data only for logging. No ambient-branch read
  survives anywhere as a predicate input. This probe passes cleanly.
- **The Ship Step 5 structural extraction is confirmed accurate.** An independent
  re-derivation from `.github/agents/_ship.agent.md` confirmed the duplicated item
  number `7` and confirmed that items 7b/7c currently precede the second item 7 and
  items 8/9/10. The structural claim is true.
- **Fail-closed behaviour on the API surface is correct.** Empty effective-rules
  response, 404, 403, rate limit, network error, schema error, and unencoded `/` all
  halt. An empty `[]` is correctly read as *uncovered*, not as *unrestricted*.
- **The F-09 splits are genuine seams, not relabelling.** U3→U3+U11, U4→U4+U12,
  U9→U9+U13 each divide along a real boundary (discovery/recovery, Step 5/Step 6,
  schema/discovery). Two reviewers confirmed this explicitly, while still finding the
  residual units oversized — see F-09 below.
- **Abandoned-ID hygiene is clean.** No unit, verification, or traceability row uses
  `143-F`, `143-S`, or `143.001-T`…`143.014-T` as a live implementation target. The
  plan remains harvestable into fresh IDs.
- **Frontmatter counts reconcile.** `task_count: 14`, `sub_epic_count: 3`
  (E1=5, E2=7, E3=2), `dependency_edge_count: 22` against the direct-edge list,
  acyclic with roots U0 and U2.

### P0 findings

#### F-01 (P0) — U13 installs a different Channel B protocol than the canonical block, and reinstates the discharge-rejection bug the canonical block exists to prevent

Found independently by three of four reviewers.

The canonical `RESOLUTION_OBLIGATION_RECORD` Channel B defines B0–B7 as: B1 routes
the *scan target* by PR state; B2 fetches the retained ref; B3 reads the tree
including `docs/archive/closure/`; B4 walks history; B5 binds transitions; B6 proves
ancestry. U13 AC3 installs a materially different sequence: B1 is an unconditional
`origin/main` tree scan, B2 is the ruleset proof, B3 is the fetch, B4 is a tree read
that **omits `docs/archive/closure/`**, B5 is history, B6 is binding. The plan header
asserts the protocol is "not restated differently anywhere." It is.

Worse, the canonical block splits provenance into PV-B4 (introduction) and PV-B6
(discharge) precisely because a `CLOSED` transition is committed on
`post-merge/{feature_slug}` while the record's embedded `pr` and `branch` still name
the *implementation* PR — the plan's own rationale states that a single "embedded `pr`
equals the PR it was read from" rule "structurally rejected every correct discharge."
U13 AC5 reinstalls exactly that rejected rule: PV-B4 requires `pr` to equal the PR the
record was read from, and PV-B6 requires `branch` to equal that PR's live
`headRefName`. Under U13 as written, POST-B can never pass provenance validation.

This is round-9 finding F-15 re-opened inside the unit that was created to fix it.
Because task cards are declared exact (V6), an implementer installs U13's text, not
the canonical text.

**Remediation**: U13 AC3 and AC5 must reference the canonical block by name and
reproduce it verbatim — including the `docs/archive/closure/` follow and the
introduction/discharge provenance split — or the canonical block must be deleted and
U13 made the single source. One of the two must go.

#### F-02 (P0) — U13 AC6 halts on the exact state the residual-window machinery was built to handle

U13 AC6 states that "an `OPEN` record whose PR has since merged is the 139-S shape and
halts." That is wrong. The 139-S shape is a resolution commit that is **not** an
ancestor of `main`. An `OPEN` record whose introducing commit *did* merge is the
normal RQ-7 interval: the merge succeeded and POST-B has not yet been written. The
canonical Channel B B1 routes merged PRs to `origin/main` for exactly this reason, and
the canonical `LAST_MILE_RECOVERY` merged + `RESOLUTION_PUBLISHED` row says to assert
ancestry against `origin/main` and then *continue the P-001 closure set*.

U13 turns the protocol's own happy-path interval into a permanent operator halt.

**Remediation**: Align U13 AC6 with canonical B1 and the LMR merged row — merged plus
`OPEN` plus introducing commit ancestor of `origin/main` continues to POST-B; halt only
when ancestry fails.

#### F-03 (P0) — The fetch and ancestry contract is self-contradictory; V22 and U11 cannot both pass

Found independently by two reviewers.

The canonical `LAST_MILE_RECOVERY` step 1b uses
`git merge-base --is-ancestor <sha> FETCH_HEAD`; the resolution-state classifier uses
`git cat-file -e FETCH_HEAD:<file>`; WP-3 and U11 AC9 use
`git fetch origin refs/pull/<pr>/head` with **no destination refspec**. Meanwhile U5
AC2 forbids bare `FETCH_HEAD`, hardening D11 and the D14 discharge both claim the
target is a unique retained ref, and V22 requires "zero occurrences of bare
`FETCH_HEAD`" and "zero fetches without a `:refs/autoharness/scan/pr-<n>` destination."

U11 is required to install the canonical block *and* to pass V22. Those outcomes are
mutually exclusive. Three different fetch/ancestry protocols are specified across the
canonical block, the units, the hardening document, and the verification suite.

This also means **hardening D14's discharge claim is factually false**: it asserts U11
fetches to a unique retained ref; U11 AC9 does not.

**Remediation**: One fetch form and one ancestry target, everywhere. Retained refs are
the correct choice (they are what makes the multi-candidate scan safe). Purge
`FETCH_HEAD` from the canonical `LAST_MILE_RECOVERY` block, WP-3, the classifier, and
U11 AC9; keep V22 as the enforcement grep.

#### F-04 (P0) — U0's ruleset, as specified, breaks branch cleanup and rebase repository-wide, and the plan does not acknowledge it

The ops prerequisite specifies `deletion` and `non_fast_forward` over
`refs/heads/feat/**`, `refs/heads/chore/**`, and `refs/heads/post-merge/**`, with
`bypass_actors: []` and `current_user_can_bypass: never`.

- The `deletion` rule in a GitHub ruleset forbids **any** deletion of a matching ref,
  merged or not. With no bypass actors, neither a human, nor Ship, nor GitHub's
  `delete_branch_on_merge` automation can delete a merged `feat/*`, `chore/*`, or
  `post-merge/*` branch. `gh pr merge --delete-branch` fails. Every branch ever created
  accumulates permanently. This directly contradicts `_ship.agent.md`'s own branch
  management rule, which deletes feature and closure branches after their PRs merge.
- The `non_fast_forward` rule makes every in-progress working branch strictly
  append-only from first push. Rebasing onto `main` to resolve conflicts, squashing WIP
  commits, and `git commit --amend` followed by `--force-with-lease` are all rejected by
  the forge. The repository's own `git-merge.instructions.md` prescribes rebase flows
  that this rule would break.

The plan treats force-push and deletion exclusively as adversarial erasure vectors. It
never enumerates the legitimate workflows they also serve, never mitigates them, and
its rollback statement ("the ruleset is re-configurable") is not a rollback procedure —
rolling the ruleset back immediately trips OB-8 and halts every subsequent agent
session.

This is a finding against the *operator-selected mechanism*, not merely against its
write-up. It is reported plainly per the directive's instruction to gate honestly.

**Remediation, in order of preference**:

1. **Narrow the ref scope.** The property actually needed is that the obligation
   record's introducing commit stays reachable. That does not require immutability of
   working branches. Pushing an immutable marker ref — `refs/autoharness/obligation/{shipment_id}`
   or an annotated tag — at S4 and protecting *that namespace* delivers the same
   reachability guarantee with near-zero blast radius. Working branches stay fully
   rebaseable and deletable.
2. If branch-level protection is retained, scope it to `post-merge/**` only (the branch
   that carries the `CLOSED` transition), and permit bypass for repository
   administrators and GitHub's branch-cleanup automation.
3. Whatever is chosen, U0 must carry the exact API payload, the exact rollback payload,
   and an explicit enumeration of the workflows it breaks, so the approving operator
   approves the real consequence rather than an abstract requirement.

### P1 findings

#### F-05 (P1) — Channel B candidate selection is circular: PV-8 cannot be evaluated from API fields alone

The candidate diagram applies PV-1, PV-2, PV-3 **and PV-8** "using API FIELDS ALONE"
before any body or tree is read. But PV-8 admits a record "naming shipment X" only if
the PR is shipment X's backlog-recorded implementation PR. The shipment name comes from
*inside the record*, which has not been fetched yet. The GitHub pull-request API returns
no shipment field. The prefilter as specified cannot run.

Secondarily, no live backlog field is named that would carry the "backlog-recorded
implementation PR" binding, and no unit populates one.

**Remediation**: Prefilter with PV-1…PV-3 only; fetch and parse; then apply PV-8 against
a specifically named, populated backlog field. Add the unit that populates it, or drop
PV-8.

#### F-06 (P1) — With PV-8 removed from the prefilter, the trusted candidate set becomes every PR in the repository, and OB-2/OB-8 then halt on ordinary contributor history

This is the operational consequence of F-05, and it is severe enough to stand alone.
Once PV-8 cannot prefilter, the trusted set is every same-repo open and closed-unmerged
PR. Then:

- Any open PR on a non-Ship branch (`docs/*`, `dependabot/*`, a hotfix) returns `[]`
  from the effective-rules probe, and B2 halts under OB-8.
- Any closed-unmerged PR whose head branch was deleted — GitHub's default behaviour —
  hits B1's "if the branch is gone, that is OB-2, not clean" and halts.

A single abandoned contributor PR permanently blocks every future agent session in the
repository. The mechanism fails closed so aggressively that it becomes a
denial-of-service on the harness itself.

**Remediation**: Bound the candidate set to PRs positively bound to active shipments
before OB-2 and OB-8 are armed. Coverage and branch-existence obligations must apply
only to PRs the protocol actually owns.

#### F-07 (P1) — An `OPEN` record on a PR that is closed without merging has no discharge path and deadlocks startup forever

POST-B can only run on the `post-merge/{feature_slug}` branch of a **merged** PR. If a
unit executes S4, publishes `OPEN`, and the PR is then abandoned — re-scoped, superseded,
or failed — there is no defined transition to `CLOSED`, no `ABANDONED` state, and no
authorized cancellation procedure. U13 AC6 classifies it as an unrecoverable orphan and
halts. Every subsequent session re-discovers it and halts again.

The four-row lifecycle (`none→none`, `none→OPEN`, `OPEN→CLOSED`) has no terminal state
for abandoned work.

**Remediation**: Add an explicit, authorized abandonment transition with its own
provenance requirements, or forbid publishing `OPEN` until merge is imminent.

#### F-08 (P1) — `RESOLUTION_POSTCONDITION` POST-B is ordered both before and after P-020 compaction

Found independently by two reviewers.

The canonical entry condition requires "the FULL required post-merge closure set" first.
POST-B's closure PR must be merged to discharge the obligation. Yet POST-B must precede
compaction. U12 AC5 says POST-B runs "only after the full P-001 post-merge closure set
is complete and verified, including the P-020 compaction record"; U12 AC6 says POST-B's
`CLOSED` write "completes before any compaction or archival that can move the closure
artifact."

Both cannot hold. Literal execution either writes POST-B after its carrier merged —
recreating the exact orphaning defect this plan exists to fix — or violates its own
entry condition. POST-A can also mark the locator `RECONCILED` before POST-B has merged.

Round-9 F-01 and F-07 are therefore only partially closed: the "commits nothing"
contradiction is gone, but the ordering contradiction it was entangled with is not.

**Remediation**: Fix one total order — POST-B commit, then closure-branch review, push,
merge, verify that merge, then P-020 compaction, then POST-A locator write, then final
P-001 completion. State it once, in the canonical block, and have U12 reference it.

#### F-09 (P1) — The P-020 compaction exclusion is declared on a surface that does not own candidate selection

P-020 makes `compact-context` the owner of compaction candidate selection. The plan's
changed-file set includes `operational-closure/SKILL.md` and the instructions file, but
**not** `.github/skills/compact-context/SKILL.md`. The path-stability invariant — that
`OPEN` and `none` artifacts are excluded from compaction candidates — is therefore
written where it is not enforced. An unmodified `compact-context` can archive an aged
`OPEN` closure artifact and move the record out from under OB-1.

**Remediation**: Bring `compact-context/SKILL.md` into scope as its own unit, or replace
the exclusion with an ordering guarantee that does not depend on candidate selection
behaviour.

#### F-10 (P1) — The history probes cannot detect the transitions they exist to detect

Found independently by two reviewers.

B4 uses `git log -S'resolution_obligation'`. Pickaxe `-S` detects a change in the number
of occurrences of the string. The key `resolution_obligation` is present in `none`,
`OPEN`, and `CLOSED` states alike, so its occurrence count does not change across
`none→OPEN` or `OPEN→CLOSED`. Neither transition is found.

U9 AC7 is worse: it searches `-S'"resolution_obligation": "OPEN"'` — JSON syntax —
against the YAML schema U9 AC1 installs (`resolution_obligation:` / `  status: OPEN`).
It can never match.

The canonical recovery procedure ("Recovering resolution commits without a SHA field")
does the right thing — walk `git log --reverse <scan_ref> -- <path>` and parse
parent/child YAML — but U9 and B4 install something else.

**Remediation**: One executable procedure: enumerate commits touching the artifact path
on the retained scan ref and parse the YAML `status` on each side. Delete both pickaxe
forms.

#### F-11 (P1) — Channel B cannot distinguish merged from closed-unmerged PRs with the fields it collects

B1 routes on `state == MERGED` versus `state == CLOSED, unmerged`. But U3's exact REST
projection collects `state` and not `merged_at`, and the GitHub pulls-list API reports a
merged PR as `state: "closed"` with a non-null `merged_at`. There is no `MERGED` value
to match. The router cannot decide between scanning `origin/main`, scanning a live head,
and reporting an unrecoverable orphan.

Additionally, `origin/main` is never explicitly refreshed before the merged-history scan,
so that scan can read a stale local ref.

**Remediation**: Add `merged_at` to the projection; define merged as `merged_at != null`;
add an explicit `git fetch origin main` before the `origin/main` scan.

#### F-12 (P1) — S3.5 permits bypass actors that U0 AC3 and RQ-13 forbid

Found independently by two reviewers.

RQ-13 and hardening D16 require "no bypass actors" and halt on "any bypass capability."
U0 AC3 requires `bypass_actors` to be empty. V21 and U13 use the strong wording. But the
canonical S3.5 predicate accepts `.bypass_actors is empty OR contains no actor the acting
identity holds, at any bypass_mode`.

An administrator who later adds a CI service account or an admin team to `bypass_actors`
leaves S3.5 passing while a third party retains the ability to force-push or delete the
branch — defeating the only property the ruleset was bought for.

**Remediation**: S3.5 must require `bypass_actors` length zero and
`current_user_can_bypass == "never"`. Align every restatement.

#### F-13 (P1) — No ruleset proof is performed on the `post-merge/*` branch that carries the `CLOSED` transition

U0 AC1 includes `refs/heads/post-merge/**` on the justification that this branch carries
the `OPEN → CLOSED` discharge. But S3.5 lives inside `RESOLUTION_PREFIX`, and the plan
states "no step of `RESOLUTION_PREFIX` may be performed on a merged branch; the prefix
ends before merge." U12 creates `post-merge/{feature_slug}` and commits POST-B without
ever invoking S3.5 or any equivalent probe on it.

The discharge commit is written to an unverified branch. Equally, merged records route to
`origin/main` with no runtime re-proof that `main` is still protected, so main-ruleset
drift is outside OB-8's implemented route.

**Remediation**: Require an effective-rules proof on `post-merge/{feature_slug}` before
POST-B is committed, and on `origin/main` before any merged-history scan is trusted.

#### F-14 (P1) — The S7 late-checkpoint loop is illegal under the record lifecycle

S7 sends a nonzero re-enumeration back through S3 → S3.5 → S4. After an earlier nonzero
S4, however, the record is already `OPEN`, and the lifecycle admits only `none→none`,
`none→OPEN`, and `OPEN→CLOSED`. There is no `OPEN→OPEN` amendment, and the locator would
have to regress from `RESOLUTION_PUBLISHED` back to phase 1, which the three-phase
locator forbids.

The advertised race recovery is therefore executable only when the original count was
zero — the case that needed it least.

**Remediation**: Either halt on late checkpoints discovered after a nonzero S4, or define
an explicit, auditable `OPEN` amendment transition and a locator re-publication rule.

#### F-15 (P1) — Stage cannot satisfy U8's mandatory carrier binding, because Stage checkpoints predate the PR that carries them

U8 makes `context.pr` and `context.branch` mandatory at every Stage checkpoint creation
and prohibits checkpoint creation without an open staging PR. But live Stage creates
checkpoints throughout its session (Session-continuity, mid-session checkpoints), Stage's
own role boundary forbids it from creating or pushing PRs, and the Orchestrator creates
the staging PR only after Stage completes. A mid-session Stage checkpoint cannot know its
future PR number. U8 nevertheless asserts that no other Stage behaviour changes.

Separately, even when an open PR does exist, the one-time `state == OPEN` read is a TOCTOU:
the PR can merge between the read and the resolve.

Note that the **round-9 P0 itself is closed** — the predicate no longer reads the ambient
branch. This finding is about whether the replacement binding can be produced at all.

**Remediation**: Name who produces and durably backfills the carrier binding and when;
reconcile with Stage's mid-session checkpoint obligations; add a final live-state plus
ancestry guard immediately before resolve.

#### F-16 (P1) — The "standing executable" drift check V20 has no implementation and cannot run inside markdownlint

Found independently by two reviewers.

Round-9 F-08 required an executable drift check across the duplicated protocol surfaces.
V20 claims semantic role extraction and comparison "run as part of V1's lint pass." V1 is
markdownlint. The plan's own constraints forbid adding scripts. No unit adds a checker or
a lint rule. V20 is therefore another one-time manual instruction wearing the label of a
standing check.

V20 also conflicts with U4 AC1, which requires Ship's Step 5 not to restate "any of its
steps, orderings or rationale" — if Step 5 does not state an order, V20 has nothing to
extract from it; if it does, AC1 fails.

**Remediation**: Either bring a real checker into scope and wire it to an existing gate, or
delete V20 and record the drift exposure honestly as a residual risk. Resolve the AC1/V20
conflict either way.

#### F-17 (P1) — Several verifications are unsatisfiable against a correct implementation

- **V8** requires every enumerated PR to have a non-empty body; **V17** deliberately
  empties one, and the live repository currently contains an empty-body PR. V8 as written
  fails on correct behaviour.
- **V14** asserts "the locator, the resolutions and the `OPEN` obligation record ride one
  commit." The locator is PR-body metadata, published before and after that commit; it
  does not ride a commit at all.
- **V23** asserts no mutation after the S5 review except POST-A/POST-B, while live Step 6
  performs backlog archival, closure generation, documentation updates, compound refresh,
  compaction, and index sync.
- **V19** parses "the schema block and its lifecycle text" as YAML; the lifecycle is a
  markdown table and the schema uses `none | OPEN | CLOSED` as a prose union. Neither
  parses.

**Remediation**: V8 → assert body-field presence, not non-emptiness. V14 → separate
PR-body metadata from commit contents. V23 → scope to the implementation branch and the
review cycle it belongs to. V19 → parse the schema mapping only, or delete.

#### F-18 (P1) — U4 AC6 forbids post-S4 mutation that the canonical gate-failure loop requires

U4 AC6 states "no branch-mutating step remains after S4" and that every mutating item
sits in S1 or S2. The canonical gate-failure loop pushes a remediation commit *on top of*
the resolution commits and re-enters at S5. Step 5 item 2 (the session-memory commit) is
mutating and is not mapped into S1 or S2 in U4's extract.

**Remediation**: Scope AC6 to the ordinary path, name the remediation-loop exception
explicitly, and map item 2.

#### F-19 (P1) — The Constitution Check maps a constitution this workspace does not have

Found independently by two reviewers.

The plan's "Principles I–XI" table lists *Specification before implementation, Single
source of truth, Traceability, Incremental delivery, Test/verification first,
Reversibility, Fail closed, Least privilege, Auditability*. The actual
`.github/instructions/constitution.instructions.md` principles are *Safety-First Rust,
Test-First Development, Workspace Isolation and Security Boundaries, CLI Workspace
Containment, Structured Observability, Single Responsibility, Git-Friendly Persistence,
Agent Context Efficiency, Merge Commit History Preservation*. Only VII and VIII coincide.

Round-9 F-27 asked for the workspace constitution's own principles to be mapped.
Revision 10 answered it by inventing a plausible-sounding different rubric. F-27 is not
discharged, and genuinely relevant principles go unchecked — notably **IV (CLI Workspace
Containment)**, which U0 violates most directly by reaching outside the workspace to
mutate forge settings.

**Remediation**: Map the real Principles I–XI, marking genuine N/A explicitly, and add a
real check for IV against U0.

#### F-20 (P1) — P-019 states that the harness never edits rulesets; U0 is not reconciled with it, and P-019 is cited nowhere

`workflow-policies.md` P-019 states verbatim that ruleset configuration "remains an
operator configuration action — the harness never edits rulesets." U0's ProposedAction box
says "the **implementing agent** MUST obtain explicit operator/admin approval immediately
before **applying this change**," implying an agent applies it. P-019 appears in none of
the three artifacts' `policies:` frontmatter and in no Constitution Check row.

This compounds with a least-privilege problem: creating a ruleset requires
`Administration: write`. If Ship applies U0, Ship must hold an admin token — a
significant and unnecessary privilege escalation for a documentation harness. If the
operator applies it, U0 is a human prerequisite, not one of fourteen agent tasks, and it
must carry the exact payload the operator will execute.

**Remediation**: Add P-019 to the Constitution Check. Reclassify U0 explicitly as a manual
operator prerequisite performed outside the agent pipeline, with its exact create payload
and exact rollback payload inline. Forbid any agent from holding ruleset-write credentials.

#### F-21 (P1) — The Definition of Done can be satisfied while the load-bearing prerequisite is absent

Found independently by two reviewers.

Decision §7 permits the ruleset to have "been explicitly deferred with the consequence
recorded." But S3.5 halts before obligation publication on an uncovered branch, and the
plan's own measured evidence shows Ship source branches return `[]` today. Deferring U0
and shipping U1–U13 yields a release that is "done" while every nonzero-checkpoint unit
halt-loops at S3.5.

This also collides with the traceability contract's "exactly one owner per requirement":
RQ-13 is assigned to "U0 + U2."

**Remediation**: DoD must require U0 applied and V21 green, or require the S3.5/RQ-13
durability claims to be withdrawn. Remove the "deferred but done" branch. Assign RQ-13 one
owner and classify the other as enforcement.

### P2 findings

#### F-22 (P2) — Same-file units are not fully serialized, and the count is wrong

The file-serialization paragraph says "Four units share
`github-pr-automation.instructions.md`" and then lists five (U2, U3, U11, U9, U13),
calling them strictly sequential. The edge list has `U3→U11` and `U3→U9→U13` but neither
`U11→U9` nor `U11→U13`. The claim that U11/U13 is the only pair without a direct edge is
also false — U11/U9 lack one too. Dependency-driven execution could edit the canonical
file concurrently.

#### F-23 (P2) — U0's branch globs are stated two incompatible ways

The `BRANCH_PROTECTION_PREREQUISITE` table uses `feat/*`, `chore/*`, `post-merge/*`; U0
AC1 uses `refs/heads/feat/**`, `refs/heads/chore/**`, `refs/heads/post-merge/**`. In
GitHub ruleset `ref_name.include`, `*` and `**` are not equivalent, and the `refs/heads/`
prefix changes matching. One include list, derived from `_ship.agent.md` with line
citations, must be used by U0, S3.5, and V21 alike.

#### F-24 (P2) — Channel B has no adoption boundary for pre-existing closure artifacts

B3 halts on any closure artifact whose schema is invalid, while OB-5 exempts only
artifacts "created after S2" — a test that is never made deterministic. The working tree
currently contains 177 closure and archive artifacts, none of which carry
`resolution_obligation`. A literal implementation either halts on all of them or exempts
them by an unspecified rule.

#### F-25 (P2) — The P-009 guardrail is described as unmodified and as amended

The P-018 row says "the merge-commit guardrail item is retained unmodified" and S8's tail
says the P-009 item "applies unchanged," while U4 AC10 and V4(f) both add an API-side
merge-commit-mode check and a two-parent assertion to that same item. This is the same
defect class as round-9 F-02 (the "item 15 unmodified" contradiction), which revision 10
swept for — and missed in its item-16 form.

#### F-26 (P2) — The drift surface is five layers deep and V20 covers one of them

One protocol rule now lives in the canonical block, in unit acceptance criteria, in
verification expected-results, in hardening D1–D16, and in the decision's RQ table.
Changing a single rule — "do not use `FETCH_HEAD`" — already requires six edits, and the
canonical/unit disagreements catalogued in F-01, F-03, F-10 and F-12 are direct products
of this. V20 compares only S1–S8 role ordering and would have caught none of them.

#### F-27 (P2) — U4 AC3's zero-checkpoint omission list is narrower than the canonical rule

The canonical rule omits segments S3.5 and S4 in their entirety for a zero-checkpoint
unit. U4 AC3 enumerates five specific omissions and misses the `OPEN` record write and the
remote-head proof.

#### F-28 (P2) — The Step 5 extract is a normalized mapping presented as verbatim

The structure is confirmed accurate, but the extract states that item 15 re-fetches "only"
the P-018 verdict and `headRefOid`; the live item 15 also conditionally re-runs §1.9 when
HEAD has changed. The table also annotates item 2 as "yes (commit)" where live item 2 says
only to write the memory file. Both are inferences, so "verbatim" overclaims.

#### F-29 (P2) — Rollback of U0 is undefined and self-trapping

U0's reversibility note observes that the ruleset is re-configurable, but there is no
rollback payload and no procedure for unwinding in-flight units. If U0 must be rolled back
because of F-04, in-flight sessions halt at S7 and every future startup halts at B2 under
OB-8, with no defined recovery.

#### F-30 (P2) — The repository-supported ruleset fallback path is unpaginated

The effective-rules probe is correct, but the `GET /rulesets` fallback is issued without
`--paginate`. Truncation produces a false halt. This is the same defect class as round-9's
V8 pagination finding, in a new location.

### P3 findings

- **F-31 (P3)** — The Constitution Check row is titled "P-011 Worktree topology"; P-011 is
  actually named "Branch-Before-Mutation." The substance attached is right; the label is wrong.
- **F-32 (P3)** — U12's header says "Depends on: U4, U9, U10" but the 22-edge list contains
  only `U4→U12`; U9 and U10 reach U12 transitively. Either add the direct edges and update the
  count, or annotate the transitivity.
- **F-33 (P3)** — U0 declares `action_class`, `ActionRisk`, `approval_required`, approver,
  surface, and reversibility, but never the `ActionResult` state the Action Contract expects
  (`planned` at plan time).
- **F-34 (P3)** — Revision 10 repeatedly describes round 9 as "15 P1"; the retained round-9
  block contains 18 P1 headings. It also calls the seven-persona panel "five-persona" in one
  place.
- **F-35 (P3)** — Hardening D1–D4 cite stale U2/U4 criterion numbers after the revision-10
  renumbering (for example, phase-1 timing is U2 AC7, not AC5; non-self-reference is AC10, not
  AC3/AC8).

### Results of the seven mandated probes

| Probe | Result | Basis |
|---|---|---|
| Channel B with an emptied PR body | **FAIL** | The candidate set is genuinely body-independent — that part works. But PV-8 cannot prefilter (F-05), the canonical and U13 algorithms differ (F-01), the transition probes cannot match (F-10), merged/closed-unmerged cannot be distinguished (F-11), and `origin/main` is never refreshed. The procedure is not executable end to end. |
| Ruleset drift and bypass | **FAIL** | `[]` is correctly read as uncovered and the API surface fails closed. But S3.5 permits non-acting bypass actors that RQ-13/U0 AC3 forbid (F-12), no proof runs on `post-merge/*` or `origin/main` (F-13), and an admin can disable, rewrite, and restore protection between observations. |
| Zero-checkpoint vs nonzero | **PARTIAL / FAIL** | The zero path is now genuinely satisfiable: four always-present HEAD values, vacuous-safe ancestry, a complete-zero S7 result, and explicit POST-A/POST-B no-ops on `none`. Round-9 F-04/F-05 are closed. The **nonzero** path is not satisfiable: POST-B is ordered both before and after compaction (F-08), and a late checkpoint after a nonzero S4 cannot legally re-enter S4 (F-14). |
| P-020 ordering | **FAIL** | F-08 (contradictory order) and F-09 (exclusion declared on a surface that does not own candidate selection). V23 also excludes Step 6 mutations that genuinely occur (F-17). |
| Exact Ship Step 5 extraction | **PARTIAL** | Structure independently re-derived and confirmed: the duplicated item `7` is real, and 7b/7c do precede the second 7 and 8/9/10. The "item 15 re-fetches only…" claim and the item-2 commit annotation are inferences, so the "verbatim" label overclaims (F-28). |
| D14 discharge | **FAIL** | U11 AC9–AC12, U5 AC9 and V12/V12a–g correctly invoke the named procedure rather than restating it — that structural requirement is met. But WP-3 and U11 AC9 still fetch without a destination refspec and assert against bare `FETCH_HEAD`, contradicting V22 and falsifying D14's own discharge claim (F-03). |
| P-010 ambient-branch independence | **PASS** | Unanimous. U8 AC2 keys off the checkpoint's validated `context.pr`/`context.branch`, prohibits `gh pr list --head`, `git branch --show-current`, and `rev-parse --abbrev-ref` as predicate inputs, and confines ambient branch data to logging. The round-9 P0 is genuinely closed. The separate question of whether the binding can be produced is F-15, not a regression of F-11. |

### Reviewer convergence

Findings reached independently by more than one reviewer, with no shared context, and
therefore carrying the highest confidence:

| Finding | Reviewers | Models |
|---|---|---|
| F-01 Channel B canonical/U13 divergence | 3 of 4 | GPT-5.6 Sol, Grok 4.6, Gemini 3.8 Flash |
| F-03 `FETCH_HEAD` contract unsatisfiable | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| F-08 POST-B ordering contradiction | 2 of 4 | GPT-5.6 Sol, Claude Opus 4.8 |
| F-10 pickaxe probes cannot match | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| F-12 S3.5 bypass weaker than RQ-13 | 2 of 4 | GPT-5.6 Sol, Gemini 3.8 Flash |
| F-16 V20 is not a standing check | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| F-19 fabricated constitution mapping | 2 of 4 | Claude Opus 4.8, Grok 4.6 |
| F-21 DoD permits a deferred prerequisite | 2 of 4 | GPT-5.6 Sol, Grok 4.6 |
| Residual oversizing of U2/U4/U11 | 3 of 4 | Claude Opus 4.8, Grok 4.6, GPT-5.6 Sol |
| P-010 probe passes | 4 of 4 | all |

### The structural judgement, stated plainly

Two reviewers reached the same conclusion by different routes, and it is the most
important thing in this review.

The defect this plan exists to close — Defect-2 — is that Ship resolves checkpoints
*after* the carrying PR merges, orphaning the resolution commit on a dead branch. The
load-bearing fix is small and is already fully specified in this plan: move resolve before
merge, push it, re-run the review at that HEAD, assert ancestry at merge, and forbid
post-merge resolution in both Ship and Stage.

Everything built on top of that — the obligation record, Channel B's B0–B7, PV-1…PV-8,
OB-1…OB-8, `LAST_MILE_RECOVERY`'s WP-0…WP-7, the seven-part S8 bar applied to every
merge including zero-checkpoint chores, and now U0's repository-wide ruleset — exists to
*detect* a residual window rather than to *prevent* the defect. And by the plan's own V17,
the detection path terminates in a halt: it does not recover, it fail-closes. The
machinery buys detection of PR-body deletion and force-push at the cost of four P0s, a
repository-wide workflow regression, and a startup path that halts on ordinary contributor
history.

Ten revisions of escalating residual-window machinery around a one-step ordering bug is
itself the finding. Round 11 should not attempt to repair thirty findings in place. The
recommendation from this panel is to take the smaller design back to the operator:

1. Keep the ordering fix, the locator, the merge-authority bar, and the ancestry assertion.
   These close Defect-2 and are the parts every reviewer found sound.
2. Replace the branch-ruleset prerequisite with an immutable marker ref or annotated tag
   pushed at S4 and protected in its own namespace. This delivers the same reachability
   property with near-zero blast radius, leaves working branches rebaseable and deletable,
   and avoids the P-019 conflict and the admin-token requirement entirely.
3. Demote Channel B, the obligation record, and the OB/PV invariant sets to an explicitly
   optional hardening layer scoped to PRs bound to active shipments — or drop them from this
   unit and record the residual honestly.

That is a recommendation to the operator, not a decision taken by this review. The gate
outcome stands on the findings regardless of which design is chosen.

### Gate

**BLOCKED.** 4 P0 and 17 P1 findings. The plan is not harvestable, and this PR is not
mergeable in its current state. No harvest, no shipment claim, no activation, and no merge
was performed. The eighteen unresolved Copilot review threads on PR #396 remain
unresolved by deliberate instruction; no comment cycle was started in this round.

<!-- plan-review-attempt: 5 -->
