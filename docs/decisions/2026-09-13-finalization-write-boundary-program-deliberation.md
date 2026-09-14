---
title: "Finalization write boundary — program deliberation (deep)"
doc_type: decision
date: 2026-09-13
agent: stage
depth: deep
status: decided
promotion: plan+queue
supersedes:
  - docs/decisions/2026-09-13-checkpoint-resolution-ordering-restage-decision.md
  - docs/decisions/2026-09-13-checkpoint-persistence-substrate-deliberation.md
  - docs/decisions/2026-09-13-finalization-freeze-and-pr-backed-pause-deliberation.md
source_stash_id: 4EF24729
branch: chore/checkpoint-resolution-ordering-restage
---

## Problem Frame

### The originating defect

Stash `4EF24729` (active, `kind: task`, `priority: medium`, `DEFERRED SCOPE EXPANSION`,
source refs `task=N/A, feature=142-F, shipment=139-S, PR=394 (merged), review-thread=N/A`):

> Resolving a tracked Ship checkpoint by committing the resolution only to an already-merged
> closure-PR branch (`post-merge/*`) leaves the checkpoint active on `main`/`origin/main` after
> merge, since further commits to a merged branch never reach `main` without a new PR.

Verified live this session: `.backlogit/checkpoints/` holds **24 git-tracked files**;
`backlogit checkpoint resolve <filename>` flips `status` inside a tracked JSON file and therefore
**requires a commit** to become durable on `main`.

### Why three prior attempts failed

| Attempt | Thesis | Terminal finding |
|---|---|---|
| 1 (`3cc8d5a5`) | Ordering discipline alone | `D1` partition failed: the incident checkpoint is in **both** the "committed" and "terminal pause" classes |
| 2 (`5550b1a0`) | Untrack `.backlogit/checkpoints/` (Option D) | Residual cycle via tracked `docs/memory/`; commit `ce3b2fba` proves checkpoint **and** memory landed in one HEAD-advancing merge-gate commit |
| 3 (`25e6961e`) | Finalization freeze + hybrid continuity | 2 P0 / 32 P1. Freeze is not a partition (P-005, P-021 C2, P-007 all write inside it); Stage-parity pillar unexecutable under P-010; open-PR check could *clear* a P-001 blocker |

Every attempt enlarged the same monolith. The recurring shape is identical: **a single plan tried
to settle several independent policy seams at once, and each seam's unresolved question resurfaced
as a P0/P1 in the next attempt.**

### The generalised defect

The checkpoint is one instance. The general statement, established across all three attempts:

> **Any tracked repository write performed after a PR's final reviewed/approved HEAD has no legal
> destination.** It either (a) advances HEAD and invalidates the gate that just passed
> (`commit_id == HEAD` re-arms Copilot — `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`),
> or (b) lands on a branch that is already merged and never reaches `main` (the `4EF24729` instance).

### Stable goals

| G | Goal |
|---|---|
| G1 | Every mandatory tracked write in the Ship/Stage lifecycle has exactly one legal, reachable destination |
| G2 | No bookkeeping-only push occurs inside an armed review gate |
| G3 | Checkpoint state on `origin/main` accurately reflects live session state |
| G4 | P-001 single-active detection is not weakened by any change made here |
| G5 | Each change is independently reviewable and independently shippable |

### Stakeholders

Ship agent (the mutating party), Stage agent (planning + intake), Orchestrator (routing + staging
gate), the operator (approval authority at every merge), and the backlogit tool (substrate owner).

### Non-negotiable policies in scope

P-001 (single-active + closure), P-005 (violation telemetry → memory checkpoints), P-007 (archive
restoration), P-009 (merge-commit-only), P-010 (role boundary), P-014 (readiness gate + operator
approval), P-015 (partial-feature safe-close), P-016 (single worktree), P-018 (Copilot-review
completion gate), P-020 (compact-context at closure), P-021 C1/C2/C3/C5/C6 (bounded fix scope and
deferred capture). Constitution IX (Git-Friendly Persistence) and XI (Merge Commit History).

