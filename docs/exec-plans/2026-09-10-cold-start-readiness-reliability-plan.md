# Implementation Plan — Engram daemon liveness safety and cold-start measurement

* **Date:** 2026-09-10
* **Source decision:** `docs/decisions/2026-09-10-content-addressed-generation-cold-start-decision.md`
  (accepted as **architecture direction only**; implementation planning deferred —
  see *Deferred Phase 2* below)
* **Source spike:** `docs/decisions/2026-09-10-engram-cold-start-readiness-spike.md`
* **Spike:** `002-SP` (critical, status `done`)
* **Release units:** `143-F` (Workstream A), `144-F` (Workstream B — Phase 1 only)
* **Shipments:** `143-S` (queued), `144-S` (queued, measurement-only, depends on `138-S`)
* **Requires plan hardening:** yes
* **Revision:** 3 (strategic scope reset after the Revision 2 `plan-review` FAIL —
  see `## Plan Review — Revision 2` at the end of this document)
* **Author:** Stage agent (planning only — no source changes)

## Revision 3 scope reset — what changed and why

The Revision 2 gate FAILed because the content-addressed generation-reuse work
was **under-measured and over-designed**: eight implementation units (B3–B8 plus
their supporting leaves) were specified in detail against a cost model that has
never been measured, while the design questions that determine whether that
specification is even correct remained open.

Revision 3 does not patch that detail. It **removes it from the executable
plan**:

1. **`144-S` is now a measurement and observability shipment only.** It ships no
   cold-start fix. Its product is a **measured report that returns to Stage**.
2. **The 28 Phase 2 implementation items** (`144.003-T` … `144.008-T` and their
   subtasks) are **blocked and deferred**, removed from every shipment manifest
   and from queue ordering, and stripped of all dependency edges. They are
   **not archived and not deleted**: they remain under `144-F` as traceable
   follow-on candidates, each reduced to a concise blocked-scope record. They
   are **not implementation-ready** and are **not claimable**.
3. **The content-addressed generation architecture remains accepted as
   direction.** It is not an implementation plan. The unresolved design
   questions are recorded below as **mandatory inputs to a future Stage plan**
   that may only be written after the measurement shipment reports.
4. **Workstream A is unchanged in intent and finalized in detail.** It was
   never the reason for the FAIL and it is the only unit that stops the
   observed resource burn.

Everything the plan asserts about Phase 2 below is **reference material for a
future planning pass**. No leaf in either shipment implements it.

## Objective

Eliminate the critical service-availability defect in which the Engram daemon
never reaches readiness after a branch checkout, without raising
`ENGRAM_READY_TIMEOUT_MS` (explicitly rejected).

Revision 3 pursues this in two stages rather than two parallel fixes:

* **`143-S` (Workstream A) — bound the blast radius.** A daemon that cannot
  reach readiness must stop, loudly and diagnosably, regardless of *why* it is
  slow. This is a complete, shippable safety outcome on its own.
* **`144-S` (Workstream B, Phase 1) — measure the cause.** Establish durable,
  attributable, agent-visible startup measurement, then **return to Stage** with
  a report. No fix is authorised by this shipment under any measured result.

## Problem Frame

Two distinct defects combine to produce the observed outage. They share a
symptom ("daemon never becomes usable") but have independent causes, independent
fixes, and independent blast radii — which is why they are split across two
release units.

### Defect 1 — Unbounded work on the pre-readiness path (`144-F`)

`src/tools/lifecycle.rs::background_db_hydration` publishes readiness
**immediately** after `connect_db` returns, before `hydrate_code_graph`,
`detect_offline_changes`, and the optional `sync_code_graph` re-index. Therefore
no post-readiness hydration work can explain a pre-readiness stall: a daemon
stuck before readiness is stuck inside `connect_db`, or in the admission /
`prepare_branch_owner` step that precedes it.

Within `connect_db` (`src/db/cozo_backend/mod.rs`), three of four instrumented
sub-phases are hard-bounded by construction — `process_lock` (uncontended at
startup), `file_lock` (explicit 30 s deadline), and `database_open`
(`MAX_REOPEN_ATTEMPTS = 10` with 250 ms-capped backoff ⇒ ≈1.6–2.4 s). Only
`schema_bootstrap` (`run_schema_bootstrap` → `run_scripts`) has no timeout, no
deadline, and no cancellation.

`connect_db` can therefore spend at most ≈32.5 s outside `schema_bootstrap`.
**Conditional on the stall being pre-readiness**, `schema_bootstrap` accounts for
≥99.6 % of the observed ~2.5 h. That qualifier is load-bearing and must not be
dropped: the spike could **not** exclude the competing explanation that the
daemon was ready-but-unreachable, with the CPU burn coming from post-ready
hydration or re-indexing. Both remain consistent with the external evidence.

**This is precisely why Revision 3 stops here and measures.** The entire Phase 2
design rests on a premise — that HNSW index construction inside
`schema_bootstrap` dominates cold-start cost — that has never been observed
directly. Discriminating between "genuine pre-ready stall" and
"ready-but-unreachable", and attributing time *within* `schema_bootstrap`, is
the whole job of `144-S`.

Compounding this, storage is keyed purely by branch name
(`data_dir.join("cozo").join(&branch_safe)`), so a branch created from a synced
parent with a byte-identical tree shares nothing and pays the full cost again.
Measured: 45 per-branch databases consuming 2.29 GB. This observation motivates
the deferred Phase 2 direction; it does not by itself justify implementing it.

### Defect 2 — The liveness backstop is disabled exactly when needed (`143-F`)

`src/daemon/ipc_server.rs::accept_loop` calls `ttl.reset()` on **every** accepted
connection (the `// T049` comment, per the `S046` contract), while
`src/shim/mod.rs`'s recovery monitor probes a non-ready daemon up to once per
second. A health probe is thus counted as useful activity, so the idle TTL — the
only remaining backstop — can never fire. There is additionally no
maximum-startup watchdog bounding total pre-readiness time.

Live proof: PID `30528` ran ~2.5 h, burned 72 CPU-minutes at ~198 % of one core,
held 1.84 GB resident, grew its branch DB 7.4 → 99.9 MB, never reached readiness,
and never self-terminated.

This defect is independent of *why* startup is slow, which is what makes it
shippable first and separately — and what makes it the correct thing to ship
while the cause is still being measured.

## Requirements Trace

Each requirement maps to at least one executable unit and its backlog ID.
Requirements whose only satisfying design is deferred are marked **DEFERRED**
and are explicitly **not** owned by any leaf in either shipment.

| # | Requirement | Source | Unit → backlog ID |
|---|---|---|---|
| R1 | A daemon that cannot reach readiness must not run indefinitely | Spike §7 | A3 → `143.003.001-ST`, `143.003.002-ST`, `143.003.006-ST` |
| R2 | Health probes must not be counted as idle-TTL activity; genuine work still must | Spike §7; compound `…ttl-livelock-2026-09-04` | A1 → `143.001-T`; A2 → `143.002.001-ST`, `143.002.002-ST`, `143.002.003-ST` |
| R3 | "Shim gave up" and "daemon self-terminated" must be distinguishable in triage | Spike §7 | A4 → `143.004.001-ST`, `143.004.002-ST` |
| R4 | `ENGRAM_READY_TIMEOUT_MS` must not be raised | Spike §8.2; decision "Rejected up front" | A3 → `143.003.001-ST` (G9 AC5) |
| R5 | Per-phase pre-readiness timing must be durably observable mid-stall | Spike §4 "What remains explicitly unproven" | B1 → `144.001.001-ST`, `144.001.002-ST` |
| R6 | A genuine pre-ready stall must be distinguishable from ready-but-unreachable | Spike §4 unproven-item 1 | B1 → `144.001.004-ST` |
| R7 | Time inside `schema_bootstrap` must be attributable per sub-operation | Spike §4 unproven-item 2 | B2 → `144.002.001-ST`, `144.002.002-ST`, `144.002-T` |
| R8 | Index construction must leave the readiness path | Spike §9; decision "Scope split" | **DEFERRED.** No leaf owns this in Revision 3. `144.002-T` **measures whether the premise holds** and returns the result to Stage; a future Stage plan decides ownership. Ship may not adopt R8 in either shipment. |
| R9 | Generation identity must be content-addressed and false-positive-proof | Decision "Recommended identity key" | **DEFERRED** — Phase 2 candidate `144.004-T` (blocked) |
| R10 | `connect_db` must resolve storage through a generation store | Decision "Decisive implementation factor" | **DEFERRED** — Phase 2 candidate `144.005-T` (blocked) |
| R11 | A dirty tree must never publish a shared generation | Decision "Dirty-working-tree behaviour" | **DEFERRED** — Phase 2 candidate `144.006-T` (blocked) |
| R12 | Publication must be atomic and crash-safe | Decision "Atomic publication" | **DEFERRED** — Phase 2 candidate `144.007-T` (blocked) |
| R13 | Generation storage must be reclaimable and bounded | Decision "Consequences / risks" | **DEFERRED** — Phase 2 candidate `144.008-T` (blocked) |
| R14 | Workstream B must not be claimed while `138-S` is active | Decision "Sequencing constraint" | Shipment edge `144-S → 138-S`; CP-1 |
| R15 | Two concurrent openers of one generation must never share a writable DB | Revision 1 review P1-21 | **DEFERRED** — mandatory input Q5 to the future Phase 2 plan |
| R16 | Client-visible startup/warming state must be identical across CLI and MCP | Revision 1 review P1-14, C12 | `144.001.004-ST`, `143.003.005-ST`, `143.004.002-ST` |

**Deferred requirements are not dropped.** R8–R13 and R15 remain live product
requirements; Revision 3 asserts only that no plan may own them until the
measurement exists. The future Phase 2 plan inherits them together with the
mandatory design inputs in *Deferred Phase 2* below.

## Non-negotiable engineering constraints

Every task below inherits these:

1. **Test-first (constitution II, NON-NEGOTIABLE):** a failing RED test is
   committed before implementation; GREEN follows. No exceptions. Where a unit's
   RED test cannot coexist with a green gate run in the same claimable unit, the
   RED test and the implementation that turns it green belong to **the same
   leaf** — never to two separately claimable leaves (see A1/A2).
2. **Authoritative quality gates, run in this order, none skipped**
   (`.github/instructions/constitution.instructions.md` → *Quality Gates*):

   ```text
   cargo fmt --all -- --check
   cargo clippy --all-targets -- -D warnings -D clippy::pedantic
   cargo dev-test
   cargo audit
   ```

   `cargo build` is **not** one of the four gates. It is retained only as an
   **additional precheck** that may be run first for fast failure; passing it
   satisfies no gate. Note the `--` separator in the clippy invocation: it is
   required, and an invocation missing it does not pass `-D` flags to the lint
   driver.
