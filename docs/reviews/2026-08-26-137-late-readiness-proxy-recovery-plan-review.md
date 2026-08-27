---
title: Plan review — verify and ship the late-readiness stdio-proxy recovery change set
date: 2026-08-26
type: plan-review
status: approved-with-changes
reviewer: stage (adversarial gate)
plan: docs/exec-plans/2026-08-26-137-late-readiness-proxy-recovery-verification-plan.md
source: docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md
feature: 137-F
review_artifact: 137.001-R
cycles: 1
---

## Verdict

**Approved with changes.** One review-fix cycle was applied before harvest.
Findings F1, F2, F3, and F5 were folded back into the plan. F4 is accepted as a
task-level acceptance note. The plan is cleared for decomposition into `137-F`
and for assembly into a single queued shipment.

This is a **retro-staged** review: the implementation already exists in the
working tree. The gate therefore checks that the plan verifies and releases the
existing diff honestly, rather than pretending design work remains.

## Scope Check

| Gate | Result |
|---|---|
| Plan traces to an authoritative RCA | Pass — the scale-spike decision doc and session memory are cited, not re-derived |
| Plan avoids fabricating unfinished design work | Pass — framed as verification, scope audit, reconciliation, release |
| Ad-hoc `136-F` / `136.001-T` history preserved, not duplicated | Pass after F2 — archived artifacts retained immutably and linked |
| Every unit fits the 2-hour rule | Pass — V1–V5 each single-concern and ≤ 2h |
| Width isolation | Pass after F3 — shim, gates, DB/IPC audit, docs, and release are separate tasks |
| Fail-closed semantics preserved | Pass — only `DaemonError::NotReady` is recoverable; terminal classes unchanged |
| Rollback documented per risk | Pass — per-file revert paths, DB instrumentation revertible independently |
| Validation commands explicit and runnable | Pass after F1 |
| Known repository blockers surfaced | Pass after F5 |
| Stage role boundary respected | Pass — no source edits, builds, branches, commits, or PR operations performed while staging |

## Findings

### F1 — Major: inherited green evidence is not verification

The draft accepted the prior session's `fmt` / `clippy` / `dev-test` / `audit`
results as sufficient. Those runs happened in the same ad-hoc session that
bypassed the pipeline, on an uncommitted tree that nothing pins. Reusing them
would make the whole shipment a self-attestation.

**Resolution (applied):** V2 requires an independent re-run of all four gates
against the current worktree, with command, exit status, and warning count
recorded on the task. Prior results are explicitly rejected as substitutes.
`cargo audit` is bounded to **exactly** the 14 known pre-existing allowed
warnings; a 15th finding is a blocker.

### F2 — Major: reopening the archived `136` artifacts would corrupt history

The draft considered moving `136-F` / `136.001-T` back to `active`. Those items
are terminal, archived, and record a real completed implementation. Reopening
them would rewrite history to hide the pipeline bypass, and re-creating the same
titles under new IDs would produce misleading duplicate implementation work.

**Resolution (applied):** `136-F` and `136.001-T` stay archived and immutable.
`137-F` is a distinct corrective wrapper whose tasks are verification, audit,
reconciliation, and release — never re-implementation — and is linked to both
archived artifacts so the trail from ad-hoc work to governed release is
continuous and legible.

### F3 — Major: adjacent DB and IPC diffs were unpoliced scope

`src/db/cozo_backend/mod.rs` (+99/−57) and `src/daemon/ipc_server.rs` are not
proxy-recovery code. Shipping them inside a shim reliability change without an
explicit gate is exactly the width violation the pipeline exists to catch. A
`std::env::var` startup-delay hook in a daemon startup driver is a
production-risk shape on its face.

**Resolution (applied):** V3 is a dedicated read-only blast-radius audit that
must prove (a) `ENGRAM_TEST_STARTUP_DELAY_MS` is compiled out of release builds
via `#[cfg(debug_assertions)]`, (b) `DbStartupTimings` does not alter lock
ordering, the 30 s file-lock deadline, error mapping, or schema bootstrap, and
(c) the readiness log change is wording-only. Failure routes to **splitting the
file out of the shipment**, not to an in-place fix.

### F4 — Minor: recovery test timing determinism

`shim_recovers_after_timed_out_daemon_later_becomes_ready` coordinates a real
child daemon, a real named pipe, and a 50 ms→1 s backoff monitor. A single green
run does not distinguish a correct implementation from a lucky schedule,
particularly on Windows named pipes.

**Disposition:** accepted as a task acceptance note on V1 — the recovery and
teardown cases must be run at least three consecutive times with zero flakes,
determinism must come from the injected startup delay rather than a bare sleep,
and any flake is a blocker rather than a retry. No plan restructuring required.

### F5 — Major: the shipment could not physically be committed

The draft ended at "commit and PR" without acknowledging that
`.backlogit/stash.jsonl` is in an unresolved merge state. `git commit` fails
repository-wide with `.backlogit/stash.jsonl: needs merge` (confirmed by a
read-only `--dry-run`). V5 would have been dispatched to Ship as an
unsatisfiable task, burning a failure cycle.

**Resolution (applied):** V5 declares two hard preconditions — V1–V4 complete,
and operator resolution of the stash conflict outside this shipment. The plan
records that no agent in this pipeline is authorized to repair that file. The
plan also now forbids touching the pre-existing staged `.gitignore`
modification, which a naive `git commit -a` would have swept in.

## Residual Risk Accepted

* The change set remains uncommitted and therefore unprotected until V5 clears.
  If the worktree is lost, the archived `136` artifacts and this plan describe
  the fix but the diff itself is not recoverable from the repository.
* Post-ready daemon **restart** recovery is out of scope; a daemon that dies
  after reaching `Ready` still degrades the session until reconnect.
* The 5,000-file release-mode index-and-query gate does not yet exist, so the
  scale spike's performance claims remain unpinned by CI.

## Degraded-Mode Note

Backlog index sync is failing with
`unmarshal stash entry: invalid character '<'` because of the same pre-existing
`.backlogit/stash.jsonl` conflict. All backlog operations for this review were
performed with item- and manifest-level commands that do not parse the stash.
The conflict was **not** repaired, edited, or archived.
