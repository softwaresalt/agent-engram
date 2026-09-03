---
title: "Separate indexer and reliable read server implementation plan"
description: "Implements watcher-independent read serving with isolated index generations and atomic publication"
date: "2026-09-02"
source:
  - "docs/research/2026-09-02-engram-read-only-snapshot-mode-requirements.md"
  - "docs/decisions/2026-09-02-separate-indexer-read-server-deliberation.md"
status: reviewed
---

## Problem Frame

Engram currently starts one daemon that serves IPC reads while also owning
startup sync, offline scanning, recursive watcher registration, and
watcher-triggered indexing. `start.ps1` attempts direct prewarming, but its
15-second fail-open wrapper can release Copilot before indexing or functional
readiness is established. On the reported customer machine, the shim initialized
but recorded `failure_class: readiness_timeout`.

The target architecture separates failure domains:

* A non-agent indexer builds a complete immutable generation in an isolated
  data directory
* A read-server daemon disables workspace watching and index mutation
* An atomic manifest publishes a validated generation
* The daemon changes generations only after opening and validating the
  replacement, while requests already in flight retain their cloned prior
  workspace snapshot
* CLI and MCP reads continue through the existing IPC and shared dispatch path

The existing `AppState::snapshot_workspace` behavior makes a bounded
availability-preserving implementation possible. Each request clones a
`WorkspaceSnapshot`, and read tools derive their database path from that clone.
Changing the active snapshot's `data_dir` therefore affects new requests while
in-flight requests retain the previous generation path.

## Requirements Trace

| Requirements | Implementation units |
|---|---|
| R1-R5, R13-R17, R28-R29 | U8, U9 |
| R6-R12, R25-R26, R30-R33 | U1, U2, U3 |
| R18-R20 | U2, U9 |
| R21-R24 | U1, U3, U7, U9 |
| R27 superseded by Revision 3 | No implementation; retain as historical requirement |
| R34-R37 | U4, U5 |
| R38-R40 | U6, U7 |
| R41 | U4, U7 |
| R42 | U4, U6 |

## Implementation Units

### U1 — Resolve immutable daemon operating mode

**Execution posture:** Test-first.

Add a typed `DaemonMode` with `Managed` and `ReadServer` variants to
`PluginConfig`. Resolve the effective mode once during daemon startup and inject
it into `AppState`. A persisted `read_server` declaration may be tightened by a
CLI or environment input but never loosened. Malformed explicit mode values
fail startup instead of silently selecting `Managed`.

**Files**

* `src/models/config.rs`
* `src/server/state.rs`
* `tests/unit/plugin_config_test.rs`

**Tests**

* Default configuration resolves to `Managed`
* Persisted read-server mode survives reload
* Tightening override selects `ReadServer`
* Malformed explicit mode fails closed

**Milestone:** Runtime code can query one immutable effective mode without
re-reading configuration.

### U2 — Enforce deny-by-default read capabilities

**Execution posture:** Test-first security boundary.

Create one authoritative tool capability classification adjacent to the shared
dispatch catalog. In `ReadServer` mode, dispatch permits only methods positively
classified as read-only. Unknown, write, evaluation, flush, and index methods
return a stable non-retryable error with remediation directing the operator to
the external indexer. Both CLI and MCP inherit the same refusal.

**Files**

* `src/tools/mod.rs`
* `src/errors/mod.rs`
* `tests/contract/read_server_dispatch_test.rs`

**Tests**

* Every registered tool has an explicit capability
* Read methods remain callable
* Known writes and an unknown method are refused before side effects
* Error code, classifier, retryability, and remediation are stable

**Milestone:** Agent-facing transports cannot mutate the served generation.

### U3 — Remove watcher and sync work from read-server lifecycle

**Execution posture:** Characterization-first, then test-first.

Pass `DaemonMode` into `run_with_shutdown_v2`. Preserve the current managed path
unchanged. In read-server mode, bind IPC and hydrate the published generation,
but do not start the watcher, run startup sync, queue offline scans, or flush
the frozen code graph during shutdown. Record mode in daemon health.

**Files**

* `src/daemon/mod.rs`
* `src/daemon/ipc_server.rs`
* `tests/integration/read_server_lifecycle_test.rs`

**Tests**

* Managed mode preserves startup and watcher counters
* Read-server mode reaches functional readiness without watcher events
* Read-server shutdown does not modify the frozen artifact hash set
* Existing Windows named-pipe lifecycle remains unchanged

**Milestone:** A daemon can serve a fixed persisted generation without workspace
monitoring or index writes.

### U4 — Define generation manifest and storage layout

**Execution posture:** Test-first persistence contract.

Add a generation module owning:

* `.engram/generations/{generation_id}/`
* `.engram/generations/active.json`
* Workspace identity, branch, schema version, source revision, creation time,
  and validation status
* Same-directory temporary-file-and-rename publication
* Resolution of active and previous generations
* Retention selection without deletion side effects

Use opaque sortable generation IDs. Validate all resolved paths remain under
the workspace `.engram/generations` root.

**Files**

* `src/services/generations.rs`
* `src/services/mod.rs`
* `tests/unit/generation_manifest_test.rs`

**Tests**

* Valid manifest round-trip
* Torn or malformed manifest rejection
* Path traversal rejection
* Active/previous retention selection

**Milestone:** Generation metadata can be written and resolved atomically
without changing the current daemon database layout.

### U5 — Build and validate an isolated generation

**Execution posture:** Test-first.

Add a non-agent CLI command that acquires a dedicated indexer lock, creates a
candidate generation directory, runs the existing direct index pipeline against
that directory, dehydrates the graph, and validates identity, schema, database
readability, and a repository-independent statistics/read probe. Failed
candidates remain unpublished and are reported explicitly.

**Files**

* `src/cli/commands/generations.rs`
* `src/bin/engram.rs`
* `tests/integration/generation_build_cli_test.rs`

**Tests**

* Successful build produces a validated unpublished generation
* Failed indexing leaves active manifest unchanged
* Concurrent builders are rejected
* Candidate data is isolated from the active generation

**Milestone:** A separate process can construct a complete candidate while the
read daemon serves another directory.

### U6 — Activate generations without read interruption

**Execution posture:** Test-first concurrency boundary.

Add an `AppState` activation operation that opens and validates the candidate
database before taking the workspace publication mutex. Under that short
critical section, replace the active `WorkspaceSnapshot.data_dir` and generation
provenance. Requests that already cloned the prior snapshot continue against
the prior directory. Failure before publication leaves state unchanged.

**Files**

* `src/server/state.rs`
* `src/tools/lifecycle.rs`
* `tests/integration/generation_activation_test.rs`

**Tests**

* Failed candidate open preserves active generation
* New requests use the replacement after activation
* An in-flight request completes against the prior generation
* Health/status provenance changes atomically

**Milestone:** The daemon can change immutable generations without an availability
gap.

### U7 — Publish, notify, retry, and retain

**Execution posture:** Test-first protocol change.

Add an internal daemon control method, omitted from the MCP tool catalog, that
activates the already-published manifest generation. The indexer publishes the
manifest only after validation, sends the control request, retries within a
bounded budget, and verifies generation identity through health. A lost signal
is recoverable because daemon startup resolves `active.json`. Retain active and
previous generations; expose explicit cleanup selection but do not delete from
the read path.

**Files**

* `src/daemon/protocol.rs`
* `src/daemon/ipc_server.rs`
* `tests/integration/generation_publish_reload_test.rs`

**Tests**

* Publication followed by reload converges
* Lost notification leaves old reads healthy and restart adopts manifest
* Failed reload preserves prior generation and reports pending failure
* Agent MCP catalog cannot invoke the control method

**Milestone:** A non-agent supervisor can publish a generation while reads stay
available.

### U8 — Make launcher pre-session readiness fail closed

**Execution posture:** Characterization-first supersession of 118-S Guardrail 4.

Update `start.ps1` and `start.sh` to:

1. Build and publish an initial generation when needed
2. Start or verify a read-server daemon
3. Wait within one shared, cold-start-sized outer budget
4. Verify daemon health and active generation
5. Execute a CLI read probe and an MCP-equivalent shim probe
6. Launch Copilot only after every gate passes

Preserve 118-S Guardrails 2 and 3: one outer budget and exact-child-only bounded
cleanup. Replace the existing fail-open contract tests with explicitly approved
fail-closed expectations.

**Files**

* `start.ps1`
* `start.sh`
* `tests/contract/start_launcher_test.rs`

**Tests**

* Slow successful prewarm remains within configured outer budget
* Failed build, daemon readiness, CLI probe, or MCP probe blocks Copilot
* Cleanup terminates only the exact owned process
* Paths containing spaces remain supported

**Milestone:** An agent session cannot begin with Engram degraded.

### U9 — Prove CLI/MCP parity and restart behavior

**Execution posture:** Test-first end-to-end verification.

Create an end-to-end matrix covering every read capability in read-server mode,
mutation refusal on both surfaces, one bounded auto-restart, and generation
identity equality. Reuse existing shim stdio and daemon harnesses.

**Files**

* `tests/contract/read_server_cli_mcp_parity_test.rs`
* `tests/integration/read_server_restart_test.rs`

**Tests**

* CLI and MCP return the same generation and equivalent results
* Both surfaces return the same mutation refusal
* One restart preserves read-server mode and active generation
* A second failure is terminal and structured

**Milestone:** Transport choice cannot weaken reliability or permissions.

### U10 — Document operation and release controls

**Execution posture:** Documentation after verified behavior.

Document the indexer/read-server split, startup gate, generation provenance,
refresh workflow, rollback to the previous generation, storage growth, and
monitoring. Update architecture boundaries without duplicating implementation
details.

**Files**

* `docs/ARCHITECTURE.md`
* `docs/troubleshooting.md`
* `docs/references/read-server-operations.md`

**Verification**

* Documentation review passes
* Every command and response field matches the shipped CLI
* Cross-references resolve

**Milestone:** Operators can run, diagnose, refresh, and roll back the service.

## Dependency Graph

```text
U1 -> U2
U1 -> U3
U4 -> U5
U1 + U4 -> U6
U5 + U6 -> U7
U2 + U3 + U7 -> U8
U2 + U3 + U7 -> U9
U8 + U9 -> U10
```

The graph is acyclic. U1 and U4 may be developed independently, but repository
policy still permits only one active implementation release unit at a time.

## Decisions and Rationale

* **Immutable generations instead of same-database concurrency:** isolates
  database locks, CPU, memory, and failure from serving
* **Manifest plus explicit notification:** durable restart convergence without
  relying on workspace file watching
* **Open before swap:** a bad generation cannot displace a healthy one
* **Snapshot cloning as the drain mechanism:** reuses current request behavior
  rather than introducing a global read lock
* **Server-side deny-by-default capabilities:** prevents CLI/MCP drift and
  future write-tool bypass
* **Persisted read-server mode:** auto-spawn cannot accidentally restore managed
  watcher behavior
* **One shared launcher deadline:** preserves the bounded behavior established
  by 118-S while raising its magnitude for observed customer startup cost

## Risks and Caveats

* **Cozo handle lifetime on Windows:** superseded generation directories cannot
  be removed until all handles close. Retention and cleanup must treat deletion
  as a later explicit operation.
* **Disk amplification:** two full generations are the minimum rollback set.
  Health must report sizes so operators can alert before disk exhaustion.
* **Schema bootstrap writes on open:** `connect_db` currently runs idempotent
  schema bootstrap. Read-server activation must validate whether this changes
  bytes; if so, add an open-existing path that refuses missing schema rather
  than bootstrapping.
* **Manifest/daemon divergence:** health reports both published and active IDs.
  A mismatch is degraded but must not interrupt reads from the active
  generation.
* **Security boundary scope:** arbitrary local shell access is outside the
  agent tool boundary. Direct generation build remains a trusted supervisor
  operation.
* **Launcher compatibility:** replacing fail-open behavior intentionally
  supersedes 118-S Guardrail 4 and requires explicit release notes.

## Plan Hardening Signals

| Signal | Present | Justification |
|---|---|---|
| Public API, schema, or contract change | Yes | Adds config, CLI, health/status, error, manifest, and internal IPC contracts |
| Security, auth, permission, or compliance-sensitive behavior | Yes | Establishes a server-side read-only capability boundary |
| Migration, backfill, destructive data/config action, or irreversible step | Yes | Introduces a new storage layout and later generation cleanup |
| External integration, operator checkpoint, or external dependency | Yes | Launcher and non-agent supervisor coordinate daemon readiness |
| High runtime, rollout, or rollback risk | Yes | Changes startup, database selection, and live serving |

**Requires plan hardening: yes.**

## Runtime Verification and Closure

### Runtime verification

* Build two distinct generations and prove reads continue during publication
* Exercise CLI and real MCP stdio reads against the same generation
* Modify source files and prove no watcher event or implicit sync occurs
* Kill the daemon and verify one bounded restart preserves mode and generation
* Run on Windows named pipes and Unix domain sockets
* Measure initial build, candidate validation, activation latency, query
  latency, memory, and disk amplification

### Monitoring plan

| Signal | Baseline | Alert threshold | Owner |
|---|---|---|---|
| Read query success | Current healthy daemon | Any sustained failure or two consecutive availability errors | Release operator |
| Active/published generation match | Equal | Mismatch longer than reload budget | Release operator |
| Generation activation latency | Establish in test | Above configured reload deadline | Release operator |
| Watcher events in read-server mode | Zero | Any event | Engram maintainer |
| Generation disk usage | Establish after two builds | Less than one additional generation of free space | Release operator |

### Rollback

The active manifest retains the previous validated generation. Rollback
atomically republishes that generation and invokes the same bounded activation
path. If activation code itself is unhealthy, restart the prior binary against
the prior generation. Never delete the prior generation during the validation
window.

### Observation window

Observe the first release candidate for one full working day on Windows and one
Unix platform. Record startup success, read availability, reload outcomes,
latency, disk growth, and any fallback. Runtime-affecting work is not closed
until the observation outcome and rollback readiness are recorded under
`docs/closure/`.

## Plan Hardening

Hardening is required because this plan changes the storage-selection contract,
daemon startup, live database publication, agent permissions, and launcher
failure policy.

### Reinforcing context

The hardened plan incorporates:

* `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`
  for Windows handle-release lag and bounded open retries
