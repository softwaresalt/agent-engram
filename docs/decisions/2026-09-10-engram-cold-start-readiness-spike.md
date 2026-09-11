---
title: "Which Cozo open or schema-bootstrap phase dominates daemon cold start?"
type: spike
date: 2026-09-10
time_box: "4h"
conclusion: "proceed"
confidence: "medium"
linked_parent_work_item: "002-SP"
promoted_to:
  - "plan"
  - "queue"
plan_artifact: "docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md"
promoted_work_items:
  - "143-F"
  - "144-F"
tags:
  - "daemon"
  - "cozo"
  - "cold-start"
  - "readiness"
  - "hnsw"
  - "release-blocker"
---

# Spike 002-SP — Engram daemon cold-start / readiness reliability

* **Date:** 2026-09-10
* **Spike:** `002-SP` (promoted `high` → `critical` by operator decision)
* **Status:** Executed — findings recorded, bounded conclusion reached. **Findings
  remain durable and are unchanged; the implementation plan derived from them did
  not pass review** (see the terminal-status note below).
* **Severity:** Critical service-availability defect, release blocker
* **Author:** Stage agent (planning/research only — no source changes)

## Terminal status note (added 2026-09-10, Revision 3 plan-review)

**The spike findings below are DURABLE and UNCHANGED. The implementation plan
derived from them did NOT pass its review gate.**

`002-SP` itself remains **done**. Its investigation, evidence, and bounded
conclusion (`conclusion: proceed`) stand as recorded — nothing in this note
retracts, weakens, or reopens any finding in this document.

