# Stage session — Engram cold-start / readiness reliability (002-SP)

* **Date:** 2026-09-10
* **Agent:** Stage (planning/research only)
* **Entry point:** queued spike `002-SP`, operator-targeted
* **Outcome:** Two shipment records (`143-S`, `144-S`), **both non-claimable**. **Nothing committed** — fail-closed publication.

> ## TERMINAL STATUS CORRECTION — supersedes every claim below
>
> The Revision 3 `plan-review` gate returned a **terminal FAIL** after three
> consecutive review cycles. The review-cycle **circuit breaker is OPEN**. This
> memory was originally written *before* that verdict, and its later remediation
> sections were written between cycles. **Every statement anywhere below that
> asserts a review PASS, a publication-ready package, or that `143-S` may be
> claimed is SUPERSEDED AND FALSE.**
>
> Authoritative current state:
>
> * **Plan review verdict: FAIL (terminal), attempt 3 of 3.** See
>   `docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md`,
>   section `## Plan Review — Revision 3`. Attempt 4 is **not authorized**.
> * **Both `143-S` and `144-S` are NOT CLAIMABLE.** Every manifest member of both
>   shipments — including the covering features `143-F` and `144-F` — carries
>   `status: blocked`. The shipment records read `queued` only because the
>   shipment lifecycle has no blocked state; that is a storage artifact, not an
>   authorization to ship. **Ship MUST NOT claim either shipment.**
> * **Publication guidance: evidence-only draft PR.** Do not merge. Do not claim.
>   Do not begin a fourth remediation, redesign, or decomposition cycle without an
>   explicit, fresh operator-authorized Stage cycle.
> * **Durable terminal rationale:**
>   `docs/memory/2026-09-10/stage-rev3-terminal-plan-review-fail.md`.
>
> The historical detail below is **preserved intentionally** as the audit trail of
> the Revision 1–3 remediation attempts. It records what was believed at the time
> it was written, not currently authorized state.

## Session inputs

Operator decisions applied:

* `002-SP` promoted `high` → `critical` ✅ applied
* Treated as critical service-availability defect / release blocker ✅
* Raising `ENGRAM_READY_TIMEOUT_MS` rejected ✅ recorded as out-of-scope in plan
* Pre-warmed DB reuse for same-content branches ✅ adopted
* Content-identity immutable generations + branch overlay ✅ adopted (Option 4)

## Tool status

* `TOOL_OK: backlogit CLI` (v1.10.1). MCP surface not exposed this session; CLI
  fallback used per registry `cli_command` declarations. **Not** a silent ad-hoc
  fallback — the registry declares these commands.
* Engram daemon/MCP **unavailable** — degraded fallback to local file/source
  search used throughout. No daemon readiness probes issued; no daemon stopped.
* `INDEX_SYNC_OK` at session start.
* Registry `features.sizing` **absent** → size/complexity recorded as prose in
  task descriptions (degradation, flagged in summary).

## Checkpoint recovery

Enumerated all 20 checkpoints, no `agent`/`status` filter. No anomalies, no
quarantined records. **Zero stage-owned active checkpoints** → normal startup,
no recovery. The single active checkpoint (`checkpoint-20260910-222318.json`)
is `ship`-owned for `138-S` and was **not** touched (verified unchanged,
LastWriteTime 15:23:18).

## Key findings (see spike doc for full evidence)

1. Readiness is published **immediately** after `connect_db` returns
   (`background_db_hydration`), so a pre-readiness stall is provably inside
   `connect_db`.
2. All `connect_db` sub-phases except `schema_bootstrap` are hard-bounded
   (file lock 30 s deadline; `database_open` ≈1.6–2.4 s via
   `MAX_REOPEN_ATTEMPTS=10` + 250 ms-capped backoff) → `schema_bootstrap`
   accounts for ≥99.6 % of pre-readiness time.
3. **Not proven:** which sub-operation dominates. `run_scripts` emits no
   sub-phase timings and the `connect_db` timing log fires only *after* the
   blocking closure returns. HNSW (`m: 50`, `dim: 384`, ×3) is the
   highest-support hypothesis on the 198 %-CPU / 1.6 MB-min signature, but is
   recorded as hypothesis, not fact.