* `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`
  for the IPC-bind-before-background-work invariant
* `docs/compound/concurrency-issues/early-hydration-ready-before-heavy-io-2026-05-09.md`
  for the distinction between health readiness and functional read readiness
* `docs/compound/best-practices/auto-reindex-oom-gate-2026-05-09.md`
  for default-off expensive startup behavior
* `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`
  for one shared launcher budget and exact-child cleanup
* Constitution requirements for test-first Rust, workspace containment,
  explicit destructive approval, and merge-commit history

### Protected invariants

1. The active generation is never modified after publication.
2. A candidate is never visible to readers before validation completes.
3. Publication failure never displaces the current generation.
4. Managed mode retains its current watcher, startup-sync, shutdown-flush, and
   IPC behavior.
5. Read-server mode cannot be loosened by an agent-facing request or ambient
   override.
6. The same capability classifier governs CLI, MCP, and hand-crafted IPC
   requests.
7. Windows named-pipe identity and Unix socket derivation remain unchanged.
8. At least one prior validated generation remains available throughout the
   release observation window.

### Risky actions

**ProposedAction RS1**

* `summary`: Introduce a generation-specific data layout and active manifest
* `targets`: `.engram/generations/`, direct index path, daemon workspace snapshot
* `change_kind`: schema and runtime storage selection
* `rollback`: ignore the manifest and restore legacy `.engram` data resolution
  while retaining generated directories
* `approval_required`: yes, before changing the default launcher to consume
  generations
* `ActionRisk`: high
* `ActionResult`: planned

**ProposedAction RS2**

* `summary`: Replace fail-open launcher behavior with fail-closed readiness
* `targets`: `start.ps1`, `start.sh`, launcher contract tests
* `change_kind`: operator-visible startup contract
* `rollback`: restore the prior launcher while keeping read-server mode
  opt-in; document that degraded sessions may recur
* `approval_required`: yes, because this supersedes 118-S Guardrail 4
* `ActionRisk`: high
* `ActionResult`: planned

**ProposedAction RS3**

* `summary`: Retire generations older than the active and previous validated set
* `targets`: generation directories selected by the retention planner
* `change_kind`: deletion
* `rollback`: none after deletion; retain by default during initial rollout
* `approval_required`: yes for every cleanup execution until a separately
  reviewed retention command exists
* `ActionRisk`: destructive
* `ActionResult`: planned

### Added verification gates

* Establish a byte-hash baseline for the active generation before every
  mutation-refusal and publication test
* Inject failures after candidate creation, database build, dehydration,
  validation, manifest temporary write, manifest rename, daemon candidate open,
  and state swap
* Assert the active generation and prior-generation hashes after each injected
  failure
* Hold an in-flight read open across activation to prove request draining
* Exercise rapid sequential reloads on Windows to surface delayed SQLite handle
  release
* Verify the MCP tool list contains no supervisor control methods
* Verify direct IPC calls to ordinary mutating methods receive the same
  read-server refusal
* Measure startup and reload against explicit budgets; a timeout is a blocked
  gate, not a warning

### Rollout and rollback gates

1. Ship mode/config and dispatch refusal behind an opt-in setting.
2. Ship generation build and validation without changing the default active
   data path.
3. Exercise publication and reload under an opt-in release-candidate workspace.
4. Change the launcher only after CLI/MCP parity, restart, and rollback tests
   pass on Windows and Unix.
5. Keep legacy data and the previous generation untouched for the full
   observation window.

Rollback triggers:

* Any read availability error attributable to generation activation
* Any mutation of a published generation
* Active/published generation mismatch beyond the reload deadline
* Any watcher event or automatic sync in read-server mode
* Query p95 latency exceeding twice the managed-mode baseline for 15 minutes

On trigger, atomically republish the prior generation. If the new binary cannot
activate it, stop only the owned daemon process and run the previous binary
against the preserved legacy data path. Do not invoke generation cleanup.

### Operator checkpoints

* Approve the explicit supersession of fail-open launcher behavior before U8
* Approve the generation layout before enabling it outside tests
* Approve any destructive cleanup independently from build/publication
* Approve merge only after runtime evidence includes Windows and Unix results

No unresolved technical question blocks test-first implementation. The
operator's selection of the separate-indexer architecture resolves the primary
design choice; the checkpoints above govern rollout rather than code
construction.

## Plan Review

**Gate decision: FAIL — revision required before harvest.**

Plan hardening was required and present, including strict-safety action
classification. The multi-persona gate nevertheless found load-bearing
architecture and authority gaps.

### P0 findings

* U5 placed privileged generation building in the same CLI offered to agents.
  Calling a command "non-agent" does not create an authority boundary.
* U6 proposed swapping only `WorkspaceSnapshot.data_dir`, but current handlers
  snapshot state and reopen databases multiple times. This cannot pin an
  in-flight request to one validated generation.

### P1 findings

* Read-server startup still referenced hydration and `connect_db`, both of which
  write to a supposedly immutable generation.
* Catalog omission did not protect the generation activation control method
  from hand-crafted IPC calls.
* Manifest publication lacked a reviewed Windows replace-existing and
  durability contract.
* Delayed notifications could reactivate an older generation because no
  monotonic publication revision was defined.
* Generation retention did not track live readers across multiple activations.
* The refusal and availability error payloads were not fully preserved through
  dispatch, IPC, CLI, and MCP.
* Mode resolution could not be fail-closed within the files allocated to U1.
* Path containment did not address symlinks, junctions, reparse points, or
  validate-then-replace races.
* The direct indexing pipeline was still coupled to `DaemonLock`, preventing a
  candidate build while the read daemon was alive.
* The plan omitted the constitution-required `## Constitution Check`.
* Most units exceeded the repository's strict task-size limits.

### P2 and P3 findings carried into remediation

* Define branch-pinned manifest behavior.
* Use one validation predicate in the service layer.
* Derive or exhaustively verify CLI, MCP, and dispatch catalogs from one
  capability registry.
* Put complete generation and availability provenance on agent-facing reads.
* Defer destructive generation cleanup until lease-aware deletion has its own
  reviewed plan.
* Keep generation IDs simple; ordering comes from publication revision.

Runtime verification and closure were present but must be updated to test
pinned handles, authenticated supervisor control, no-follow access, reordered
notifications, and immutable open behavior.

## Remediation Revision 1

This revision supersedes U1-U10 for harvest and implementation. The original
units remain above as review history.

### Revised architecture

1. `engram-indexer` is a separate supervisor executable. It is not installed in
   the agent CLI path or advertised through MCP.
2. The supervisor owns an ephemeral capability delivered through an inherited
   handle. It is never stored in the workspace, environment, arguments, or
   logs. Deployments requiring hostile same-user resistance use a separate OS
   identity.
3. Candidate generations are created through a capability-rooted generations
   directory. A strict `GenerationId` is one path component; no-follow access
   and object identity checks prevent redirection.
4. Candidate validation seals an inventory and digests before atomic manifest
   publication. The manifest uses a monotonic publication revision independent
   of generation identity.
5. The daemon opens an existing candidate without bootstrap or hydration,
   producing `Arc<GenerationReadContext>`. Shared dispatch captures that Arc
   once and every read handler uses it.
6. Activation compares publication revision under one lock and swaps the
   context only after successful open and validation. Old contexts remain alive
   through request-held Arcs.
7. Destructive cleanup is not part of this release. All generations are
   retained and lease state plus disk usage are observable.

### Revised implementation units

Each unit is limited to one skill domain, at most two primary files, and at
most three test scenarios. Test harness commits precede implementation commits.

#### P1 — Typed daemon mode contract

* Files: `src/models/config.rs`, `tests/unit/plugin_config_test.rs`
* Add `DaemonMode::{Managed, ReadServer}` and a strict mode-intent parser
* Preserve managed defaults only when mode intent is absent
* Test default, persisted read server, and malformed explicit intent

#### P2 — Startup mode resolution and propagation

* Files: `src/daemon/mod.rs`, `src/shim/lifecycle.rs`
* Resolve mode once and preserve it across explicit and auto-spawn startup
* Inject the immutable result into daemon construction
* Test explicit startup, shim auto-spawn, and no loosening override

#### P3 — Open an existing generation without writes

* Files: `src/db/cozo_backend/mod.rs`,
  `tests/integration/read_existing_db_test.rs`
* Add `open_existing_db` without create, bootstrap, hydration, or mutation
* Characterize SQLite sidecars and hash the defined immutable artifact set
* Test success, missing/schema mismatch, and byte stability

#### P4 — Generation identity and manifest schema

* Files: `src/services/generations.rs`,
  `tests/unit/generation_manifest_test.rs`
* Define strict `GenerationId`, `PublicationRevision`, sealed inventory, and
  branch-pinned manifest
* Reject separators, dot components, unsupported encodings, and unknown schema
* Test round-trip, malformed IDs, and branch mismatch

#### P5 — Capability-rooted generation filesystem

* Files: `src/services/generations.rs`,
  `tests/integration/generation_path_safety_test.rs`
* Open one trusted root and use no-follow relative operations
* Verify stable file identity and sealed inventory digests
* Test symlink, Windows reparse/junction, and candidate replacement attacks

#### P6 — Durable cross-platform manifest replacement

* Files: `src/services/generations.rs`,
  `tests/integration/generation_manifest_publish_test.rs`
* Serialize publishers and use a reviewed replace-existing primitive
* Sync temporary content and required directory metadata
* Test existing-manifest replacement, injected failure, and crash recovery

#### P7 — Extract lock-free target-directory indexing service

* Files: `src/services/code_graph.rs`, `src/cli/direct.rs`
* Keep `DaemonLock` in the legacy direct CLI wrapper
* Extract indexing into a service accepting an explicit candidate data root
* Test legacy lock behavior and candidate build while read daemon lock is held

#### P8 — Supervisor candidate build

* Files: `src/bin/engram-indexer.rs`,
  `tests/integration/generation_build_test.rs`
* Create an exclusive candidate and run the extracted indexing service
* Use a generation-build lock independent of `DaemonLock`
* Test complete build, concurrent builder rejection, and failed-build isolation

#### P9 — Shared candidate validation and sealing

* Files: `src/services/generations.rs`,
  `tests/integration/generation_validation_test.rs`
* Validate identity, schema, bounded artifact shapes, database reads, and
  deterministic non-empty probe facts
* Seal inventory and digests for activation revalidation
* Test valid, corrupt/oversized, and validate-then-replace candidates

#### P10 — Pinned generation read context

* Files: `src/server/state.rs`, `tests/unit/generation_context_test.rs`
* Add `GenerationReadContext` with opened DB, provenance, and publication
  revision
* Publish `Arc<GenerationReadContext>` as immutable active request context
* Test context construction, cloning, and retired-context liveness

#### P11 — Capture one generation per dispatch

* Files: `src/tools/mod.rs`, `tests/contract/dispatch_snapshot_test.rs`
* Capture the active context once at dispatch entry
* Make missing context an explicit availability error
* Test one acquisition, unavailable state, and activation race

#### P12 — Migrate database-backed read handlers

* Files: `src/tools/read.rs`, `tests/integration/read_generation_pin_test.rs`
* Consume the dispatch context rather than re-snapshotting or reopening by path
* Cover multi-region handlers such as `unified_search` and statistics
* Test one-generation response, in-flight old read, and new-generation read

#### P13 — Authoritative capability registry

* Files: `src/tools/capabilities.rs`,
  `tests/contract/tool_capability_registry_test.rs`
* Classify every dispatch method as `Read`, `Write`, or `Control`
* Unknown methods are not read
* Test exhaustive dispatch coverage, explicit classifications, and defaults

#### P14 — Enforce read-server refusal at dispatch

* Files: `src/tools/mod.rs`, `src/errors/mod.rs`
* Refuse every non-read capability before side effects
* Include stable non-retryable classifier and out-of-session remediation
* Test read allow, write refusal, and unknown/control refusal

#### P15 — Preserve structured errors across transports

* Files: `src/daemon/protocol.rs`, `src/shim/transport.rs`
* Carry stable code, classifier, retryability, restart facts, generation, and
  remediation through IPC and MCP
* Test direct IPC, MCP translation, and field preservation

#### P16 — Verify CLI and MCP catalog parity

* Files: `src/shim/tools_catalog.rs`, `src/cli/runner.rs`
* Derive or exhaustively validate both surfaces against the capability registry
* Supervisor operations have no agent CLI or MCP exposure
* Test unmapped tool failure, schema parity, and supervisor exclusion

