---
type: session-memory
timestamp: 2026-08-16T23:25:06-07:00
agent: ship
shipment_id: 117-S
feature_id: 121-F
pull_request: https://github.com/softwaresalt/agent-engram/pull/342
status: blocked
blocker: capability-rooted source-read containment
---

# PR 342 TOCTOU Continuation Memory

## Continuation outcome

The operator authorized continuation from the committed three-cycle review
breaker without resetting prior counters. Ship completed only the remaining
gate-blocking backlog, closure, and security work.

Supported backlogit operations restored feature `121-F` from premature
pre-merge archive to `active` in `e8881dcc`. Shipment `117-S` remains active.
No terminal shipment archive or merge was attempted.

## Security blocker

A dedicated security review confirmed that discovery-time symlink rejection
does not prevent a discovered file or ancestor directory from being replaced
before later pathname metadata/read operations. External `.tfvars` content
could be persisted and exposed under an in-workspace path.

Portable complete containment requires a capability-rooted shared source
reader and rerouting full index, startup/explicit sync, prepass, and postpass
reads. Safe `std` checks cannot atomically validate and open every path
component, and workspace code forbids unsafe platform-handle implementations.
This is broader dependency/architecture scope and remains a P1 merge blocker.

Evidence:

- `docs/closure/2026-08-16-hcl-source-read-toctou-security-review.md`
- `docs/closure/2026-08-16-hcl-family-parser-operational-closure.md`
- evidence commit `a147ac97`

The operational closure now reports `BLOCKED`, binds implementation
`40c5b1fbdba38e371cc53244969ec08ca0b5bf83` and tree
`e12e19ee1f7a7708e28f16e6a2dca28900f45351`, and marks the original runtime
verification stale after production review fixes.

## Stowaway search

Repeated supported stash reads across root, shipment, and contained Stage
worktrees plus refreshed local/remote refs found no published eighth entry.
The exact seven entries from commit
`aa14af6ec4d47846c094feb6ea7a1b1e3a17b8dd` remain unchanged:

- `4D08C3D9`
- `0B729BFE`
- `60A58C8D`
- `C64FD73F`
- `B82ABA6E`
- `1328405A`
- `AA96FC45`

No operator content was invented, overwritten, edited, harvested, or
implemented.

## Required next release unit

The appropriate Stage/security workflow must plan a capability-rooted source
reader using the exact RED and control coverage in the security review. After
that architecture ships, `117-S` would still require:

1. final-implementation full quality gates;
2. cold/startup, explicit-sync, live-event, malformed, oversize, ignored,
   static-link, deterministic replacement-race, persistence/restart, and
   rollback runtime verification;
3. refreshed operational closure;
4. current-HEAD CI and Copilot review;
5. zero unresolved threads and clean mergeability; and
6. merge-commit-only delivery and post-merge reconciliation.

No subsequent shipment was selected.
