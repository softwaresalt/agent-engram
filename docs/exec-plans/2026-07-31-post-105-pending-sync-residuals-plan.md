---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
updated: 2026-08-01
status: "reviewed (fresh finding-remediation PASS)"
source: "docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue and the R2 producer/finisher backstop. Current code still separates `set_workspace_and_config` from `begin_scan_generation`, lets `write.rs` publish against a generation read outside its coherent dispatch snapshot, lets lifecycle consume and execute a stale full-mask claim after an acquired-lock race, and leaves startup publication after failed CAS without a deterministic RED pause point.

The operator authorized a fresh review-fix cycle. This plan keeps the eight-task dependency chain but repairs the contracts: one state+lifecycle transition token, direct write adoption of the snapshot token with no retry/error path, exact-snapshot lifecycle claim validation, and a private startup test seam. Scope excludes `015-D`, `017-D`, `025-S`, `081-S`, schema, wire/response, persistence, and unrelated code-graph work. Stage changes planning artifacts only.

## Provenance and Supersession

- Source deliberation: `018-D`; source stash: `FF55E51A`, `88EB5FB1`, `1E70A289`.
- Archived `105-F` remains immutable.
- This cycle supersedes earlier 109-F PASS wording where it conflicts with the current contracts.
- `102-S` and `103-S` remain separate release units. `104-S` keeps `operator_order: 3`.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| G1 coherent rebind | State atomically publishes binding, config, next generation, cancel ownership, and queue floor, returning one `pub(crate)` opaque transition token that includes the generation-specific cancel receiver; lifecycle consumes it once | `109.001-T` G1a |
| G2 generation-qualified publication | Token-taking publisher applies newer-replaces/equal-OR/older-ignored semantics | `109.001-T` G1b/G1c |
| W1 coherent write producer | `DispatchSnapshot` carries a crate-private opaque generation token from the exact workspace/config snapshot; `write.rs` adopts it directly | `109.005-T` W1a/W1b |
| L1 generation-owned claim | State atomically claims pending/revalidate/backfill as one opaque token; lifecycle validates it against the exact workspace/config/generation snapshot before acquired-lock work or republishes the exact token after lost lock | `109.007-T` L1a/L1b/L1c |
| S1 startup handoff | Startup captures an opaque generation token before CAS and uses explicit-token publication with exactly one finisher | `109.003-T` race/control |
| API containment | Every new cross-module generation/transition/claim API is `pub(crate)` and token fields are private; legacy unqualified pending publishers become `pub(crate)` compatibility-only APIs and have no production caller after migration | Structural inventory |
| Contract/build safety | No response/error change; each GREEN leaves a buildable intermediate state and all touched production functions count toward the stated caps | Per-task review |

## Task Table

| Order | ID | Responsibility | Production files | RED scenarios | Touched production-function cap | Target |
|---:|---|---|---|---:|---:|---|
| 1 | `109.001-T` | RED coherent transition and explicit publication | none; private state/lifecycle tests | 3 | 0 | 70-95 min |
| 2 | `109.002-T` | GREEN transition token and explicit publisher | `state.rs`, `lifecycle.rs` | 0 | <=8, including visibility-only/removal touches | 95-110 min |
| 3 | `109.005-T` | RED write snapshot-token adoption | none; private `write.rs` tests | 2 | 0 | 55-80 min |
| 4 | `109.006-T` | GREEN dispatch token exposure/adoption | `state.rs`, `write.rs` | 0 | <=3 | 65-95 min |
| 5 | `109.007-T` | RED lost-lock and acquired-lock claim safety | none; private `lifecycle.rs` tests | 3 | 0 | 65-90 min |
| 6 | `109.008-T` | GREEN claim validation/re-arm | `state.rs`, `lifecycle.rs` | 0 | <=6, including legacy-helper adaptation | 80-105 min |
| 7 | `109.003-T` | RED startup final-peek handoff | none; private `ipc_server.rs` tests | 2 | 0 | 60-80 min |
| 8 | `109.004-T` | GREEN startup explicit-token backstop | `ipc_server.rs` | 0 | <=3, counting every touched production function | 60-90 min |

Every task is <=2 production files, <=3 scenarios, and <=110 minutes. Function caps count modified, added, removed, renamed, and visibility-only production functions; test helpers are separately capped on their cards.