3. **Do NOT use `cargo ci` / `cargo lint`** as the gate: both pass
   `--all-features` and fail to compile on a pre-existing `opentelemetry_sdk`
   API drift in `src/server/observability.rs` (stash `E12542FF`, `9A7C9F8F`).
   That break is out of scope here and must not be conflated with this work.
   This is a named exclusion of a **broken aggregate wrapper**, not a
   substitution for the four authoritative gates above, all four of which still
   run per leaf.
4. **Workspace-root containment** (constitution III/IV) for every filesystem
   operation.
5. **No raising of the readiness timeout** in any task.
6. **Every acceptance requirement lives in the claimable leaf.** No leaf may be
   claimed on the understanding that an additional requirement is stated only in
   this plan. Where this plan states a requirement, the owning leaf's body
   carries it as an acceptance criterion.
7. **Strict safety mode.** Every unit here runs under the workspace strict
   safety posture: no destructive command without prior operator approval
   (constitution VII/VIII), no irreversible action inside a claimable leaf whose
   approval checkpoint has not been recorded, and halt-as-blocked rather than
   proceed when an approval is unavailable.
8. **`144-S` implements no behaviour change.** Every Workstream B leaf is
   additive observability. Startup ordering, readiness semantics, storage
   resolution, and index construction are unchanged by this shipment, and
   `144.002-T` AC7 audits that.

## Implementation Units

Unit IDs are planning labels; each maps to a real backlog work item. Executable
leaves — the items Ship actually claims — are marked **leaf**. Parent task
artifacts retained in a shipment manifest are marked **integration milestone**
and carry their own bounded acceptance criteria; Revision 3 contains **no
prose-only, non-claimable container in any shipment manifest**.

Sizing note: the installed backlog registry does not advertise structured
`size`/`complexity` fields, so both axes are recorded as prose in each item body
(for example `Size: M | Complexity: medium`). This is a recorded tooling
degradation, not an omission.


## Workstream A — Daemon liveness safety (`143-F` / `143-S`, ships first)

Caps blast radius regardless of *why* startup is slow. No dependency on any
Workstream B result, on gate G3, or on `144-F`/`144-S`. Revision 3 removed the
last such coupling.

### A1 — Deterministic idle-TTL characterization (`143.001-T`, **leaf**)

Establishes the test scaffold every later A leaf depends on, and proves the
current behaviour before changing it.

* Deterministic, injectable time source for the idle-TTL owner so TTL tests do
  not wall-clock sleep.
