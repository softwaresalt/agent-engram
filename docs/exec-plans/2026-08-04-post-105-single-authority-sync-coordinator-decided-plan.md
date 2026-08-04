---
title: "Decided plan — post-105 single-authority sync coordinator"
date: 2026-08-04
feature: 109-F
shipment: 104-S
status: shipped
source_plan: "docs/archive/plans/2026-08-02-post-105-single-authority-sync-coordinator-plan.md"
---

# Decided Plan — Post-105 Single-Authority Sync Coordinator

## Decision

Replace the split generation, owner flag, pending-mask, and lifecycle authority
with one private `SyncCoordinator`. The package is non-published, so no public
Rust compatibility bridge is retained. CLI, MCP, wire, schema, persistence,
queued-response, and startup contracts remain stable.

## Final Authority Model

- `CoordinatorCell` is the sole authority for binding identity, generation
  floor, owner phase, full work mask, cancellation, handoff, and completion
  timestamp.
- A non-cloneable `AdmissionGuard` owns pre-acquisition cancellation and an
  enabled notification. Acquisition moves that ownership into one armed,
  non-cloneable `OwnerPermit`.
- Requests return `Acquired`, guarded `Waiting`, `Enqueued`, or `Stale`.
  Non-empty work returns `Enqueued` only after the complete mask is
  coordinator-owned.
- Direct idle Index and Sync preserve their requested owner kind. Only
  coalesced transferred work normalizes to Sync.
- Exact completion transfers the full mask once or releases the owner.
  Armed permit Drop performs mandatory recovery without optional caller
  cleanup.

## Binding Retirement and Quiescence

An active rebind atomically advances the visible binding/floor and converts
the current owner into one retirement barrier.

- Same binding moves `owner mask OR pending` into one deferred slot; the
  complete heavy mask is `0b111`.
- Distinct binding carries zero old work.
- Current requests coalesce behind the barrier.
- No successor can acquire before the exact retired permit acknowledges that
  all database/file-capable work ended.
- Explicit acknowledgement or armed Drop publishes deferred work once and
  notifies after unlocking.
- Stale terminals mutate nothing. No mutex crosses an await, and no timeout
  may force a barrier open.

Retirement acknowledgement atomically installs a reissued same-kind successor
when applicable, preventing an empty background waiter from stealing the baton
or downgrading forced Index work.

## Driver and Child Ownership

Hydration, Startup, legacy Watcher, v2 Watcher, write progress, and transferred
drivers retain parent-owned task guards. Normal paths drain and join; guard
Drop aborts. Mutation-capable child work always ends before permit terminal or
retirement acknowledgement. Raw detached receivers, permits, task handles,
work masks, and database drivers are forbidden.

Hydration and Index refresh Git HEAD before I/O through the branch-owner
preparation path. Every running and terminal progress publication is fenced to
the exact current owner.

## Final Execution Chain

The accepted Phase 6 correction required caller migration before API deletion:

```text
109.027-T
  -> 109.028-T RED lifecycle double-authority proof
  -> 109.029-T lifecycle permit-only migration
  -> 109.032-T fixture and ingress retirement
  -> 109.030-T state compatibility deletion and zero-caller proof
  -> 109.031-T final validation
```

Tasks `109.001-T`–`109.012-T` remain superseded and blocked outside the
shipment. The completed replacement chain is `109.014-T`–`109.032-T`.

## Verification Decision

Correctness proof is deterministic and private:

- compile each RED before executing its intended failing assertion;
- use barriers, oneshots, and notifications, never sleeps or live timing for
  race correctness;
- prove cancellation for every owner kind, full-mask single ownership,
  child-before-ack, no successor-before-ack,
  `max_active_db_drivers == 1`, and zero old work after acknowledgement;
- preserve exact queued JSON and public schema snapshots;
- require zero legacy caller, adapter, receiver-extraction, and detached-child
  inventory.

Windows runtime validation supplements rather than replaces deterministic
proof. The final candidate passed the all-target suite, 16/16 named-pipe
observations, restart/reconciliation, and a complete-unit rollback restart.

## Rejected Alternatives

- **Public permit API:** rejected because the crate is non-published and a
  public bridge would expand the supported surface.
- **Packed atomic authority:** rejected in favor of readable, full-width,
  mutex-linearized transitions.
- **Tokenless compatibility bridge or split drain:** rejected because either
  recreates a second authority.
- **Detached mutation-capable tasks or optional cleanup:** rejected because
  caller abort and early return must remain safe by ownership.
- **Partial deployment or rollback:** rejected because coordinator state and
  caller signatures form one release unit.
- **Sleep, timeout, or live-daemon race proof:** rejected as nondeterministic
  and capable of hiding overlap.

## Rollout and Rollback

No schema, data, feature-flag, or reindex action is required. Monitor the first
released daemon session for IPC reachability, stable binding identity, one
database driver, finite owner progress, exact retirement acknowledgement, and
heavy-mask retention after file errors.

On invariant failure, create a dedicated branch from current `main` and run
`git revert --no-edit -m 1 d8fba2c3c4538e061e2ac4f56da83f82801d78e9`.
Push that complete-unit revert through a reviewed PR, restart only the tracked
daemon PID after merge, and verify bind, status, and no-op sync. Partial source
rollback remains forbidden.