### Success criteria

* S1 — A written taxonomy classifies every mandatory tracked write by lifecycle phase and names its
  legal destination. No write is left unclassified.
* S2 — Staging-PR creation has exactly one named owner that does not contradict P-010.
* S3 — Open-PR discovery can only **add** blockers. A formal proof obligation exists that it can
  never clear one.
* S4 — The originating checkpoint defect is closed by a change small enough to review in one pass.
* S5 — No package's acceptance depends on another package's *unreviewed* output.

### Explicit exclusions

* **Defect 1 — dark-mode same-scope continuation auto-routing.** Separate open deliberation.
  Out of scope here; not harvested here.
* **PR #396.** Historical evidence only. Not edited, closed, replied to, resolved, or merged.
* **`143-F` / `143-S` / `143.*`.** Abandoned (`143-S` was "Daemon liveness safety", unrelated).
  Never revived, reused, or re-parented.
* **Upstream backlogit source changes.** Evaluated below and found not required for the selected
  structure; re-evaluated per package rather than assumed.
* Crash-window hardening beyond the narrow ordering fix.

---

## Research Findings

### R1 — Ship's tracked-write sites, verified against live file text

`.github/agents/_ship.agent.md` (1109 lines):

| Site | Line | Write | Phase |
|---|---|---|---|
| Task memory checkpoint | 543 | `docs/memory/` | per-task, pre-gate |
| Session memory summary | 561 (Step 5 item 2) | `docs/memory/` | **before** PR creation — legal |
| P-014 readiness gate | 7b | — | gate arms here |
| P-018 Copilot gate | 7c | — | gate arms here |
| runtime-verification | Step 5 item **7** (dup) | evidence artifact | **after** 7b/7c |
| operational-closure | Step 5 item **8** | `docs/closure/` | **after** 7b/7c |
| Stash follow-ups (P-021 C2) | Step 5 item **9** | `.backlogit/stash.jsonl` | **after** 7b/7c |
| Push branch | Step 5 item **10** | — | HEAD advances **after** gates |
| Operator approval | Step 5 item 14 | — | — |
| Last-mile re-check | Step 5 item 15 | — | catches (10) but pays a full re-arm cycle |
| Backlog archival commit | 836 | `git add .backlogit/` | post-merge branch |
| compact-context (P-020) | 859 | `docs/memory/`, `docs/exec-plans/`, `docs/closure/`, `docs/archive/` | post-merge branch |
| Circuit-breaker record | 931 | `docs/memory/` | any time |
| Mid-session checkpoint | 1035, 1044 | `docs/memory/` + `.backlogit/checkpoints/` | any time |
| **Session end final memory** | **1057** | `docs/memory/` | **after** Step 6 item 10 (`checkout main`) |
| **Session end checkpoint resolve** | **1058** | `.backlogit/checkpoints/` | **after** Step 6 item 10 |

Step 5 contains a genuine **duplicate item number `7`**. Step 6.0 item 4 reads *"After all closure
work is committed, push the branch and create a PR."* Step 6 item 10 returns to `main` **after the
closure PR merges**. Session end items 1 and 2 therefore execute on `main` with **no PR in flight** —
this is the exact `4EF24729` instance, and it is **structural, not incidental**.

Stage's own `Session end` items 1 and 2 are identical → symmetric exposure.

### R2 — The decisive prior art the earlier attempts missed

`docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md`
(137-S dark-factory halt) already establishes a **sanctioned destination** for exactly this class:

> Three recognized carry-forward classes — `.backlogit/stash.jsonl` (append-only backlog intake),
> `docs/memory/**` (mandatory session continuity records), and intentional `.gitignore` updates —
> must be validated and landed through the **NEXT staging branch/staging PR**, merged, and main
> returned clean BEFORE Ship claims the release-unit shipment.
>
> *Prevention note*: "Session-end memory writes and stash updates are **inputs to the next staging
> round**, not merge-gate-time commits on the implementation branch."

