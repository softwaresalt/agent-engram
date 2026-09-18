---
title: "Group B (checkpoint lifecycle / autonomous continuation) — scope determination and fail-closed halt"
type: decision
doc_type: decision
date: 2026-09-17
agent: stage
session_id: stage-group-b-checkpoint-lane-2026-09-17b
decision_status: accepted
outcome: no-shipment-fail-closed
phase: step-2-deliberation-feasibility-gate
supersedes: none
related_stash: ["CDBE7B7A", "4EF24729", "AA5698E3"]
predecessor_record: docs/memory/2026-09-17-stage-group-b-checkpoint-lane-startup-gate-halt.md
---

# Group B — Scope Determination and Fail-Closed Halt

## Verdict

**Group B yields NO release unit.** All three candidate stash entries are
excluded at the Step 2 deliberation/feasibility gate, each for an independent,
evidence-backed reason. **No plan was written, no harvest was performed, and no
shipment was assembled.** Creating a shipment here would have been a P-005
violation (shipment after a failed gate).

The preceding Group A checkpoint was disposed of first, under explicit operator
authorization, so this halt is *not* a repeat of the startup-gate halt recorded
in the predecessor record.

## Part 1 — Group A checkpoint disposition (complete)

Operator directive, 2026-09-17 19:54 PDT: select
`checkpoint-20260918-023719.json`, abandon its active recovery state without
retrying Group A, preserve planning artifacts, close the review circuit,
proceed to Group B.

| Protocol step | Result |
|---|---|
| Unfiltered enumeration (no `--status`, no `--agent`) | `total: 26`, `needs_quarantine: 0`, `quarantined: 0` |
| Anomaly/quarantine fail-closed scan (runs first, on full enumeration) | **0 anomalies** — clean |
| `stage`-owned AND `active` partition | exactly 1: `checkpoint-20260918-023719.json` |
| Explicit operator selection by filename | satisfied (operator quote above) |
| Owner validation (`agent` field) | `agent: stage` — exact match |
| Retrieve + validate | `backlogit checkpoint get` → `valid: true`, `schema_version: 1` |
| Disposition | `backlogit checkpoint abandon` → `disposition: abandoned` |

`abandon` is the correct verb and is **disjoint from `resolve` and
`quarantine`** by the tool's own design: `resolve` asserts successful
completion, which is false here; `quarantine` applies only to malformed files,
and this checkpoint was schema-valid. The recorded disposition reason states
explicitly that the Group A plan-review circuit is **closed by explicit
abandonment, not by success**.

Verified terminal state after disposition:

* `checkpoint get checkpoint-20260918-023719.json` → `status: abandoned`,
  `valid: true`, `disposition_at: 2026-09-18T02:55:50.3914331Z`
* `checkpoint list --agent stage --status active` → `total: 0`
* `checkpoint list --status active` (all agents) → `total: 0`

**No active checkpoint blocks fresh work.**

### Group A artifacts preserved (untouched, byte-unchanged)

* `docs/decisions/2026-09-17-shipment-safe-close-contract-reconciliation-deliberation.md` (15,770 B)
* `docs/exec-plans/2026-09-17-shipment-safe-close-contract-reconciliation-plan.md` (23,224 B)
* `docs/exec-plans/2026-09-17-shipment-safe-close-contract-reconciliation-hardening.md` (9,883 B)
* Commit `38026c6c` preserved. No amendment, no revision 3, no re-run of
  plan-review. Group A's 8 stash entries (`B9CC92AC`, `77A4E71C`, `F35EA0E6`,
  `76153F55`, `F9767C12`, `28C0E138`, `F9D1C495`, `B2E3C372`) remain `active`
  and unconsumed.

## Part 2 — P-021 C5/C6 obligations discharged

`4EF24729` and `AA5698E3` both carry the literal `DEFERRED SCOPE EXPANSION`
marker. Per the Step 1 precedence rule this forces the `deliberate` route
regardless of shape or priority, and triggers two independent obligations.
`CDBE7B7A` does **not** carry the marker (it is an operator-reported entry
captured by Stage), so C5/C6 does not formally apply to it.

### (A) Unconditional duplicate detection — CLEAN for both

Scanned all **131** active stash entries for checkpoint-lifecycle keywords
(`checkpoint`, `dark-mode`, `post-merge`, `resolution`, `stale`, `quarantine`,
`continuation`). The only checkpoint-lifecycle entries are the three Group B
candidates, each describing a distinct concern (selection / resolution
durability / retirement hygiene).

**Result: CLEAN DUPLICATE SCAN for `4EF24729` and for `AA5698E3`.** No duplicate
entry exists for either expansion; no merge or archival was required. This
outcome is recorded explicitly because the detection trigger is unconditional —
an unrecorded clean scan is indistinguishable from a scan that never ran.

### (B) Late-identifier reconciliation