## Implementation Units

### Unit 1 - RED coherent transition (`109.001-T`)

Add exactly three deterministic scenarios across one private state/lifecycle test width:

1. G1a pauses the current lifecycle sequence after binding/config publication but before generation/cancel publication and proves the torn state; GREEN must instead return one transition token whose generation and cancel receiver belong to that binding.
2. G1b resumes explicit stale G publication after G+1 owns the queue and proves stale intent is ignored without relabel or heavy leakage.
3. G1c proves same-G full-mask OR coalescing.

Use explicit barriers/state steps only. No sleeps, process-global mutation, real daemon/IPC, or public/release test seam. Maximum three test helpers and 95 minutes.

### Unit 2 - GREEN state+lifecycle transition (`109.002-T`)

In `src/server/state.rs` and `src/tools/lifecycle.rs` only:

- replace the lifecycle-visible split `set_workspace_and_config` then `begin_scan_generation` with one state-owned transition that performs capacity validation before mutation, publishes binding/config/next generation/cancel ownership/queue floor coherently, and returns a `pub(crate)` opaque transition token containing the generation token and its generation-specific cancel receiver;
- consume that token exactly once in lifecycle `set_workspace` when spawning hydration; do not reread generation or call a separate begin API;
- provide explicit token-taking publication/reacquire semantics: newer replaces, equal full-mask OR-coalesces, older is ignored;
- make all cross-module transition/generation APIs `pub(crate)` with private token fields; adapt the legacy unqualified pending publishers to `pub(crate)` compatibility only so they are not a public bypass; and
- correct ordering comments without claiming SeqCst is a mutex happens-before proof.

Cap: two production files, at most eight touched production functions total, including the folded/removed begin function and visibility-only helper changes; 95-110 minutes. Stop for a ninth function, third file, public token/field, guard across `.await`, second queue, or non-buildable intermediate state.

### Unit 3 - RED write snapshot token (`109.005-T`)

Add exactly two deterministic `write.rs` scenarios:

1. W1a obtains a coherent `DispatchSnapshot`, pauses after failed indexing CAS, advances to G+1, then resumes publication; pre-GREEN current-generation publication relabels the old request, while GREEN carries snapshot G and is ignored with no heavy leak.
2. W1b is a stable same-G control proving queued status/message and revalidate/backfill mask behavior remain unchanged with no retry or new error path.

No mismatch-retry or retry-exhaustion scenario is permitted. Use private/cfg(test) synchronization only, at most three test helpers, no sleeps/daemon/IPC/public seam, and 80 minutes.

### Unit 4 - GREEN write token adoption (`109.006-T`)

In `src/server/state.rs` and `src/tools/write.rs` only:

- add a crate-private generation-token member to `DispatchSnapshot` (or the existing equivalent snapshot result); its type is `pub(crate)` and its fields remain private;
- capture the token in the same coherent read window as the exact workspace/config pair; and
- have `sync_workspace` carry that token directly through failed-lock publication/reacquire.

There is no before/after retry, retry budget, exhaustion error, later-generation fallback, response change, or extra lock acquisition. Cap: at most three touched production functions total (expected: snapshot construction, write producer, and only if necessary one existing snapshot adapter), two files, 65-95 minutes.

### Unit 5 - RED lifecycle claim safety (`109.007-T`)

Add exactly three deterministic lifecycle scenarios:

1. L1a combines stale lost-lock pending plus heavy companions: claim full G mask, advance G+1, lose lock, resume exact-G re-arm, and prove no stale pending/revalidate/backfill bit is relabeled or leaked.
2. L1b covers stale acquired-lock: claim G, advance the exact workspace/config/generation snapshot to G+1, acquire the indexing lock, and prove validation rejects G before routine sync, revalidation, or backfill executes against the new binding.
3. L1c covers same-G re-arm and drain: the full mask is restored after lost lock and later drained exactly once.

Use barriers/explicit steps only, one private test surface, at most three helpers, no sleep/daemon/IPC/public seam, and 90 minutes.

### Unit 6 - GREEN lifecycle validation/re-arm (`109.008-T`)

In `src/server/state.rs` and `src/tools/lifecycle.rs` only:

