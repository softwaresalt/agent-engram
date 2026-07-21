---
type: session-memory
agent: stage
date: 2026-05-12
topic: daemon-release-readiness
shipment_id: 035-S
feature_id: 034-C
status: complete
---

# Stage memory — daemon release readiness

## Tooling status

* backlog registry present at `.autoharness/backlog-registry.yaml`
* backlogit MCP surface unavailable in-session; CLI fallback used successfully
* `backlogit sync` succeeded at session start and end
* agent-intercom and agent-engram overlays were installed but their tool
  surfaces were not available in this session, so visibility/search ran in
  degraded local mode

## Intake handled

Direct operator request, not stash-driven:

* stage a high-priority release-hardening workstream for daemon and CLI fragility
* capture real-world lock-file, PID-state, workspace lifecycle, and workflow
  validation concerns
* create implementation-ready backlog structure without invoking Ship or
  implementing code

## Relevant prior context carried forward

* blocked shipment `025-S` and feature `041-F` remain upstream-blocked on
  CozoDB >= 0.8 and were not changed
* queued deliberation `004-D` and active stash entries around markdown chunking
  were intentionally left untouched
* prior learnings used:
  * `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`
  * `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`
  * `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`
  * `docs/closure/2026-05-09-034-S-daemon-startup-reliability-closure.md`

## Artifacts created

### Deliberation

* `docs/decisions/2026-05-12-daemon-release-readiness-deliberation.md`

### Plan

* `docs/exec-plans/2026-05-12-daemon-release-readiness-plan.md`

### Backlog hierarchy

* `034-C` — Daemon and CLI release-readiness hardening
* `034.001-T` — Add stale PID and dead-daemon recovery coverage
* `034.002-T` — Add restart-safe workspace lifecycle workflow coverage
* `034.003-T` — Add core lifecycle CLI workflow coverage
* `034.004-T` — Add indexed workflow CLI coverage
* `034.005-T` — Add named regressions for top workspace failures
* `034.006-T` — Add release smoke daemon and CLI suite
* `034.007-T` — Wire release smoke entry point
* `034.008-T` — Add release-readiness checklist and rollback signals

### Shipment

* `035-S` — Daemon and CLI release-readiness hardening

## Dependency intent

Planned execution order for Ship:

1. `034.001-T`
2. `034.002-T`
3. `034.003-T`
4. `034.004-T`
5. `034.005-T`
6. `034.006-T`
7. `034.007-T`
8. `034.008-T`

## Notes and cautions

* No stash entries were consumed, so stash archival was skipped
* Existing worktree mutations were preserved: `.backlogit/stash.jsonl` remained
  modified and `.backlogit/queue/004-D.md` remained present
* A temporary archive-file formatting drift on archived `034.001-T` was
  corrected before session end; final diff was cleared

## Deferred items

Deferred because out of scope for this session:

* `4B9F5511` — markdown chunking feature intake
* `180618FC` — markdown chunking feature intake variant
* `8334B2EA` — markdown compaction spike
* `025-S` / `041-F` — blocked upstream on CozoDB >= 0.8