#### P17 — Read-server lifecycle without background mutation

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/read_server_lifecycle_test.rs`
* Use a dedicated bind/open path with no hydration, scan, watcher, startup sync,
  or shutdown flush
* Preserve managed mode unchanged
* Test zero forbidden calls, managed characterization, and byte stability

#### P18 — Revision-ordered generation activation

* Files: `src/server/state.rs`,
  `tests/integration/generation_activation_test.rs`
* Open and revalidate a context before the publication lock
* Compare expected revision and swap only to the latest manifest
* Test failed open, reordered A/B notifications, and A-to-B-to-C publication

#### P19 — Generation leases and observability

* Files: `src/services/generations.rs`, `src/tools/lifecycle.rs`
* Track strong request contexts and report lease, active, published, pending,
  failed, source revision, and disk-usage state
* Do not expose or execute deletion
* Test active/published match, mismatch, and retired live-reader reporting

#### P20 — Dedicated authenticated supervisor control plane

* Files: `src/daemon/control.rs`,
  `tests/integration/supervisor_control_auth_test.rs`
* Use a separate local endpoint and an inherited ephemeral capability
* Bind requests to daemon/workspace identity and publication revision
* Test authorized activation, missing/wrong capability, and replay rejection

#### P21 — Supervisor publish and convergence loop

* Files: `src/bin/engram-indexer.rs`,
  `tests/integration/generation_convergence_test.rs`
* Publish only sealed candidates, notify with expected revision, retry within
  one budget, and verify health provenance
* Test success, lost notification with restart convergence, and terminal timeout

#### P22 — PowerShell fail-closed launcher

* Files: `start.ps1`, `tests/contract/start_launcher_test.rs`
* Preserve one shared budget and exact-child cleanup
* Start supervisor, verify health plus CLI/MCP probe facts, then launch Copilot
* Test success, each blocked gate, and paths with spaces

#### P23 — Unix fail-closed launcher

* Files: `start.sh`, `tests/contract/start_sh_launcher_test.rs`
* Implement P22 semantics without changing platform contracts
* Test success, blocked gate, and exact-child cleanup

#### P24 — Bounded read-daemon restart

* Files: `src/shim/lifecycle.rs`,
  `tests/integration/read_server_restart_test.rs`
* Attempt one restart in persisted read-server mode
* Return the shared terminal availability payload after failure
* Test successful restart, terminal second failure, and restart logging

#### P25 — End-to-end CLI/MCP generation parity

* Files: `tests/contract/read_server_cli_mcp_parity_test.rs`
* Verify all read capabilities, provenance, refusal, and generation equality
* Include Windows named-pipe and Unix socket execution in CI matrix

#### P26 — Operator and architecture documentation

* Files: `docs/ARCHITECTURE.md`, `docs/troubleshooting.md`
* Document trust model, supervisor deployment, refresh, rollback, disk growth,
  fail-closed launch, and the absence of destructive cleanup
* Explain JSON manifest use as machine-owned atomic runtime state rather than
  Git-mergeable harness state

### Revised dependencies

```text
P1 -> P2
P3 + P4 -> P5
P4 + P5 -> P6
P7 -> P8
P3 + P5 + P8 -> P9
P3 + P4 -> P10
P10 -> P11 -> P12
P1 + P13 -> P14 -> P15
P13 -> P16
P2 + P3 + P10 -> P17
P6 + P9 + P10 + P12 -> P18
P18 -> P19
P6 + P18 -> P20 -> P21
P15 + P16 + P17 + P19 + P21 -> P22
P22 -> P23
P2 + P15 + P17 -> P24
P12 + P14 + P15 + P16 + P19 + P24 -> P25
P22 + P23 + P25 -> P26
```

### Constitution Check

| Principle | Plan compliance |
|---|---|
| I. Safety-First Rust | Safe Rust only; typed modes, IDs, revisions, contexts, and errors; no unsafe code |
| II. Test-First Development | Every P-unit declares a RED harness before implementation |
| III. Workspace Isolation | Capability-rooted generation access and strict IDs prevent escape |
| IV. CLI Workspace Containment | All data remains under the configured workspace unless a separately configured service-owned root is explicitly selected |
| V. Structured Observability | Provenance, leases, restart facts, activation failures, and deadlines are structured |
| VI. Single Responsibility | No dependency additions are planned; units are split by one domain |
| VII. Destructive Approval | Generation deletion is removed from this release; future cleanup requires a separate approved plan |
| VIII. Safety Modes | Execution uses **investigate-first** for storage/open semantics and **freeze-scope** for `.engram/generations`, daemon lifecycle, agent dispatch, and launchers |
| IX. Git-Friendly Persistence | `active.json` is machine-owned runtime state, not harness state; JSON is required for atomic parser-stable publication and is excluded from Git |
| X. Agent Context Efficiency | Compact provenance accompanies reads; catalogs remain structured and bounded |
| XI. Merge History Preservation | Delivery remains merge-commit-only |

No constitutional violation is accepted. The JSON runtime-manifest format is a
documented non-conflicting application of Principle IX because the state is
tool-managed and intentionally not Git-merged.

### Revised review disposition

All P0/P1 findings above are addressed structurally in Revision 1. P2/P3
findings are either incorporated or explicitly deferred: destructive cleanup is
removed, branch behavior is pinned in the manifest, disk usage is observability
rather than a new health authority, and generation ordering uses publication
revision rather than a sortable ID.

The plan requires a new multi-persona review before harvest.

## Plan Review — Revision 1

**Gate decision: FAIL.** The second multi-persona review found one P0 and
seventeen deduplicated P1 findings.

The P0 was incomplete request pinning: database-backed reads outside
`src/tools/read.rs` still reopen databases and resnapshot workspace state.

The P1 findings covered:

* an underspecified inherited-capability/control-endpoint lifecycle;
* unauthenticated durable publication despite authenticated notification;
* Cozo path-open TOCTOU relative to the stronger R46 threat model;
* missing immutable effective mode in `AppState`;
* enforcement below pre-dispatch `_health` and `_shutdown`;
* mutable `set_workspace` remaining a competing data-directory authority;
* an extracted indexing service accepting arbitrary active data roots;
* incomplete CLI error-envelope propagation;
* missing successful-response provenance;
* no deterministic launcher state machine;
* no revised R1-R47 traceability or runtime-verification section;
* missing RED harness paths and understated test scenarios;
* no proven safe-Rust mechanism for no-follow handles, inherited handles, or
  Windows atomic replacement.

The Learnings Researcher also reconfirmed existing load-bearing precedent:
bind IPC before optional watcher work, preserve the Cozo busy/locked panic
mitigation until upstream behavior changes, acquire compound state under one
coherent snapshot, and keep one shared launcher deadline with exact-child
cleanup.

## Remediation Revision 2

This revision supersedes both U1-U10 and Revision 1 P1-P26 for harvest and
implementation.

### Simplified architecture

* There is no generation control endpoint, capability token, notification, or
  activation RPC.
* A separately distributed `engram-indexer` executable builds, validates, and
  atomically publishes immutable generations. It is absent from the agent
  `engram` CLI and MCP catalogs.
* `active.json` is the sole publication authority. It contains a monotonic
  checked revision and branch identity.
* The read daemon reconciles `active.json` at startup and synchronously at the
  shared request choke point. It opens and validates only a higher revision,
  swaps one `Arc<ReadRequestContext>`, then dispatches.
* A request context pins the database handle, registry-derived state, workspace
  identity, effective mode, and provenance for the complete request.
* The threat model covers legitimate concurrent processes and crash safety, not
  hostile arbitrary code running as the workspace owner.
* Destructive generation cleanup remains deferred.

### Mandatory feasibility spike

#### S1 — Prove storage and publication primitives

* Artifacts:
  `docs/decisions/2026-09-02-generation-storage-feasibility-spike.md`
* Experiments:
  1. prove whether Cozo can open an existing SQLite database read-only without
     bootstrap or sidecar mutation;
  2. if not, measure and validate a sealed-generation-to-private-runtime-copy
     fallback;
  3. prove replace-existing atomicity/durability behavior on Windows and Unix;
  4. identify the exact safe-Rust dependency set and confirm no unsafe code is
     required in this crate.
* Stop condition: any failed feasibility claim returns the plan to revision
  before P1.

### Revised implementation units

Every implementation unit names its RED harness. Where a behavior needs more
than three scenarios, it is split into a separate test unit.

#### P1 — Strict daemon mode parsing

* Files: `src/models/config.rs`, `tests/unit/plugin_config_test.rs`
* Parse `Managed` and `ReadServer`; preserve legacy defaults only when intent is
  absent; reject malformed explicit intent.

#### P2 — Immutable effective mode in application state

* Files: `src/server/state.rs`, `tests/unit/app_state_mode_test.rs`
* Require `DaemonMode` in `AppState::new`; expose no setter.

#### P3 — Propagate mode through explicit and shim startup

* Files: `src/daemon/mod.rs`,
  `tests/integration/daemon_mode_propagation_test.rs`
* Resolve once and pass the value to daemon construction.
* `src/shim/lifecycle.rs` is a follow-on P4 to preserve task width.

#### P4 — Preserve mode across shim auto-spawn and restart

* Files: `src/shim/lifecycle.rs`,
  `tests/integration/read_server_restart_test.rs`
* Spawn and restart only in the persisted effective mode; one restart maximum.

#### P5 — Generation domain types and branch-pinned manifest

* Files: `src/services/generations/manifest.rs`,
  `tests/unit/generation_manifest_test.rs`
* Define strict ID, checked publication revision, sealed inventory, provenance,
  and branch identity.

#### P6 — Generation-root containment

* Files: `src/services/generations/store.rs`,
  `tests/integration/generation_store_containment_test.rs`
* Enforce strict components, canonical containment, regular files, and
  publisher serialization under one configured root.

#### P7 — Cross-platform atomic manifest publication

* Files: `src/services/generations/publish.rs`,
  `tests/integration/generation_publish_test.rs`
* Implement the S1-proven safe-Rust replace-existing primitive, checked revision
  increment, file sync, supported metadata sync, and temporary-file recovery.

#### P8 — Existing-database read strategy

* Files: `src/db/cozo_backend/mod.rs`,
  `tests/integration/generation_db_open_test.rs`
* Implement the S1-selected read-only open or private-runtime-copy strategy.
* Preserve the existing narrow Cozo busy/locked panic mitigation and rethrow
  unrelated panics.

#### P9 — Extract candidate-only indexing service

* Files: `src/services/code_graph.rs`,
  `tests/integration/candidate_indexing_service_test.rs`
* Accept only a private `CandidateRoot` minted by generation store code.
* P10 updates the legacy direct wrapper separately.

#### P10 — Preserve direct-sync lock behavior

* Files: `src/cli/direct.rs`,
  `tests/integration/direct_sync_lock_test.rs`
* Keep `DaemonLock` and existing direct-sync semantics while calling P9.

#### P11 — Build and seal a candidate generation

* Files: `src/bin/engram-indexer.rs`,
  `tests/integration/generation_build_test.rs`
* Build only into an exclusive unpublished candidate root; validate before seal.

#### P12 — Publish from the supervisor executable

* Files: `src/bin/engram-indexer.rs`,
  `tests/integration/generation_supervisor_publish_test.rs`
* Require a sealed candidate and publish through P7.
* The agent binary and MCP catalog have no build/publish entry.

#### P13 — Construct a pinned read request context

* Files: `src/server/state.rs`,
  `tests/unit/read_request_context_test.rs`
* Pin opened DB, generation-owned registry/config facts, workspace identity,
  immutable mode, and provenance in one Arc.

#### P14 — Reconcile manifest at the IPC request choke point

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/request_entry_reconciliation_test.rs`
* Before classifying a read, load a complete manifest and open only a higher
  revision; retain the prior context on any failure.
* Preserve bind-before-optional-work startup ordering.

#### P15 — Exclude the retired HTTP/SSE transport

* Files: `docs/ARCHITECTURE.md`,
  `tests/contract/supported_transport_surface_test.rs`
* Record that HTTP/SSE is no longer a supported Engram endpoint.
* Assert that the supported agent surfaces are direct daemon IPC, the `engram`
  CLI over IPC, and stdio MCP through `engram shim`.
* Do not implement generation reconciliation or parity for retired HTTP code.

#### P16 — Migrate core search and graph reads

* Files: `src/tools/read.rs`,
  `tests/integration/core_read_generation_pin_test.rs`
* Remove internal resnapshot/reopen seams and consume the captured context.

#### P17 — Migrate lifecycle and health reads

* Files: `src/tools/lifecycle.rs`,
  `tests/integration/lifecycle_read_generation_pin_test.rs`
* Status and health consume the captured context; identity-equal bind is a
  side-effect-free no-op and retargeting is refused.

#### P18 — Migrate diagnostic and report reads

* Files: `src/tools/eval.rs`,
  `tests/integration/report_read_generation_pin_test.rs`
* Migrate evaluation/report-family reads; P19 covers lint/doctor separately.

#### P19 — Migrate lint and doctor reads

* Files: `src/tools/lint.rs`,
  `tests/integration/lint_read_generation_pin_test.rs`
* Migrate read-capable lint paths.
* Add doctor-path coverage in a separate P20 to preserve task width.

#### P20 — Migrate doctor and supporting read services

* Files: `src/tools/doctor.rs`,
  `tests/integration/doctor_read_generation_pin_test.rs`
* Pass request context or pinned query handles into all supporting services.
* A repository contract test forbids read paths from calling `connect_db` or
  deriving `data_dir` from a fresh workspace snapshot.

#### P21 — Canonical tool descriptor registry

* Files: `src/tools/capabilities.rs`,
  `tests/contract/tool_descriptor_registry_test.rs`
* Define method, capability, surface exposure, CLI mapping, schema reference,
  provenance envelope, and read-server availability in one registry.
* Classify `_health` as Read, `_shutdown` and retargeting bind as Control, and
  identity-equal bind as `ReadServerSafeLifecycle`.

#### P22 — Enforce capability at IPC entry

* Files: `src/daemon/ipc_server.rs`,
  `tests/contract/read_server_ipc_refusal_test.rs`
* Refuse non-read capabilities before side effects, including raw
  `_shutdown`.

#### P23 — Enforce capability at shared tool dispatch

* Files: `src/tools/mod.rs`,
  `tests/contract/read_server_dispatch_refusal_test.rs`
* Apply the same registry decision as defense in depth for non-IPC transports.

#### P24 — Derive MCP catalog from tool descriptors

* Files: `src/shim/tools_catalog.rs`,
  `tests/contract/mcp_tool_catalog_parity_test.rs`
* Remove independent tool-count authority and exclude supervisor operations.

#### P25 — Derive CLI read surface from tool descriptors

* Files: `src/cli/runner.rs`,
  `tests/contract/cli_tool_catalog_parity_test.rs`
* Preserve user-friendly formatting as a view over the canonical descriptor.

#### P26 — Typed domain error envelope

* Files: `src/errors/mod.rs`,
  `tests/unit/read_server_error_envelope_test.rs`
* Define code, classifier, retryability, restart facts, last-known generation,
  consumed deadline, and remediation.

#### P27 — Preserve error envelope over daemon IPC

* Files: `src/daemon/protocol.rs`,
  `tests/contract/ipc_error_envelope_test.rs`
* P28 updates the current lossy conversion separately.

#### P28 — Remove lossy IPC error conversion

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/ipc_error_round_trip_test.rs`
* Map the complete domain envelope into wire data.

#### P29 — Preserve error envelope through MCP

* Files: `src/shim/transport.rs`,
  `tests/contract/mcp_error_envelope_test.rs`

#### P30 — Preserve error envelope through CLI JSON

* Files: `src/cli/runner.rs`,
  `tests/contract/cli_error_envelope_test.rs`

#### P31 — Attach provenance to successful read responses

* Files: `src/tools/mod.rs`,
  `tests/contract/read_response_provenance_test.rs`
* Decorate once from the request-captured Arc, never from later global state.

#### P32 — Read-server lifecycle policy

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/read_server_lifecycle_test.rs`
* Skip hydration, source scan, watcher, startup sync, and shutdown graph flush.
* Managed mode retains the characterized bind-first and watcher behavior.