- atomically claim the complete pending/revalidate/backfill mask as a `pub(crate)` opaque token with private fields;
- obtain the exact coherent workspace/config/generation snapshot;
- after acquired lock, validate claim generation against that exact snapshot before constructing paths or executing routine sync, revalidation, or backfill; stale claims perform none of that work and release safely;
- after lost lock, republish the exact claim token: older ignored, equal full-mask coalesced, newer ownership untouched; and
- preserve same-G exactly-once drain, all `finish_indexing` drain sites, and the bounded 64-pass loop.

Cap: at most six touched production functions total, including claim API, validation/republication API, lifecycle drain, and any legacy take/set helper adaptation. No third file, public token field, fifth scenario, recursion, unbounded drain, guard across `.await`, or 110-minute overrun.

### Unit 7 - RED startup handoff (`109.003-T`)

Add exactly two scenarios in the private `ipc_server.rs` test surface: the final-peek race and a no-contention control. The race may add a private `cfg(test)`-only pause seam precisely between failed CAS and publication. The seam must not be public, serialized, feature-enabled, or present in release behavior. No sleeps, real daemon, IPC timing, or third scenario.

### Unit 8 - GREEN startup handoff (`109.004-T`)

In `src/daemon/ipc_server.rs` only, capture the opaque generation token before initial CAS and pass that exact token to the established explicit publication/reacquire API after failure. Distinguish initial owner, queued-under-holder, and producer-reacquired outcomes privately; a reacquirer releases and drains once without duplicate startup work. Cap every touched production function, including outcome adapters, at three total. Preserve responses, bounded drain, registry ingestion, backfill, and flush behavior.

## Production Publication Inventory and Intermediate Builds

| Site | Final disposition |
|---|---|
| `write.rs::sync_workspace` failed-lock branch | Uses generation from coherent `DispatchSnapshot`; no retry/error path. |
| `lifecycle.rs::drain_pending_sync` | Claims full token, validates acquired path against exact snapshot, or exact-token re-arms lost path. |
| `ipc_server.rs::try_start_startup_sync` | Captures pre-CAS token and uses explicit R2 handoff. |
| Legacy unqualified state publishers | `pub(crate)` compatibility only during migration; no production caller remains after Unit 8; never public API. |

Each GREEN introduces or adapts its crate-private primitive before a later consumer needs it. Existing in-crate callers remain buildable through crate-private compatibility until migrated; no task may leave a signature mismatch for its successor to repair.

## Dependency Graph

```text
109.001-T -> 109.002-T -> 109.005-T -> 109.006-T
    -> 109.007-T -> 109.008-T -> 109.003-T -> 109.004-T
```

This order is authoritative and is also the order stored in `104-S.custom_fields.items` after the covering feature.

## Verification Plan

Stage validates artifact structure only. Ship records compiling RED evidence before each GREEN, then runs targeted G1, W1, L1, and startup fixtures followed by ordered repository gates. Structural review must also prove:

1. transition and claim types/APIs are `pub(crate)` with private fields;
2. no public unqualified pending publisher remains as a bypass;
3. write has no mismatch retry, retry-exhaustion error, or response change;
4. acquired-lock lifecycle work validates the claim against the exact snapshot first;
5. the startup pause seam is cfg(test)-only and absent from release behavior;
6. all production publication and every `finish_indexing` drain site is classified; and
7. per-task file/scenario/function/time caps include every touched production function.

Any failure returns the affected task and `104-S` blocked. Stage does not run builds, tests, or linters.

## Risks, Rollback, and Monitoring

- Risk: torn binding/generation/cancel ownership. Mitigation: one transition token consumed once.
- Risk: stale write intent relabeled. Mitigation: direct snapshot token, no retry/fallback.
- Risk: stale acquired lifecycle work mutates the new binding. Mitigation: exact-snapshot validation before any sync/heavy action.
- Risk: stale lost-lock mask leaks. Mitigation: atomic full-mask claim and exact-token republish.
- Risk: startup lost wakeup or duplicate body. Mitigation: deterministic seam and exactly-one-finisher outcome.

Rollback is release-unit commit revert plus daemon restart. Do not partially revert one dependent unit. No workspace migration or repair is planned.