* A **characterization test that passes GREEN** against today's behaviour:
  repeated probe-shaped connections hold the TTL open indefinitely. It documents
  the defect as the current contract; it does not assert the desired contract.
  (Revision 1 P1-02: an always-RED test cannot coexist with a green gate run in
  the same claimable unit. A1 is therefore GREEN-only, and the RED→GREEN pair
  for the fix lives entirely inside A2's leaves.)
* No production behaviour change.

### A2 — Probe-aware idle accounting with a reference-counted lease

The fix for R2. Revision 2 packed the lease, the classification, and the `S046`
contract text into one leaf; Revision 3 splits them.

#### `143.002.001-ST` (**leaf**) — reference-counted RAII useful-work lease

* A lease handle acquired when a connection is determined to be doing useful
  work and released on drop, with **reference counting** so overlapping useful
  requests do not release the TTL early.
* **The final release re-arms a FULL idle TTL measured from the release
  instant** — not from connection accept, and not from the last reset. This is
  the invariant Revision 1 P1-07 identified as unsatisfiable with a one-shot
  `ttl.reset()`.
* RED then GREEN in this leaf: overlapping-lease, nested-lease, and
  final-release-rearm tests.

#### `143.002.002-ST` (**leaf**) — classification and non-nominal release paths

* Classification happens **after** enough request information exists to tell a
  health/readiness probe from real work — never at accept time.
* A probe acquires **no** lease. Real work acquires one for its whole lifetime.
* **Cancelled and errored requests release exactly once**, via the same RAII
  path as success. A panic in a handler must not leak a lease.
* RED then GREEN: probe-holds-nothing, work-holds-lease, cancel-releases,
  error-releases.

#### `143.002.003-ST` (**leaf**) — supersede the `S046`/`T049` contract in place

* The `S046` contract text and the `// T049` comment in
  `src/daemon/ipc_server.rs::accept_loop` are **superseded in place** so the
  code and the written contract cannot disagree.
* The superseding text states the new rule (probes do not reset; leases do) and
  references this plan and the owning leaf.
* Documentation-width leaf; no behaviour change of its own.

#### `143.002-T` (**integration milestone**, retained in the manifest)

Closes **gate G8** end to end: with the three leaves merged, a daemon subjected
only to recovery-monitor probes reaches idle-TTL expiry on schedule, and a
daemon doing real work does not. Carries its own bounded acceptance criteria and
is claimable; it is not a prose container.

### A3 — Never-ready watchdog

The fix for R1 and R4. Revision 3 resolves the readiness-observer boundary that
the Revision 2 gate flagged as unresolved, and splits the hard deadline from the
forced-exit proof.

#### `143.003.001-ST` (**leaf**) — arming, disarm, and the observation seam

* The watchdog arms at a single, named point on the startup path and is
  **disarmed by readiness**.
* **One stable read-only readiness-observation seam.** The watchdog observes
  readiness through a single seam that is shared by the current managed mode and
  by the future `138-S` read-server mode. There is exactly one seam, not one per
  mode.
* The seam is **strictly read-only**: the watchdog **MUST NOT** write
  `ReadinessView`, **MUST NOT** drive activation, and **MUST NOT** refresh the
  useful-work TTL. Observation is not activity.
* **Dual disarm contract tests:** disarm is proven in managed mode *and* in the
  activation-gated mode.
* **Mandatory post-138 compatibility recheck.** The seam is rechecked against
  the as-merged `138-S` readiness architecture, with the outcome recorded,
  **before `138-S` resumes and before `144-S` is claimed**. This is an
  obligation, not an advisory note; it is carried in the leaf body and in both
  shipment bodies.
* `ENGRAM_READY_TIMEOUT_MS` is read but never raised (**G9** AC5).

#### `143.003.002-ST` (**leaf**) — hard deadline: stop, flush, drain

The bounded, cooperative part of the deadline, in three ordered steps with
per-step and summed bounds:

1. **Stop accepting** new connections.
2. **Bounded durable terminal-record flush** — the diagnostic that explains the
   termination is written and durable before exit, under its own bound.
3. **Bounded cooperative drain** of in-flight work.

Each step has an explicit bound; the sum is itself bounded, so the deadline
cannot be extended by a slow step. **PA-2 boundary:** simulation-safe tests
against an injected exit seam may run before approval; the **first real
termination execution** requires CP-3 approval.

#### `143.003.006-ST` (**leaf**) — exact-process forced exit and reacquisition proof

* Forced exit happens **without awaiting non-cancellable `spawn_blocking`
  work**. A blocked `schema_bootstrap` on a worker thread must not be able to
  hold the deadline open — this was Revision 1 P1-08.
* The exit targets the **exact process identity** that armed the watchdog; it
  may not signal a recycled PID.
* **Reacquisition proof:** a subsequent process successfully reacquires the IPC
  endpoint and the `fd_lock` after a forced exit. Without this the watchdog
  converts a hang into a permanent outage.
* A distinct exit code, allocated as in A4.

#### `143.003-T` (**integration milestone**, retained in the manifest)

End-to-end watchdog lifecycle: arm → observe → deadline → terminal record →
forced exit → reacquisition, with the retry-ledger surface live. Requires CP-3.
Claimable, with its own acceptance criteria.

### A4 — Distinguishable termination diagnostics and a bounded retry ledger

The fix for R3, plus the durable respawn bound (Revision 1 P1-09). Revision 3
corrects the Revision 2 ordering inversion: **the ledger precedes its consumer.**

Order: failure taxonomy → ledger storage → ledger attribution → backoff policy →
status surface.

#### `143.004.002-ST` (**leaf**) — failure taxonomy and wire/exit allocation

* A **distinct `ShimFailureClass` variant** for watchdog self-termination, so
  "shim gave up" and "daemon self-terminated" are never conflated.
* The CLI exit code and MCP wire code are allocated **through the existing
  authority** — `src/errors/mod.rs` currently defines `AdmissionFailure`,
  `ReadinessTimeout`, `EndpointDerivationFailure`, `TransportFailure`,
  `ProtocolIncompatible` at exit codes 10–14 and wire codes 15001–15005; the new
  variant takes the next unused value in each contiguous range. No ad-hoc code.
* **Contract tests** pin the variant, the exit code, and the wire code.
* **Parity documentation updated** (`docs/cli-mcp-parity.md`).

#### `143.004.001-ST` (**leaf**) — Workstream-A-owned diagnostics sink

* The termination diagnostic sink is **owned by Workstream A**. It is explicitly
  **not** the `144.001.001-ST` sink, and no A leaf reads or writes the B sink.
  This removes the cross-shipment coupling of Revision 1 P1-10 / P2-02.

#### `143.003.004-ST` (**leaf**) — durable retry-ledger storage protocol

* Cross-process durable storage for the consecutive-termination count.
* **Versioned format**, **inter-process lock**, flush, and **atomic replace**.
* A scope key that contains the ledger to the right workspace/branch/endpoint.

#### `143.003.007-ST` (**leaf**) — retry-ledger attribution semantics

* **Exact-child attribution**: only the child this shim actually spawned
  increments the count.
* **Reset on readiness**, **cooldown** after the bound is reached.
* Correct behaviour under **repeated short-lived CLI invocations** and under a
  **single long-lived MCP session** — the two shapes have different process
  lifetimes and must agree.

#### `143.003.003-ST` (**leaf**) — bounded backoff consuming the ledger

* The backoff policy **reads** the ledger; it does not maintain its own counter.
* **Fail-closed** when the ledger is unreadable: back off rather than respawn
  freely.

#### `143.003.005-ST` (**leaf**) — agent-visible retry state

* Consecutive-termination **count**, **remaining attempts**, and **cooldown
  remaining** are surfaced through the shared `DaemonStatus` / `HealthReport`
  structures.
* **CLI/MCP parity contract tests** prove both clients see identical state
  (R16), and the parity document is updated.

#### `143.004-T` (**integration milestone**, retained in the manifest)

Records **gate G7** closure evidence across `143.003.002-ST`,
`143.003.006-ST`, `143.004.001-ST`, and `143.004.002-ST`. Claimable, with its
own acceptance criteria.

### Workstream A ↔ `138-S`

`143-S` declares **no shipment dependency** on `138-S`, deliberately: `143-S` is
the recovery shipment, and a completion dependency would make the recovery path
unreachable if `138-S` is requeued or abandoned. This is **not** permission for
concurrent execution:

* **P-001** independently prohibits claiming `143-S` while `138-S` is claimed.
* **CP-1** requires an explicit operator **disposition or requeue of `138-S`**,
  recorded, before `143-S` is claimed.
* The `143.003.001-ST` **post-138 seam compatibility recheck** must be recorded
  before `138-S` resumes.


## Workstream B — Cold-start measurement, Phase 1 only (`144-F` / `144-S`)

**This workstream implements no fix.** Every leaf is additive observability. Its
product is a measured report that returns to Stage for Phase 2 re-planning.

Blocked by `138-S` **completion** — deliberately retained, because measuring the
pre-138 startup path would produce a baseline for a path that is about to be
replaced, which is worse than no baseline.

### B1 — Durable startup attempt records and live classification

#### `144.001.001-ST` (**leaf**) — sink, serialization, and the phase protocol

* A durable, append-oriented startup record sink that survives a mid-stall kill.
* **A single serializing writer.** Records from concurrent attempts interleave
  without tearing (Revision 1 P1-13).
* **The phase protocol:** a durable `phase_started` record is emitted **before**
  every potentially blocking phase and a `phase_completed` record **after** it.
  A phase that never completes is therefore diagnosable *mid-stall*, which is
  the entire point — a record written only on completion cannot describe a hang.
* Versioned record schema.
* **B-owned.** Workstream A does not read or write this sink.

#### `144.001.002-ST` (**leaf**) — attempt identity and the complete phase set

* Every record carries: **startup-attempt ID**, **workspace identity**,
  **branch**, **generation identity**, **endpoint**, **process-start identity**,
  and a **monotonic sequence number**. Without these, records from concurrent or
  successive attempts cannot be separated and a single attempt cannot be
  classified (Revision 1 P1-11).
* **The complete phase set** is instrumented: `admission`,
  `prepare_branch_owner`, `process_lock`, `file_lock`, `database_open`,
  `schema_bootstrap`, **readiness publication**, `hydrate_code_graph`,
  `detect_offline_changes`, and the optional `sync_code_graph`.
* IPC-bind and watcher initialization are retained as **explicit additions if
  measured** — named in the leaf so a measured blocking phase is not silently
  omitted from the taxonomy.

#### `144.001.003-ST` (**leaf**) — heartbeat supervision with an outer async finalizer

* A heartbeat proves the difference between "stalled" and "dead".
* **Lifecycle:** an **outer async finalizer** cancels the heartbeat task **and
  JOINS it** before the terminal record is written. A synchronous `Drop` guard
  may *request* cancellation, but the join happens in the async finalizer.
  Revision 2 specified "cancel and join in a drop guard", which is **impossible**
  — `Drop` is synchronous and cannot await a join. Revision 3 removes that
  impossible contract.
* The final **phase state and terminal state are published atomically**, so no
  reader can observe a terminal record with a stale phase.
* Closes the post-terminal-emission race of Revision 1 P1-13.

#### `144.001.004-ST` (**leaf**) — correlated live classification, CLI and MCP

* Classification **correlates same-attempt readiness state with transport/probe
  evidence**. `readiness_published` alone is not evidence of reachability
  (Revision 1 P1-12).
* Three outcomes are tested: **healthy**, **pre-ready stall**, and
  **ready-but-unreachable**.
* The classification is exposed through the **shared `HealthReport` /
  `DaemonStatus`** structures to **both CLI and MCP**, with a **parity contract
  test** (R16). Filesystem logs alone do not satisfy this.

#### `144.001.005-ST` (**leaf**) — performance baseline

* Per-phase wall time and **per-phase peak RSS**.
* A **named reference environment** recorded by its measured attributes so a
  later comparison is meaningful.
* **G1, G2, and G3 baseline dimensions are recorded as measured values**, not
  enforced as thresholds.
* **Explicit PID 30528 exclusion**: the incident daemon's figures are excluded
  from the baseline and the exclusion is stated in the artifact.
* **OD-1 halt condition:** if the incident daemon is still running and stop
  approval has not been given, this leaf **halts and returns to Stage**.
  Measuring around a live CPU-burning process produces numbers that look like
  data and are not.

#### `144.001-T` (**integration milestone**, retained in the manifest)

End-to-end observability integration: a real cold start produces a complete,
identified, serialized, classified record set, and the baseline artifact is
filed with OD-1 disposition evidence attached. Claimable, with its own
acceptance criteria.

### B2 — Schema and HNSW sub-phase attribution, and the decision gate

This is the unit that answers R7 and decides Phase 2's fate.

#### `144.002.001-ST` (**leaf**) — per-script and per-migration attribution

* `run_scripts` is instrumented **per schema script** and **per migration**.
* Measurement only: no script is reordered, skipped, or made conditional.

#### `144.002.002-ST` (**leaf**) — per-HNSW-operation timing with create-vs-skip outcome

* **Each HNSW operation** is timed individually.
* **Each operation records an explicit create-vs-skip outcome.** A skipped index
  that is timed as though it were created inverts the conclusion, which is why
  Revision 1 P1-15 flagged the Revision 0 ordering. In Revision 3 this leaf
  **precedes** the consolidated report rather than sitting behind it.

#### `144.002-T` (**integration milestone**, retained in the manifest)

Consolidates the measurement into a report and applies **the decision gate**:

> **PROCEED** requires HNSW construction at **≥ 50 %** of measured
> `schema_bootstrap` time on the named reference environment, with **median AND
> worst case both consistent**. **Any other result HALTS and returns to Stage.**

Both outcomes return to Stage. **This shipment authorises no Phase 2
implementation under any result.** The disposition is recorded at **CP-2**.

`144.002-T` **AC7** additionally audits the measurement-only boundary: no change
to startup ordering, readiness semantics, or storage resolution; no index
construction moved off the readiness path; no warming state introduced; no
generation created, published, attached, or deleted; no threshold enforced.

**No B3 implementation leaf occurs in `144-S`.**

## Deferred Phase 2 — accepted architecture direction, not an implementation plan

`docs/decisions/2026-09-10-content-addressed-generation-cold-start-decision.md`
remains **accepted as direction**. Revision 3 asserts only that it is **not yet
an implementation plan**, and that no plan may make it one until `144-S`
reports.

The 28 items `144.003-T` … `144.008-T` and their subtasks are **blocked**,
removed from every shipment manifest and from queue ordering, stripped of all
dependency edges, and retained under `144-F` as **traceable follow-on
candidates**. Each body has been replaced with a concise blocked-scope record.
They are **not implementation-ready** and **must not be claimed**. They were
deliberately **not deleted or archived**: destructive disposition was not
approved, and the traceability from requirement → deferred candidate is the
record that a future plan needs.

### Mandatory inputs to the future Phase 2 Stage plan

A future Stage plan may not be written until it can answer **all** of the
following. These are the unresolved questions the Revision 2 gate identified;
listing them here is how Revision 3 keeps them from being lost.

| # | Unresolved design question |
|---|---|
| Q1 | **Catalog vs active pointer.** Is a multi-generation catalog authoritative, or a single `active.json`? The two imply different attach, publication, and GC protocols, and Revision 2 assumed both in different places. |
| Q2 | **Clean-candidate producer.** What process builds a clean generation, when, and under what lifecycle? Revision 2 never named an owner, which is why the build window was left open. |
| Q3 | **Serving copy vs snapshot source.** Is the served artifact a snapshot of a quiesced source, or the source itself? This determines whether Cozo/SQLite quiescence under exclusive authority is required before publication (Revision 1 P1-23). |
| Q4 | **Attach integrity.** What is the fail-closed precondition set at attach — opener identity vs manifest, directory name vs ID, artifact digest vs sealed manifest, publishability — and what separates a recoverable cache miss from a corruption/I-O/lock failure that must propagate (Revision 1 P1-22)? |
| Q5 | **Runtime-copy lease.** How is a per-opener session lease held so two concurrent openers of one generation never share a writable database (R15, Revision 1 P1-21)? |
| Q6 | **Publication.** What makes publication atomic and crash-safe, and what is the exact quiescence authority it runs under? |
| Q7 | **Reader lease and GC.** What shared atomic pin/lease closes the TOCTOU window between GC selection and irreversible deletion (Revision 1 P1-24)? |

Supporting concerns that also carry forward: the effective indexing fingerprint's
content-affecting input set and per-field mutation tests (P1-17); building clean
generations from immutable Git tree objects rather than mutable working-tree
files (P1-18); eliminating heuristic "nearest matching clean generation" attach
(P1-19); and capability-rooted, no-follow, owner-only, workspace-contained I/O
for runtime-copy and GC paths (P1-20). The Plan Hardening **security signal is
PRESENT** for all of this (P1-25) and the future plan inherits that finding.

### `R-LEGACY-DB-RECLAIM`

The 45 legacy per-branch databases consuming 2.29 GB remain **deferred**.
Revision 3 **removes** the Revision 2 arrangement that made creating its
follow-up a closure condition of `144-S`: reclamation belongs to the Phase 2
implementation plan, not to a measurement shipment's closure. It is recorded
here and inherited by the future plan.


## Dependency Graph

Edges are recorded in the backlog and are authoritative there; this graph is the
human-readable view. All Revision 2 edges touching deferred Phase 2 items were
**removed**, and two Workstream A inversions were corrected.

### `143-S` — Workstream A

```text
143.001-T (GREEN characterization + deterministic TTL scaffold)
   └─> 143.002.001-ST (reference-counted RAII lease, full-TTL re-arm on final release)
          └─> 143.002.002-ST (post-request-information classification; cancel/error release)
                 └─> 143.002.003-ST (supersede S046/T049 contract text in place)
                        └─> 143.002-T  [milestone — gate G8]

143.003.001-ST (arm/disarm; ONE read-only readiness-observation seam; dual disarm tests;
                mandatory post-138 compatibility recheck)
   └─> 143.003.002-ST (hard deadline: stop-accepting → bounded durable flush →
                       bounded cooperative drain)
          ├─> 143.003.006-ST (no-await forced exit; exact process identity;
          │                   endpoint + fd_lock reacquisition proof)
          └─> 143.004.002-ST (ShimFailureClass variant; exit/wire code via existing
                              authority; contract tests; parity doc)
                 └─> 143.003.004-ST (durable retry-ledger storage: versioned,
                                     inter-process locked, atomic replace)
                        └─> 143.003.007-ST (attribution: exact-child, reset, cooldown,
                                            CLI and MCP shapes)
                               └─> 143.003.003-ST (bounded backoff CONSUMING the ledger,
                                                   fail-closed)
                                      └─> 143.003.005-ST (count/remaining/cooldown via
                                                          DaemonStatus + HealthReport,
                                                          CLI/MCP parity tests)

143.004.001-ST (A-owned diagnostics sink)  ──> 143.003.002-ST

143.003-T  [milestone] ← 143.003.005-ST, 143.003.006-ST
143.004-T  [milestone — gate G7 evidence] ← 143.004.002-ST, 143.003.006-ST
```

**Ordering correction (Revision 3).** Revision 2 ordered the backoff policy
*before* the durable ledger it consumes, so the bound could pass a
single-process test while being unbounded in production. The chain is now
taxonomy → ledger storage → ledger attribution → backoff → status surface.

### `144-S` — Workstream B, Phase 1

```text
144.001.001-ST (durable serialized sink; phase_started/phase_completed protocol)
   └─> 144.001.002-ST (attempt identity ×6 + monotonic sequence; complete phase set)
          ├─> 144.001.003-ST (heartbeat outer async finalizer: cancel AND join;
          │                   atomic phase+terminal publication)
          │      └─> 144.001.004-ST (correlated classification; healthy /
          │                          pre-ready-stall / ready-but-unreachable;
          │                          shared HealthReport+DaemonStatus; CLI/MCP parity)
          ├─> 144.001.005-ST (per-phase peak RSS; reference environment;
          │                   G1/G2/G3 baseline dimensions; PID 30528 exclusion;
          │                   OD-1 halt condition)
          └─> 144.002.001-ST (per-schema-script and per-migration attribution)
                 └─> 144.002.002-ST (per-HNSW-operation timing; explicit
                                     create-vs-skip outcome)

144.001-T  [milestone] ← 144.001.004-ST, 144.001.005-ST
144.002-T  [milestone — decision gate, returns to Stage]
             ← 144.002.002-ST, 144.001.004-ST, 144.001.005-ST

144-S ──depends_on──> 138-S   (completion dependency, deliberately retained)
```

**Dependency-inversion corrections (Revision 3).** Revision 2 contained
`144.002-T → 144.003.001-ST` (a milestone depending on a deferred implementation
leaf) and `144.003.002-ST → 144.002-T`. Both edges are removed along with the
other 27; no executable item depends on a blocked item anywhere in either
shipment.

## Decisions and Rationale

Revision 3 retains the decisions that survive the scope reset, supersedes those
that assumed Phase 2 execution, and adds five.

| # | Decision | Status | Rationale |
|---|---|---|---|
| D1 | Split into two shipments rather than one | **Retained** | The defects are independent in cause, fix, and blast radius. Bundling would block the small safety fix behind the large correctness fix — the opposite of the risk-adjusted order. |
| D2 | Workstream A ships first | **Retained, and now more strongly** | It bounds the damage regardless of root cause. With Phase 2 deferred, A is the *only* unit that converts an indefinite 2-core/1.84 GB burn into a bounded, diagnosable failure. |
| D3 | Measure before fixing | **Retained — and this is now the whole of `144-S`** | H1 (HNSW dominance) is explicitly **unproven**, and the spike's ≥99.6 % result is *conditional* on a stall that is itself unverified. Revision 2 acknowledged this and then specified eight implementation units anyway. Revision 3 makes D3 structural rather than rhetorical. |
| D4–D9 | Content identity on tree OID; `content_key_kind` discriminator; TLV canonical encoding; untracked-files-make-dirty; no promotion of dirty builds; reuse the `136-S` subsystem | **Deferred** | Sound as *direction* and preserved in the accepted decision record. Not implementable until Q1–Q7 are answered. |
| D10 | Probe classification occurs after request information, not at `accept()` | **Retained** | At bare `accept()` the daemon cannot distinguish a probe from a real client; classifying there would guess, and guessing "probe" could starve a genuine client into premature shutdown. |
| D11 | Supersede the `134-S` "should not distinguish in production" guidance | **Retained** | That guidance was correct for its evidence (a *test* probe against a *ready* daemon). The new evidence is a *production* shim probe against a *never-ready* daemon, where the reset is never semantically justified and disables the only remaining backstop. Supersession is narrow: genuine work still holds the daemon alive. |
| D12 | `143-S` carries no shipment dependency on `138-S` | **Retained** | The workstreams are surface-disjoint and `143-S` must remain claimable as a recovery shipment if `138-S` is requeued or abandoned. P-001 still forbids concurrent claiming, and CP-1 requires an explicit recorded disposition. |
| D13 | Watchdog default 15 min, provisional | **Retained** | 2× the slowest observed *successful* cold start (~7.5 min, 2026-08-23) and ~30× the readiness probe timeout, so it cannot fire on any observed healthy profile. Re-validation against `144.001.002-ST`'s measured distribution remains required. `ENGRAM_READY_TIMEOUT_MS` is a **separate** knob pinned unchanged by G9. |
| D14 | An adverse measurement HALTs and returns to Stage | **Retained and widened** | In Revision 2 only an adverse B2 result halted. In Revision 3 **both** outcomes return to Stage, because no measured result authorises implementation from this plan. |
| D15 | A reference-counted RAII useful-work lease replaces the one-shot TTL reset | **Retained** | A single `ttl.reset()` is correct only for non-overlapping requests: with two concurrent requests the longer one's completion is unaccounted for, and a cancelled or errored request leaves no trace. A lease held for the request's lifetime and released on drop is correct under overlap, cancellation, and error. The invariant is binding; any equivalent proven in-tree guard protocol satisfies it. |
| D16–D19 | Exact-match dirty attach; per-opener runtime-copy keys; quiescent publication; shared reader/GC lease | **Deferred → promoted to mandatory design inputs** | Each was a *partial answer* to a question Revision 2 never fully posed. They are re-expressed as Q3–Q7 so the future plan must resolve them as a coherent set rather than as scattered patches. |
| D20 | Containers carry no acceptance criteria and are never claimable | **Superseded by D23** | Correct in intent, wrong in application: Revision 2 then left those non-claimable containers *inside shipment manifests*, so Ship was handed items it could not claim. |
| **D21** | **`144-S` is measurement-only and produces a report, not a fix** | **New** | The Revision 2 FAIL was not a detail failure; it was a category failure. Specifying an implementation whose premise is unmeasured produces plan churn, not progress. A shipment whose deliverable is a decision is an honest release unit. |
| **D22** | **Speculative Phase 2 items are blocked and retained, never deleted or archived** | **New** | Destructive disposition was not approved, and the requirement → candidate → future-plan trace is exactly what a re-planning pass needs. Blocking removes them from claimable work without destroying the record. |
| **D23** | **Every item in a shipment manifest must be executable** | **New** | A manifest is a claim list. A prose-only container in a manifest is an item Ship must skip, which makes the manifest a poor description of the work. Parent task artifacts are therefore either given a bounded integration milestone with real acceptance criteria, or removed from the manifest while their hierarchy is preserved. |
| **D24** | **The watchdog observes readiness through ONE stable read-only seam** | **New** | Revision 2 left the observer boundary unresolved, which permitted a watchdog that could write `ReadinessView`, drive activation, or refresh the useful-work TTL — any of which makes the backstop self-defeating. One seam, read-only, shared by managed and future `138-S` read-server mode, with a mandatory post-138 recheck. |
| **D25** | **Heartbeat join happens in an outer async finalizer, not in `Drop`** | **New** | Revision 2 required "cancel and join in a drop guard". `Drop` is synchronous and cannot await a join, so that contract was unimplementable. A sync guard may *request* cancellation; the async finalizer performs the join before the terminal record. |

## Measurable acceptance gates

Every gate names an **executable leaf or claimable milestone**, never a
prose-only container. Gates whose only satisfying implementation is deferred are
recorded as **deferred gates** — they are not owned by any item in either
shipment, and the future Phase 2 plan inherits them.

| # | Gate | Threshold | Owner | Status in Revision 3 |
|---|---|---|---|---|
| G1 | Pre-synced same-tree branch readiness | Ready **< 10 s** on the named reference environment | `144.001.005-ST` (baseline dimension) | **Measured, not enforced.** The current value is recorded as a baseline dimension; the target is inherited by the future plan. |
| G2 | Warm reopen | Ready **< 5 s**, median and worst case | `144.001.005-ST` (baseline dimension) | **Measured, not enforced.** Same treatment as G1. |
| G3 | Bounded memory | Peak RSS **< 1 GB** during startup, platform-named counter (observed failure: 1.84 GB) | `144.001.005-ST` | **Measured, not enforced.** Per-phase peak RSS is recorded; enforcement was owned by a now-deferred leaf and is inherited by the future plan. |
| G4 | No cross-branch leakage | Branch-A queries never return branch-B-only content | — | **Deferred gate.** No owner in either shipment. |
| G5 | Dirty-tree correctness | Dirty tree never publishes a shared generation | — | **Deferred gate.** No owner in either shipment. |
| G6 | Crash recovery | No partially visible generation after a kill at any publication point | — | **Deferred gate.** No owner in either shipment. |
| G7 | Never-ready self-termination | A daemon that cannot reach readiness self-terminates within budget **and** records a distinct, durable, client-visible terminal cause | `143.003.002-ST` + `143.003.006-ST` + `143.004.001-ST` + `143.004.002-ST` (joint); evidence recorded at `143.004-T` | **Enforced by `143-S`.** |
| G8 | Probe-safe idle accounting | Probes alone do not prevent idle expiry; genuine work — including overlapping, cancelled, and errored work — still holds the daemon alive | `143.002-T` | **Enforced by `143-S`.** |
| G9 | No timeout inflation | `ENGRAM_READY_TIMEOUT_MS` default unchanged | `143.003.001-ST` | **Enforced by `143-S`.** |

**Why G1/G2/G3 changed status.** Revision 2 recorded them as release-blocking
thresholds owned by implementation leaves that Revision 3 defers. Leaving them
as enforced gates with deferred owners would be dishonest — an unownable gate is
not a gate. Recording them as **measured baseline dimensions** keeps the numbers
live, makes the current state visible, and hands the future plan a real
before-figure instead of an aspiration. G3 in particular becomes *measurable*
now and *true* later, which is the correct order.

**No gate in this plan is closed by a blocked item.**

## Risks and Caveats

| # | Risk | Mitigation |
|---|---|---|
| H1 | HNSW dominance of `schema_bootstrap` is **unproven** | **This is now the plan's central subject rather than an assumption.** `144.002.002-ST` measures it per operation with explicit create-vs-skip outcomes, and `144.002-T` applies a binary threshold whose adverse branch halts to Stage. |
| H2 | The stall may be ready-but-unreachable rather than pre-ready | `144.001.004-ST` correlates same-attempt readiness with transport/probe evidence and tests all three outcomes. Revision 3 does not build on either hypothesis before this leaf lands. |
| H3 | Instrumentation itself perturbs the measurement | All Workstream B leaves are additive; `144.002-T` AC7 audits that startup ordering, readiness semantics, and storage resolution are unchanged. The rollback for every B leaf is a straight revert. |
| H4 | The watchdog fires on a startup that would have succeeded | D13's default is 2× the slowest observed successful cold start; the 7-day observation window in *Rollback and monitoring* must show zero healthy-startup firings before the default is treated as validated. |
| H5 | The forced exit leaves the IPC endpoint or `fd_lock` unreclaimable, converting a hang into a permanent outage | `143.003.006-ST` makes subsequent-process reacquisition of both an explicit acceptance criterion. |
| H6 | `cargo ci` / `cargo lint` fail on pre-existing `opentelemetry_sdk` drift | Named exclusion of a broken aggregate wrapper; all four authoritative gates still run per leaf. Not to be conflated with this work. |
| H7 | The retry bound passes a single-process test while being unbounded in production | `143.003.004-ST` gives the counter a durable, versioned, inter-process-locked, atomically replaced cross-process home, and `143.003.007-ST` tests both the repeated-short-lived-CLI and long-lived-MCP shapes. |
| H8 | Probe classification regresses open, so everything looks like useful work | Monitoring records probe-vs-useful classification rate and outstanding-lease count; a rate trending to 100 % is the rollback trigger. |
| H9 | The A and B sinks converge, coupling the shipments | Sink ownership is an acceptance criterion on both sides: `143.004.001-ST` is A-owned, `144.001.001-ST` is B-owned, and neither workstream reads or writes the other's sink. |
| H10 | The `143.003.001-ST` observation seam breaks when `138-S` merges | The post-138 compatibility recheck is **mandatory and recorded** before `138-S` resumes and before `144-S` is claimed — carried in the leaf body and in both shipment bodies, not as an advisory note. |
| H11 | Concurrent writers tear the startup record stream | `144.001.001-ST` mandates a single serializing writer and a versioned record schema. |
| **H18** | **Deferred Phase 2 items are mistaken for ready work** | Each of the 28 items is `status: blocked` with a `blocked_reason`, carries a concise blocked-scope record instead of an implementation body, is absent from every shipment manifest and from queue ordering, and has no dependency edges. |
| **H19** | **The measurement shipment quietly grows a fix** | `144.002-T` AC7 is an explicit measurement-only audit, and this plan states that no measured result authorises implementation. Any such change is a scope violation that returns to Stage. |
| **H20** | **The deferred design questions are lost between plans** | Q1–Q7 are recorded in this plan as mandatory inputs, the accepted decision record is annotated to say implementation planning is deferred pending the measurement report, and the 28 candidate items remain traceable under `144-F`. |
| `R-LEGACY-DB-RECLAIM` | 45 legacy per-branch databases consuming 2.29 GB | **Deferred.** Revision 3 removes the Revision 2 arrangement that made creating its follow-up a `144-S` closure condition; it belongs to the Phase 2 implementation plan. Reclamation remains operator-approved and never agent-initiated. |

## Plan Hardening

**Hardening signals: PRESENT.** Irreversible action (process termination),
trust-boundary work (cross-process and cross-branch isolation), and
client-visible contract change are all present in `143-S`. The **security**
signal is **PRESENT** (Revision 1 P1-25) and is inherited in full by the future
Phase 2 plan, where the irreversible-deletion surface lives.

### Risky actions

| # | Proposed action | `ActionRisk` | `ActionResult` | Gate |
|---|---|---|---|---|
| PA-1 | Publish a shared immutable generation | high | **deferred** | Not in either shipment. Inherited by the future Phase 2 plan. |
| PA-2 | **Self-terminate a running daemon** | high | **blocked** | **Approval is required before the FIRST REAL TERMINATION EXECUTION**, including in test and development runs — not merely before merge. **Simulation-safe tests against an injected exit seam MAY run before approval.** Gated at CP-3; mirrored into `143.003.002-ST` and `143.003.006-ST`. |
| PA-3 | Serve a database resolved from a shared generation | high | **deferred** | Not in either shipment. Inherited by the future Phase 2 plan. |
| PA-4 | Reclaim generation storage (irreversible deletion) | high | **deferred** | Not in either shipment. Inherited by the future Phase 2 plan, which must satisfy constitution VII. |

### Added constraints

| # | Constraint | Owning leaf |
|---|---|---|
| H12 | Watchdog arming point is named and disarm is proven in **both** managed and activation-gated modes | `143.003.001-ST` |
| H13 | Durable cross-process consecutive-termination counter | `143.003.004-ST` (storage) + `143.003.007-ST` (attribution) |
| H14 | Platform-explicit durability; flush before rename | `143.004.001-ST` |
| H15 | Named reference environment recorded by measured attributes | `144.001.005-ST` |
| H16 | GC deletion authority and materialized snapshot | **Deferred** — inherited by the future Phase 2 plan |
| H17 | Kill switch and resolution provenance | **Deferred** — inherited by the future Phase 2 plan |
| **H21** | **The readiness-observation seam is read-only**: no `ReadinessView` write, no activation, no useful-work TTL refresh | `143.003.001-ST` |
| **H22** | **The hard deadline is bounded per step and in sum**: stop-accepting, bounded durable terminal-record flush, bounded cooperative drain | `143.003.002-ST` |
| **H23** | **Forced exit does not await non-cancellable `spawn_blocking` work** and targets the exact process identity | `143.003.006-ST` |
| **H24** | **Heartbeat cancel-and-JOIN happens in an outer async finalizer**; phase and terminal state publish atomically | `144.001.003-ST` |
| **H25** | **OD-1 blocks every recorded figure**: if the incident daemon is still running without stop approval, the measurement halts and returns to Stage | `144.001.005-ST`, `144.002-T` |

### Requirement-mirroring record

**Every** constraint, risky-action obligation, and checkpoint condition stated in
this plan is mirrored into the acceptance criteria of the owning **executable
leaf or claimable milestone**. No item may be claimed on the understanding that
an additional requirement lives only in this plan. If a future hardening pass
adds a constraint, it must be mirrored into the owning item in the same pass.

| Plan requirement | Owning item | Mirrored as |
|---|---|---|
| H12, H21, G9, post-138 recheck | `143.003.001-ST` | AC1–AC9 |
| H22, PA-2/CP-3 approval boundary | `143.003.002-ST` | bounded-step ACs + approval AC |
| H23 | `143.003.006-ST` | no-await, exact-identity, reacquisition ACs |
| H13 storage | `143.003.004-ST` | versioned/locked/atomic ACs |
| H13 attribution | `143.003.007-ST` | exact-child, reset, cooldown, CLI+MCP ACs |
| Ledger-before-backoff, fail-closed | `143.003.003-ST` | consumer ACs |
| R16 agent-visible retry state | `143.003.005-ST` | `DaemonStatus`/`HealthReport` + parity ACs |
| H14, A-owned sink | `143.004.001-ST` | durability + ownership ACs |
| R3, exit/wire allocation | `143.004.002-ST` | taxonomy + contract-test + parity-doc ACs |
| R5, serialized sink, phase protocol | `144.001.001-ST` | AC set |
| R5, attempt identity, complete phase set | `144.001.002-ST` | AC set |
| H24 | `144.001.003-ST` | finalizer + atomic-publication ACs |
| R6, R16 | `144.001.004-ST` | correlation + parity ACs |
| H15, H25, G1/G2/G3 baseline, PID 30528 exclusion | `144.001.005-ST` | AC1–AC5 |
| R7 sub-phase attribution | `144.002.001-ST`, `144.002.002-ST` | per-script / per-operation ACs |
| CP-2, decision threshold, measurement-only audit | `144.002-T` | AC3, AC4, AC7 |

## Runtime Verification and Closure

| Unit | Runtime verification | Closure condition |
|---|---|---|
| `143.001-T` | Deterministic TTL harness runs without wall-clock sleep; characterization test GREEN | Scaffold merged; current behaviour documented as the existing contract |
| `143.002.001-ST` … `143.002.003-ST` | Overlap, nesting, final-release re-arm, probe-no-lease, cancel, error | All four authoritative gates green per leaf |
| `143.002-T` | Probe-only traffic reaches idle expiry on schedule; real work does not | **G8 closed** |
| `143.003.001-ST` | Arm/disarm proven in managed **and** activation-gated modes; seam proven read-only | **G9 closed**; post-138 recheck scheduled and recorded |
| `143.003.002-ST`, `143.003.006-ST` | Bounded stop/flush/drain within the summed bound; forced exit without awaiting `spawn_blocking`; endpoint and `fd_lock` reacquired by a subsequent process | CP-3 approval recorded before first real termination execution |
| `143.003.004-ST`, `143.003.007-ST`, `143.003.003-ST`, `143.003.005-ST` | Cross-process ledger survives process death; exact-child attribution; reset and cooldown; repeated-CLI and long-lived-MCP shapes; backoff fail-closed on unreadable ledger; CLI/MCP parity | Retry state visible through `DaemonStatus`/`HealthReport` with parity tests green |
| `143.004.001-ST`, `143.004.002-ST` | Distinct terminal cause durable and client-visible; exit and wire codes pinned by contract tests; parity doc updated | **G7 evidence recorded at `143.004-T`** |
| `143.003-T`, `143.004-T` | End-to-end watchdog lifecycle and G7 evidence | `143-S` closable |
| `144.001.001-ST` … `144.001.003-ST` | A mid-stall kill leaves a durable record showing the in-progress phase; no torn interleaving; no heartbeat record after the terminal record | Sink live and versioned |
| `144.001.004-ST` | Healthy, pre-ready-stall, and ready-but-unreachable each classified correctly and identically via CLI and MCP | **R6, R16 satisfied** |
| `144.001.005-ST` | Per-phase wall time and peak RSS on the named reference environment; PID 30528 excluded | Baseline artifact filed with OD-1 disposition evidence |
| `144.001-T` | A real cold start yields a complete, identified, serialized, classified record set | `144.001-T` milestone green |
| `144.002.001-ST`, `144.002.002-ST` | Per-script, per-migration, and per-HNSW-operation attribution with explicit create-vs-skip outcomes | Attribution data complete |
| `144.002-T` | Threshold evaluated on median **and** worst case; measurement-only audit performed | **Report filed, disposition recorded at CP-2, RETURNED TO STAGE**; `144-S` closable |

`R-LEGACY-DB-RECLAIM` is **not** a closure condition of `144-S`.

## Rollback and monitoring

Owners are role-based (OD-6). "Ship" is the executing owner; "Operator" is the
approving owner.

| Unit(s) | Rollback trigger | Rollback procedure | Monitoring signal | Owner | Validation window |
|---|---|---|---|---|---|
| `143.002.001-ST` … `143.002.003-ST`, `143.002-T` | A genuine tool call is cut off mid-flight, or useful-work lease acquisition rate rises toward 100 % (classifier regressed open) | Revert to unconditional `ttl.reset()`; restore the prior `S046`/`T049` contract text in the same revert | Probe-vs-useful classification counts; outstanding-lease count; idle-TTL expiry events — emitted to the **A-owned** sink | Ship | 3 CI runs + 5 local dev sessions with zero premature shutdowns |
| `143.003.*`, `143.004.*` (**roll back together**) | Any watchdog firing on a startup that would have succeeded; any shim refusal to respawn a healthy workspace | Disable the watchdog arm via its sentinel, then revert the Workstream A watchdog and ledger leaves as one change | Watchdog terminations by failure class; ratio of shim `readiness_timeout` to daemon watchdog terminal cause (this ratio **is** the R3 triage discriminator); retry-ledger count, remaining, and cooldown — all emitted to the **A-owned** sink and surfaced via `DaemonStatus`/`HealthReport` | Ship; Operator approves at CP-3 | 7-day observation with zero healthy-startup firings before the 15-min default is treated as validated; D13 re-validation against measured data still separately required |
| `144.001.*`, `144.002.*` | Startup wall-time regression outside noise, or heartbeat records appearing after a terminal record | Revert instrumentation; every B leaf is additive and independently revertible | Per-phase durations, per-phase peak RSS, startup classification distribution | Ship | One full cold start on each reference platform |

**Sink routing.** Workstream A signals go to the **A-owned** sink
(`143.004.001-ST`); Workstream B signals go to the **B-owned** sink
(`144.001.001-ST`). Neither reads or writes the other's, which is what keeps
`143-S` independently shippable. No new observability subsystem is introduced.

## Operator checkpoints

If the operator is unavailable at a checkpoint, the correct behaviour is to
**halt as blocked**, never to proceed (strict-safety Rule 2; otherwise P-005).

| # | When | Decision | Evidence required |
|---|---|---|---|
| CP-1 | **Before any claim in either shipment** | `138-S` disposition. Neither shipment may be claimed while `138-S` is claimed (P-001). `143-S` additionally requires an explicit disposition or requeue of `138-S`; `144-S` additionally requires `138-S` **completion** | Authoritative `138-S` status **and** the Ship checkpoint state. **Publication update (2026-09-10) — the Revision 3 conflict is RESOLVED.** *Historical (Revision 3, superseded):* the evidence was recorded as in conflict — the backlog record read `queued`, commit `60f0ae4d` asserted a claim, and the most recent checkpoint record was archived and quarantined. *Resolved state (authoritative):* the operator explicitly selected the Ship checkpoint `checkpoint-20260910-222318.json` (`agent: ship`, `status: active`, shipment `138-S`) and approved RS5/F17 implementation for all of `138-S`; commit `60f0ae4d` is authoritative; **`138-S` is active**; the current-branch/base copy of `138-S` reading `queued` is **stale**. Consequently neither `143-S` nor `144-S` may be claimed concurrently with `138-S` (P-001), and `144-S`'s completion dependency is unaffected — active is not complete. The checkpoint is **Ship-owned and was not modified by Stage**. Ship still captures `138-S` status and checkpoint state fresh at claim time. Plus **full P-016 branch and worktree topology evidence**: `git worktree list`, `git branch --show-current` per worktree, `git status --porcelain` per worktree, and confirmation that no second implementation worktree or parallel implementation branch exists. Plus the exact exclusion boundary restated |
| CP-2 | Before `144-S` closes | Record the `144.002-T` disposition: **proceed** or **halt** — both return to Stage. No result authorises implementation from this plan | Path to the filed `144.002-T` measurement report |
| CP-3 | **Before the first REAL termination execution** — including in development runs, not merely before merge | Approve PA-2: the 15-minute default and the stop-respawning terminal behaviour. Simulation-safe tests against an injected exit seam may run before approval | Budget-margin rationale plus the H12/H13/H22/H23 acceptance criteria demonstrated green in simulated form |
| CP-4 | **Deferred** | Was PA-3 approval; no owner in either shipment | Inherited by the future Phase 2 plan |
| CP-5 | **Deferred** | Was PA-4 (irreversible deletion) approval under constitution VII; no owner in either shipment | Inherited by the future Phase 2 plan |
| CP-6 | **Removed as a `144-S` closure condition** | `R-LEGACY-DB-RECLAIM` follow-up creation belongs to the future Phase 2 implementation plan, not to a measurement shipment's closure | Recorded in this plan and inherited |

## Unresolved operator decisions

These block safe execution and are not Stage's to decide.

* **OD-1 — Incident daemon PID `30528`. OPEN, and now blocking.** Left running
  (~2.5 h, ~198 % CPU, 1.84 GB RSS) because stopping it was never separately
  approved. It **blocks every figure `144-S` would record**, because timings
  measured alongside it are invalid. Decision: terminate it — and if so, whether
  to first capture a process dump and/or its branch DB as evidence, since
  terminating it destroys the only live instance of the observed failure.
  `change_kind: process termination`, `ActionRisk: moderate` (destroys evidence,
  not data), `ActionResult: blocked`.
* **OD-2 — Kill-switch default for generation reuse. OPEN but no longer
  urgent.** Deferred with Phase 2; inherited by the future plan. Stage's
  recommendation stands: default-off for a first release, because the failure
  mode is silent.
* **OD-3 — A4 sink choice. CLOSED in Revision 2.** `143.004.001-ST` mandates the
  A-owned sink outright and removes the shared-file option, so no per-platform
  atomic-append proof is needed. Retained as a closed record.
* **OD-4 — Reference environment for the baseline. OPEN.** Which workspace,
  which platforms, cold or warm page cache, repetition count, and reported
  statistic. Now required by `144.001.005-ST` rather than by a Phase 2 gate.
* **OD-5 — Validation-window length. OPEN.** The windows above are Stage's
  proposal. A window may be shortened only by substituting stronger evidence,
  never by removing it.
* **OD-6 — Named accountable owner. OPEN.** This plan carries role owners only.
  If the release gates require a named human owner, the operator must name one;
  Stage cannot assign it.

## Constitution Check

Mapped against `.github/instructions/constitution.instructions.md`, Core
Principles **I–XI**.

| Principle | Applicable | Compliance |
|---|---|---|
| **I. Safety-First Rust** | Yes | All leaves are Rust 2024 under `#![forbid(unsafe_code)]`; no leaf requires `unsafe`. Errors propagate as `Result<T, EngramError>` via `?`; `unwrap_used`/`expect_used` remain denied. The watchdog exit in `143.003.006-ST` is an explicit orderly shutdown with a distinct exit code, not a `panic!`. Gate 2 (`cargo clippy --all-targets -- -D warnings -D clippy::pedantic`, note the `--`) runs on every leaf. |
| **II. Test-First Development (NON-NEGOTIABLE)** | Yes | Every leaf declares an execution posture and a RED-before-GREEN requirement **within the same claimable unit**. `143.001-T` is GREEN-only characterization precisely so that no leaf is required to be simultaneously always-red and gate-green. Three-tier layout respected; contract tests in `tests/contract/` for the CLI/MCP parity surfaces in `143.003.005-ST`, `143.004.002-ST`, and `144.001.004-ST`. `cargo dev-test` is gate 3 for every leaf. |
| **III. Workspace Isolation and Security Boundaries** | Yes | Every new filesystem surface resolves under the workspace root: the A-owned diagnostics sink, the shim retry ledger (`143.003.004-ST`, with a containing scope key), and the B-owned startup sink (`144.001.001-ST`). Path-traversal rejection is explicit. The deferred Phase 2 surfaces — runtime copies and GC — carry the capability-rooted, no-follow, owner-only requirement forward as a mandatory input, which is why the security signal remains **PRESENT**. |
| **IV. CLI Workspace Containment (NON-NEGOTIABLE)** | Yes | No leaf creates, modifies, or deletes anything outside the working-directory tree. Forced termination in `143.003.006-ST` targets **this exact process identity only**, never a name- or pattern-matched set, so it cannot reach a sibling workspace's daemon. This Stage pass itself edited only the permitted planning/backlog package on the current branch — no commit, no branch switch, no worktree. |
| **V. Structured Observability** | Yes | This is the plan's central deliverable. Startup records carry attempt ID, workspace, branch, generation, endpoint, process-start identity, and a monotonic sequence (`144.001.002-ST`), written through a serialized, versioned, durable sink with `phase_started`/`phase_completed` bracketing (`144.001.001-ST`). Terminal causes are structured: `143.004.002-ST` routes the watchdog cause through the live `ShimFailureClass` taxonomy with distinct exit and wire codes, and `143.003.005-ST` and `144.001.004-ST` expose retry and classification state as machine-parseable client data rather than log prose. |
| **VI. Single Responsibility** | Yes | No new external dependency. Every leaf reuses shipped in-tree primitives: the existing `ShimFailureClass` taxonomy and its code ranges, the existing lock ladder, the shared `DaemonStatus`/`HealthReport` structures, and the existing parity drift guard. Where a reviewer recommendation named a specific internal type, this plan binds the **invariant** and permits any equivalent proven in-tree protocol, avoiding speculative abstraction. Revision 3 removes the largest speculative abstraction in the package outright. |
| **VII. Destructive Command Approval (NON-NEGOTIABLE)** | Yes | The only destructive surface remaining in either shipment is `143.003.006-ST`'s self-termination, which destroys a process rather than data and is gated at CP-3 — approval required **before the first real execution**, with simulation-safe tests explicitly carved out. The irreversible-deletion surfaces (generation GC, `R-LEGACY-DB-RECLAIM`) are **deferred out of both shipments** and inherited by the future plan, which must satisfy this principle before they can be planned. |
| **VIII. Explicit Safety Modes for Elevated Risk** | Yes | Three modes are declared and structurally enforced. **investigate-first**: this is now the entirety of `144-S` — measurement precedes and gates any implementation plan, and both outcomes return to Stage. **freeze-scope**: Workstream A is confined to `src/daemon/`, `src/shim/`, `src/errors/mod.rs`, and `docs/cli-mcp-parity.md`; Workstream B to additive instrumentation in `src/db/cozo_backend/`, `src/tools/lifecycle.rs`, and the shared status/health structures. **careful**: applies to `143.003.002-ST` and `143.003.006-ST`. Risky work is expressed as `ProposedAction`/`ActionRisk`/`ActionResult` (PA-1…PA-4). |
| **IX. Git-Friendly Persistence** | Yes | Every artifact persisted is human-readable and Git-mergeable, and every write is atomic. The retry ledger is a versioned file written under an inter-process lock with flush and atomic replace (`143.003.004-ST`); the startup and diagnostics sinks are line-oriented, versioned, serialized, and flushed before rename. All Stage-side planning and backlog artifacts are Markdown with YAML frontmatter. |
| **X. Agent Context Efficiency** | Yes | Backlog state is queried through backlogit SQL and queue operations rather than file scans. Each item carries its own complete acceptance set, so Ship never needs to read the whole plan to claim one item. Revision 3 reduces the executable plan surface substantially by moving deferred design detail into a single reference section. |
| **XI. Merge Commit History Preservation (NON-NEGOTIABLE)** | Yes | No item alters version-control history. This Stage pass makes no commit, no rebase, no force-push, and no branch switch. |
| **Task Granularity — 2-Hour Rule (NON-NEGOTIABLE)** | Yes | Revision 2 still carried leaves bundling three or four concerns. Revision 3 splits them: A2 became three leaves, the watchdog hard deadline became two, and the retry ledger became four (storage, attribution, backoff consumer, status surface). Every executable leaf targets fewer than 3 primary files, fewer than 5 functions, and no more than 4 test scenarios. Parent task artifacts retained in a manifest are **claimable integration milestones with bounded acceptance criteria**, not prose containers. |

## Out of Scope

* Raising `ENGRAM_READY_TIMEOUT_MS` (explicitly rejected).
* **All content-addressed generation-reuse implementation** — identity,
  resolution, dirty-tree overlay, publication, attach, runtime-copy leasing, and
  GC. Accepted as direction; deferred pending measurement and a fresh Stage
  plan.
* `R-LEGACY-DB-RECLAIM` — reclaiming the 45 legacy per-branch databases
  (2.29 GB). Deferred and operator-approved, never agent-initiated.
* The pre-existing `opentelemetry_sdk` API drift in
  `src/server/observability.rs` that breaks `cargo ci` / `cargo lint`
  (stash `E12542FF`, `9A7C9F8F`).
* The pre-existing `src/bin/engram.rs` crate-root unsafe-lint observation.
  **Conditional carve-out:** if any leaf ends up editing that file, that leaf's
  acceptance criteria MUST include preserving and enforcing
  `#![forbid(unsafe_code)]` at the crate root. No leaf is currently scoped to
  touch it.
* Any change to `138-S`, to any `142.*` item, to `.backlogit/stash.jsonl`, to
  the 137-S carry-forward memory, or to any checkpoint audit file.

## Revision 3 pre-review self-assessment (historical)

> ### ⛔ TERMINAL SUPERSESSION NOTE — the bullets below are FALSE as of the terminal review
>
> **The bullets in this section are the PRE-REVIEW Revision 3 self-assertion,
> written before the gate ran. They did NOT survive review and are retained for
> audit history only. They are NOT a statement of current state.**
>
> The Revision 3 `plan-review` gate returned **FAIL (terminal)** — attempt 3 of 3,
> circuit breaker **OPEN** (see `## Plan Review — Revision 3` below).
>
> **Current authoritative state, which contradicts the bullets below:**
>
> * **Every member of the `143-S` and `144-S` manifests is `status: blocked`** —
>   all 27 tasks/subtasks plus the covering features `143-F` and `144-F`. The
>   first bullet's claim that every manifest item is "executable — no blocked
>   item" is **false**.
> * **No claim is authorized.** Neither `143-S` nor `144-S` is claimable; intake
>   must fail for both. Their `queued` shipment status is a storage artifact only,
>   because the shipment lifecycle has no blocked state.
> * Blocking finding **(h)** specifically rejects the root feature artifacts as
>   executable closure milestones, and finding **(g)** rejects the granularity of
>   multiple surviving leaves — directly refuting the "no prose-only container"
>   and executability assertions below.
> * **No fourth remediation, redesign, decomposition, or PASS attempt is
>   authorized** without an explicit, fresh operator-authorized Stage cycle.
>
> Durable rationale: `docs/memory/2026-09-10/stage-rev3-terminal-plan-review-fail.md`.

**Historical record of the pre-review assertion. Do not edit.**

This plan does **not** declare its own review verdict. Revision 3 must be gated
by a **fresh, independent `plan-review`**.

What Revision 3 asserted, pre-review, was narrower and checkable (**all
superseded by the terminal FAIL above**):

* Every item in the `143-S` and `144-S` manifests is **executable** — no blocked
  item, no prose-only container. — *SUPERSEDED: all manifest members are now
  blocked.*
* No executable item depends on a blocked item. — *SUPERSEDED.*
* Every gate in the gates table names an owner that exists in a shipment, or is
  explicitly recorded as a **deferred gate** with no owner.
* Every requirement is either owned by an executable item or explicitly marked
  **DEFERRED** with its future home named.
* The plan makes **no implementation promise** about content-addressed
  generation reuse.
* The 28 deferred items are retained, blocked, unlinked, and out of every
  manifest — traceable but not claimable.

## Plan Review — Revision 1

**Historical record. Do not edit.** This section preserves the verdict and
findings of the first independent `plan-review` gate on this plan. It is
retained after remediation so the remediation is auditable against what was
actually found, rather than against a summary written afterwards.

* **Reviewed artifact:** `docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` (Revision 0)
* **Reviewed backlog package:** `143-F`/`143-S` and `144-F`/`144-S` hierarchies as they stood at Revision 0
* **Review date:** 2026-09-10
* **Gate:** **FAIL**
* **Plan-review attempt:** 1

<!-- plan-review-attempt: 2 -->

### Verdict summary

Hardening was judged **satisfied**: the `## Plan Hardening` section carried a
genuine risky-action classification (PA-1…PA-4), added constraints H12–H17,
rollback/monitoring/owner/validation-window records, operator checkpoints
CP-1…CP-6, and unresolved operator decisions OD-1…OD-6. The hardening gate
(P-006) was not the failure.

The plan nonetheless **FAILED** on two structural grounds:

1. **Incomplete task synchronization.** The hardening pass added binding
   acceptance requirements (H12–H17, PA-1…PA-4, CP-1…CP-6) to the plan artifact
   only. The plan's own "Carry-forward note" conceded this and instructed Ship
   to read the hardening section alongside each item body at claim time. A
   requirement that exists only in a planning document and not in the claimable
   backlog leaf is not an acceptance criterion — it is an expectation. Ship
   claims leaves, not plans.
2. **Incomplete decomposition.** Several items marked *leaf* carried three or
   four independent engineering concerns each and could not be executed inside
   the 2-hour/width-isolation rule. The decomposition that had been done
   (`144.003-T`, `144.005-T`) was correct in kind but was not applied to the
   remaining oversized leaves.

### Consolidated P1 findings (release-blocking)

| ID | Finding |
|---|---|
| P1-01 | The task-level gate list was non-authoritative: it listed `cargo build` as a gate and omitted `cargo audit` entirely, contradicting the constitution's ordered gate set (`fmt` → `clippy` → `dev-test` → `audit`). The Constitution Check table additionally carried a malformed clippy invocation missing the `--` separator. |
| P1-02 | `143.001-T` was specified as a standalone task that must simultaneously hold an always-RED test and pass `cargo dev-test`. Those two requirements are mutually unsatisfiable in one claimable unit. |
| P1-03 | Six items marked *leaf* bundled three or four independent concerns: `143.003-T`, `144.001-T`, `144.004-T`, `144.006-T`, `144.007-T`, `144.008-T`. Each violated the 2-hour rule and width isolation. |
| P1-04 | H12–H17, PA-1…PA-4, and CP-1…CP-6 acceptance requirements were never mirrored into the owning backlog leaves; the plan instead instructed Ship to merge plan-only requirements mentally. |
| P1-05 | Dependencies were expressed container-to-container (`144.005-T → 144.003-T`, `144.005-T → 144.004-T`, `143.004-T → 143.003-T`). A container edge does not order the executable leaves Ship actually claims, so the intended sequencing was unenforced. |
| P1-06 | The Constitution Check was written against a six-principle list that is not the authoritative constitution. Principles V (Structured Observability), VI (Single Responsibility), VII (Destructive Command Approval), VIII (Explicit Safety Modes), and IX (Git-Friendly Persistence) were unmapped or mapped to the wrong numbers. |
| P1-07 | `143.002-T` specified TTL classification without saying where classification may read request state, and specified in-flight liveness as a one-shot `ttl.reset()`. A single reset cannot hold liveness across overlapping, cancelled, or errored requests. |
| P1-08 | The watchdog specification had no arming point in the backlog, no account of non-cancellable `spawn_blocking` work, and no proof that the IPC endpoint and `fd_lock` are reacquirable after a forced exit. |
| P1-09 | The shim's consecutive-termination counter (H13) had no durable, cross-process home in any backlog leaf, so the respawn bound could pass a single-process test while being unbounded in production. |
| P1-10 | `143.004-T`'s diagnostics sink was specified without stating that it is Workstream-A-owned; as written it could be read as depending on `144.001-T`'s sink, coupling the two shipments. The new terminal cause was not routed through the live `ShimFailureClass` taxonomy and had no CLI exit code or MCP wire code. |
| P1-11 | Startup records carried no unique startup-attempt identity, no workspace/branch/generation/endpoint/process-start identity, and no monotonic sequence, so records from concurrent or successive attempts were not separable and a single attempt could not be classified. |
| P1-12 | `readiness_published` was treated as sufficient evidence of reachability. A healthy start whose probe evidence was never correlated would be labelled "ready-but-unreachable". |
| P1-13 | The heartbeat was specified as cancelled but not **joined** before the terminal record, leaving a real post-terminal-emission race. Sink writes were not serialized. |
| P1-14 | Classification was exposed only through filesystem logs, not through structured error/status data visible to CLI and MCP clients. |
| P1-15 | Ordering contradiction: `144.003.001-ST` (HNSW create-vs-skip outcome) sat behind `144.002-T` through the container edge, yet `144.002-T`'s measurement is untrustworthy without it. |
| P1-16 | The B3 contingency authorized Ship to re-scope `144.003-T` in place (case ii) or supersede it and rewire dependencies (case iii). Both let Ship silently drop requirement R8 and gate G3 ownership without returning to Stage. |
| P1-17 | The effective indexing fingerprint omitted content-affecting inputs (max file size, include/exclude, language set, parser/extractor/canonicalization versions, embedding model/dimensions, chunking, feature flags, schema/Cozo/HNSW parameters) and had no per-field mutation test. The multi-root ordering contract was ambiguous about whether permutation or relabelling changes the key. |
| P1-18 | Clean-generation builds were specified against mutable working-tree files rather than immutable Git tree objects, leaving a clean→dirty mutation window during the build. |
| P1-19 | Dirty attach was specified as attaching to the "nearest matching clean generation" — a heuristic match that can serve a wrong base index. |
| P1-20 | Workspace identity did not require the existing capability-rooted, no-follow, reparse-rejecting reader, and runtime-copy/GC I/O was not constrained to be capability-relative, no-follow, owner-only, and workspace-contained. |
| P1-21 | B5 attach used `generation_id` alone as the runtime-copy path. Two concurrent openers of the same generation would therefore share one writable database. No session lease was held. |
| P1-22 | Attach had no fail-closed precondition set (opener identity vs manifest, directory name vs ID, artifact digest vs sealed manifest, publishability), no statement of whether a multi-generation catalog or a single `active.json` is authoritative, and no separation between recoverable cache-miss fallback and corruption/I-O/lock failures that must propagate. |
| P1-23 | Publication did not require Cozo/SQLite quiescence under exclusive authority. Snapshotting a live SQLite database with an open WAL yields a corrupt or torn generation. |
| P1-24 | Reader liveness and GC selection were two independent mechanisms with no shared atomic pin/lease, leaving a TOCTOU window between GC selection and deletion. |
| P1-25 | The Plan Hardening security signal was recorded **ABSENT**. Cross-branch and cross-process isolation and irreversible GC deletion are trust-boundary concerns; the signal is **PRESENT**. |

### Consolidated P2 findings (must-fix, non-blocking individually)

| ID | Finding |
|---|---|
| P2-01 | After decomposition, gates G1–G9 risked losing their named owners because ownership was recorded against items that became containers. |
| P2-02 | Workstream A's monitoring signals were routed to the sink established by `144.001-T`, recreating the cross-shipment coupling that P1-10 flags. |
| P2-03 | CP-1 required "current shipment status and Ship checkpoint state" but no P-016 branch/worktree topology evidence, which is what actually proves no parallel execution surface exists. |
| P2-04 | `R-LEGACY-DB-RECLAIM` was deferred with a disposition ("to be stashed") that nothing enforced. Without a mandatory closure condition it is lost when `144-S` closes. |
| P2-05 | `.github/instructions/mcp-server.instructions.md` and `docs/cli-mcp-parity.md` were not in the consulted-sources table, despite the plan adding client-visible state. |
| P2-06 | A crate-root unsafe-lint observation in `src/bin/engram.rs` surfaced during review was not dispositioned as in-scope or residual. |

### Reviewer notes carried forward as acceptance requirements

Security Lens and Rust reviewer recommendations recorded during this gate are
binding acceptance requirements for Revision 2, with one qualification: where a
recommendation named a specific internal type, an equivalent proven protocol
that satisfies the same invariant is acceptable. The invariant is binding; the
type is not.

### Disposition

Returned to Stage for remediation. Revision 2 must be gated by a **fresh
independent `plan-review`**; this plan may not declare its own verdict.


## Plan Review — Revision 2

**Historical record. Do not edit.** This section preserves the verdict and
findings of the second independent `plan-review` gate on this plan. It is
retained after the Revision 3 scope reset so the reset is auditable against what
was actually found, rather than against a summary written afterwards.

* **Reviewed artifact:** `docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` (Revision 2)
* **Reviewed backlog package:** `143-F`/`143-S` and `144-F`/`144-S` hierarchies as they stood at Revision 2
* **Review date:** 2026-09-10
* **Gate:** **FAIL**
* **Plan-review attempt:** 2

<!-- plan-review-attempt: 3 -->

**Attempt counter.** The authoritative counter is the **last**
`plan-review-attempt` marker in this file. The marker above supersedes the one
in `## Plan Review — Revision 1`. The next independent review is **attempt 3**.

### Verdict summary

Revision 2 closed most of the Revision 1 findings at the level of individual
items: hardening requirements were mirrored into leaves, gates regained named
owners, the constitution was remapped to the authoritative I–XI principle set,
and the oversized Revision 1 leaves were decomposed.

It nonetheless **FAILED**, and not on detail. The finding was structural: the
**generation-reuse implementation is still under-measured and over-designed.**
Revision 2 specified eight implementation units in depth against a cost premise
that has never been observed, while the design questions determining whether
that specification is even correct remained open. Adding more implementation
detail to an unmeasured premise increases the plan's confidence without
increasing its evidence.

### Distilled reasons for FAIL

| # | Reason |
|---|---|
| F1 | **Readiness-observer boundary unresolved.** The watchdog's relationship to readiness state was never pinned down: whether it observes through one seam or several, whether it may write `ReadinessView` or drive activation, whether observation refreshes the useful-work TTL, and how the seam survives the `138-S` read-server mode. An observer that can write the state it observes is not a backstop. |
| F2 | **Dependency inversions and leaf acceptance-criteria drift.** Executable items depended on items that would later be deferred (`144.002-T → 144.003.001-ST`, `144.003.002-ST → 144.002-T`), and the retry-backoff policy was ordered **before** the durable ledger it consumes — so the respawn bound could pass a single-process test while being unbounded in production. Several leaves had acceptance criteria that had drifted from the plan text they were supposed to mirror. |
| F3 | **Task granularity still exceeded.** Decomposition was applied to the Revision 1 offenders but not carried through: the probe-aware TTL work, the watchdog hard deadline, and the retry ledger each still bundled three or four independent engineering concerns in a single claimable leaf, exceeding the 2-hour rule and width isolation. |
| F4 | **Generation lifecycle, catalog, producer, and snapshot semantics unresolved.** The plan did not settle whether a multi-generation catalog or a single active pointer is authoritative; which process produces a clean candidate and under what lifecycle; whether the served artifact is a snapshot of a quiesced source or the source itself; what the fail-closed attach precondition set is; or how reader leases and GC share one atomic pin. Different sections assumed different answers. |
| F5 | **Agent-visible state gaps.** Client-visible startup, retry, and classification state was specified unevenly — present in some leaves as structured `DaemonStatus`/`HealthReport` data with parity tests, absent or log-only in others — so CLI and MCP consumers would not have seen the same picture. |
| F6 | **Impossible heartbeat contract.** The plan required the heartbeat task to be cancelled **and joined** inside a drop guard. `Drop` is synchronous and cannot await a join, so the stated contract was unimplementable as written. |

### Disposition

Returned to Stage. The remediation instruction was explicit: **do not keep
patching speculative implementation detail** — narrow to the smallest honest,
safe release units.

Revision 3 is that reset. It is summarised in `## Revision 3 scope reset` at the
top of this document, and must be gated by a **fresh independent
`plan-review`**. This plan may not declare its own verdict.

## Plan Review — Revision 3

**Terminal record. Do not edit.** This section records the third and final
independent `plan-review` gate on this plan. The review-cycle limit is reached.
No fourth remediation, redesign, decomposition, or PASS attempt is authorized
against this plan under the current operator cycle.

* **Reviewed artifact:** `docs/exec-plans/2026-09-10-cold-start-readiness-reliability-plan.md` (Revision 3)
* **Reviewed backlog package:** `143-F`/`143-S` and `144-F`/`144-S` hierarchies as they stood at Revision 3
* **Review date:** 2026-09-10
* **Gate:** **FAIL (terminal)**
* **Plan-review attempt:** 3 of 3 — **circuit breaker open**

<!-- plan-review-attempt: 4 -->

**Attempt counter.** The authoritative counter is the **last**
`plan-review-attempt` marker in this file. The marker above supersedes the one
in `## Plan Review — Revision 2`. Attempt 4 is **not authorized** and must not be
started without an explicit, fresh operator-authorized Stage cycle.

### Verdict summary

Plan hardening was **required and is present** — the plan carries a
`## Plan Hardening` section, the Revision 3 scope reset removed the speculative
Phase 2 implementation detail that failed Revision 2, and the surviving package
is materially smaller and more honest than its predecessors.

The gate nonetheless **FAILS**. The reason is not missing hardening and not
missing detail. After three cycles, the plan's **executable contracts remain
unsafe or unimplementable as written**. Several leaves still specify behaviour
that either cannot be built as stated, cannot be observed by the component
assigned to observe it, or would ship a live safety mechanism on evidence that
does not establish the property it claims. A plan may not pass a gate on the
strength of having improved; it passes on whether the units it authorizes are
safe to execute. These are not.

### Consolidated blocking findings

| # | Finding |
|---|---|
| a | **`143-S` — `_health` is an actual request and must remain `ProbeOnly`.** The shipment's probe-aware idle accounting classifies `_health` in a way that can still reset the useful-work TTL. Because `_health` is a real request on the wire, the current classification can **preserve the very livelock the shipment exists to break**: a never-ready daemon polled by its own health checks never goes idle and never expires. `_health` must be classified `ProbeOnly` and must never count as useful work. |
| b | **Stable readiness seam and post-138 recheck are temporally incompatible with active `138-S` recovery ordering.** The watchdog's single read-only readiness-observation seam is specified against a readiness architecture that `138-S` is actively replacing, while `143-S` is deliberately claimable *before* `138-S` resolves. The recheck cannot be both a pre-resume precondition and a post-merge verification of the same seam. Managed mode and activation-gated mode need **separate, explicitly owned pre-resume and post-merge ownership**, which the plan does not assign. |
| c | **Watchdog child attribution/ledger and daemon-absent status ownership remain incomplete.** The retry ledger and terminal-cause attribution still assume an observer that outlives the event being observed. **Short-lived CLI shims cannot observe a 15-minute child death.** Attribution of that death requires a durable one-time termination record or a supervisor process that owns it; neither is specified. Ownership of `DaemonStatus` when no daemon is present is likewise unassigned, so the absent case has no author. |
| d | **Watchdog/heartbeat finalization still has unbounded or impossible ordering around non-cancellable blocking work.** Revision 2's impossible synchronous-`Drop`-awaits-join contract was moved, not resolved. The outer async finalizer still orders cancel-then-join against blocking bootstrap work that is **not cancellable**, so the finalizer is either unbounded in time or unreachable in the stall it is meant to report. |
| e | **Startup classification lacks in-progress/staleness state and complete correlated transport evidence.** The classification derived from the sink has no explicit in-progress or stale state and no complete correlated evidence across CLI and MCP transports. As specified, **absent evidence collapses to "healthy"** — the single most dangerous default for a readiness classifier, and precisely the failure mode under investigation. |
| f | **Measurement report selects the wrong attempts and under-specifies the threshold.** The report selects **stalled** attempts, even though complete phase timings can only come from **healthy completed starts**; a stalled attempt has no `phase_completed` by construction, so the selection yields the records that cannot answer the question. The HNSW decision threshold additionally needs the **schema-bootstrap share of total pre-ready time** and a **full outcome taxonomy**, not a create-vs-skip pair, before a binary PROCEED/HALT gate can rest on it. |
| g | **Multiple leaves still exceed the non-negotiable 2-hour / fewer-than-4-scenario rule.** Granularity was corrected for the Revision 1 and Revision 2 offenders but not carried through the Revision 3 package. Several surviving leaves still bundle independent engineering concerns in one claimable unit. |
| h | **Root feature artifacts in the manifests are not executable closure milestones.** `143-F` and `144-F` sit in their shipment manifests as covering containers, not as units with bounded, verifiable acceptance criteria. A manifest member that cannot be closed by evidence cannot gate shipment closure. |
| i | **Durable Workstream B sink and ledger need hardened owner-only I/O and typed fields.** The sink and ledger require **capability-relative, no-follow, reparse-point-rejecting, owner-only** file I/O and a **typed allowlisted field set**. As written they accept untyped content through path-resolving I/O, which is not an acceptable contract for a workspace-contained durable sink. |

### Security disposition

**No P0 security exploit exists and no source change has been made.** This gate
failure is a **planning and execution-safety failure**, not a vulnerability
disclosure. Findings (a), (c), (d), (e), and (i) describe contracts that would
be unsafe *if executed*; none of them is a live defect introduced by this
package, because this package has shipped no code. The repository's source tree
is untouched by this Stage lineage.

### Disposition

**Per the circuit-breaker and review-cycle policy, the remaining findings are
not remediated in this session.** Three consecutive independent review cycles
have failed on the same class of problem — executable contracts that are unsafe
or unimplementable — and continuing to patch within the same cycle is exactly
the behaviour the breaker exists to stop.

Actions taken in this terminal pass, and only these:

1. This review record was written as the terminal verdict for the plan.
2. Every `143-S` and `144-S` manifest member, plus `143-F` and `144-F`, was set
   to `blocked` with a terminal blocked reason, so neither shipment can pass
   intake and neither can be claimed.
3. The shipments themselves remain `queued` **only because the shipment
   lifecycle has no blocked state**. Queued status here is a storage artifact,
   not an authorization to ship.

**A future Stage pass on this work requires an explicit new operator cycle.**
It must begin from the findings above, not from the Revision 3 plan text, and it
must re-derive the executable contracts rather than patch them.