**`4EF24729`** — `N/A` fields: `task`, `review-thread`.
Retrieval source: `docs/memory/2026-09-13/139-s-checkpoint-resolution-remediation.md`.
That record shows the `N/A` values were **not** an omission: Copilot review on
PR #395 flagged the missing source-ref fields and Ship deliberately populated
them as `N/A` via `backlogit stash edit 4EF24729` (commit `5719fab1`), because
they were genuinely unavailable. Supplementary provenance recovered and recorded
here: remediation **PR #395**, commits `29cb9ae5` / `5719fab1`, branch
`chore/139-s-checkpoint-resolution-sync`.
**Result: no new binding source-ref identifier surfaced.** `task=N/A` and
`review-thread=N/A` **STAND as truthful terminal records** — the expansion is a
workflow defect discovered during closure, not a task-scoped or thread-scoped
finding. Non-blocking, as the protocol requires.
The entry itself was **not mutated** — see Part 3, exclusion 2: the program lock
forbids mutating it.

**`AA5698E3`** — no source-ref block at all.
Retrieval sources: `docs/closure/134-S-2026-09-04-post-merge-closure.md`
(frontmatter `follow_up_stash` list, lines 26/337/386) and
`docs/memory/2026-09-04-ship-134-s-manual-shipment-archival-session.md`.
**Result: identifiers recovered — `shipment = 134-S`, `PR = #379`**, discovered
at 134-S session startup (Step 0 Crash-Resumption Protocol). `task` and
`review-thread` remain genuinely `N/A` (startup discovery, not a review thread).
Recorded here rather than written back, pending operator disposition of the
entry (Part 3, exclusion 3).

## Part 3 — Group B scope determination

The predecessor record carried all three entries forward as in-scope, but
performed **no** verification (it halted at Step 0 before Step 1). This session
verified each, as the operator directed. All three are excluded.

### Exclusion 1 — `CDBE7B7A` (high, bug): NOT IMPLEMENTABLE IN THIS REPOSITORY

Intent: dark-mode-only auto-selection of exactly one schema-valid, role-matched,
same-scope checkpoint, fail-closed otherwise. The intent is **sound and the
defect is real** — the predecessor session is a live reproduction of it.

The blocker is **feasibility of the target surface**, not merit:

| Evidence | Finding |
|---|---|
| Target surface | `.github/agents/_stage.agent.md:812-850`, `_ship.agent.md:981-1019`, `_orchestrator.agent.md` Crash-Resumption routing step |
| `.autoharness/harness-manifest.yaml` `artifacts[]` | All three are **managed artifacts** with recorded checksums and `template:` pointers |
| `template:` values | `templates/agents/_stage.agent.md.tmpl`, `_ship.agent.md.tmpl`, `_orchestrator.agent.md.tmpl` |
| `autoharness_home` | `C:\Python\Python314\Lib\site-packages\autoharness\data` — **outside this repository** |
| `templates/` in repo | **Does not exist** (`Test-Path templates` → `False`) |
| `preserved_artifacts` (7 entries) | None of the three agent files is listed — they are **not** workspace-authored |
| Injection-point seam (Primitive 6 installed) | **No** injection/custom markers present in either agent file |
| Upstream template content | `_stage.agent.md.tmpl` contains `Crash-Resumption` (812), `ZERO-CANDIDATE NORMAL STARTUP` (823), `Never auto-pick` (830) at **identical line numbers** to the rendered file; `_ship.agent.md.tmpl` `Never auto-pick` at 1001; `_orchestrator.agent.md.tmpl` `Crash-Resumption` at 189-218 |
| Template source is a git working tree? | **No** — `.git` absent at both `…/autoharness/data` and `…/autoharness` |

The rendered agent files are 1:1 copies of upstream templates (line-for-line
identical in the relevant region; the byte/checksum delta is variable
expansion). **The authoritative text CDBE7B7A must change is authored in the
autoharness product, not in `softwaresalt/agent-engram`.** An in-repo edit to
`.github/agents/*.agent.md` would be a rendered-artifact edit that the next
`merge-install` / `auto-tune` re-renders from upstream, and it would never reach
the product.