| SLI | Healthy | Block/rollback trigger | Owner/window |
|---|---|---|---|
| Transition fixtures | coherent token/cancel receiver and queue floor | torn binding, relabel, heavy leak | Ship; targeted gate |
| Write producer | direct snapshot token; unchanged queued response | retry/error path or later-G fallback | Ship; targeted gate |
| Lifecycle claim | all three scenarios pass | stale acquired work, stale leak, same-G loss | Ship; targeted gate |
| Startup | exactly one finisher; seam test-only | stranded request, duplicate body, release seam | Ship; targeted gate |
| Runtime drain | zero attributable bound warnings within existing 30-second debug budget | warning or controlled restart over budget | Ship; three restarts plus 15 min |

## Plan Hardening

Hardening is required because partial concurrency changes can silently run work for the wrong binding. Reinforcing context: strict-safety and concurrency instructions; packed-atomic-clear, all-finish-site drain, and take-before-lock compound learnings.

Protected invariants: coherent transition ownership, opaque crate-private tokens, exact snapshot/claim equality, full-mask atomicity, no retry-generated response behavior, bounded drain, one startup finisher, buildable intermediate states, and frozen public/wire contracts.

**ProposedAction PA-1**
- summary: introduce coherent transition and dispatch generation tokens
- targets: `state.rs`, `lifecycle.rs`, `write.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-2**
- summary: generation-validate lifecycle claim and startup handoff
- targets: `state.rs`, `lifecycle.rs`, `ipc_server.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert the release unit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

## Runtime Verification and Closure

Ship uses repository fixtures and controlled daemon restarts only. Stage performed no implementation/runtime verification. Monitoring and rollback records must carry into closure; no operator workspace mutation is required or authorized by this plan.

## Harvest Shape

Existing `104-S` and `109-F` remain. No new task or shipment is created. Manifest and dependency order:

1. `109-F`
2. `109.001-T`
3. `109.002-T`
4. `109.005-T`
5. `109.006-T`
6. `109.007-T`
7. `109.008-T`
8. `109.003-T`
9. `109.004-T`

Keep `operator_order: 3`. Do not touch `025-S`, `081-S`, `015-D`, or `017-D`.

## Plan Review - Fresh Finding-Remediation Cycle: PASS

**Review mode:** complete Stage hardening and persona review under the configured `.Stage` model with no override. Cross-model dispatch was unavailable, so all triggered lenses used the caller model as permitted by the skill.

**Gate:** PASS. No open P0, P1, P2, or P3 finding remains.

### Resolved findings

- P1 resolved: `109.002-T` is now a coherent <=2-file `state.rs` + `lifecycle.rs` GREEN returning/consuming one transition token with the generation-specific cancel receiver.
- P1 resolved: write directly adopts the generation token from the coherent dispatch snapshot; retry, exhaustion error, and response-change language is removed.
- P1 resolved: lifecycle now validates an atomically claimed full-mask token against the exact workspace/config/generation snapshot before acquired-lock work.
- P1 resolved: lifecycle RED remains three scenarios by combining stale lost-lock pending/heavy coverage, adding stale acquired-lock coverage, and retaining same-G re-arm/drain.
- P2 resolved: startup explicitly permits only a private cfg(test)-only pause seam between failed CAS and publication.
- P2 resolved: cross-module token APIs are `pub(crate)` with opaque private fields; legacy unqualified publishers are not a public bypass.
- P2 resolved: function caps count every touched production function and intermediate task states must build.
- P3 resolved: `104-S.custom_fields.items` is topologically ordered.

### Persona decisions

- Constitution Reviewer: PASS - RED/GREEN order, <=2 files, <=3 scenarios, <=110 minutes, and Stage/Ship boundaries are explicit.
- Rust/Concurrency Reviewer: PASS - token ownership, exact-snapshot validation, full-mask claims, no guard across await, and no retry error close the identified races.
- Scope Boundary Auditor: PASS - no schema/wire/public API or unrelated backlog scope is added.
- Learnings Researcher: PASS - full-mask atomicity, take-before-lock re-arm, and all-finish-site drain guidance are preserved.
- Architecture Strategist: PASS - primitives precede consumers and each intermediate state remains buildable.
- Agent-Native Parity Reviewer: PASS - queued status/message and CLI/MCP behavior are unchanged.
- Security Lens Reviewer: not triggered.

**Decision:** keep `104-S`, `109-F`, and all eight tasks queued in the authoritative dependency order.