Separately, the third and final independent `plan-review` gate on
`docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` returned a
**terminal FAIL** (see that plan's `## Plan Review — Revision 3`). The
review-cycle circuit breaker is **open**. Accordingly:

* Every manifest member of `143-S` and `144-S`, including the covering features
  `143-F` and `144-F`, is now **blocked** and **not claimable**.
* Both shipment records remain `queued` only because the shipment lifecycle has
  no blocked state; that is a storage artifact, not an authorization to ship.
* A future plan built on these findings **requires an explicit new
  operator-authorized Stage cycle**.

The distinction matters: the gate failed on the **executable contracts** of the
plan — unsafe or unimplementable as written — not on the quality or validity of
the investigation recorded here.

## 1. Scope and constraints

Goal per `002-SP`: identify which Cozo open or schema-bootstrap operation
dominates cold start, so the daemon can reach readiness within budget.

Hard constraint (restated by the operator, and already recorded in `002-SP`):
**increasing `ENGRAM_READY_TIMEOUT_MS` is explicitly rejected.** It masks the
symptom without fixing the expensive open/bootstrap path.

Investigation constraint: the Engram daemon/MCP surface is unavailable. Per the
documented degraded fallback, all evidence below comes from local
indexed-independent source/file inspection and non-invasive OS-level process
observation. No daemon readiness probes were issued and no daemon process was
stopped during this spike.

## 2. Live reproduction evidence

Daemon PID `30528` (the clean replacement started after the operator-approved
stop of PID `28640`) was still running and still not ready at the time of this
spike, roughly **2.5 hours** after start.

Non-invasive 60-second sample taken during this spike:

| Measurement | Value |
|---|---|
| Elapsed sample window | 60.0 s |
| Branch DB size | 98.28 MB → 99.91 MB |
| DB growth rate | 0.027 MB/s (~1.6 MB/min) |
| CPU consumed in window | 118.7 s = **198 % of one core** |
| Total CPU since start | 4,296.7 s (~72 CPU-minutes) |
| Working set | 1,843.5 MB (flat, not growing) |

Branch database file:
`.engram/cozo/feat__138-s-generation-activation-request-context-startup-gate-and-request-entry/engram.db`
— last written at 16:24:44 local, i.e. **actively being written** while the
daemon reports not-ready.

Size trajectory across the whole incident: **7.4 MB → 32.7 MB → 97.6 MB →
99.9 MB**, still climbing.

**Workload signature:** ~2 cores saturated, flat resident memory, and low
sustained I/O (1.6 MB/min). This is a **CPU-bound compute workload**, not
DDL execution and not I/O-bound bulk loading.

## 3. Where readiness is actually published

`src/tools/lifecycle.rs::background_db_hydration` publishes readiness
**immediately** after `connect_db` returns, before any of the longer hydration
work:

```rust
let db = match connect_db(&data_dir, &branch).await { ... };
let cg_queries = CodeGraphQueries::new(db);

// Signal "ready" immediately after the DB connects so the shim's
// poll_until_ready succeeds while the longer hydration continues.
let _ = state.set_hydration_ready_for_generation(binding_generation);

// hydrate_code_graph / detect_offline_changes / optional re-index follow
```

**Consequence:** `hydrate_code_graph`, `detect_offline_changes`, and the
optional `sync_code_graph` re-index all run *after* readiness is published, so
none of them can explain a pre-readiness stall. If the daemon is stuck before
readiness, it is stuck **inside `connect_db`** (or in the admission /
`prepare_branch_owner` step preceding it).

## 4. Bounding the pre-readiness phases (Finding 1)

`src/db/cozo_backend/mod.rs::connect_db` has four instrumented sub-phases. Three
are hard-bounded by construction:

| Sub-phase | Bound | Evidence |
|---|---|---|
| `process_lock` (in-process mutex) | Uncontended on a fresh daemon; no second same-process opener exists at startup | `connect_db_open_lock`, `DB_OPEN_LOCKS` registry |
| `file_lock` (cross-process `fd_lock`) | **Hard 30 s deadline**, then returns `Err` | explicit `deadline = Instant::now() + Duration::from_secs(30)` polling loop |
| `database_open` (`DbInstance::new`) | `MAX_REOPEN_ATTEMPTS = 10`, back-off capped at 250 ms → **≈ 1.6–2.4 s** | `open_db_with_retry`, `reopen_backoff`, and the existing unit test `reopen_backoff_is_bounded_and_capped` |
| `schema_bootstrap` (`run_schema_bootstrap` → `run_scripts`) | **UNBOUNDED — no timeout, no deadline, no cancellation** | `run_scripts` runs ~30 `:create` scripts, 3 migrations, and 3 HNSW creates with no time budget |

### Conclusion (bounded, not speculative)

`connect_db` can spend **at most ≈ 32.5 seconds** outside `schema_bootstrap`.
Observed pre-readiness time is ~2.5 hours. Therefore:

> **If the daemon is stalled pre-readiness, `schema_bootstrap` accounts for
> ≥ 99.6 % of that time.** `schema_bootstrap` is the only unbounded phase in the
> pre-readiness path.

This is a rigorous *conditional* proof from code structure plus hard-coded
bounds — it does not depend on runtime profiling.

### What remains explicitly unproven

Two things cannot be established without either stopping PID `30528` (not
separately approved) or durable per-phase logs:

1. **Whether the daemon is genuinely stalled pre-readiness, or is
   ready-but-unreachable** with the observed CPU burn coming from post-ready
   hydration/indexing. Both remain consistent with the external evidence.
2. **Which operation inside `schema_bootstrap` dominates.** `run_scripts`
   emits **no** sub-phase timings — creates, migrations, and HNSW creation are
   completely unattributed.

Both gaps have the same root cause, which is itself a defect:

* The `"CozoDB startup timing"` `tracing::info!` in `connect_db` is emitted
  **only after** the `spawn_blocking` closure returns. A daemon stuck inside
  the closure emits **nothing**. The timing struct is populated but never
  observed on exactly the failure it was built to diagnose.
* There is **no durable daemon log sink**: `.engram/logs/` has not been written
  since 2026-07-05, and `.engram/diagnostics/shim-startup-failures.jsonl`
  records only the shim-side outcome (`failure_class: readiness_timeout`) with
  no daemon-internal phase attribution.

This is why the first implementation unit must be observability, not a fix.

### Ranked hypothesis for the dominant sub-operation (evidence-weighted, NOT proven)

**H1 — HNSW vector index construction (highest support).**
`schema.rs` creates three HNSW indexes at every bootstrap:

```
::hnsw create function_embedding:embedding_hnsw {
    fields: [embedding], dim: 384, distance: Cosine, m: 50, ...
}
```

* `m: 50` is a very high graph-connectivity parameter (common defaults are
  16–32). Construction cost scales with `m`: every insertion evaluates on the
  order of `m` candidate neighbours per layer, each a 384-dimension cosine
  distance. This is precisely a multi-threaded, CPU-saturating, incrementally
  file-appending workload — matching the observed 198 % CPU with 1.6 MB/min
  growth and flat memory.
* HNSW creation runs on **every** `connect_db`, and its error handler
  suppresses `"already exists"` as benign — so the cost is paid on a fresh
  branch DB and the suppression list hides whether it was skipped.

**H2 — `:replace`-based migrations.** `migrate_calls_edge_resolution` performs
a `:replace` rewrite of the whole `calls_edge` relation. It is documented as
shape-detected and idempotent ("no-op once the column exists"), which lowers
but does not eliminate suspicion on a freshly created branch DB.

**H3 — the ~30 `:create` scripts.** Lowest support: `:create` on an existing
relation errors and is ignored, and on a fresh DB it writes empty relations
(kilobytes). Cannot plausibly account for 90+ MB of writes or 72 CPU-minutes.

H1 must be **confirmed by measurement**, not adopted on inference. The
observability unit exists to produce that measurement.

## 5. Storage-model evidence (Finding: per-branch DBs do not share work)

`connect_db` keys the database purely by branch name:

```rust
let branch_safe = branch.replace(['/', '\\', ':'], "_");
let db_dir = data_dir.join("cozo").join(&branch_safe);
```

Measured consequence in this workspace:

* **45 per-branch database directories**, totalling **2.29 GB**.
* `main` = 148.37 MB; the 138-S branch DB was rebuilt from zero to ~100 MB.
* Many near-duplicate directories exceed 100 MB
  (`feat__rustsec-…` 119.41 MB, `chore__stage-publication-dark-factory`
  118.75 MB, `post-merge__135-s-…` 143.48 MB, …).

A branch created from `main` with an **identical source tree** shares none of
`main`'s already-computed index. A `direct sync` on `main` therefore cannot
pre-warm a newly created feature branch. Every branch checkout pays the full
cold-start cost again. This is the structural driver of the defect and the
direct justification for content-identity-based reuse.

## 6. Pre-existing generation infrastructure (materially changes the fix)

The workspace **already contains** an immutable-generation subsystem, landed by
shipment `136-S`, that is **not yet wired into the daemon cold-start path**:

* `src/services/generations/` — `mod.rs`, `store.rs`, `manifest.rs`,
  `publish.rs` (33.7 KB), `context.rs`
* `GenerationStore`, `IndexTarget`, `IndexTargetKind`, `GenerationId`,
  `GenerationRevision`, `GenerationReadContext`
* `GenerationManifest` already carries `WorkspaceIdentity`, `BranchIdentity`
  (name + `source_revision`), `SealedInventory` (per-file SHA-256 digests),
  `GenerationProvenance`, and `schema_version`
* `GenerationStore::seal_candidate` / `seal_legacy_direct`, atomic publication
  via `active.json` + `.publisher.lock`
* `src/db/cozo_backend/mod.rs::open_existing_generation_via_runtime_copy` and
  `publish_runtime_copy` — copy-on-open of a published generation into a
  private runtime copy

**This means the recommended direction is largely an integration and
identity-key problem, not a green-field build.** The remaining gap is that
`connect_db` still resolves storage by branch name and never consults
`GenerationStore`.

Caution: shipment `138-S` (currently active, out of scope here) is
"generation activation, request context, startup gate and request entry" and
touches adjacent surfaces. The plan derived from this spike must sequence
behind 138-S and must not modify it.

## 7. Liveness defect (independent of cold start)

Two behaviours combine into a livelock that keeps a never-ready daemon alive
indefinitely:

* `src/shim/mod.rs` — the recovery monitor probes a non-ready daemon up to
  **once per second**.
* `src/daemon/ipc_server.rs` — `accept_loop` calls `ttl.reset()` on **every
  accepted connection** (`// T049: every accepted connection resets the idle
  timer (S046).`).

A health probe is therefore counted as useful activity. A non-ready daemon can
never reach its four-hour idle shutdown while any shim is probing it. PID
`30528` is the live proof: ~2.5 hours old, 72 CPU-minutes burned, 1.84 GB
resident, never ready, never self-terminated.

There is also **no maximum-startup watchdog**: nothing bounds total time spent
pre-readiness, so a pathological bootstrap runs forever.

This defect is **independent of the cold-start cause** and must ship
separately — it is the safety net that caps blast radius regardless of why
startup is slow.

Prior art confirming this is a recurring class:
`docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md` and
`docs/compound/concurrency-issues/early-hydration-ready-before-heavy-io-2026-05-09.md`.

### Why the prior compound learning is superseded here

`docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md` (shipment
`134-S`) diagnosed the *same* mechanism but drew a deliberately narrower
conclusion. It states that `accept_loop` "does not (**and should not, in
production**) distinguish 'was this a meaningful request' before resetting its
idle clock", and it therefore fixed the problem **test-side only** — by
converting `await_endpoint_released` into a bounded settle-then-check loop. That
scoping was correct **on the evidence available at the time**: the only observed
victim was a test probe polling at 10 Hz against a *healthy, ready* daemon, where
treating every accepted connection as activity is a safe and conservative
production default.