4. Live measurement of PID `30528`: 4,296 CPU-s, 1.84 GB flat RSS, DB
   7.4→99.9 MB, ~2.5 h, never ready. Process left running (stopping it was not
   separately approved).
5. Storage: 45 per-branch DBs = **2.29 GB**; `main` 148 MB. Branch-name keying
   means a synced parent cannot pre-warm a same-tree branch.
6. **Generation subsystem already exists** (`src/services/generations/`, from
   136-S) but `connect_db` never consults it. The fix is integration, not
   green-field.
7. Livelock confirmed in source: `accept_loop` `ttl.reset()` on every accepted
   connection + shim probing ≤1 Hz.

## Artifacts

* `docs/decisions/2026-09-10-engram-cold-start-readiness-spike.md`
* `docs/decisions/2026-09-10-content-addressed-generation-cold-start-decision.md`
* `docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` (plan +
  hardening H1–H8; the Revision 1 "PASS" verdict recorded here historically was
  **superseded** — the plan's terminal verdict is the Revision 3 gate
  **FAIL (terminal)**)

## Backlog created

* `143-F` (critical) + `143.001-T`…`143.004-T` → **`143-S` queued**
* `144-F` (critical) + `144.001-T`…`144.008-T` → **`144-S` queued**
* `144.003-T` and `144.005-T` are **containers**, each decomposed into three
  executable subtasks (`144.003.001-ST`…`144.003.003-ST`,
  `144.005.001-ST`…`144.005.003-ST`) during the 2026-09-10 remediation pass
* `002-SP` set to status **`done`** and retained **in the queue** (not archived)
  as the traceability anchor for `143-F`/`144-F`
* 14 task/subtask dependency edges + `144-S → 138-S` shipment edge

> **Current status of the above:** every one of the 27 `143.*`/`144.*` manifest
> members listed here, plus `143-F` and `144-F`, is now `status: blocked`
> (terminal Revision 3 plan-review FAIL). The `queued` shipment status shown
> above is a **storage artifact only** — neither shipment is claimable.

## Remediation pass — 2026-09-10 (Stage, branch `chore/stage-critical-engram-readiness`)

> **HISTORICAL — superseded by the terminal Revision 3 plan-review FAIL.** This
> section records an intermediate remediation cycle. The package it describes did
> **not** subsequently pass review. Retained for audit only.

A local multi-persona review of the staged publication package found blocking
plan/backlog defects. All were corrected surgically on this branch. No source,
config, or Ship-owned artifact was touched.

**Findings remediated (13):**

1. **Spike contract/traceability** — added required YAML frontmatter
   (`title`/`type`/`date`/`time_box`/`conclusion: proceed`/`confidence: medium`/
   `linked_parent_work_item`/`promoted_to`/`plan_artifact`/`tags`) per
   `.github/skills/spike/SKILL.md`. `002-SP` → `done` with references to the
   findings artifact, decision, and plan.
2. **Decision metadata** — added `title`/`description`/`topic`/`depth: deep`/
   `decision_status: decided`/`promoted_to: both`/`linked_artifacts`/`tags`.
   Accepted decision (Option 4) preserved verbatim.
3. **Plan conformance** — restructured to the impl-plan contract: Problem Frame,
   Requirements Trace (R1–R14), Implementation Units with real backlog IDs,
   Dependency Graph, Decisions and Rationale (D1–D14), Risks and Caveats
   (H1–H11 + `R-LEGACY-DB-RECLAIM`), **Plan Hardening Signals** with all five
   signals marked present/absent and `Requires plan hardening: yes`, Runtime
   Verification and Closure, and a Constitution Check mapping all applicable
   AGENTS.md principles. Evidence and gates G1–G9 preserved.