#### P33 — Generation lease and status observability

* Files: `src/tools/lifecycle.rs`,
  `tests/integration/generation_observability_test.rs`
* Report active/published revision, source revision, timestamps, activation
  failure, live retired contexts, and disk usage; expose no deletion.

#### P34 — Shared preflight command

* Files: `src/bin/engram-indexer.rs`,
  `tests/integration/preflight_gate_test.rs`
* Under one supplied deadline: build, seal, publish, start/verify read daemon,
  wait for matching provenance, run one defined non-empty CLI read and the same
  MCP read, compare provenance, and emit one structured verdict.

#### P35 — PowerShell process wrapper

* Files: `start.ps1`, `tests/contract/start_launcher_test.rs`
* Invoke P34, enforce the same outer deadline, clean up exact children, and
  launch Copilot only on success.

#### P36 — PowerShell blocked-gate matrix

* Files: `tests/contract/start_launcher_failure_test.rs`
* Cover build, readiness, CLI probe, and MCP probe failures.
* Rewrite `launcher_fails_open_to_copilot_within_one_prewarm_budget` to the
  fail-closed contract.

#### P37 — PowerShell ownership and path matrix

* Files: `tests/contract/start_launcher_safety_test.rs`
* Preserve `launcher_timeout_does_not_terminate_unowned_descendant` and cover
  paths with spaces.

#### P38 — Unix process wrapper

* Files: `start.sh`, `tests/contract/start_sh_launcher_test.rs`
* Match P35 through the same P34 verdict.

#### P39 — Full agent-surface parity

* Files: `tests/contract/read_server_cli_mcp_parity_test.rs`
* Exhaustively execute every Read descriptor through direct IPC, CLI, and stdio
  MCP; assert pinned generation, successful provenance, and refusal/error
  equivalence.

#### P40 — Operator documentation

* Files: `docs/troubleshooting.md`
* Document preflight, failure classes, refresh, rollback, disk growth, and
  same-user trust scope.

#### P41 — Architecture documentation

* Files: `docs/ARCHITECTURE.md`
* Document immutable generations, request-entry reconciliation, request pinning,
  branch binding, supported transports, and why active JSON is machine-owned
  runtime state.

### Revision 2 dependencies

```text
S1 -> P1..P41
P1 -> P2 -> P3 -> P4
P5 -> P6 -> P7
P5 + P6 + P8 + P9 -> P11 -> P12
P9 -> P10
P2 + P5 + P8 -> P13
P7 + P8 + P13 -> P14
P14 -> P15
P13 -> P16 -> P17 -> P18 -> P19 -> P20
P21 -> P22 -> P23 -> P24 -> P25
P1 + P21 -> P22 + P23
P26 -> P27 -> P28 -> P29 -> P30
P13 + P21 -> P31
P2 + P8 + P13 -> P32
P13 + P14 -> P33
P4 + P12 + P24 + P25 + P29 + P30 + P31 + P32 + P33 -> P34
P34 -> P35 -> P36 -> P37
P34 -> P38
P16..P33 -> P39
P35 + P38 + P39 -> P40 + P41
```

### Revision 2 requirements trace

| Requirements | Units |
|---|---|
| R1-R5, R16-R18, R26-R32 | P1-P4, P32, P34-P38 |
| R6-R8, R10, R14-R15, R19-R21, R25, R33 | P21-P30, P39 |
| R9, R38, R44, R47 | P13-P20, P31, P39 |
| R11-R13, R22-R24 | P33-P41 |
| R34-R37, R40, R42 | P5-P12, P14, P33-P34 |
| R39, R48 | P11-P12, P21, P24-P25 |
| R41 | P33, P40 |
| R43, R46, R49 | S1, P5-P8 |
| R45 | P5, P7, P14-P15, P34 |

### Revision 2 runtime verification and closure

1. Hash the sealed active generation before and after startup, all read
   descriptors, one activation, daemon restart, and shutdown.
2. Hold representative core, lifecycle, report, lint, and doctor reads across a
   publication; each response must report the generation it actually queried.
3. Publish A, B, and a stale A revision concurrently; only the greatest valid
   revision may activate.
4. Corrupt or partially publish a candidate; the last-known-good context must
   remain available and health must report the failed revision.
5. Execute the canonical descriptor matrix over direct IPC, CLI, and stdio MCP;
   assert that documentation and supported-surface contracts do not advertise
   retired HTTP/SSE endpoints.
6. Run P34 from both launchers with success, each blocked stage, deadline
   exhaustion, spaces in paths, and unowned descendant processes.
7. Run the repository gates in order: format, clippy, `cargo dev-test`, and
   audit.
8. Record healthy signals (served/published revision equality, zero activation
   failures, successful CLI/MCP parity), rollback trigger (three consecutive
   activation failures or stale served revision beyond one request), and
   rollback action (restore prior manifest under publisher lock and restart the
   read daemon).

### Revision 2 Constitution Check

| Principle | Plan compliance |
|---|---|
| I. Safety-First Rust | S1 proves exact safe-Rust mechanisms before implementation; no unsafe code in this crate |
| II. Test-First Development | Every P-unit has a named RED harness or is test-only |
| III. Workspace Isolation | Strict IDs and canonical generation-root containment |
| IV. CLI Workspace Containment | All runtime data remains beneath the workspace `.engram` root |
| V. Structured Observability | Shared provenance/error envelopes and preflight verdicts |
| VI. Single Responsibility | Units are split by domain; S1 must justify any new dependency |
| VII. Destructive Approval | No generation deletion is implemented |
| VIII. Safety Modes | `investigate-first` applies to S1; implementation is freeze-scoped to generation storage, read daemon/dispatch, and launchers |
| IX. Git-Friendly Persistence | `active.json` is ignored machine runtime state, not harness state |
| X. Agent Context Efficiency | One compact provenance envelope and canonical descriptor registry |
| XI. Merge History Preservation | Merge-commit-only delivery |

### Revision 2 safety actions

**ProposedAction RS4**

* `summary`: Replace fail-open prewarm with a fail-closed shared preflight.
* `targets`: P34-P38.
* `change_kind`: launcher and process-lifecycle contract.
* `rollback`: restore the previous launcher while preserving generation code.
* `approval_required`: yes, before Ship implements P35.
* `ActionRisk`: high.
* `ActionResult`: planned.

**ProposedAction RS5**

* `summary`: Add immutable generation publication and request-entry activation.
* `targets`: P5-P15.
* `change_kind`: persistence and runtime storage selection.
* `rollback`: restore the prior manifest revision and restart in managed mode.
* `approval_required`: yes, before Ship implements P7/P14.
* `ActionRisk`: high.
* `ActionResult`: planned.

Revision 2 must pass a fresh multi-persona gate before harvest.

## Plan Review — Revision 2

**Gate decision: FAIL.** The third multi-persona review found no P0 but retained
P1 findings in Rust feasibility, request-path availability, complete pinning,
transport retirement, task wiring, publication bounds, and dependency order.
The Learnings Researcher reviewed an unrelated plan despite the absolute path;
that pass is excluded as stale. Relevant compound precedents from the prior pass
remain binding.

## Remediation Revision 3

Revision 3 consists of the following authoritative amendments to Revision 2.
Unchanged Revision 2 units remain in force.

### Architecture amendments

1. **Request entry order is fixed:** validate frame, resolve the canonical tool
   descriptor, refuse unknown/non-read operations, perform a bounded revision
   probe only for database-backed reads, trigger one background activation if
   needed, capture the current `Arc<ReadRequestContext>` once, then dispatch.
2. `_health` is constant-time liveness plus current provenance. `_shutdown` is
   Control. Neither triggers reconciliation.
3. Post-start activation never blocks a read. One single-flight task validates,
   opens, and swaps a newer generation under a configured activation deadline.
   Failed revision/content fingerprints are cached until the manifest changes.
4. `ReadRequestContext` is mode-agnostic: managed mode wraps the existing
   workspace database behavior; read-server mode wraps an immutable generation.
5. The activation service is one total predicate: parse typed manifest, enforce
   size/time limits, check workspace and branch identity, resolve through the
   generation store, revalidate sealed inventory digests, open through the
   selected P8 strategy, then swap. Any failure retains the prior Arc.
6. If S1 selects a private runtime copy, `GenerationStore` mints and owns a
   `RuntimeCopy` under `.engram/generations/runtime/`. Copy publication uses a
   unique temp directory, bounded copy, digest revalidation, and atomic seal.
   Partial copies are never opened. Runtime copies are retained and reported in
   this release; deletion remains deferred.
7. Candidate building accepts a sealed crate-private
   `IndexTarget::{LegacyDirect, Candidate}`. Only the direct wrapper can mint
   `LegacyDirect`; only `GenerationStore` can mint `Candidate`.
8. Supervisor publishing uses a cross-process lock from candidate seal through
   checked revision increment and manifest replacement. Generation IDs and
   candidate directories are exclusive-created. Leftover temporary manifests
   are ignored, never promoted.
9. `engram-indexer` moves to a separate workspace crate and separate release
   artifact. The agent archive and installer do not contain it.

### Unit amendments and additions

* **S1** adds checked-in Windows and Unix probe harnesses with explicit
  pass/fail contracts for Cozo open/sidecars, runtime-copy fallback, each
  manifest crash point, replace-existing behavior, and safe-Rust dependencies.
* **P2a — AppState constructor migration** owns compiler-driven migration of
  every `AppState::new` call after P2; harness:
  `tests/contract/app_state_constructor_migration_test.rs`.
* **P5a — Generation module wiring** owns private module declarations in
  `src/services/mod.rs` and `src/services/generations/mod.rs`; harness:
  `tests/contract/generation_module_visibility_test.rs`.
* **P7** explicitly uses a cross-process publisher lock and ignores orphaned
  temporaries.
* **P8** depends on P6 and consumes a store-minted `OpenedGeneration` or
  `RuntimeCopy`; it never accepts an arbitrary path.
* **P9** depends on P6 and uses the sealed `IndexTarget` constructors described
  above.
* **P11** enforces unique IDs, exclusive candidate creation, bounded manifest
  and inventory sizes, bounded generation bytes, and concurrent-builder tests.
* **P12** moves to `crates/engram-indexer/`; a new packaging unit **P12a**
  creates its independent release artifact and proves the agent archive and
  installer exclude it.
* **P13** adds both managed and generation-backed context constructors plus
  managed-mode characterization before read migrations.
* **P14** implements the fixed entry order and single-flight non-blocking
  activation. Its RED harness covers malformed/oversized manifest, post-seal
  corruption, branch/workspace mismatch, rejected-fingerprint caching, and
  first-request latency.
* **P15a — Remove retired server modules** deletes `src/server/mcp.rs`,
  `src/server/router.rs`, and `src/server/sse.rs`.
* **P15b — Remove retired feature and dependencies** removes `legacy-sse` and
  transport-only dependencies from `Cargo.toml` and updates build wiring.
* **P15c — Supersede transport decision and docs** supersedes ADR-0016 and
  ADR-0003, and corrects operator docs to direct IPC, CLI, and stdio MCP only.
  All architecture references use the real lowercase
  `docs/architecture.md` path.
* **P16a — Core `read.rs` reads** migrates search, symbols, graph, and memory.
* **P16b — `read.rs` report reads** migrates statistics, health, evaluation,
  branch metrics, token savings, mutable-script retry metrics, and
  feature-gated `query_changes`.
* **P20a — Search/embedding service migration** updates service signatures to
  accept pinned queries/context.
* **P20b — Registry/evaluation service migration** removes fresh path/snapshot
  derivation from read paths.
* **P20c — Metrics/query-stat service migration** completes the service layer.
* **P20d — Pinning enforcement contract** fails if any Read descriptor or
  supporting service calls `connect_db` or derives generation data from a fresh
  workspace snapshot.
* **P21a — Tool module wiring** privately registers canonical descriptors and
  requires compilation before P21.
* **P21** covers daemon methods and CLI workflows. `doctor --smoke` is replaced
  in read-server mode by the non-destructive descriptor-backed readiness probe.
* **P26a — Stable error code allocation** reserves named codes in
  `src/errors/codes.rs`; P26 defines typed variants and detail structs.
* **P34** is a typed state machine with legal transitions:
  `Build -> Seal -> Publish -> DaemonVerified -> HealthVerified ->
  CliProbeVerified -> McpProbeVerified -> Succeeded`, plus terminal
  `Failed { stage, failure_class }`. One absolute deadline, expected generation,
  and exact owned-child records flow through every transition. A pre-existing
  daemon is reused only when mode, workspace, and served revision match;
  otherwise preflight fails closed.
* **P39** executes every Read descriptor for success/provenance and every Write,
  Control, unsafe lifecycle, CLI workflow, and unknown method for identical
  refusal envelopes and zero side effects across direct IPC, CLI JSON, and
  stdio MCP.

### Corrected dependencies

The prior range shorthand is descriptive only and MUST NOT be harvested.
Harvest creates explicit edges. The corrected load-bearing edges are:

```text
S1 -> P5
S1 -> P6
S1 -> P7
S1 -> P8
P1 -> P2 -> P2a -> P3 -> P4
P5a -> P5 -> P6
P6 -> P7
P6 -> P8
P6 -> P9 -> P10
P5 + P6 + P8 + P9 -> P11
P7 + P11 -> P12 -> P12a
P2 + P2a + P5 + P8 -> P13
P15a -> P15b -> P15c
P15b -> P13
P21a -> P21 -> P22 -> P23 -> P24 -> P25
P1 + P21 -> P22
P5 + P6 + P7 + P8 + P13 + P21 + P22 -> P14
P13 -> P16a
P13 -> P16b
P13 -> P17
P13 -> P18
P13 -> P19
P13 -> P20a
P13 -> P20b
P13 -> P20c
P16a + P16b + P17 + P18 + P19 + P20a + P20b + P20c -> P20d
P26a -> P26 -> P27 -> P28 -> P29 -> P30
P13 + P21 + P23 -> P31
P2 + P4 + P8 + P13 + P14 -> P32
P13 + P14 -> P33
P4 + P12a + P14 + P24 + P25 + P29 + P30 + P31 + P32 + P33 -> P34
P34 -> P35 -> P36 -> P37
P34 -> P38
P4 + P14 + P20d + P22 + P23 + P24 + P25 + P29 + P30 + P31 + P33 -> P39
P15c + P35 + P38 + P39 -> P40
P15c + P35 + P38 + P39 -> P41
```