That premise does not hold for the evidence in this spike. The new, previously
unavailable fact is a **live, non-ready daemon** (PID `30528`): 2.5 hours old,
72 CPU-minutes burned, 1.84 GB resident, never ready, never self-terminated,
with the shim's *production* recovery monitor — not a test — probing it at up to
1 Hz. Three differences make the 134-S conclusion inapplicable:

1. **The prober is production, not a test.** `src/shim/mod.rs`'s recovery
   monitor ships to users. A test-side-only fix cannot reach it, so the 134-S
   remedy leaves the production livelock fully intact.
2. **The daemon is non-ready, so the reset is never semantically justified.** A
   probe against a *ready* daemon is at least ambiguous evidence of a live
   client. A probe against a daemon that has never published readiness cannot
   possibly be useful work — it is definitionally the health check itself, and
   counting it as activity is simply wrong.
3. **The failure is unbounded resource burn, not a hung test.** 134-S's blast
   radius was one CI job exhausting its budget. Here the idle TTL is the *only*
   remaining backstop against an indefinitely running 2-core, 1.84 GB process,
   and probe-reset disables it permanently.

**Supersession, precisely scoped:** the 134-S learning's *mechanism* analysis
stands and is reused. Only its production-side recommendation ("should not
distinguish in production") is superseded, and only for connections that carry
no useful request. The replacement rule is narrower than "stop resetting on
probes": genuine tool calls must still reset the TTL (see `143.002-T` AC2), so
the 134-S concern about premature shutdown of a busy daemon is preserved as an
explicit non-regression requirement rather than discarded. This supersession is
recorded against the old `S046` / `T049` contract, which `143.002-T` must update
rather than silently contradict.

## 8. Answers to the spike questions

1. **Which pre-readiness phase dominates?** Bounded, not fully proven:
   `schema_bootstrap` is the only unbounded phase and accounts for ≥ 99.6 % of
   pre-readiness time *if* the stall is pre-readiness. Within it, HNSW
   construction (`m: 50`, `dim: 384`, three indexes) is the highest-support
   hypothesis on the CPU/IO signature, but sub-phase timings do not exist and
   the runtime profile could not be captured because PID `30528` owns the DB
   and stopping it was not separately approved.
2. **Is raising the readiness timeout viable?** No — rejected by the operator
   and by `002-SP`. At ~2.5 hours and still climbing there is no timeout value
   that would help; the work itself must not happen on the startup path.
3. **Can a pre-synced parent pre-warm a branch?** Not today — storage is keyed
   by branch name only. Fixing this requires content identity, which is what
   the existing generation subsystem already models.

## 9. Recommendation

Split into two independent workstreams:

* **Workstream A (liveness safety, ships first, small):** stop counting health
  probes as idle activity; add a maximum-startup watchdog that self-terminates
  a never-ready daemon. Independent of root cause.
* **Workstream B (cold-start correctness):** first add durable, incremental
  per-phase startup instrumentation (including *inside* `run_scripts`) to
  convert H1 from hypothesis to measurement; then move index/HNSW construction
  off the readiness path and reuse a content-addressed generation when a new
  branch has the same tree content as its parent.

Option analysis and the recommended identity key are recorded separately in
`docs/decisions/2026-09-10-content-addressed-generation-cold-start-decision.md`.

## 10. Evidence index

* `.backlogit/queue/002-SP.md`
* `docs/memory/2026-09-10/circuit-break-engram-readiness.md`
* `src/db/cozo_backend/mod.rs` — `connect_db`, `open_db_with_retry`,
  `reopen_backoff`, `MAX_REOPEN_ATTEMPTS`, `open_existing_generation_via_runtime_copy`
* `src/db/cozo_backend/schema.rs` — `run_schema_bootstrap`, `run_scripts`,
  `HNSW_FUNCTION_EMBEDDING`, `migrate_calls_edge_resolution`
* `src/tools/lifecycle.rs` — `background_db_hydration`
* `src/daemon/ipc_server.rs` — `accept_loop`, `ttl.reset()`
* `src/shim/mod.rs` — recovery monitor
* `src/services/generations/**` — existing generation subsystem
* `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`
* `docs/decisions/2026-08-29-v0.3.0-rc.1-rollback-and-observability.md`
* `docs/compound/test-probe-resets-idle-ttl-livelock-2026-09-04.md`
* `docs/compound/concurrency-issues/early-hydration-ready-before-heavy-io-2026-05-09.md`
