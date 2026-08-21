---
title: Compaction report — Stage dark-factory batch 20260820
date: 2026-08-21
type: compaction-report
skill: compact-context
agent: stage
target: all
threshold_days: 14
---

## Trigger

Invoked at Stage batch completion, as required by the session contract. Both
manual thresholds were exceeded at assessment time:

* `docs/memory/` file count: 124 (threshold 40)
* `docs/memory/` total size: 519.7 KB (threshold 500 KB)

## Phase 1 — Assessment

| Area | Observed |
|---|---|
| `docs/memory/` dated directories | 30 directories, 73 files, ~324 KB |
| `docs/memory/compacted/` | 51 prior compacted summaries, ~196 KB |
| `docs/archive/memory/` | 207 previously archived originals |
| `docs/exec-plans/` plans with appended reviews | 4 (all created this session) |
| `docs/closure/` records for completed units | none new this session |

Files older than the 14-day threshold (dated `2026-08-07` or earlier): 45.

## Phase 2 — Candidate Identification

**Plans: 0 candidates.** The four plans created this session
(`2026-08-21-870b1aff-*`, `2026-08-21-568b257c-*`, `2026-08-21-c2413934-*`,
`2026-08-21-de460a88-*`) carry appended `Plan Hardening` and `Plan Review`
sections, but consolidation into decided-plans requires the associated feature to
be complete. Features 124-F through 127-F are `queued` and have not been
executed. Ship needs the full review and hardening detail to build against.
Consolidating now would remove substance, not verbosity.

**Closure records: 0 candidates.** No closure artifacts were produced this
session; the closure tasks (124.007-T, 125.008-T, 126.008-T, 127.008-T) are
queued for Ship.

**Memory: 0 candidates archived this run — deferred.** 45 dated files exceed the
age threshold, but the skill requires candidates to be *both* older than the
threshold *and* not referenced by any active work item, and to belong to a
completed release unit. Establishing that cross-reference requires the backlogit
index, which is `TOOL_DEGRADED` for this session:

`backlogit sync` fails deterministically with
`build canonical artifact index: 19 artifact files failed to parse`, caused by 19
pre-existing malformed artifacts inherited from `origin/main`
(`.backlogit/archive/029.004-C.md` through `029.009-T.md`, and
`.backlogit/queue/030.005-C.md`). None were modified by this session. Because the
SQLite cache cannot be rebuilt, `backlogit query` returns stale results and
cannot be trusted to answer "is this memory file referenced by active work?".

Several of the 45 also appear to fall under existing summaries
(`2026-07-14-to-2026-07-29-completed-release-units-compacted.md`,
`2026-08-04-104-s-109-f-compacted.md`, `2026-08-05-107-s-111-f-compacted.md`,
`2026-08-06-108-s-112-f-compacted.md`, `2026-08-06-109-s-113-f-compacted.md`),
which would make their originals safe to archive — but confirming that mapping
also depends on the degraded index.

## Phase 3 — Compaction

No files were archived. Archiving memory for a still-active work item would
violate the skill's behavioral constraint "never compact checkpoints for active
work items", and the evidence needed to rule that out is unavailable. Fail-closed
was chosen over a plausible-looking bulk archive.

This session's own memory
(`docs/memory/2026-08-21/dark-factory-batch4-stage-memory.md`, 0 days old) is
outside the threshold and is intentionally preserved verbatim for Ship.

## Phase 4 — Report

* Files compacted: **0**
* Space recovered: **0 KB**
* Active/queued checkpoints preserved: **all**
* Plans consolidated into decided-plans: **0** (4 evaluated, all deferred —
  features queued, not complete)
* Closure records compacted: **0**
* Candidates deferred pending index repair: **45 memory files**

## Deferred Work

Repairing the 19 malformed backlogit artifacts is unrelated to this batch and was
deliberately not attempted, since the session contract forbids scope expansion.
Once `backlogit sync` succeeds, a follow-up compact-context run can complete the
deferred memory compaction with reliable active-work cross-referencing.

The four plans become decided-plan candidates once their features reach
completion, which is Ship's post-merge closure step, not Stage's.