4. **Measurement foundation** — `144.001-T` / plan B1 now cover the
   pre-`connect_db` `admission` / `prepare_branch_owner` step as a first-class
   phase, add a durable `readiness_published` record plus a decision rule that
   distinguishes a **genuine pre-ready stall** from **ready-but-unreachable**,
   and specify heartbeat synchronization (cross-`spawn_blocking` cell) and
   lifecycle (drop-guard cancellation on every exit path, no post-terminal
   emission) to prevent orphaned tasks.
5. **Identity correctness** — clean/dirty defined as a testable oracle
   **including untracked non-ignored files**; fallback kinds `inventory-v1`
   (non-git / unborn `HEAD`) and `multi-root-v1` added with the
   `content_key_kind` discriminator inside the hash; git tree OID retained as
   the O(1) fast path; canonical encoding named unambiguously as a
   length-prefixed domain-separated TLV (`engram.generation_id.v1`); four extra
   false-positive tests added.
6. **Dirty build publication** — removed the unsafe "publish once the tree
   becomes clean" statement from both the decision record and `144.006-T`.
   Publication now requires **rebuild** or **complete content-inventory
   verification** against the exact clean identity; partial/sampled checks
   explicitly rejected. New AC3/AC4.
7. **Liveness task correctness** — `143.001-T` now specifies a GREEN baseline
   characterization **and** a separate RED desired-behaviour test;
   `143.002-T` classifies post-request-information (not at bare `accept()`),
   preserves in-flight useful-request liveness, and updates the `S046`/`T049`
   contract in place; `143.003-T` specifies orderly/durable termination, a
   15-minute default budget with an explicit 2× margin over the slowest observed
   healthy startup, and bounded shim backoff with no-respawn after 3 cycles;
   `143.004-T` pins sink ownership (daemon-owned, or shared with tested append
   safety) and requires flush/fsync before exit. G7 is closed **jointly** by
   `143.003-T` + `143.004-T`. Both A2 and A3 are forbidden from writing
   `ReadinessView`/activation state owned by `138-S`.
8. **Release-gate ownership** — every gate G1–G9 now has a named owning item and
   closing AC. Newly assigned: **G2** → `144.005.002-ST`; **G3** →
   `144.001-T` (measurement) + `144.003.002-ST` (enforcement); **G9** →
   `143.003-T` AC4 (regression test pinning the `ENGRAM_READY_TIMEOUT_MS`
   default).
9. **Measurement contingency** — `144.003-T` now carries a mandatory disposition
   for all three B2 outcomes: proceed / re-scope in place / supersede **with
   removal of the `144.005-T → 144.003-T` edge**, so `144-S` cannot deadlock on
   a void task.
10. **GC lifecycle** — `144.008-T` now requires reclamation of crash and
    watchdog orphans (no close path ⇒ liveness-driven sweep), forbids reclaiming
    copies in active use, and treats indeterminate liveness as in-use
    (fail-safe). Legacy per-branch DB cleanup is a **named** deferred follow-up
    `R-LEGACY-DB-RECLAIM` under AGENTS.md V, not orphan prose.
11. **Task granularity** — `144.003-T` and `144.005-T` (both L/high, violating
    the 2-hour rule) converted to non-executable containers, decomposed into six
    bounded subtasks, dependency-wired, and added to `144-S`.
12. **Conditional claims** — the `schema_bootstrap ≥99.6 %` figure is restored to
    its **conditional** form ("conditional on the stall being pre-readiness") in
    `144-F`, the plan, and `002-SP`. `143-F` now explains why the prior compound
    learning (`test-probe-resets-idle-ttl-livelock-2026-09-04.md`, which
    concluded production *should not* distinguish probes) is **superseded**:
    that conclusion was drawn from a *test* probe against a *ready* daemon,
    whereas the new evidence is a *production* shim probe against a *never-ready*
    daemon, where the reset is never semantically justified and disables the only
    remaining backstop. Supersession is narrow — genuine work still resets the
    TTL.
13. **Exclusion boundary preserved and memory updated** (this section).