Independent edges fan out; no range expression is emitted to backlog.

### Corrected trace and verification

* R44 maps to P13, P16a-P20d, P31, and P39.
* R47 maps to P26-P31, P33-P34, and P39.
* R50 maps to P15a-P15c and P39.
* R51 maps to P13-P14, P20d, and P32.
* R52 maps to S1, P5-P8, P11, P14, P33-P34.
* R53 maps to P12-P12a, P24-P25, and P39.

Runtime verification additionally:

1. modifies source files while read-server mode is serving and proves no source
   watcher event, implicit sync, hydration, or same-database indexing occurs;
2. corrupts a sealed artifact after publication and proves activation fails
   while the prior context remains available;
3. starts two legitimate publishers and proves one complete generation wins
   under the cross-process lock;
4. verifies first-read latency stays within the read budget while background
   activation performs open/copy work;
5. verifies managed mode against its pre-change characterization suite;
6. proves no HTTP/SSE listener, feature, binary surface, or documentation claim
   remains;
7. proves the agent release archive contains no `engram-indexer` executable.

### Revision 3 gate readiness

All Revision 2 P1 findings are addressed by these amendments. P2 items accepted
into implementation include `_health` provenance, branch-divergence degraded
state, explicit orphan-temp discard, and error-envelope forward compatibility.
Revision 3 requires one final bounded plan-review confirmation before harvest.

## Plan Review — Revision 3

**Gate decision: FAIL.** The bounded confirmation found no valid P0 in the
authoritative revision, but retained P1 findings for startup activation
ownership, supervisor crate packaging, complete service-input pinning,
descriptor surface semantics, direct-sync behavior in read-server mode, and
complete HTTP/SSE retirement. Findings that cited superseded U1-U10 as active
were rejected as stale.

## Remediation Revision 4 — Authoritative Plan

**This section is the sole executable plan.** Every earlier unit list is
non-executable review history and MUST NOT be harvested.

### Final runtime design

The daemon binds IPC first so `_health` can report `starting`. In read-server
mode, a dedicated activation service then blocks readiness and database-backed
dispatch until the initially published generation is validated and opened.
After readiness, request entry classifies and authorizes first, checks only
manifest metadata under a hard cap, triggers one bounded background activation
when metadata changed, and immediately pins the current context. Managed mode
does no manifest work and preserves its existing database behavior.

`ReadRequestContext` owns all facts a read may consume:

* opened database/query handle;
* mode and workspace/branch identity;
* generation-owned registry/index artifacts;
* explicit operational inputs that are permitted to remain live;
* served generation and publication provenance.

Every Read descriptor declares its supported surfaces and input ownership.
Direct IPC-only liveness methods are not forced into CLI or MCP. Cross-surface
parity is required on every surface a descriptor declares.

### Concrete bounds

Defaults are fail-closed and may be lowered by deployment configuration:

| Input/work | Limit |
|---|---:|
| `active.json` | 1 MiB |
| Inventory entries | 4,096 |
| Individual sealed artifact | 4 GiB |
| Total generation/runtime copy | 16 GiB |
| Background activation | 120 seconds |
| Initial activation under preflight | remaining shared preflight deadline |

The request path performs only metadata checks and refuses an `active.json`
larger than 1 MiB without reading or parsing it. Full parse, digest validation,
copy, and open occur in the activation service. Immutable validation failures
are cached by revision plus content fingerprint. Busy, timeout, and transient
I/O failures retry with bounded backoff and are never permanently cached.

### Final work units

#### F01 — Storage feasibility harness

* Files: `tests/integration/generation_storage_probe_test.rs`,
  `docs/decisions/2026-09-02-generation-storage-feasibility-spike.md`
* RED-first Windows/Unix probes select the exact safe-Rust atomic replacement
  and Cozo open/runtime-copy strategy.
* Stop if byte stability, crash recovery, or `#![forbid(unsafe_code)]` cannot be
  met.

#### F02 — Strict mode contract

* Files: `src/models/config.rs`, `tests/unit/plugin_config_test.rs`
* Add strict `DaemonMode` parsing with absent-only managed default.

#### F03 — Immutable mode in state

* Files: `src/server/state.rs`, `tests/unit/app_state_mode_test.rs`
* Require mode at construction and expose no setter.

#### F04 — Mode constructor migration

* Files: `src/daemon/ipc_server.rs`,
  `tests/contract/app_state_constructor_migration_test.rs`
* Migrate all production/test constructors without a default-mode escape hatch.

#### F05 — Shim mode propagation

* Files: `src/shim/lifecycle.rs`,
  `tests/integration/read_server_restart_test.rs`
* Preserve persisted mode across auto-spawn and one bounded restart.

#### F06 — Generation module and domain types

* Files: `src/services/generations/mod.rs`,
  `src/services/generations/manifest.rs`
* Private module; strict ID, checked revision, branch/workspace identity,
  inventory, digests, and provenance.

#### F07 — Generation-store containment and targets

* Files: `src/services/generations/store.rs`,
  `tests/integration/generation_store_test.rs`
* Canonical containment, regular files, exclusive candidates, and sealed
  `IndexTarget::{LegacyDirect, Candidate}` constructors.

#### F08 — Atomic publication and publisher lock

* Files: `src/services/generations/publish.rs`,
  `tests/integration/generation_publish_test.rs`
* Apply F01 primitive, cross-process lock, checked revision, durable replacement,
  and ignore-never-promote orphaned temporaries.

#### F09 — Existing-generation open/runtime copy

* Files: `src/db/cozo_backend/mod.rs`,
  `tests/integration/generation_db_open_test.rs`
* Apply F01 strategy through store-minted `OpenedGeneration`/`RuntimeCopy`.
* Runtime copies stay under `.engram/generations/runtime`, use bounded
  temp-plus-seal publication, and are retained/observed in this release.

#### F10 — Candidate indexing service

* Files: `src/services/code_graph.rs`,
  `tests/integration/candidate_indexing_service_test.rs`
* Accept sealed `IndexTarget`; never accept an arbitrary path.

#### F11 — Direct-sync mode boundary

* Files: `src/cli/direct.rs`, `tests/integration/direct_sync_mode_test.rs`
* Legacy direct index/sync remains available only in Managed mode under
  `DaemonLock`. ReadServer mode returns the stable non-retryable refusal and
  directs operators to `engram-indexer`.

#### F12 — Supervisor crate foundation

* Files: `crates/engram-indexer/Cargo.toml`,
  `crates/engram-indexer/src/main.rs`
* Separate workspace crate with `#![forbid(unsafe_code)]`; build/seal/publish
  only through F07-F10.

#### F13 — Supervisor workspace wiring

* Files: `Cargo.toml`,
  `tests/contract/supervisor_workspace_boundary_test.rs`
* Add the workspace member without adding a binary to the agent package.

#### F14 — Supervisor release artifact

* Files: `.github/workflows/release.yml`,
  `tests/contract/supervisor_release_artifact_test.rs`
* Publish a distinct artifact; the agent archive excludes it.

#### F15 — Supervisor installer separation

* Files: `src/installer/mod.rs`,
  `tests/contract/supervisor_install_exclusion_test.rs`
* Agent installation never installs or exposes the supervisor.

#### F16 — Mode-agnostic request context

* Files: `src/server/state.rs`,
  `tests/unit/read_request_context_test.rs`
* Add managed and generation-backed constructors and Arc lifetime tests.

#### F17 — Activation service

* Files: `src/services/generations/activation.rs`,
  `tests/integration/generation_activation_test.rs`
* Implement `activate_initial` and single-flight `maybe_activate_newer`.
* Enforce typed manifest, bounds, branch/workspace identity, store resolution,
  digest revalidation, F09 open, deadline, immutable rejection cache, and
  transient backoff.

#### F18 — Startup activation gate

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/read_server_startup_activation_test.rs`
* Bind first, report starting, call `activate_initial`, and withhold ready/read
  dispatch until one context is open. Managed mode is unchanged.

#### F19 — Canonical descriptor schema

* Files: `src/tools/capabilities.rs`,
  `tests/contract/tool_descriptor_registry_test.rs`
* Each descriptor declares method/workflow, capability, schema, supported
  surfaces (`direct_ipc`, `cli`, `stdio_mcp`), read-server availability, and
  input ownership.
* `_health` is direct-IPC liveness; `_shutdown` is Control.
* `doctor --smoke` uses a non-destructive readiness workflow in ReadServer mode.

#### F20 — Request-entry order and background activation

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/request_entry_activation_test.rs`
* Frame -> descriptor -> authorize -> mode-gated capped metadata probe ->
  maybe-activate -> capture current Arc -> dispatch.
* `_health`, `_shutdown`, unknown, and refused methods do not activate.

#### F21 — Shared dispatch enforcement

* Files: `src/tools/mod.rs`,
  `tests/contract/read_server_dispatch_refusal_test.rs`
* Defense-in-depth capability gate and one context capture contract.

#### F22 — MCP catalog derivation

* Files: `src/shim/tools_catalog.rs`,
  `tests/contract/mcp_tool_catalog_parity_test.rs`
* Derive exposed stdio MCP tools from F19.

#### F23 — CLI surface derivation

* Files: `src/cli/runner.rs`,
  `tests/contract/cli_tool_catalog_parity_test.rs`
* Derive declared CLI workflows from F19; preserve formatting as a view.

#### F24 — Read-input ownership inventory

* Files: `tests/contract/read_input_ownership_inventory_test.rs`
* Exhaustively classify every input reachable from every Read descriptor as
  generation-owned, pinned operational, or disallowed in ReadServer mode.
* The test fails on an unclassified new input.

#### F25 — Core read handler migration

* Files: `src/tools/read.rs`,
  `tests/integration/core_read_generation_pin_test.rs`
* Migrate search, symbols, map/impact, memory, graph, and statistics.

#### F26 — Report handler migration

* Files: `src/tools/read.rs`,
  `tests/integration/report_read_generation_pin_test.rs`
* Migrate health/evaluation/branch/token/retry reports and feature-gated
  `query_changes`.

#### F27 — Lifecycle handler migration

* Files: `src/tools/lifecycle.rs`,
  `tests/integration/lifecycle_read_generation_pin_test.rs`
* Side-effect-free same-workspace bind only; retargeting refused.

#### F28 — Eval and lint handler migration

* Files: `src/tools/eval.rs`, `tests/integration/eval_read_pin_test.rs`
* F29 handles lint separately.

#### F29 — Lint and doctor handler migration

* Files: `src/tools/lint.rs`, `tests/integration/lint_read_pin_test.rs`
* F30 handles doctor separately.

#### F30 — Doctor handler migration

* Files: `src/tools/doctor.rs`, `tests/integration/doctor_read_pin_test.rs`

#### F31 — Search and embedding service migration

* Files: `src/services/search.rs`,
  `tests/integration/search_service_pin_test.rs`

#### F32 — Registry and evaluation service migration

* Files: `src/services/registry.rs`,
  `tests/integration/registry_service_pin_test.rs`

#### F33 — Retrieval evaluation service migration

* Files: `src/services/retrieval_eval.rs`,
  `tests/integration/retrieval_eval_service_pin_test.rs`

#### F34 — Metrics and query-stat service migration

* Files: `src/services/metrics.rs`,
  `tests/integration/metrics_service_pin_test.rs`

#### F35 — DAX lint service migration

* Files: `src/services/dax_lint.rs`,
  `tests/integration/dax_lint_service_pin_test.rs`

#### F36 — Git graph service migration

* Files: `src/services/git_graph.rs`,
  `tests/integration/git_graph_service_pin_test.rs`

#### F37 — Pinning enforcement

* Files: `tests/contract/read_path_pinning_enforcement_test.rs`
* Forbid fresh `connect_db`, workspace snapshot, workspace-root, and `.engram`
  reads unless F24 classifies them as pinned operational inputs.

#### F38 — Stable error codes and domain envelope

* Files: `src/errors/codes.rs`, `src/errors/mod.rs`
* Reserve typed refusal/availability codes and structured detail fields.

#### F39 — IPC error transport

* Files: `src/daemon/protocol.rs`,
  `tests/contract/ipc_error_envelope_test.rs`