This dissolves attempts 2 and 3's terminal blockers. The post-gate tracked write does **not** need a
new substrate and does **not** need a freeze exemption carve-out. It needs to be **routed to the next
staging round**, a mechanism that already exists (Orchestrator Step 1.5) and is already the
documented remedy for two of the three problem classes. Checkpoint resolution is a fourth member of
the same family, not a new problem.

Corroborating: `docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md`
Family 6 — *"Reviewed-HEAD/checkpoint metadata written as its own push is self-invalidating: the push
that records it advances the HEAD it names."* 4 of 11 review rounds on PR #385 were caused by
doc/PR-body-only pushes.

### R3 — The P-010 / Orchestrator Step 1.5 seam is real

`.github/policies/workflow-policies.md:211-249` (P-010):

* Stage MUST NOT *"Create, push, or merge pull requests"*.
* Ship MAY *"Create, update, and merge pull requests (with operator approval)"*.
* *"The Orchestrator agent must not perform Stage or Ship work directly — it routes to them as subagents."*

`.github/agents/_orchestrator.agent.md:270-297` (Step 1.5) instructs the **Orchestrator** to
*"Commit any uncommitted backlog files to a staging branch"*, *"Push the staging branch and create a
PR to `main`"*, and *"Attempt a direct push to `main` first."*

PR creation is Ship-classed work in P-010's own table. Step 1.5 therefore has the Orchestrator
performing Ship-classed work directly, which P-010's third sentence forbids. **Three agents, and
none of them is cleanly authorised to create the staging PR.** This is an independent contract
defect that exists today regardless of any freeze design — and R2's remedy routes straight through
it, so it must be settled.

### R4 — Open-PR enumeration is absent and its hazards are documented

Orchestrator Step 0 assesses shipments and stash only; there is **no open-PR enumeration anywhere**
in the Orchestrator. Yet `docs/compound/workflow-issues/ship-single-pr-serialization-and-stash-handoff-2026-05-14.md`
establishes: *"Open-PR state — not just backlog status — is the authoritative single-active blocker
signal."*

Two hazards constrain any design:

* `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` — `gh api` returns
  page 1 only; *"a page-1-only read is a silent correctness defect in gate logic."* Exhaustive
  pagination is mandatory.
* Attempt 3's P1: an unconditional *"zero open PRs → proceed"* rule would **clear** a P-001 blocker
  that exists for a different reason (merged unit with `compaction: pending` per P-020). A discovery
  mechanism must be strictly **additive**.

Live state confirms the hazard is not hypothetical: this repository currently has **#396**
(superseded, open, 34 unresolved threads) and **#390** (draft). A fail-closed "multiple carrying PRs"
rule would wedge the Orchestrator on every session start.

### R5 — Substrate facts (verified, correcting an earlier false claim)