This is the **same root-cause class** that failed Group A's plan review twice
(recorded in that checkpoint's `verified_repo_facts`: *"templates/ DOES NOT
EXIST so all 3 contract files are unpropagatable generated artifacts"*).
Planning it here would have walked into an already-known blocking finding.

**Correct destination:** the autoharness product repository, against
`templates/agents/_stage.agent.md.tmpl`, `_ship.agent.md.tmpl`, and
`_orchestrator.agent.md.tmpl`. Not actionable by Ship in this workspace.

`CDBE7B7A` remains `active` in the stash, unmutated.

### Exclusion 2 — `4EF24729` (medium, task): PROHIBITED BY A PUBLISHED PROGRAM LOCK

`docs/decisions/2026-09-14-checkpoint-resolution-decomposed-program-lock.md` is
an **accepted** decision, **published on `origin/main`** at commit `be2bf778`.
Its `### Explicitly not authorized` section (line 449 ff.) states:

> * Planning, deliberating, or harvesting G1, G2, B, C, A, D, or E — no
>   downstream package planning of any kind.
> * **Consuming, archiving, or otherwise mutating stash `4EF24729`.**

The lock's evidence table records `4EF24729` as *"Intake origin, referenced
read-only and left `active`; not consumed or archived by this operation."*

Its fix is already registered as **Package E (`034-D`)** — *"Core
checkpoint-resolution ordering fix: tracked mutations and checkpoint
resolutions before the final reviewed HEAD"* — which is verbatim the remedy
`4EF24729` proposes. Package E is marked **DEFERRED** behind the chain
G0 → G1 → G2 → B/C → A → D → E, and the chain head **G0 (`027-D`) is
`status: blocked`**, pending an explicit operator transition after a
generation-3 feasibility escalation.

Deliberating, planning, or harvesting `4EF24729` into a new Group B unit would
(a) consume a stash entry the lock declares untouchable, (b) constitute
prohibited downstream package planning, and (c) create a second, conflicting
authority for work Package E already owns. **Excluded on authority grounds.**
The entry was deliberately **not** edited, so the reconciled identifiers in
Part 2 are recorded here only.

### Exclusion 3 — `AA5698E3` (low, task): PREMISES VERIFIED OBSOLETE

The operator directed that its claimed anomaly count be verified against
backlogit 1.10.1 before planning. Both of its premises fail verification:

| Claim | Verified state (backlogit 1.10.1-0.20260823032255) |
|---|---|
| Two stale ship-owned checkpoints need cleanup: `checkpoint-20260808-030834.json`, `checkpoint-20260715-181946.json` | **Already retired.** Both carry `status: abandoned` in the current enumeration. |
| "8 pre-existing schema-legacy validation anomalies among checkpoints" | **Zero.** Unfiltered `checkpoint list` → `needs_quarantine: 0`, `quarantined: 0`, `total: 26`. Per-checkpoint `checkpoint get` across **all 26** → every one `valid: true`, `schema_version: 1`. Anomaly count: **0/26.** |

The predecessor record flagged this discrepancy and asked that it be
re-verified rather than assumed resolved. It has now been re-verified at the
per-checkpoint level. **`AA5698E3` has no remaining work.**

This also dissolves the predecessor record's claim that `AA5698E3` is a
*precondition* for `CDBE7B7A`'s "zero quarantined anomalies" gate condition:
that condition is already satisfiable — enumeration reports zero anomalies today.

`AA5698E3` was **not** archived (the operator authorized proceeding with Group B,
not disposing of this entry). **Recommended operator disposition: archive as
obsolete**, via `backlogit stash archive AA5698E3`.

## Part 4 — Why no shipment

Group B's three candidates reduce to zero plannable entries. There is no
coherent covering feature to deliberate, therefore no plan to harden, no plan to
review, and nothing to harvest. Per the Stage behavioural constraint *"never
create a shipment after a failed gate"* and the guardrail *"do not assemble a
shipment if the harvest step produced no items"*, **no shipment was created.**

No source, template, or config file was modified. No build was run. No shipment
was claimed. No PR was created. No branch and no worktree were created —
all work occurred on `main`. Shipments `140-S`, `141-S`, `142-S` and feature
`142-F` were not read for mutation, not claimed, and not modified.

## Tool status (P-012)

| Tool | Status | Evidence |
|---|---|---|
| `backlogit` CLI | **OK** | v1.10.1-0.20260823032255; all reads and the abandon write succeeded |
| `backlogit` MCP | **DEGRADED → CLI fallback** | MCP tool surface not exposed to this session; registry `cli_command` fallbacks used throughout, per the registry's declared fallbacks. Disclosed, not hidden. |
| Index sync | **INDEX_SYNC_OK** | `backlogit sync` → 1378 artifacts, 0 parse failures |
| `engram` daemon | **UNAVAILABLE** | No `target/release/engram.exe` built; 10 orphaned `engram` PIDs present but non-responsive (unchanged from the predecessor session). Symbol/surface lookup unavailable. |

**Engram degradation did not affect this determination.** Every exclusion rests
on `backlogit` CLI output, the harness manifest, the published program lock, and
filesystem/`git grep` evidence — none required engram. Where the harness
prescribes engram-first discovery, `git grep` was used as the declared fallback
and the substitution is disclosed here rather than silently made.

## Follow-ups for the operator

1. **`CDBE7B7A` — route upstream.** The fix belongs in the autoharness product
   templates. This workspace cannot durably carry it. Consider re-capturing it
   in the autoharness repository's own backlog.
2. **`4EF24729` — leave to Package E.** Unblocking it means advancing the locked
   program: transition `027-D` to `queued` (G0 generation 3) or amend the lock.
3. **`AA5698E3` — archive as obsolete** (evidence in Part 3, exclusion 3).
4. **Group A** remains fully planned but ungated. Its plan-review circuit is
   closed by abandonment; a future session would need a fresh plan generation
   with a new attempt counter, not a revision-3 continuation of the abandoned one.