#### F40 — Remove lossy IPC conversion

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/ipc_error_round_trip_test.rs`

#### F41 — MCP structured success and error transport

* Files: `src/shim/transport.rs`,
  `tests/contract/mcp_envelope_test.rs`
* Put successful provenance in `structured_content`; retain text as an optional
  human view.

#### F42 — CLI JSON success and error transport

* Files: `src/cli/runner.rs`,
  `tests/contract/cli_envelope_test.rs`

#### F43 — Successful response provenance

* Files: `src/tools/mod.rs`,
  `tests/contract/read_response_provenance_test.rs`
* Decorate from the captured context, never later global state.

#### F44 — Read-server lifecycle policy

* Files: `src/daemon/ipc_server.rs`,
  `tests/integration/read_server_lifecycle_test.rs`
* No hydration, source scan, watcher, implicit sync, or shutdown graph flush.

#### F45 — Generation observability

* Files: `src/tools/lifecycle.rs`,
  `tests/integration/generation_observability_test.rs`
* Report active/published/failed revision, branch divergence, retained runtime
  copies/contexts, deadlines, and disk usage; no deletion.

#### F46 — Remove retired server modules and tests

* Files: `src/server/mod.rs`, `tests/integration/connection_test.rs`
* Remove legacy module declarations and retire the HTTP/SSE integration test.
* Delete `src/server/mcp.rs`, `src/server/router.rs`, and `src/server/sse.rs` in
  the same reviewed change.

#### F47 — Remove retired feature and dependencies

* Files: `Cargo.toml`, `tests/contract/supported_transport_surface_test.rs`
* Remove `legacy-sse` and only dependencies proven to have no remaining users.

#### F48 — Remove retired installer and documentation claims

* Files: `src/installer/mod.rs`, `docs/architecture.md`
* Remove HTTP port/hook claims and state the three supported surfaces.

#### F49 — Supersede retired transport decisions

* Files: `docs/adrs/0016-legacy-sse-feature-gate.md`,
  `docs/adrs/0003-rate-limiting.md`
* Mark both superseded by the retirement decision.

#### F50 — Typed preflight state machine

* Files: `crates/engram-indexer/src/preflight.rs`,
  `tests/integration/preflight_gate_test.rs`
* One deadline and expected generation through Build -> Seal -> Publish ->
  DaemonVerified -> HealthVerified -> CliProbeVerified -> McpProbeVerified ->
  Succeeded or typed Failed.

#### F51 — PowerShell launcher wrapper

* Files: `start.ps1`, `tests/contract/start_launcher_test.rs`
* Launch Copilot only on F50 success; exact-child cleanup.

#### F52 — PowerShell failure and ownership matrix

* Files: `tests/contract/start_launcher_failure_test.rs`
* Cover each stage, paths with spaces, and unowned descendants; rewrite the
  existing fail-open assertion.

#### F53 — Unix launcher wrapper

* Files: `start.sh`, `tests/contract/start_sh_launcher_test.rs`

#### F54 — Descriptor-driven parity matrix

* Files: `tests/contract/read_server_cli_mcp_parity_test.rs`
* Exercise each descriptor only on declared surfaces.
* Read descriptors assert success/provenance; Write, Control, unsafe lifecycle,
  direct mutation workflow, and unknown methods assert refusal and no effects.

#### F55 — Operator documentation

* Files: `docs/troubleshooting.md`

### Explicit dependency graph

```text
F01 -> F06
F01 -> F07
F01 -> F08
F01 -> F09
F02 -> F03 -> F04 -> F05
F06 -> F07 -> F08
F07 -> F09
F07 -> F10 -> F11
F07 + F08 + F09 + F10 -> F12
F12 -> F13 -> F14 -> F15
F03 + F06 + F09 -> F16
F06 + F07 + F08 + F09 + F16 -> F17
F05 + F17 -> F18
F19 -> F20 -> F21 -> F22 -> F23
F02 + F19 -> F20
F17 + F19 -> F20
F16 + F19 -> F24
F16 + F24 -> F25
F16 + F24 -> F26
F16 + F24 -> F27
F16 + F24 -> F28
F16 + F24 -> F29
F16 + F24 -> F30
F16 + F24 -> F31
F16 + F24 -> F32
F16 + F24 -> F33
F16 + F24 -> F34
F16 + F24 -> F35
F16 + F24 -> F36
F25 + F26 + F27 + F28 + F29 + F30 + F31 + F32 + F33 + F34 + F35 + F36 -> F37
F38 -> F39 -> F40 -> F41 -> F42
F16 + F19 + F21 -> F43
F04 + F16 + F17 + F18 -> F44
F16 + F17 -> F45
F46 -> F47 -> F48 -> F49
F46 -> F19
F12 + F14 + F15 + F18 + F22 + F23 + F41 + F42 + F43 + F44 + F45 -> F50
F50 -> F51 -> F52
F50 -> F53
F05 + F18 + F22 + F23 + F37 + F40 + F41 + F42 + F43 + F45 + F47 -> F54
F48 + F49 + F51 + F53 + F54 -> F55
```

### Requirements trace

| Requirements | Units |
|---|---|
| R1-R5, R16-R18, R26-R32 | F02-F05, F18, F44, F50-F53 |
| R6-R8, R10, R14-R15, R19-R21, R25, R33 | F11, F19-F23, F27, F38-F43, F54 |
| R9, R38, R44, R47, R51 | F16, F24-F45, F54 |
| R11-R13, R22-R24 | F45, F50-F55 |
| R34-R37, R40, R42 | F06-F18, F45, F50 |
| R39, R48, R53 | F12-F15, F19, F22-F23, F54 |
| R41 | F45, F55 |
| R43, R46, R49 | F01, F06-F09 |
| R45, R52 | F06-F09, F17-F20, F45, F50 |
| R50 | F46-F49, F54-F55 |

### Final verification and closure

1. Prove initial health remains `starting` until one generation is open.
2. Prove post-start publication does not add first-read latency beyond the
   capped metadata probe and old reads remain available.
3. Hold each class of read across activation and match response provenance to
   the actual queried context.
4. Corrupt and oversize published artifacts; retain last-known-good service and
   report the failed fingerprint.
5. Run concurrent publishers; observe one complete monotonic winner.
6. Modify source while serving; observe no watcher, implicit sync, hydration,
   or same-database indexing.
7. Run managed-mode characterization unchanged.
8. Prove no HTTP/SSE module, feature, test, listener, installer claim, or
   documentation claim remains.
9. Prove the agent archive/install contains no supervisor executable.
10. Run every descriptor on only its declared surfaces, including all refusal
    classes and successful structured provenance.
11. Run format, clippy, `cargo dev-test`, and audit in repository order.
12. Record monitoring for active/published equality, activation latency/failure,
    stale-serving duration, disk growth, and preflight stage failures.
13. Roll back after three consecutive immutable activation failures or
    stale-serving beyond one publication cycle by restoring the prior manifest
    revision under the publisher lock and restarting the read daemon.

### Constitution and safety

The Revision 2 Constitution Check remains valid with these changes. `F01` is
investigate-first. F06-F18, F38-F45, and F50-F53 are freeze-scoped. RS4 and RS5
remain `ActionRisk: high` and require operator approval before Ship implements
F08/F17 or F50/F51. No destructive generation cleanup is in scope. HTTP/SSE
source deletion in F46 is a separately reviewed removal of an already retired
surface and requires operator approval immediately before execution.

**Revision 4 is ready for one gate confirmation.**

## Plan Review — Revision 4

**Gate decision: FAIL.** Scope and security passed. Rust and architecture
retained four blocking findings: F06/F12 lacked named RED harnesses; generation
context layering formed a server/service/db cycle; the supervisor crate lacked a
minimal shared generation API; and error-producing units did not depend on the
canonical error envelope. R1/R32 also needed explicit supersession.

## Remediation Revision 5 — Final Amendments

These amendments are authoritative over Revision 4. No other Revision 4 unit
changes.

1. **F06** adds
   `tests/unit/generation_domain_test.rs` as its RED harness. It exposes only a
   minimal safe public supervisor facade required by the separate
   `engram-indexer` crate; implementation details and constructors remain
   crate-private.
2. **F09** defines only the database-owned `ExistingDbLocation` and open
   strategy. It does not import generation-service types.
3. **F16** no longer defines generation contexts in `server::state`. A new
   **F16a — Generation context domain** owns
   `src/services/generations/context.rs` and
   `tests/unit/generation_context_test.rs`. It contains
   `GenerationReadContext` and runtime-copy ownership. It imports database
   infrastructure but never server state.
4. **F17** constructs `GenerationReadContext` and does not import
   `server::state`. F16 consumes it to construct the mode-agnostic
   `ReadRequestContext`. The layering is:
   `server -> generation activation/context -> db`, never the reverse.
5. **F12** adds
   `crates/engram-indexer/tests/supervisor_boundary_test.rs` as its RED
   harness. Both build and publish entry points live in the supervisor crate and
   consume the minimal public facade from F06-F10. The agent package defines no
   supervisor binary.
6. **F11** depends on F02's one shared mode resolver. Managed mode preserves
   direct sync under `DaemonLock`; ReadServer mode refuses it per amended
   R1/R32.
7. **F38** is a foundation. F11, F17, F20, F21, F44, and F45 must consume its
   named codes and typed envelope rather than mint local errors.
8. **F20** is the sole request-context capture site. F21 only re-checks
   capability and asserts a supplied context is present.
9. **F19** has no dependency on transport deletion. It positively declares the
   three supported surfaces. HTTP/SSE retirement converges at F54/F55.
10. **F24** classifies each non-database input explicitly. MCP-only
    `get_retrieval_eval_report` stays MCP-only unless a separate CLI requirement
    is approved; `_health` stays direct-IPC-only. F54 generates its matrix from
    each descriptor's declared surfaces.
11. **F46** also removes the retired test and module files. F47 serializes root
    `Cargo.toml` ownership before F13. F48 serializes
    `src/installer/mod.rs` ownership before F15.

### Final dependency corrections

Replace the affected Revision 4 edges with:

```text
F01 -> F06
F01 -> F07
F01 -> F08
F01 -> F09
F02 -> F03
F02 -> F11
F03 -> F04 -> F05
F06 -> F07 -> F08
F06 -> F09
F06 -> F16a
F07 -> F09
F07 -> F10
F02 + F07 + F10 + F38 -> F11
F07 + F08 + F09 + F10 -> F12
F12 -> F13
F13 -> F14
F12 -> F15
F03 + F09 + F16a -> F16
F06 + F07 + F08 + F09 + F16a + F38 -> F17
F16a + F17 -> F16
F05 + F16 + F17 + F38 -> F18
F19 -> F20
F19 -> F21
F19 -> F22
F19 -> F23
F02 + F17 + F38 -> F20
F20 -> F21
F16 + F19 -> F24
F38 -> F11
F38 -> F17
F38 -> F20
F38 -> F21
F38 -> F39
F38 -> F41
F38 -> F42
F38 -> F44
F38 -> F45
F39 -> F40
F46 -> F47
F47 -> F13
F47 -> F48
F48 -> F15
F48 -> F49
F46 + F47 + F48 + F49 -> F54
```

All unchanged Revision 4 edges remain. There is no `F46 -> F19` edge.

### Final trace additions

* R1 and R32 map to F02, F11-F15, and F50-F54.
* R54 maps to F06, F09, F16a-F18.
* F38 maps to every refusal and activation-error producer listed above.

### Gate disposition

The four Revision 4 blocking findings are resolved. This is the final review-fix
cycle; any new P0/P1 must halt implementation under the circuit breaker rather
than trigger another plan revision.

## Plan Review — Revision 5

**Gate decision: FAIL — circuit breaker open.**

The Rust confirmation found one P1: F12's RED harness cannot run under the
workspace gate until workspace membership is established. The architecture
confirmation found one P1: F04, F18, F20, F40, and F44 co-own
`src/daemon/ipc_server.rs` without a total ordering; F18 and F20 both define
request admission and would force rework.

Per the declared final review-fix limit, no further plan revision or
implementation is authorized in this session. Resume only after an operator
chooses one of these bounded remediations:

1. Move minimal workspace membership before F12's RED harness and serialize
   `ipc_server.rs` owners as `F04 -> F18 -> F20 -> F40 -> F44`.
2. Split `ipc_server.rs` into startup, request-entry, error-transport, and
   lifecycle-policy modules, then give each unit exclusive file ownership.

## Remediation Revision 6 — Operator-Directed Module Separation

**Operator decision (2026-09-02): remediation 2 — module separation, not mere
ordering — plus the workspace-wiring-before-RED ordering fix from remediation 1.**
The circuit breaker is reset by explicit operator direction.

This revision is authoritative over Revisions 4 and 5. It changes **file
ownership and unit boundaries only**. No architectural decision from Revision 4
is reopened: separately packaged `engram-indexer`, immutable generations, no
live generation control endpoint, initial activation gating readiness,
background single-flight post-start activation that never blocks reads, pinned
request context across all read inputs, Managed-mode compatibility, direct
IPC/CLI/stdio-MCP only, retired HTTP/SSE removal, trusted same-user threat
model, and no destructive generation cleanup all stand unchanged.

### A. IPC server module separation

`src/daemon/ipc_server.rs` is 2,452 lines. Five units co-owning it is a symptom
of an existing god-module, so the fix is decomposition rather than a longer
serialization chain.

**New unit F04a — IPC server seam extraction and module decomposition**

* Files: `src/daemon/ipc_server.rs`, `src/daemon/mod.rs`,
  `src/daemon/startup_activation.rs`, `src/daemon/request_entry.rs`,
  `src/daemon/error_transport.rs`, `src/daemon/lifecycle_policy.rs`,
  `tests/contract/ipc_server_seam_test.rs`
* Behavior-preserving mechanical extraction. Each new module ships with a
  pass-through implementation carrying today's semantics, so the tree is GREEN
  at extraction and no behavior change is bundled with the move.
* Declares every seam signature up front. After this unit, no downstream unit
  edits the seam file to wire itself in.
* `src/daemon/ipc_server.rs` is reduced to a composition root: framing, the
  accept loop, and delegation to the four seam points.

**Seam contract (declared by F04a, implemented downstream)**

| Seam point | Signature | Sole implementing unit |
|---|---|---|
| Initial startup gate | `startup_activation::run_initial_gate(&AppState) -> StartupOutcome` | F18 |
| Readiness publication | `startup_activation::readiness(&AppState) -> ReadinessView` | F18 (sole writer) |
| Request admission | `request_entry::admit(&AppState, &Frame) -> Admission` | F20 (sole authority) |
| Error transport | `error_transport::to_wire(DomainError) -> WireError` | F40 |
| Lifecycle policy | `lifecycle_policy::on_start(&AppState)` / `on_shutdown(&AppState)` | F44 |

**Single-admission rule.** This resolves the Revision 5 finding that F18 and F20
both defined request admission. `request_entry::admit` is the only admission
authority in the daemon. F18 never gates a request; it only publishes readiness
state through `ReadinessView`, which `admit` consults. One writer (F18), one
reader (F20), no overlapping decision logic and no rework.

**Amended file ownership**

| Unit | Owned files after Revision 6 |
|---|---|
| F04a | the seam + four new modules + `ipc_server_seam_test.rs` |
| F04 | `src/daemon/ipc_server.rs` (constructor call sites only), `tests/contract/app_state_constructor_migration_test.rs` |
| F18 | `src/daemon/startup_activation.rs`, `tests/integration/read_server_startup_activation_test.rs` |
| F20 | `src/daemon/request_entry.rs`, `tests/integration/request_entry_activation_test.rs` |
| F40 | `src/daemon/error_transport.rs`, `tests/integration/ipc_error_round_trip_test.rs` |
| F44 | `src/daemon/lifecycle_policy.rs`, `tests/integration/read_server_lifecycle_test.rs` |

F18, F20, F40, and F44 no longer touch `src/daemon/ipc_server.rs` at all.
Remaining ownership of that file is exactly two units in a declared,
non-competing order: `F04a` (structural move) then `F04` (constructor
signatures). F04a runs first so F04 lands on a small seam rather than the
monolith.

### B. Workspace membership before the supervisor RED harness

**New unit F12a — Supervisor crate skeleton and workspace membership**

* Files: `Cargo.toml` (root — `[workspace] members` only),
  `crates/engram-indexer/Cargo.toml`, `crates/engram-indexer/src/main.rs`
* Minimal buildable stub crate with `#![forbid(unsafe_code)]`. Adds
  `crates/engram-indexer` to the existing
  `members = [".", "crates/powerbi-tmdl-parser"]`.