* `.github/agents` = **23 tracked files**; `.github/instructions` = 34; `.github/skills` = 28;
  `.github/policies` = 2. `git check-ignore .github/agents/_ship.agent.md` exits 1.
  **Contract fixes in this repository are durable.** (Attempt 1's "generated from gitignored
  `.copilot/`" claim was false and is corrected here for the second time.)
* No `templates/` directory exists in this workspace. The authoritative policy catalog is
  `.github/policies/workflow-policies.md`, which defines through **P-021**. There is no P-022.
* `backlogit 1.10.1`; `features.shipments/checkpoints/dependencies/queue: true`.
  `features.sizing` is **absent from the registry**, but the CLI **does** expose `--size`,
  `--complexity`, `--size-source`, `--size-ruleset-version` as body-preserving, mutually exclusive
  update flags. Structured sizing is therefore achievable via a three-call sequence.
* `backlogit checkpoint resolve` takes a filename and no flags. No non-mutating resolve exists.
  No `--match-head-commit` exists anywhere; §1.9 HEAD pinning is record comparison.

### R6 — Live preflight state

Checkpoints enumerated **unfiltered, anomaly-first**: **24 total, 0 `needs_quarantine`,
0 quarantined, 0 empty `agent`/`status`, 0 `active`** (18 resolved / 6 abandoned) →
**ZERO-CANDIDATE NORMAL STARTUP**. No restore, resume, prune, or resolve. Hook queue polled for
`consumer_id: stage` → `events: []`, `derived_signals: []`; no ack issued. `backlogit sync` →
`INDEX_SYNC_OK` (1366 artifacts). Single worktree; branch
`chore/checkpoint-resolution-ordering-restage` @ `25e6961e` == `origin/`. Highest allocated
top-level ID is **142**; `143` is burned (abandoned `143-S` "Daemon liveness safety", no queue or
archive files remain, only `.backlogit/logs/143-S.jsonl`).

---

## Options Evaluated

### Option 1 — Strict prerequisite chain A→B/C→D→E

One linear chain: taxonomy first, then ownership and open-PR taxonomy, then the pause continuity
model, then the ordering fix.

* **Independence**: none. Every package is gated on its predecessor.
* **Blast radius**: each link inherits all upstream risk.
* **Reviewability**: good per link.
* **Incremental value**: **none until E ships.** The originating defect stays open through four
  serial review gates.
* **Fatal flaw**: A-first is *backwards*. A's whole content is "where do post-gate writes go", and
  R2 answers "the next staging round" — whose legality is B's question. Putting A first forces A to
  either pre-judge B or invent a substitute destination, which is precisely how attempts 2 and 3
  manufactured new architecture.

### Option 2 — Independent policy packages A/B/C, then D/E after convergence

Three parallel roots, then a convergence point.

* **Independence**: high for B and C; **false for A**, for the reason above.
* **Blast radius**: low per package.
* **Reviewability**: excellent.
* **Incremental value**: B and C ship immediately; both are standalone contract-defect fixes with
  value independent of the originating defect.
* **Flaw**: treating A as a root repeats attempt 3's P0 (a Stage-parity/destination pillar that is
  unexecutable because its destination's ownership is unsettled).

### Option 3 — Minimal E-only ordering fix, all residuals deferred

Plan only the narrow checkpoint-resolution reordering; defer everything else.

* **Independence**: trivially satisfied.
* **Blast radius**: smallest.
* **Reviewability**: excellent.
* **Incremental value**: closes the originating defect fastest — *if it can pass review*.
* **Fatal flaw**: **empirically disproven.** This is exactly attempt 1, plus six revisions on
  PR #396. It failed because the reordering has nowhere legal to move the write **to**. Moving
  resolution before the final reviewed HEAD requires knowing that a post-gate write is forbidden
  (A) and that a next-staging-round destination is legal (B). Without them the fix either reopens
  the gate or lands nowhere. Re-attempting it unchanged would be the ninth attempt at the same
  contradiction.

### Option 4 (synthesised) — Root-first DAG: {B, C} → A → D → E

Derived from R2 + R3. Reverses the A/B edge that Options 1 and 2 both get wrong, and keeps C as a
genuinely independent root.

* **B (staging PR ownership)** is a **root**: it is a live P-010 contract defect, provable from file
  text alone, with no dependency on the freeze question.
* **C (open-PR taxonomy)** is a **root**: purely additive detection, no dependency on A, B, D, or E.
* **A (write taxonomy)** depends on **B**, because its central disposition — "post-gate tracked
  writes are inputs to the next staging round" — requires that round to have a legal owner.
* **D (pause continuity)** depends on **A** (classification) and **C** (discovery-vs-authority split).
* **E (ordering fix)** depends on **D**.
* **F** excluded.

---

## Trade-off Comparison

| Criterion | Opt 1 chain | Opt 2 A/B/C roots | Opt 3 E-only | **Opt 4 DAG** |
|---|---|---|---|---|
| Package independence | none | high (2 of 3 real) | n/a | **high (2 true roots)** |
| Blast radius per unit | inherited | low | lowest | **low** |
| Reviewability | good | excellent | excellent | **excellent** |
| Incremental value before E | none | B, C ship | none | **B, C ship** |
| Repeats a known failure | yes (A-first) | partly (A as root) | **yes (= attempt 1)** | **no** |
| Circular dependencies | no | no | no | **no** |
| Can start this session | 1 package | 3 (one unsound) | 1 (unsound) | **3 (all sound)** |
| Survives S5 (no unreviewed-output dependency) | no | no | yes | **yes** |

---

## Decision

**Adopt Option 4 — root-first DAG `{B, C} → A → D → E`, with F excluded.**

### Rationale

1. **It is the only structure that does not repeat a disproven move.** Option 3 *is* attempt 1.
   Options 1 and 2 both place A upstream of B, which is the inverted edge that made attempts 2 and 3
   invent new architecture to supply a destination that B already governs.
2. **R2 collapses the hardest package.** The three prior attempts treated "where does a post-gate
   write go?" as an open architectural question requiring a new substrate (untracked store, PR
   labels, freeze exemptions). It is not open — the carry-forward compound doc answers it, and two
   of the three write classes are *already* routed that way. A shrinks from "design a substrate" to
   "classify writes against an existing destination", which is a document-sized, reviewable unit.