**Tooling note (finding, not a defect in the package):** `backlogit update
002-SP --status done` applied registry routing and relocated the file to
`.backlogit/archive/`, contrary to the "do not archive" constraint. The move was
reverted, the item restored to `.backlogit/queue/002-SP.md` with `status: done`,
and the stray archive copy removed. `backlogit sync` was confirmed **not** to
re-route it. Also noted: the backlogit **CLI** (v1.10.1) does support
`--size`/`--complexity`, although `.autoharness/backlog-registry.yaml` omits
`features.sizing`; the package therefore keeps size/complexity as prose for
internal consistency, and the registry/CLI drift is recorded as a residual risk.

## Publication status

**Terminal: plan review FAILED; neither shipment is claimable; publication is
evidence-only.**

Artifacts are staged on the dedicated Stage branch
`chore/stage-critical-engram-readiness` (created from `origin/main`), **not** on
`138-S`'s feature branch — so the earlier "nothing committed / contamination
risk" blocker no longer applies.

The package is **NOT publication-ready in the "ready to execute" sense**. The
third independent `plan-review` gate returned **FAIL (terminal)** and the
review-cycle circuit breaker is **open**. The only authorized publication route
is an **evidence-only draft PR**: publish the planning, spike, decision, and
review artifacts as an audit record. **Do not merge. Do not claim `143-S` or
`144-S`. Do not open a fourth remediation cycle** without an explicit, fresh
operator-authorized Stage cycle. Stage does not commit, push, or open a PR.

> *Superseded statement:* an earlier revision of this section described the
> package as "remediated and **publication-ready**, pending operator review".
> That was written between review cycles and is **false** after the terminal
> Revision 3 FAIL.

Preserved exactly, and **not** staged or edited by this session:
`.backlogit/stash.jsonl` (confirmed unstaged), untracked
`docs/memory/2026-09-08-137-s-post-merge-closure-circuit-breaker-checkpoint.md`,
active shipment `138-S` and its checkpoint, every `142.*` item, and all
source/config.

The quarantined checkpoint audit files
(`.backlogit/archive/checkpoints/checkpoint-20260910-233840.json` and its
`.disposition.json`) were treated as **read-only** and were **never edited**.
They are **preserved locally** but are **intentionally EXCLUDED from publication**
(unstaged, not deleted) because they contain **workstation-local machine paths and
operator identity metadata** that must not be published.

> *Superseded statement:* an earlier revision of this section said those two
> quarantined files "are staged". They have since been **unstaged** for the
> reason above. The files themselves remain on disk, unmodified.

`138-S` remains **ACTIVE** per the authoritative commit `60f0ae4d` (which
records `138-S`, `142.028-T`, and `142.029-T` as active). A review claim that
`138-S` was queued with no active checkpoint was a **false positive** arising
from reading this staging branch's `origin/main` base, and was correctly
rejected.

## Resume instructions

**No execution resume is authorized.** The Revision 3 plan-review gate is a
terminal FAIL and the circuit breaker is open.

1. **Do NOT claim `143-S`.** Every member of its manifest, including `143-F`, is
   `status: blocked`. Intake must fail.
2. **Do NOT claim `144-S`.** Every member of its manifest, including `144-F`, is
   `status: blocked`. Intake must fail. Its `138-S` shipment dependency is
   additionally unmet.
3. Publish the Stage artifacts from this branch as an **evidence-only draft PR**
   after operator review. Do not merge.
4. A future Stage pass on this work **requires an explicit new operator cycle**.
   It must begin from the terminal findings (a)–(i) recorded in
   `docs/memory/2026-09-10/stage-rev3-terminal-plan-review-fail.md` and the
   plan's `## Plan Review — Revision 3`, **not** from the Revision 3 plan text,
   and must re-derive the executable contracts rather than patch them.
5. The spike findings (`002-SP`, status `done`) and the accepted
   content-addressed generation architecture direction remain **durable**; only
   the implementation plan failed.

> *Superseded instructions:* earlier revisions of this section stated that
> "`143-S` … may be claimed as soon as Ship is free", named it "the recommended
> first claim", and gave in-shipment execution ordering for `144-S`. Those
> instructions are **revoked and FALSE** after the terminal Revision 3 FAIL.
> They are noted here only so the change is auditable.