* Verification: `cargo metadata` resolves the new member and
  `cargo test -p engram-indexer` executes under the workspace gate with zero
  tests. This proves the gate can run F12's RED harness before that harness is
  written.
* Contains no supervisor logic and does not depend on F07-F10.

**Amended F12 — Supervisor crate foundation**

* Files: `crates/engram-indexer/src/main.rs`, `crates/engram-indexer/src/lib.rs`,
  `crates/engram-indexer/tests/supervisor_boundary_test.rs`
* Depends on F12a, so the RED harness is runnable at the moment it is declared.
* No longer owns root `Cargo.toml`.

**Amended F13 — Supervisor workspace boundary contract**

* Files: `tests/contract/supervisor_workspace_boundary_test.rs` only.
* Asserts the agent package declares no supervisor binary and the crate boundary
  holds. No longer edits root `Cargo.toml`, so the Revision 5 `F47 -> F13`
  serialization edge is dropped.

Root `Cargo.toml` ownership is now exactly `F12a` (add workspace member) then
`F47` (remove `legacy-sse` and proven-unused dependencies) — disjoint sections
in a declared order.

### C. Residual co-ownership serialization

Three further file pairs lacked a total order. These are small, cohesive files
where decomposition is not warranted, so they are serialized explicitly:

* `src/tools/read.rs`: `F25 -> F26`
* `src/tools/lifecycle.rs`: `F27 -> F45`
* `src/cli/runner.rs`: `F23 -> F42`

After this revision every file in the plan has either a single owner or a
declared total order among its owners.

### Revision 6 dependency amendments

Replace the affected edges with:

```text
F03 -> F04a -> F04 -> F05
F04a -> F18
F04a -> F20
F04a -> F40
F04a -> F44
F12a -> F12 -> F13 -> F14
F12 -> F15
F12a -> F47
F25 -> F26
F27 -> F45
F23 -> F42
```

Dropped edges: `F47 -> F13`.

All other Revision 4 and Revision 5 edges remain in force, including
`F07 + F08 + F09 + F10 -> F12`, `F05 + F16 + F17 + F38 -> F18`,
`F02 + F17 + F38 -> F20`, `F39 -> F40`, `F38 -> F44`, and
`F04 + F16 + F17 + F18 -> F44`.

### Revision 6 trace additions

* R1-R5 and R26-R32 additionally map to F04a (seam ownership of mode-gated
  startup, admission, and lifecycle).
* R34-R37, R39, and R48 additionally map to F12a.
* No requirement loses coverage; F04a and F12a are structural enablers that
  redistribute existing coverage rather than adding scope.

### Revision 6 gate disposition

Two Revision 5 P1 findings are addressed by boundary changes only, plus three
preemptively serialized co-ownership pairs. Ready for one bounded Rust and
architecture confirmation against the final executable roster.

## Plan Review — Revision 6

**Gate decision: FAIL.** One bounded Rust and architecture confirmation was run
against the final executable roster only (F01-F55 + F16a + F04a + F12a) under
explicit operator authorization after the circuit-breaker reset.

Both Revision 5 P1 findings are **confirmed resolved**:

* Architecture confirmed the `ipc_server.rs` module separation resolves the
  five-owner conflict. Ownership is now `F04a -> F04` only; F18/F20/F40/F44 own
  four disjoint modules. The single-admission rule (F18 sole writer of
  `ReadinessView`, F20 sole `admit` authority) eliminates the competing
  request-admission rework surface.
* Rust confirmed F12a genuinely makes `cargo test -p engram-indexer` runnable
  before F12 writes its RED harness. Inside `crates/engram-indexer/`,
  `tests/supervisor_boundary_test.rs` **is** auto-discovered.

Architecture additionally confirmed the combined graph is acyclic, the
`server -> generation activation/context -> db` layering holds with no reverse
import, and no declared-ownership file lacks a total order.

Three new blocking findings remain, so harvest is **not** authorized.

### Revision 6 P0 findings

**P0-1 — New RED harnesses are never compiled; root `Cargo.toml` is implicitly
co-owned by ~30 units.**

This repository has **no** Cargo test auto-discovery: `tests/` contains only
`contract/`, `fixtures/`, `helpers/`, `integration/`, and `unit/`
subdirectories and **zero** top-level `tests/*.rs` files. Cargo auto-discovers
integration tests only at `tests/*.rs`, so every target is registered
explicitly — root `Cargo.toml` currently carries **218** `[[test]]` blocks.

Consequences for the final roster:

1. Roughly 30+ named RED harnesses are new files (F01, F04a, F06-F11, F13-F24,
   F25-F37, F39-F45, F50, F52-F54). Each is inert until a `[[test]]` entry
   exists. This is not merely inconvenient: an unregistered harness produces a
   **false GREEN**, because `cargo test` / `cargo dev-test` never compiles it.
   The test-first premise of the entire plan is void without this.
2. Revision 6's own invariant — that every file has a single owner or a declared
   total order — is false. Root `Cargo.toml` is implicitly co-owned by ~30 units
   with no declared order: the exact defect Revision 6 was directed to
   eliminate, relocated from `ipc_server.rs` to the manifest.

*Minimal fix:* extend **F12a** — already the declared first owner of root
`Cargo.toml` — from "workspace member only" to "workspace member **plus** one-time
registration of every new `[[test]] name/path` target named by F01-F55, each
with a committed placeholder harness file". Add edges `F12a -> F01` and
`F12a -> {every unit introducing a new test file}`. Root-manifest ownership stays
`F12a -> F47` over disjoint sections, and each downstream unit then writes only
its own already-registered harness. F12a verification adds:
`cargo test --test <target>` resolves for each registered target.

### Revision 6 P1 findings

**P1-1 — F12's RED harness cannot compile under its amended file list.**
F12a's stub owns `crates/engram-indexer/Cargo.toml` and by definition has no
dependency on the `engram` crate. Amended F12 owns only `src/main.rs`,
`src/lib.rs`, and `tests/supervisor_boundary_test.rs`, yet that harness must
exercise the minimal public supervisor facade from F06-F10, which requires
`engram = { path = "../.." }` in the crate manifest. As written the harness does
not build. No ownership conflict exists (`F12a -> F12` is ordered); this is a
file-list omission. *Minimal fix:* add `crates/engram-indexer/Cargo.toml` to
F12's owned files, annotated "dependency section only; after F12a".

**P1-2 — F20 has no dependency on F16.** Declared F20 in-edges are exactly
`F19 -> F20`, `F02 + F17 + F38 -> F20`, and `F04a -> F20`. F20 is the sole
request-context capture site and the Arc it captures is
`Arc<ReadRequestContext>`, which F16 defines. The graph therefore permits F20 to
start before F16 exists, at which point it cannot compile and its harness cannot
assert a pinned generation. Every other context-consuming daemon unit carries the
edge (`F05 + F16 + F17 + F38 -> F18`; `F04 + F16 + F17 + F18 -> F44`;
`F16 + F19 + F21 -> F43`); F20 is the only omission. *Minimal fix:* amend
`F02 + F17 + F38 -> F20` to `F02 + F16 + F17 + F38 -> F20`. Confirmed acyclic:
F16's transitive predecessors are disjoint from F20's out-cone.

### Revision 6 non-blocking observations

Carried forward for the next remediation; none gates harvest.

* Seam signatures should be declared `async` and fallible over `&Arc<AppState>`
  (`run_initial_gate`, `admit`, `on_start`/`on_shutdown`), since dispatch needs an
  owned `Arc` and `spawn_daemon_driver` requires `'static` futures. Must be
  settled at F04a time because downstream units may not edit the seam.
* `Admission::Proceed` should carry an opaque `request_entry::RequestScope` so
  F20 can thread `Arc<ReadRequestContext>` without touching the composition root.
* F03/F04 cannot both be green independently once `DaemonMode` becomes mandatory
  in `AppState::new`; land them as one gate-passing change or give F03 a
  temporary constructor that F04 removes.