3. **B and C deliver value on their own merits.** Neither is a scaffold for E. B closes a live
   three-way P-010 contradiction; C closes a documented single-active detection gap
   (`ship-single-pr-serialization`, PRs #138/#140). Both would be worth shipping if the checkpoint
   defect did not exist.
4. **S5 is satisfied.** No package's acceptance criteria reference another package's unreviewed
   output. Downstream packages consume *shipped* upstream contracts.

### Session scope under this decision

| Planned this session | Deferred as deliberation outcome |
|---|---|
| B, C (true roots) | A (prerequisite: B shipped) |
| A (root-adjacent; plan only, harvest gated) | D (prerequisite: A + C shipped) |
| | E (prerequisite: D shipped) |
| | F (separate program) |

A is **planned but not harvested** unless its plan-review passes *and* its dependency on B is
recorded as an explicit `blocks` edge. D and E are **not planned** this session: planning them now
would mean writing acceptance criteria against A's unreviewed output, violating S5 and reproducing
the exact failure mode of attempts 2 and 3.

---

## Package Definitions and Program-Level Acceptance

### Package B — Staging PR ownership and lifecycle (ROOT)

**Scope**: Resolve the three-way P-010 contradiction in Orchestrator Step 1.5. Name exactly one
owner for staging-PR creation, push, readiness, pause, and closure. Keep Stage's source/template
mutation prohibition explicit and unchanged.

**Out of scope**: Ship checkpoint ordering; any freeze semantics; open-PR discovery.

**Acceptance**: P-010 and Orchestrator Step 1.5 agree on a single named owner; Stage's "MUST NOT
create, push, or merge pull requests" survives verbatim; the direct-push-to-`main` fallback in Step
1.5 item 3e is classified against branch protection and P-009.

### Package C — Open PR taxonomy as additive P-001 blocker (ROOT)

**Scope**: Define PR classes (feature implementation, closure, staging/planning, unrelated/draft),
provenance and correlation inputs, exhaustive pagination, ambiguity behaviour, and the
**monotonicity rule**: open-PR discovery may only add blockers or routes; it may **never** clear an
existing claim or closure blocker.

**Out of scope**: pause-label or PR-body-marker authority (explicitly rejected as an authority
source); Ship checkpoint ordering.

**Acceptance**: A stated proof obligation that the rule is additive-only; pagination exhaustiveness
required with fail-closed on incomplete pagination; classification must tolerate this repo's live
#390 (draft) and #396 (superseded-open) without wedging; legitimate Stage/Ship overlap permitted
where P-016 allows it.

### Package A — Freeze exception and tracked-write taxonomy (depends on B)

**Scope**: Classify every mandatory tracked write in the Ship and Stage lifecycles by phase, and
assign each a legal destination: *pre-gate on the implementation branch*, *post-merge closure
branch*, or *next staging round*. Reconcile P-005 telemetry, P-021 C2 capture, P-007/P-015
restoration, circuit-breaker/escalation records, P-020/compact-context, continuous-learning, and
mandatory session memory.

**Out of scope**: implementing checkpoint ordering (that is E); changing the staging-PR owner (B).

**Acceptance**: no mandatory write left unclassified; every class names a reachable destination;
the taxonomy is stated as policy/contract text only, with zero changes to checkpoint resolution
ordering.

### Package D — Terminal pause continuity model (depends on A + C) — DEFERRED

Represent terminal PR-backed pauses without advancing HEAD. Separate **discovery hints** from
**authority**. Define resume revalidation, zero-checkpoint semantics, GitHub-unavailable behaviour,
and pre-PR/offline fallback. Classify tracked checkpoints and tracked memory writes by phase.
Determine whether backlogit source changes are needed — **do not assume prompt-only suffices**.

### Package E — Core checkpoint-resolution ordering fix (depends on D) — DEFERRED

The narrow original fix: all required tracked mutations and checkpoint resolutions occur **before**
the final reviewed/approved HEAD; no generic post-merge or session-end resolution commit;
current-HEAD review/check/thread/approval and expected-head merge; zero-checkpoint path;
closure/staging parity as permitted by B and A. Crash-window hardening stays separate unless proven
required.

### Package F — Dark-mode continuation auto-routing — EXCLUDED

Defect 1. Separate open deliberation. Linked here as explicitly out of scope. Not harvested in this
program unless its own deliberation independently reaches a reviewed decision.

### Dependency graph (acyclic)

```
  B ──────┬──> A ──┐
          │        ├──> D ──> E
  C ──────┴────────┘

  F : excluded (separate program)
```

No cycles. B and C have in-degree 0. E has out-degree 0.

---

## Rejected Alternatives

* **Untracked checkpoint substrate (attempt 2's Option D).** Sound but insufficient — it does not
  stop the co-committed tracked `docs/memory/` write (`ce3b2fba`), and "fails closed" was false
  because both agents' ZERO-CANDIDATE NORMAL STARTUP clause treats zero active checkpoints as
  explicitly not a failure. **Carried forward as a candidate component inside D**, not as a program
  root.
* **Freeze with declared exemptions (attempt 3).** The freeze thesis is correct and is preserved
  inside A's taxonomy, but "freeze + exemption list" is not a partition while P-005, P-021 C2, and
  P-007 can all fire inside the window. A's staging-round routing replaces the exemption list.
* **PR labels / body markers as pause authority.** Mutable by any actor, not correlated to HEAD,
  and explicitly excluded from C's authority sources.
* **Planning D and E this session.** Would write acceptance criteria against unreviewed upstream
  output (violates S5) and is the precise mechanism by which attempts 2 and 3 generated new
  architecture faster than they closed old defects.

---

## Unresolved Questions

* **U1** — Whether backlogit requires a source change to support a non-HEAD-advancing pause record.
  Deferred to D by design; must not be assumed away.
* **U2** — Whether Stage's symmetric `Session end` exposure (items 1–2, identical to Ship's) is
  fully covered by A's taxonomy or needs its own unit in D.
* **U3** — Disposition of the recurring shipment dependency-eligibility question (stash `77A4E71C`,
  `F35EA0E6`, `76153F55` — three occurrences on the same chain). Related to closure mechanics but
  **not** part of this program; recommend a separate workspace-wide decision.

---

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| B changes a role boundary and destabilises P-010 | B is additive-clarifying only: Stage's prohibition survives verbatim; B names an owner rather than granting Stage new authority |
| C's open-PR scan wedges the Orchestrator on #390/#396 | C's acceptance explicitly requires tolerating the live draft + superseded-open state without fail-closed wedging |
| A's taxonomy misses a write site | A's acceptance requires exhaustive classification with "no write unclassified" as a gate condition, seeded by R1's verified table |
| Deferring D/E leaves the originating defect open | Accepted and declared. Three attempts prove premature planning is more costly than an extra cycle. `4EF24729` stays **active** and unmutated as the intake record |
| Package reviews mask each other's findings | plan-review is invoked **separately per package**; a PASS in one never clears a P0/P1 in another |

---

## Quality Criteria

* 4 program structures evaluated (3 required + 1 synthesised) — satisfied.
* Dependency graph acyclic — satisfied.
* Fresh artifact name; no prior decision overwritten — satisfied (three prior decisions listed under
  `supersedes`, left byte-intact on disk).
* Exclusions explicit (F, #396, `143.*`, Defect 1) — satisfied.
* Program-level acceptance defined per package — satisfied.

---

## Post-Review Addendum (same session, after per-package plan-review)

Packages B and C were planned, hardened, and reviewed separately this session. **Both FAILED.**
Package A was **not reviewed** — its prerequisite (B) failed, so it could not be harvested on any
verdict, and reviewing it would have produced a review of a plan that must be revised once B's
ownership model is decided.

| Package | Verdict | Record |
|---|---|---|
| B | **FAIL** (4 P0, 11 P1) | `docs/reviews/2026-09-13-package-b-plan-review-fail.md` |
| C | **FAIL** (2 P0, 13 P1) | `docs/reviews/2026-09-13-package-c-plan-review-fail.md` |
| A | deferred, not reviewed | prerequisite B FAILED |

### The program structure survives; two package scopes do not

Option 4's DAG `{B, C} → A → D → E` is **not invalidated** by these results. No reviewer challenged
the dependency ordering, the exclusion of F, or the claim that B and C are in-degree-0 roots. What
failed is the *internal scope* of B and C. Three corrections are now established by evidence rather
than by argument:

* **B-CORRECTION — there is no third role.** By elimination Ship is the only P-010 role holding PR
  authority, and Ship has no staging-scoped entry point. Package B **must** include a bounded,
  additive Ship staging-intake unit. `I-B5` must narrow from "no `_ship.agent.md` diff" to "no diff
  in `_ship.agent.md` Steps 5, 6, or Session end", so the checkpoint/freeze exclusion survives while
  the intake becomes expressible. `OD-B1` collapses from a candidate search to a single go/no-go.
* **C-CORRECTION — split C, and fix the abstraction.** `C1` = owner-neutral classification modelling
  *kind*, *draft*, *correlation*, and *supersession* as **four independent fields** (not four fused
  classes), with explicit precedence and an explicit class→blocker contribution table. `C2` =
  integration at the **correct consumer** — Orchestrator Step 2 and/or Ship Step 1, not Step 0 —
  with offline/degraded semantics and an operator override.
* **NEW PREREQUISITE — Package G (executable contract assertions).** Both packages failed partly for
  an identical, previously unrecognised reason: **this harness has no mechanism to make an
  agent-contract assertion executable.** Every verification in both plans degenerated to
  `Select-String` (exits 0 regardless of match count) or `git diff --numstat` (proves deletions, not
  semantics), and two named commands could not pass as written — `autoharness verify-workspace`
  requires `--workspace <path>`, and `scripts/pre-commit-markdownlint.ps1` exits 0 on an empty staged
  set. Any future contract-text package will fail review the same way until a contract-assertion
  capability exists. **Package G is a new in-degree-0 root and is now the recommended first
  shipment of this program.**

### Revised DAG

```
  G ──> B' ──┬──> A ──┐
             │        ├──> D ──> E
  G ──> C1 ──┴> C2 ───┘

  F : excluded (separate program)
```

`G` (executable contract assertions) becomes the root of the program because it is the only package
whose own acceptance criteria can be made executable without it.

### Status of this decision artifact

The **decision stands**: root-first DAG, F excluded, no monolith. The package *scopes* for B and C
are superseded by the two corrections above and must be re-planned. This addendum is recorded in
place rather than as a new artifact because the program-structure decision itself was not
invalidated — only two of its package boundaries were.