* F06 should declare all five `generations` submodules up front and add
  `mod generations;` to `src/services/mod.rs` (in no unit's file list today).
* F18's Revision 4 prose ("withhold ready/read dispatch") is stale under the
  single-admission rule; restate as ReadinessView publication only and move the
  dispatch-withholding assertion to F20's harness.
* F04a should name the existing daemon/IPC suite as the regression gate so
  "GREEN at extraction" is verifiable rather than asserted.

### Revision 6 disposition

Harvest is blocked. Both findings the operator directed are resolved; the three
remaining findings are mechanical (one manifest-ownership extension, one file-list
addition, one dependency edge) and require no architectural change and no further
multi-persona cycle. A Revision 7 applying exactly those three corrections,
followed by a mechanical confirmation of the corrections only, is sufficient to
reach PASS.

## Remediation Revision 7 — Mechanical Corrections

**Operator direction (2026-09-02): apply exactly the three mechanical
corrections identified by the Revision 6 gate, run a confirmation limited to
those three items, then harvest.** No architecture is reopened and no broad
persona cycle is run. This revision is authoritative over Revisions 4, 5, and 6
for the three items below and changes nothing else.

### A. P0-1 — Test-manifest registration foundation

**Verified repository convention.** This workspace has zero top-level
`tests/*.rs` files; `tests/` contains only `contract/`, `fixtures/`, `helpers/`,
`integration/`, and `unit/`. Cargo auto-discovers integration tests only at
`tests/*.rs`, so every target is registered explicitly and root `Cargo.toml`
carries **218** `[[test]]` blocks. The established naming convention is
`name = "<subdir>_<basename minus _test>"` with
`path = "tests/<subdir>/<file>_test.rs"`.

Three of the plan's named harness files already exist and are already
registered (`tests/unit/plugin_config_test.rs`,
`tests/integration/connection_test.rs`,
`tests/contract/start_launcher_test.rs`). The remaining **49** are new files
and are inert until registered.

**New unit F00 — Test-manifest registration foundation.**

Per operator direction, registration is a single foundation unit executed
before any RED harness unit, rather than ~30 units sharing unordered ownership
of root `Cargo.toml`.

* Files: 49 placeholder harness files (19 `tests/contract/`,
  26 `tests/integration/`, 4 `tests/unit/`) and root `Cargo.toml`
  (`[[test]]` section, append-only: the 49 new blocks and nothing else).
* Internal order: create the 49 placeholder files first, then append the 49
  `[[test]]` blocks. Cargo does not existence-check an explicitly specified
  target `path` at metadata time; the failure surfaces at build/test time when
  rustc cannot read the file. The file-before-block order is therefore required,
  but it is an intra-unit ordering and never observable outside F00.
* Placeholder content is a doc comment naming the owning unit plus a single
  `#[test] fn placeholder_registered() {}`. Placeholders import nothing from
  `engram`, so the tree is GREEN at registration and the unit bundles no
  behavior change.
* Each downstream unit then writes only its own already-registered harness body
  and never edits root `Cargo.toml`.
* F00 has no in-edges; it is one of the graph's roots.

**Registered targets**

| Owning unit | `name` | `path` |
|---|---|---|
| F01 | `integration_generation_storage_probe` | `tests/integration/generation_storage_probe_test.rs` |
| F03 | `unit_app_state_mode` | `tests/unit/app_state_mode_test.rs` |
| F04 | `contract_app_state_constructor_migration` | `tests/contract/app_state_constructor_migration_test.rs` |
| F04a | `contract_ipc_server_seam` | `tests/contract/ipc_server_seam_test.rs` |
| F05 | `integration_read_server_restart` | `tests/integration/read_server_restart_test.rs` |
| F06 | `unit_generation_domain` | `tests/unit/generation_domain_test.rs` |
| F07 | `integration_generation_store` | `tests/integration/generation_store_test.rs` |
| F08 | `integration_generation_publish` | `tests/integration/generation_publish_test.rs` |
| F09 | `integration_generation_db_open` | `tests/integration/generation_db_open_test.rs` |
| F10 | `integration_candidate_indexing_service` | `tests/integration/candidate_indexing_service_test.rs` |
| F11 | `integration_direct_sync_mode` | `tests/integration/direct_sync_mode_test.rs` |
| F13 | `contract_supervisor_workspace_boundary` | `tests/contract/supervisor_workspace_boundary_test.rs` |
| F14 | `contract_supervisor_release_artifact` | `tests/contract/supervisor_release_artifact_test.rs` |
| F15 | `contract_supervisor_install_exclusion` | `tests/contract/supervisor_install_exclusion_test.rs` |
| F16 | `unit_read_request_context` | `tests/unit/read_request_context_test.rs` |
| F16a | `unit_generation_context` | `tests/unit/generation_context_test.rs` |
| F17 | `integration_generation_activation` | `tests/integration/generation_activation_test.rs` |
| F18 | `integration_read_server_startup_activation` | `tests/integration/read_server_startup_activation_test.rs` |
| F19 | `contract_tool_descriptor_registry` | `tests/contract/tool_descriptor_registry_test.rs` |
| F20 | `integration_request_entry_activation` | `tests/integration/request_entry_activation_test.rs` |
| F21 | `contract_read_server_dispatch_refusal` | `tests/contract/read_server_dispatch_refusal_test.rs` |
| F22 | `contract_mcp_tool_catalog_parity` | `tests/contract/mcp_tool_catalog_parity_test.rs` |
| F23 | `contract_cli_tool_catalog_parity` | `tests/contract/cli_tool_catalog_parity_test.rs` |
| F24 | `contract_read_input_ownership_inventory` | `tests/contract/read_input_ownership_inventory_test.rs` |
| F25 | `integration_core_read_generation_pin` | `tests/integration/core_read_generation_pin_test.rs` |
| F26 | `integration_report_read_generation_pin` | `tests/integration/report_read_generation_pin_test.rs` |
| F27 | `integration_lifecycle_read_generation_pin` | `tests/integration/lifecycle_read_generation_pin_test.rs` |
| F28 | `integration_eval_read_pin` | `tests/integration/eval_read_pin_test.rs` |
| F29 | `integration_lint_read_pin` | `tests/integration/lint_read_pin_test.rs` |
| F30 | `integration_doctor_read_pin` | `tests/integration/doctor_read_pin_test.rs` |
| F31 | `integration_search_service_pin` | `tests/integration/search_service_pin_test.rs` |
| F32 | `integration_registry_service_pin` | `tests/integration/registry_service_pin_test.rs` |
| F33 | `integration_retrieval_eval_service_pin` | `tests/integration/retrieval_eval_service_pin_test.rs` |
| F34 | `integration_metrics_service_pin` | `tests/integration/metrics_service_pin_test.rs` |
| F35 | `integration_dax_lint_service_pin` | `tests/integration/dax_lint_service_pin_test.rs` |
| F36 | `integration_git_graph_service_pin` | `tests/integration/git_graph_service_pin_test.rs` |
| F37 | `contract_read_path_pinning_enforcement` | `tests/contract/read_path_pinning_enforcement_test.rs` |
| F39 | `contract_ipc_error_envelope` | `tests/contract/ipc_error_envelope_test.rs` |
| F40 | `integration_ipc_error_round_trip` | `tests/integration/ipc_error_round_trip_test.rs` |
| F41 | `contract_mcp_envelope` | `tests/contract/mcp_envelope_test.rs` |
| F42 | `contract_cli_envelope` | `tests/contract/cli_envelope_test.rs` |
| F43 | `contract_read_response_provenance` | `tests/contract/read_response_provenance_test.rs` |
| F44 | `integration_read_server_lifecycle` | `tests/integration/read_server_lifecycle_test.rs` |
| F45 | `integration_generation_observability` | `tests/integration/generation_observability_test.rs` |
| F47 | `contract_supported_transport_surface` | `tests/contract/supported_transport_surface_test.rs` |
| F50 | `integration_preflight_gate` | `tests/integration/preflight_gate_test.rs` |
| F52 | `contract_start_launcher_failure` | `tests/contract/start_launcher_failure_test.rs` |
| F53 | `contract_start_sh_launcher` | `tests/contract/start_sh_launcher_test.rs` |
| F54 | `contract_read_server_cli_mcp_parity` | `tests/contract/read_server_cli_mcp_parity_test.rs` |

All 49 generated names were checked against the 218 existing target names; there
are no collisions.

**Units deliberately excluded from F00 registration**

| Unit | Reason |
|---|---|
| F02 | `tests/unit/plugin_config_test.rs` already exists and is registered |
| F12 | harness is `crates/engram-indexer/tests/supervisor_boundary_test.rs`, auto-discovered inside its own crate; the root manifest never registers it |
| F38, F48, F49, F55 | own no test file |
| F46 | deletes an existing registered harness rather than adding one |
| F51 | `tests/contract/start_launcher_test.rs` already exists and is registered |
| F12a | owns no test file; still carries an `F00 -> F12a` edge for root `Cargo.toml` section ordering |

**Root `Cargo.toml` total order.** Ownership is now exactly four units over
disjoint sections in a declared order:

```text
F00  -> [[test]] section: append the 49 new blocks
F12a -> [workspace] members: add crates/engram-indexer
F46  -> [[test]] section: remove the one pre-existing integration_connection block
F47  -> [features]/[dependencies]: remove legacy-sse and proven-unused deps
```

F00 and F46 both touch the `[[test]]` section but over disjoint blocks — F00
only appends the 49 new targets, F46 only removes the single pre-existing
`integration_connection` target — under the declared `F00 -> F46` order.

**Deregistration is owned by F46, not F47.** The unit that deletes
`tests/integration/connection_test.rs` also removes its `[[test]]` block in the
same change. Deferring the block removal to F47 would leave a broken window:
the block carries `required-features = ["legacy-sse"]`, so `cargo dev-test`
(default features) would stay GREEN and hide the breakage, while the
repository's own `cargo lint` (`clippy --all-targets --all-features`) and
`cargo ci` (`test --all-targets --all-features`) gates enable `legacy-sse` and
would fail with `couldn't read tests/integration/connection_test.rs` for the
whole span between F46 and F47. Co-locating the deletion and the
deregistration keeps every unit independently gate-passing.

**F00 verification**

1. `cargo test --test <target> -- --list` resolves for each of the 49 new
   target names. This is the authoritative proof that a target is registered
   and compiled.
2. `cargo dev-test` is GREEN with the placeholders in place.
3. `cargo ci` (`--all-features`) is GREEN, confirming registration does not
   break the feature-gated target set.

### B. P1-1 — F12 supervisor crate dependency wiring

**Amended F12 — Supervisor crate foundation**

* Files: `crates/engram-indexer/Cargo.toml` (dependency sections only; after
  F12a), `crates/engram-indexer/src/main.rs`,
  `crates/engram-indexer/src/lib.rs`,
  `crates/engram-indexer/tests/supervisor_boundary_test.rs`
* F12 adds `engram = { path = "../.." }` plus the dev-dependencies its boundary
  harness requires. Without this the harness cannot resolve the minimal public
  supervisor facade from F06-F10 and does not build.
* The workspace-stub-before-RED order is preserved unchanged: F12a creates the
  crate manifest and adds workspace membership; F12 then edits only the
  dependency sections of that manifest. `crates/engram-indexer/Cargo.toml`
  therefore has exactly two owners in the declared order `F12a -> F12` over
  disjoint sections.
* F12a remains free of any dependency on the `engram` crate, so
  `cargo test -p engram-indexer` still executes with zero tests at F12a time.

### C. P1-2 — Missing `F16 -> F20` edge

Amend `F02 + F17 + F38 -> F20` to `F02 + F16 + F17 + F38 -> F20`. F20 is the
sole request-context capture site and the value it captures is
`Arc<ReadRequestContext>`, defined by F16. This aligns F20 with every other
context-consuming daemon unit.

### Revision 7 dependency amendments

Add:

```text
F00 -> F01, F03, F04, F04a, F05, F06, F07, F08, F09, F10, F11,
       F13, F14, F15, F16, F16a, F17, F18, F19, F20, F21, F22,
       F23, F24, F25, F26, F27, F28, F29, F30, F31, F32, F33,
       F34, F35, F36, F37, F39, F40, F41, F42, F43, F44, F45,
       F47, F50, F52, F53, F54
F00 -> F12a
F12a -> F46
F16 -> F20
```

Replace: `F02 + F17 + F38 -> F20` becomes `F02 + F16 + F17 + F38 -> F20`.

The `F12a -> F46` edge completes the root `Cargo.toml` total order
`F00 -> F12a -> F46 -> F47`. It is acyclic: F12a's only predecessor is F00, and
F46's out-cone (F47, F48, F49, F15, F54, F55) does not contain F00 or F12a.

Dropped edges: none.

All other Revision 4, 5, and 6 edges remain in force.

### Revision 7 trace additions

F00 is a structural enabler and carries no requirement of its own; it makes the
test-first evidence for every requirement already traced to F01-F55 actually
executable. No requirement gains or loses coverage.

### Revision 7 gate disposition

Exactly three mechanical corrections applied. Ready for a confirmation limited
to these three items.

## Plan Review — Revision 7

**Gate decision: PASS.** A mechanical Rust and architecture confirmation was run
scoped strictly to the three Revision 7 corrections. No broad persona cycle was
run and no architectural question was reopened, per operator direction.

### Confirmation scope and result

**Item A — test-manifest registration (P0-1): RESOLVED.**

* The convention claim was independently re-verified against the working tree:
  0 files matching `tests/*.rs`, 218 `[[test]]` blocks in root `Cargo.toml`
  (enumerated exactly), and `name`/`path` pairs following
  `<subdir>_<basename>` / `tests/<subdir>/<file>_test.rs`. All 49 new paths
  were confirmed absent from the tree, so no placeholder clobbers existing
  content.
* All 49 new harnesses are registered by a single unit. The false-GREEN failure
  mode is closed: an unregistered harness can no longer exist, because no unit
  other than F00 introduces a harness path.
* Root `Cargo.toml` co-ownership is eliminated. Ownership is four units
  (`F00 -> F12a -> F46 -> F47`) over disjoint sections with a declared total
  order, satisfying the same invariant Revision 6 established for
  `ipc_server.rs`.
* The placeholder-before-block internal ordering is correct and is intra-unit.
  The mechanism is build/test-time rather than metadata-time, so F00's
  verification rests on `cargo test --test <target> -- --list`, `cargo dev-test`,
  and `cargo ci`.
* Placeholders import nothing from `engram`, compile clean under the
  repository's `-Dwarnings` and `clippy::pedantic` settings, and so F00 lands
  GREEN and introduces no behavior change. Each downstream unit's RED step is
  genuinely RED because it replaces a passing placeholder with real assertions
  against unwritten code.
* All 49 names begin with `contract_`/`integration_`/`unit_`, which the existing
  `.cargo/test-coverage-manifest.toml` surface globs already match. The
  `unit_dev_test_coverage_oracle` completeness assertion
  (`UNMAPPED_TARGETS_COUNT == 0`) and the `unit_dev_test_hcl_scope_guard`
  byte-identical block assertion therefore both stay GREEN without editing that
  manifest.
* Generated names were collision-checked against all 218 existing target names;
  no duplicates. Nearest neighbours (`contract_start_launcher` vs
  `contract_start_launcher_failure`) are distinct.
* F00's out-edge list is set-identical to the 49 registration rows plus F12a,
  cross-checked against every unit in the roster. The registration table and the
  exclusion table together account for every unit exactly once.
* The F12 exclusion is correct: `crates/engram-indexer/tests/supervisor_boundary_test.rs`
  is a top-level `tests/*.rs` **inside that crate** and is auto-discovered
  there. Registering it in the root manifest would wrongly compile it into the
  `engram` package.

**One in-cycle P1 was raised and remediated during this confirmation.**

*R7-A1 — deregistration timing.* The first draft assigned removal of the
`integration_connection` `[[test]]` block to F47. Because that block carries
`required-features = ["legacy-sse"]`, `cargo dev-test` (default features) would
have stayed GREEN and hidden the breakage, while `cargo lint`
(`clippy --all-targets --all-features`) and `cargo ci`
(`test --all-targets --all-features`) enable `legacy-sse` and would have failed
with `couldn't read tests/integration/connection_test.rs` for the entire span
between F46 and F47. Ownership was moved to F46 so the deletion and the
deregistration land together, and the `F12a -> F46` edge was added to complete
the root-manifest total order. Re-checked: the edge is acyclic and every unit
remains independently gate-passing. **Resolved.**

**Item B — F12 dependency wiring (P1-1): RESOLVED.**

* `crates/engram-indexer/Cargo.toml` now appears in F12's file list, annotated
  "dependency sections only; after F12a". The root package `engram` has a lib
  target, `../..` from `crates/engram-indexer/` resolves to the repo root, and
  `engram = { path = "../.." }` is exactly what the boundary harness needs to
  resolve the F06-F10 public facade. No dependency cycle is created: the root
  package depends only on `powerbi-tmdl-parser`.
* The workspace-stub-before-RED order is preserved. F12a still creates a
  bin-only stub with no `engram` dependency, so its verification
  (`cargo test -p engram-indexer` executing with zero tests) remains valid and
  still proves the gate can run F12's harness before that harness is written.
* Ownership of the crate manifest is exactly `F12a -> F12`, a declared total
  order over disjoint TOML sections. Workspace `[patch.crates-io]` and
  `resolver = "2"` apply to the new member automatically.

**Item C — `F16 -> F20` edge (P1-2): RESOLVED.**

* The edge is present. F20 can no longer start before `ReadRequestContext`
  exists.
* Acyclicity re-checked for this edge only. F16's transitive predecessor set
  `{F00, F01, F02, F03, F06, F07, F08, F09, F16a, F17, F38}` is disjoint from
  F20's out-cone `{F21, F22, F23, F42, F43, F50-F55}`, so no cycle is created.

### Graph-level mechanical re-check

Limited to the edges Revision 7 touches:

* F00 has no in-edges. It is one of three graph roots (F00, F02, F38). Its
  50 out-edges all point at existing nodes, so acyclicity is preserved by
  construction. F46 ceased to be a root when Revision 7 added `F12a -> F46`.
* The full edge set remains acyclic. A valid topological order exists, for
  example: `F00, F02, F38, F01, F03, F04a, F04, F05, F06, F07, F08, F09,
  F10, F11, F12a, F46, F16a, F17, F16, F12, F13, F14, F15, F19, F18, F20,
  F21, F22, F23, F24, F25-F36, F37, F39, F40, F41, F42, F43, F44, F45, F47,
  F48, F49, F50, F51, F52, F53, F54, F55`. F46 is placed after F12a to honour
  the Revision 7 `F12a -> F46` edge and before F47 to honour `F46 -> F47`.
* Every file named in the plan has either a single owner or a declared total
  order among its owners. The multi-owner files are exactly: root `Cargo.toml`
  (`F00 -> F12a -> F46 -> F47`), `crates/engram-indexer/Cargo.toml`
  (`F12a -> F12`), `src/daemon/ipc_server.rs` (`F04a -> F04`),
  `src/tools/read.rs` (`F25 -> F26`), `src/tools/lifecycle.rs`
  (`F27 -> F45`), `src/cli/runner.rs` (`F23 -> F42`), and the 49 placeholder
  harnesses (`F00 -> owning unit`).

### Findings

No open P0 or P1 findings. The one P1 raised in this cycle (R7-A1) was
remediated in place and re-confirmed above.

Carried forward as non-blocking implementation-time guidance, in addition to the
Revision 6 observations which stand unchanged:

* F20 reads `ReadinessView`, whose sole writer is F18, but no `F18 -> F20` edge
  exists. This does not break compilation, because F04a declares the seam
  signature and ships a pass-through. Flagged only for sequencing F20's
  dispatch-withholding assertion.

### Disposition

**Harvest authorized.** The plan is executable as written. Revisions 4, 5, 6,
and 7 together constitute the authoritative roster: F00, F01-F55, F04a, F12a,
and F16a — 59 units.

Execution posture remains unchanged: RS4 and RS5 stay `ActionRisk: high` and
require operator approval before Ship implements F08/F17 or F50/F51; the
HTTP/SSE source deletion in F46 requires operator approval immediately before
execution.
