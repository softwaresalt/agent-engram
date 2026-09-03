---
title: "Pre-session Engram indexing and read-only snapshot serving for agent sessions"
description: "Requirements for out-of-session index prewarming, a fail-closed startup gate, and an explicit read-only snapshot daemon mode that serves CLI and MCP reads from a persisted index without watcher or sync coupling"
doc_type: spec
source: "docs/research/2026-09-02-engram-read-only-snapshot-mode-requirements.md"
date: "2026-09-02"
source_stash_ids: []
source_research:
  - "start.ps1"
  - "src/daemon/ipc_server.rs"
  - "src/daemon/watcher.rs"
  - "src/daemon/mod.rs"
  - "src/models/config.rs"
  - "src/shim/lifecycle.rs"
  - "src/shim/tools_catalog.rs"
  - "src/shim/transport.rs"
  - "src/cli/direct.rs"
  - "src/cli/runner.rs"
  - "src/errors/mod.rs"
  - "src/errors/codes.rs"
  - "docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md"
  - "docs/compound/concurrency-issues/early-hydration-ready-before-heavy-io-2026-05-09.md"
  - "docs/compound/best-practices/auto-reindex-oom-gate-2026-05-09.md"
  - ".backlogit/queue/002-SP.md"
scope: "deep"
handoff_status: "ready_for_plan"
dark_factory_ready: false
requirement_ids:
  - "R1"
  - "R2"
  - "R3"
  - "R4"
  - "R5"
  - "R6"
  - "R7"
  - "R8"
  - "R9"
  - "R10"
  - "R11"
  - "R12"
  - "R13"
  - "R14"
  - "R15"
  - "R16"
  - "R17"
  - "R18"
  - "R19"
  - "R20"
  - "R21"
  - "R22"
  - "R23"
  - "R24"
  - "R25"
  - "R26"
  - "R27"
  - "R28"
  - "R29"
  - "R30"
  - "R31"
  - "R32"
  - "R33"
  - "R34"
  - "R35"
  - "R36"
  - "R37"
  - "R38"
  - "R39"
  - "R40"
  - "R41"
  - "R42"
  - "R43"
  - "R44"
  - "R45"
  - "R46"
  - "R47"
  - "R48"
  - "R49"
  - "R50"
  - "R51"
  - "R52"
  - "R53"
  - "R54"
---

# Pre-session Engram indexing and read-only snapshot serving for agent sessions

## Problem Frame

### The pain

Agent sessions routinely begin before Engram is functionally usable. Copilot
starts, the agent issues its first `unified_search` or `list_symbols` call, and
receives a degraded MCP result carrying
`{"failure_class":"readiness_timeout","message":"daemon did not reach a ready
state within the configured budget"}` (observed on
`0.3.0-rc.1+ge043299`). The agent then either stalls, retries, or silently falls
back to raw filesystem scanning — the exact regression that the
`agent-engram.instructions.md` engram-first discovery contract exists to prevent.

### Why it happens today (evidence, not assumption)

1. **The launcher prewarm is fail-open and under-budgeted.** `start.ps1`
   invokes `engram --format text --timeout 300 sync --direct` through
   `Invoke-EngramCommandWithProgress`, but the wrapper computes
   `$prewarmDeadline` from `$prewarmTimeoutMs = 15000` and **kills the process**
   when that 15-second wall clock expires, despite the `--timeout 300` argument
   promising a 300-second budget. `ENGRAM_PREWARM_TIMEOUT_MS` is clamped to
   `1..30000` ms, so the budget cannot be raised past 30 seconds by
   configuration. The daemon-backed fallback (`bind` then `sync`) shares the
   **same already-consumed deadline**, so it is usually dead on arrival. Every
   failure path terminates in `Write-Warning "engram sync failed (non-fatal)"`
   and Copilot launches anyway.

2. **Readiness is a race the shim usually loses on a cold index.**
   `src/shim/lifecycle.rs` defaults `ENGRAM_READY_TIMEOUT_MS` to `30_000` and
   polls `_health` with 10 ms → 500 ms backoff. Queued spike `002-SP` records a
   135 MB Cozo database taking approximately 7.5 minutes to reach ready. A
   30-second budget against a multi-minute cold start produces
   `ShimFailureClass::ReadinessTimeout` (`SHIM_READINESS_TIMEOUT = 15_002`) and a
   degraded session for every subsequent `tools/call`.

3. **Read reliability is coupled to the watcher and to startup auto-sync.**
   `run_with_shutdown_v2` binds IPC, spawns `run_startup_driver` (which calls
   `set_workspace` → `sync_workspace` → `set_hydration_ready_for_permit`), then
   `spawn_blocking(start_watcher)` and `run_watcher_driver`. There is no
   `watch`, `auto_sync`, or `startup_reindex` flag anywhere in `PluginConfig`
   or `WatcherConfig` — watching and startup sync are unconditional. Watcher
   failure degrades silently (`start_watcher` returns `Ok(None)` on debouncer
   failure, and the `Err` branch is swallowed by `.unwrap_or(None)` at the call
   site), so a partially-functional watcher is indistinguishable from a healthy
   one from the agent's perspective.

4. **`_health == "ready"` is weaker than "reads work".** The
   `early-hydration-ready-before-heavy-io` learning deliberately moved
   `set_hydration_ready()` to fire immediately after `connect_db()`, *before*
   the JSONL code-graph hydration completes, precisely so the shim would stop
   timing out. That was the right fix for liveness, but it means a `ready`
   health response does not prove that a search will return results.

### Pressure-testing the framing

The naive framing is "make the daemon start faster". That is a performance
problem already owned by spike `002-SP` and it does not make session start
deterministic — it only makes the race narrower. The higher-leverage framing is
to **move indexing out of the session entirely and make the session-start
contract a verified precondition rather than a hopeful one**. Once indexing is
an out-of-session responsibility, the in-session daemon has exactly one job:
serve reads from an index that already exists. Everything that exists to keep
that index fresh — the watcher, offline scans, startup auto-sync, watcher-driven
resync — becomes pure risk with no in-session benefit, and can be switched off
by an explicit mode rather than tolerated as background noise.

This also matches prior reliability work in this repository: hydration readiness
was moved earlier, startup auto-reindex was defaulted off behind
`ENGRAM_AUTO_REINDEX`, and indexing guards were removed from read tools. Prior
characterization identified background watcher and index activity as a
reliability confound. This feature is the terminal step of that arc: make the
absence of background index activity an explicitly configured, verifiable state
rather than an emergent one.

### Who cares

* **Agents** (Copilot CLI, Claude Code, Codex) — need Engram search to work on
  the first call of the session, over both CLI and MCP.
* **The operator** — needs a launcher that either produces a working session or
  refuses to start, with a legible reason.
* **Harness reliability** — every silent Engram degradation reintroduces raw
  grep fallback and undermines every engram-first instruction in the workspace.

## Requirements

### Group A — Out-of-session indexing (launcher / indexer)

* **R1.** A pre-session launcher/indexer stage MUST complete initial index
  construction via the daemonless `engram sync --direct` path
  (`src/cli/direct.rs`) **before** the AI CLI tool is launched, and before the
  read-serving daemon is started.
* **R2.** The launcher's wall-clock budget for the indexing stage MUST NOT be
  shorter than the budget passed to the Engram command itself. The current
  15-second wrapper deadline that kills a `--timeout 300` invocation is a defect
  and MUST be eliminated; the wrapper deadline MUST be derived from, or bounded
  below by, the command's own timeout.
* **R3.** `ENGRAM_PREWARM_TIMEOUT_MS` MUST accept values large enough for a full
  cold index of a realistic workspace. The existing `1..30000` ms validation
  clamp MUST be raised, and the default MUST be sized against observed cold-start
  evidence (`002-SP`: ~7.5 min on a 135 MB database) rather than against the
  shim's 30-second readiness default.
* **R4.** The indexing stage MUST be fail-closed. A non-zero exit, a timeout, or
  a lock contention failure from `sync --direct` MUST abort the launch with a
  non-zero exit code and a structured, human-legible diagnostic. The current
  `Write-Warning "... (non-fatal)"` fail-open behavior MUST be removed for this
  stage.
* **R5.** Because `DaemonLock::acquire` in `run_direct_sync` returns
  `LockError::AlreadyHeld` when a daemon holds the lock, the launcher MUST
  sequence indexing strictly before read-daemon start, and MUST detect and
  report a pre-existing daemon holding the lock rather than silently skipping
  the index.

### Group B — Read-only snapshot mode (daemon)

* **R6.** The daemon MUST support an explicit **read-only snapshot mode**,
  disabled by default, selectable through a persisted workspace configuration
  value (the `.engram/config.toml` / `PluginConfig` surface) so that the mode
  survives process restarts and is not carried solely by ambient process
  environment.
* **R7.** In read-only snapshot mode the daemon MUST NOT: initialize or start
  the file watcher (`start_watcher`), register recursive OS watches, run any
  offline or startup workspace scan, run startup auto-sync
  (`sync_workspace` from `run_startup_driver`), spawn the watcher event/driver
  loop (`run_watcher_driver`), or perform any index mutation for any reason.
* **R8.** In read-only snapshot mode, index-mutating operations
  (`index_workspace`, `sync_workspace`, and the feature-gated
  `index_git_history`) MUST be refused with an explicit structured error
  carrying a stable machine-readable classifier and a stable Engram error code.
  Refusal MUST NOT be a silent no-op, MUST NOT return a success envelope, and
  MUST NOT perform a partial mutation before refusing.
* **R9.** All catalogued read operations MUST function in read-only snapshot
  mode against the persisted index: `unified_search`, `list_symbols`,
  `map_code`, `impact_analysis`, `query_memory`, `query_graph`,
  `get_workspace_statistics`, `get_daemon_status`, `get_workspace_status`,
  `get_health_report`, `get_branch_metrics`, and the `report`-family tools.
* **R10.** Workspace binding (`set_workspace` / `engram bind`) MUST remain
  permitted in read-only snapshot mode because it selects the workspace under
  test, but it MUST NOT trigger a sync, a scan, or watcher registration as a
  side effect in this mode.
* **R11.** The active mode MUST be observable. Health and status responses MUST
  report whether read-only snapshot mode is active, and MUST expose snapshot
  provenance (at minimum the persisted index's last-sync timestamp) so an agent
  or operator can reason about what the index represents.
* **R12.** Default behavior MUST remain byte-for-byte backward compatible. With
  the mode unset, the daemon MUST behave exactly as it does today: watcher
  started after IPC bind, startup driver runs `sync_workspace`, watcher-driven
  auto-sync active. No existing configuration file, environment variable, CLI
  invocation, or test may change meaning.

### Group C — Fail-closed session-start gate

* **R13.** The launcher MUST NOT start the AI CLI tool until all of the
  following have succeeded, in order: (a) direct indexing completed (R1–R5);
  (b) the read daemon started in read-only snapshot mode; (c) the daemon
  reported healthy; (d) representative read probes returned results.
* **R14.** Health alone MUST NOT be accepted as the readiness proof. Because
  `set_hydration_ready()` intentionally fires before heavy JSONL hydration
  completes, the gate MUST additionally execute at least one **representative
  read probe that returns actual index-derived results** (for example a search
  or symbol listing over a known-present symbol) before releasing the session.
* **R15.** The readiness probe MUST be exercised over **both** access surfaces —
  a CLI read command and the MCP-equivalent contract for the same operation — so
  that a session is never released with one surface working and the other
  degraded.
* **R16.** The readiness budget for this gate MUST be configurable and MUST be
  sized against observed daemon cold-start evidence, not against the shim's
  30-second `ENGRAM_READY_TIMEOUT_MS` default.
* **R17.** Gate failure MUST produce a structured failure record with a stable
  `failure_class` (reusing the existing `ShimFailureClass` vocabulary where a
  class already fits, and extending it only where a genuinely new failure mode
  exists) and MUST abort launch rather than proceed degraded.

### Group D — CLI / MCP read parity

* **R18.** CLI and MCP read access MUST be interchangeable: both MUST resolve to
  the same daemon instance over the same IPC endpoint, read the same persisted
  index, and converge on the same shared `tools::dispatch` handler layer, so
  that an agent may use either surface with equivalent contracts.
* **R19.** Refusal semantics for index-mutating operations under read-only
  snapshot mode MUST be identical on both surfaces: same classifier, same error
  code, same non-mutating outcome. A mutation refused over MCP MUST NOT be
  accepted over CLI, and vice versa.
* **R20.** Windows named-pipe behavior MUST be preserved unchanged. The
  `\\.\pipe\engram-{workspace_key}` endpoint derivation, the
  `GenericNamespaced` prefix-stripping in `bind_listener_impl`, and the
  `.workspace-id`-derived daemon key MUST NOT be altered by this feature; the
  Unix domain socket path and its `/tmp` length fallback MUST likewise be
  unchanged.

### Group E — Auto-spawn and mid-session lifecycle

* **R21.** Every auto-spawn path MUST preserve read-only snapshot mode. A daemon
  auto-spawned by the shim (`spawn_daemon` in `src/shim/lifecycle.rs`) or by a
  CLI invocation MUST come up in the same mode as the pre-session daemon. The
  propagation mechanism MUST be robust to environment stripping — `spawn_daemon`
  already calls `.env_remove("ENGRAM_DATA_DIR")`, demonstrating that ambient
  environment is not a dependable carrier — therefore the persisted workspace
  configuration MUST be authoritative, with any CLI argument or environment
  override layered on top of it under a documented precedence order.
* **R22.** If the read daemon dies mid-session, the next CLI or MCP access MAY
  perform **at most one** bounded restart attempt, and that restart MUST use the
  same read-only snapshot mode.
* **R23.** After a permitted restart attempt, the access MUST terminate in one
  of exactly two outcomes: the query is served from the persisted snapshot, or
  an explicit structured availability failure is returned. Silently enabling
  watcher, scan, or sync behavior as a recovery strategy is forbidden. Falling
  back to an unbounded restart loop is forbidden.
* **R24.** The restart attempt MUST be bounded by an explicit deadline and MUST
  be observable — logged, and reflected in the structured failure payload when
  it does not succeed.

### Group F — Security boundary and snapshot semantics

* **R25.** Read-only enforcement is a **server-side security boundary**. Refusal
  MUST be enforced at the daemon-side dispatch seam so that an agent cannot
  bypass it by calling the MCP surface directly, by hand-crafting an IPC frame,
  or by invoking a CLI subcommand. Client-side filtering alone is insufficient.
* **R26.** In read-only snapshot mode the daemon MUST NOT introduce index or
  workspace write paths. Writes are limited to the existing operational surfaces
  (log files, telemetry, run-directory lifecycle files such as the PID and lock
  files). Where the storage engine supports it, the index SHOULD be opened
  without mutation intent.
* **R27.** The index is a **frozen snapshot for the duration of the agent
  session**. Source changes made during the session are intentionally not
  visible until the next explicit out-of-session refresh. This semantic MUST be
  documented in operator-facing documentation and MUST be discoverable at
  runtime through the snapshot provenance required by R11, so an agent can
  distinguish "not in the index" from "not in the repository".

## Success Criteria

1. On a cold workspace, `start.ps1` (and `start.sh`) either launch a session in
   which the agent's first `unified_search` returns real results, or refuse to
   launch with a non-zero exit and a named failure reason. There is no third
   outcome.
2. No `failure_class: readiness_timeout` degraded MCP session is produced by the
   normal launch path on a workspace whose index was successfully prewarmed.
3. With read-only snapshot mode active, an OS-level or instrumentation-level
   check confirms that no file-system watch is registered for the workspace and
   no sync or index operation is executed for the lifetime of the daemon.
4. With read-only snapshot mode active, `engram sync` and `engram index`
   (daemon-routed) and the `sync_workspace` / `index_workspace` MCP tools all
   return the same structured refusal, and the on-disk index is byte-identical
   before and after the attempt.
5. Every catalogued read tool returns successfully in read-only snapshot mode
   over both CLI and MCP, verified by an automated parity test.
6. With the mode unset, the full existing test suite passes unchanged and daemon
   startup ordering, watcher behavior, and startup sync are demonstrably
   identical to the pre-change baseline.
7. Killing the read daemon mid-session results in at most one restart attempt,
   after which the next query either succeeds from the snapshot or returns a
   structured availability failure — and in neither case is a watcher started.
8. Windows named-pipe integration tests pass unchanged, including in read-only
   snapshot mode.

## Scope Boundaries

### In scope

* A persisted, explicitly configured read-only snapshot mode for the daemon,
  covering watcher suppression, startup-sync suppression, offline-scan
  suppression, and index-mutation refusal.
* Server-side refusal of index-mutating tools with a stable structured error
  contract, enforced identically for CLI and MCP.
* Mode propagation through every auto-spawn path, with the persisted
  configuration as the authoritative carrier.
* Bounded single-attempt restart semantics for a dead read daemon, with a
  structured availability failure as the alternative outcome.
* Launcher changes: fail-closed sequencing of direct index → read daemon start →
  health check → CLI and MCP read probes → AI CLI launch; removal of the
  15-second wrapper kill and of the 30-second `ENGRAM_PREWARM_TIMEOUT_MS` clamp.
* Mode and snapshot-provenance reporting in health/status output.
* Operator documentation for the frozen-snapshot semantic and the refresh
  workflow.
* Tests: RED-first coverage for each behavior, CLI/MCP parity coverage, Windows
  named-pipe coverage, and backward-compatibility characterization of the
  default path.

### Out of scope

* Improving Engram cold-start or indexing performance. That remains spike
  `002-SP` (135 MB Cozo database, ~7.5 minutes to ready).
* Any in-session index refresh, hot reload, incremental catch-up, or
  staleness-triggered resync. The snapshot is deliberately frozen.
* Removing, rewriting, or redesigning the watcher subsystem for the default
  mode.
* Changing default daemon behavior in any way (R12).
* Multi-workspace or shared read-server topologies.
* New read capabilities or new MCP tools beyond mode/provenance reporting.
* Closing the pre-existing deferred CLI/MCP parity gaps tracked by `090.004-T`
  and `090.005-T` beyond what R18–R19 require for this feature's surface.
* Migrating the storage engine or altering the on-disk index format.
* Cross-platform sandboxing or OS-level read-only mount enforcement.

## Key Decisions

* **KD1 — Mode is explicit and persisted, not inferred.** The daemon does not
  guess that it should be read-only from the presence of an index or from
  process ancestry. It is told, through a durable configuration surface that
  survives auto-spawn. This is what makes R21 achievable given that
  `spawn_daemon` deliberately strips environment.
* **KD2 — Off by default.** Read-only snapshot mode is opt-in. This preserves
  R12 backward compatibility and mirrors the precedent set by the
  `ENGRAM_AUTO_REINDEX` opt-in gate, which was introduced for exactly this class
  of risk (a costly startup operation defaulting to on).
* **KD3 — Refusal, not degradation, for mutations.** An index-mutating call in
  read-only mode is an error with a stable classifier, not a silent success.
  Agents must be able to detect the boundary programmatically.
* **KD4 — Readiness is proven by a read, not by a health flag.** Because
  `hydration_ready` is intentionally set before heavy hydration completes, the
  session gate uses a functional read probe. This is a direct application of the
  `early-hydration-ready-before-heavy-io` learning rather than a reversal of it.
* **KD5 — Both surfaces are probed.** CLI-only or MCP-only verification would
  allow a half-degraded session to be released; the gate probes both.
* **KD6 — Bind-first startup ordering is preserved.** The
  `run_with_shutdown_v2` IPC-bind-first sequence is load-bearing (see the
  `daemon-startup-hang-watcher-blocks-before-ipc-bind` learning). Read-only mode
  suppresses the watcher *after* the bind point; it does not reorder startup.
* **KD7 — The launcher owns the wait.** Accepting a multi-minute pre-session
  index is a deliberate trade: determinism at session start is worth a longer,
  visible, out-of-session wait, and it is the operator's wait rather than the
  agent's.

## Assumptions

Per the operator's autopilot directive, the following are taken as settled
requirement-level assumptions. Each was checked against code evidence and none
is contradicted by it.

* **A1.** The index is a frozen snapshot for the session's duration; source
  changes during the session are intentionally invisible until the next explicit
  out-of-session refresh. (Requirement R27.)
* **A2.** A pre-session launcher/indexer owns all sync and index mutation.
  Agents cannot invoke index-mutating operations while read-only snapshot mode
  is active. (Requirements R1, R8.)
* **A3.** The read-serving daemon has an explicit, persisted, configured
  read-only snapshot mode that disables watcher initialization, offline scans,
  startup auto-sync, watcher event loops, and index mutation; all auto-spawn
  paths preserve the mode. (Requirements R6, R7, R21.)
* **A4.** CLI and MCP read commands target the same daemon and index and offer
  equivalent contracts. Code evidence supports this today: both paths converge
  on `tools::dispatch`. (Requirement R18.)
* **A5.** Startup fails closed. Copilot does not launch until direct indexing
  completes and the read daemon passes health plus representative
  CLI/MCP-equivalent read probes. (Requirements R4, R13–R17.)
* **A6.** Existing default daemon behavior remains backward compatible unless
  read-only snapshot mode is explicitly configured. (Requirement R12.)
* **A7.** If the read daemon dies mid-session, the next access may perform one
  bounded restart in the same mode, then either serve from the snapshot or
  return an explicit structured availability failure — never silently enabling
  watcher or sync behavior. (Requirements R22–R24.)
* **A8.** A multi-minute pre-session index wait is acceptable to the operator.
  This follows from A5: fail-closed startup on a workspace with `002-SP`-class
  cold-start cost necessarily implies a long first-run wait. Recorded explicitly
  because it is the largest user-visible consequence of this feature.
* **A9.** `flush_state` is treated as state-persisting rather than
  index-mutating and is therefore not in the R8 refusal set by default; its
  exact disposition is deferred to planning (Q3).
* **A10.** `run_retrieval_eval` is treated as read-only for R9 purposes; if
  planning finds that it writes evaluation artifacts into the index or data
  directory, it moves into the R8 refusal set (Q4).

## Outstanding Questions

### Resolve Before Planning

None. Every question that would otherwise block planning has been converted to
an explicit assumption (A1–A10) under the operator's autopilot directive, or
deferred below as safe to answer during planning.

### Deferred to Planning

* **Q1.** The exact configuration key name for the mode (for example
  `read_only_snapshot` on `PluginConfig`), and the precedence order among
  persisted config, daemon CLI flag, and environment variable.
* **Q2.** Whether R8 refusal reuses an existing `EngramError` variant and error
  code or introduces a new code in `src/errors/codes.rs` with a new
  `failure_class` string.
* **Q3.** Whether `flush_state` is permitted, refused, or made a no-op under
  read-only snapshot mode (see A9).
* **Q4.** Whether `run_retrieval_eval` and the `report`-family tools write
  anything to the data directory, and therefore whether they belong in the R9
  allowed set or the R8 refusal set (see A10).
* **Q5.** Which concrete read probes the launcher uses for R14/R15, and how a
  workspace-independent "known-present symbol" is chosen without hard-coding
  repository-specific content.
* **Q6.** The concrete default value for the raised
  `ENGRAM_PREWARM_TIMEOUT_MS` bound and the new session-gate readiness budget
  (R3, R16).
* **Q7.** Whether the read daemon is started explicitly by the launcher or left
  to shim auto-spawn on first access, given that R21 makes both paths
  mode-preserving.
* **Q8.** Whether `sync --direct` should gain an explicit "index only, do not
  leave a daemon behind" affordance, or whether the existing lock semantics are
  already sufficient for R5.
* **Q9.** Whether Cozo/SurrealKV expose an open-without-mutation-intent option
  satisfying the SHOULD in R26, or whether that reduces to a dispatch-layer
  guarantee only.
* **Q10.** How mode and snapshot provenance are shaped in the health/status
  response payloads without breaking the existing golden-record catalog and
  status oracles.

## Risks

* **RK1 — Cold-start cost becomes visible startup latency.** Fail-closed
  startup on a `002-SP`-class workspace could mean a multi-minute wait before
  the agent session opens. Mitigated by A8 (accepted), by clear launcher
  progress reporting, and by the fact that the wait is now bounded and legible
  rather than a silent race.
* **RK2 — Backward-compatibility regression.** The change touches the daemon
  startup path, which is historically fragile (two prior compound learnings sit
  on this exact code). Mitigated by R12, by characterization tests over the
  default path, and by preserving the bind-first ordering (KD6).
* **RK3 — Mode loss across the auto-spawn boundary.** If mode propagation relies
  on environment, `spawn_daemon`'s `env_remove` precedent shows how easily it is
  lost. Mitigated by KD1 and R21 making persisted configuration authoritative.
* **RK4 — Refusal surface drift between CLI and MCP.** The two surfaces share
  `tools::dispatch`, but the CLI additionally has `is_indexing_method` in
  `src/cli/runner.rs`. Enforcing refusal only at the CLI seam would create a
  bypass. Mitigated by R25 requiring server-side enforcement, and R19 requiring
  parity tests.
* **RK5 — Frozen snapshot confuses agents.** An agent may interpret "symbol not
  found" as "symbol does not exist" after editing a file mid-session. Mitigated
  by R11 and R27 exposing snapshot provenance at runtime.

## Document Review Pass

A documentation review pass was executed over this artifact per the brainstorm
Phase 5.1 requirement. Findings:

| Check | Result |
|---|---|
| Check 1 — template variable drift (P0) | Pass. No `{{VARIABLE}}` placeholders. |
| Check 2 — frontmatter validity (P1) | Pass. YAML parses; `title`, `description`, `doc_type`, `source`, `date`, `scope`, `handoff_status`, `requirement_ids` present per the brainstorm artifact contract. |
| Check 3 — heading hierarchy (MD001/MD025/MD041) (P1) | Pass with one advisory. Single H1 in the body; frontmatter followed by H1; no level skips (H1 → H2 → H3 only). `markdownlint-cli2` reports MD025 because its default `front_matter_title` pattern counts the frontmatter `title:` key as a second top-level heading. This is a repository-wide baseline condition, not a defect in this artifact: the identical finding reproduces on the existing `docs/research/2026-03-29-harness-resilience-improvements-requirements.md`, and the brainstorm skill template mandates both the frontmatter `title:` key and a body H1. Recorded as P3/advisory. |
| Check 4 — cross-reference integrity (P1/P2) | Pass. All referenced source paths (`start.ps1`, `src/daemon/ipc_server.rs`, `src/daemon/watcher.rs`, `src/daemon/mod.rs`, `src/models/config.rs`, `src/shim/lifecycle.rs`, `src/shim/tools_catalog.rs`, `src/shim/transport.rs`, `src/cli/direct.rs`, `src/cli/runner.rs`, `src/errors/mod.rs`, `src/errors/codes.rs`) and all three cited `docs/compound/` learnings were verified to exist. Backlog IDs `002-SP`, `090.004-T`, `090.005-T` verified present in the backlogit queue. |
| Check 5 — stale content (P2) | Pass. All quoted code behavior was read from the current working tree at `main`; no reference to removed files or retired commands. |
| Check 6 — frontmatter completeness (P2) | Pass for a `doc_type: spec` artifact. `argument-hint` and `max_subagent_tier` are not applicable to research documents. |

No P0, P1, or P2 findings. One P3/advisory (MD025, repository-wide baseline —
see Check 3). No follow-up items required.

## Handoff

`handoff_status: ready_for_plan`. The handoff contract is satisfied:

1. `Resolve Before Planning` is empty; every otherwise-blocking question is
   converted to an explicit assumption (A1–A10) or deferred (Q1–Q10).
2. Every requirement carries a stable `R#` ID. IDs are append-only and must
   never be renumbered.
3. Success criteria and both in- and out-of-scope boundaries are explicit.
4. A document-review pass completed with zero P0/P1/P2 findings.

Resolved handoff target: **`plan`**. The artifact path is passed to `impl-plan`
as its source document. Downstream path:

```text
this artifact
  -> impl-plan
  -> plan-harden (risk-gated)
  -> plan-review
  -> harvest into backlog feature/tasks
  -> Stage shipment assembly
```

`BRAINSTORM_HANDOFF_READY` — artifact:
`docs/research/2026-09-02-engram-read-only-snapshot-mode-requirements.md`;
unresolved blocking questions: none; deferred planning questions: Q1–Q10;
handoff target: `plan`. The `agent-intercom` capability pack is not installed in
this workspace, so this event is recorded here and in the Stage session summary
rather than broadcast.

Planning constraints that MUST be preserved downstream: TDD RED-first per P-002
and P-004; the read-only enforcement security boundary (R25); CLI/MCP parity
(R18–R19); Windows named-pipe behavior (R20); and backward compatibility of the
default daemon path (R12).

## Revision 2 — Post-Plan-Review Amendments

Date: 2026-09-02. Trigger: the `plan-review` gate returned **FAIL** on plan
attempt 1 with seven converging P0 findings across six reviewer personas. Three
of those P0s are requirement-level, not plan-level, so they are resolved here
rather than in the plan. Requirement IDs are append-only; amended requirements
keep their ID and carry an explicit `Amended in Revision 2` note.

### Amended requirements

* **R2 — AMENDED.** The original text ("the wrapper deadline MUST be derived
  from, or bounded below by, the command's own timeout") was wrong about the
  defect. `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`
  (shipment 118-S, PR #344) establishes as **Guardrail 2** that direct and
  fallback pre-warm attempts MUST share **one outer wall-clock budget**, and
  `tests/contract/start_launcher_test.rs::launcher_fails_open_to_copilot_within_one_prewarm_budget`
  asserts `elapsed < Duration::from_secs(8)` with the inline rationale that a
  per-command budget reinstates the >20 s sequential path that 118-S removed.
  The defect is therefore the budget's **size** (15 s wall clock, 30 s
  configuration ceiling), not the fact that it is shared.
  **Amended R2**: the single shared outer wall-clock budget MUST be preserved as
  an architectural invariant; the budget's magnitude MUST be raised to a
  cold-start-appropriate value; and a per-command budget derivation MUST NOT be
  introduced.
* **R8 — AMENDED.** A fixed three-name deny list (`index_workspace`,
  `sync_workspace`, `index_git_history`) is not a durable boundary: a future
  mutating tool would be permitted by default. **Amended R8**: every dispatched
  method MUST be classified against an authoritative capability table, and any
  method not positively classified as read-only MUST be refused in read-only
  snapshot mode. The boundary is **deny-by-default**, not deny-by-list.
* **R26 — AMENDED.** "No new index or workspace write paths" was too vague to
  verify by hash, because usage telemetry, logs, and run-directory lifecycle
  files are written on nearly every call. **Amended R26**: the frozen artifact
  set is defined precisely as the branch database files under
  `{data_dir}/cozo/{branch}/` and the code-graph dehydration JSONL. Metrics,
  usage telemetry, log files, and run-directory lifecycle files (PID, lock) are
  explicitly excluded from the immutability claim. Immutability is asserted by
  hashing that named set.

### New requirements

* **R28 — Explicit supersession of 118-S Guardrail 4.** Assumption A5
  (fail-closed startup) directly reverses shipment 118-S Guardrail 4, "Treat
  fail-open to Copilot as a requirement, not a best-effort fallback," which is
  encoded in two currently-passing contract tests
  (`launcher_fails_open_to_copilot_within_one_prewarm_budget` and
  `launcher_timeout_does_not_terminate_unowned_descendant`). This reversal MUST
  be recorded as an explicit, operator-approved supersession carrying the
  customer evidence that motivates it (`failure_class: readiness_timeout` on
  `0.3.0-rc.1+ge043299`), and the two existing assertions MUST be rewritten to
  fail-closed expectations in the same change that alters the launcher. Landing
  the launcher change without updating them produces red CI.
* **R29 — 118-S Guardrails 2 and 3 remain in force.** The single shared outer
  wall-clock budget (Guardrail 2) and exact-child-only process termination
  (Guardrail 3, asserted by
  `launcher_timeout_cleanup_wait_is_explicitly_bounded`, which requires the
  literal bounded `WaitForExit($cleanupTimeoutMs)` and forbids the unbounded
  form) MUST be preserved. Only Guardrail 4 is superseded.
* **R30 — Single effective-mode authority injected into shared state.** The
  effective mode MUST be resolved exactly once, at daemon startup, after
  configuration and arguments are validated, and injected as immutable runtime
  state reachable from the dispatch seam. Per-call re-resolution from the
  environment or from a configuration file read is forbidden: it duplicates
  precedence logic, creates a time-of-check/time-of-use window, and would let
  the mode drift inside a session that is contractually frozen.
* **R31 — Monotonic, fail-closed mode resolution.** Read-only snapshot mode is
  a restriction, so resolution MUST be monotonic in the restrictive direction:
  any trusted input requesting snapshot mode results in snapshot mode, and no
  channel may **loosen** it. Specifically, an environment variable or command
  argument may only **tighten** the posture; neither may disable a persisted
  snapshot-mode declaration. If the operator's intent cannot be positively
  determined — unreadable or malformed configuration, ambiguous precedence — the
  daemon MUST fail closed (refuse to start, or start in the restrictive posture)
  rather than defaulting to the mutating posture.
* **R32 — Adversary model is explicitly bounded, and `sync --direct` is the
  sanctioned refresh path.** The daemonless `engram sync --direct` path
  (`src/cli/direct.rs::run_direct_sync`) bypasses the daemon and the dispatch
  seam entirely, and R1 depends on it for out-of-session indexing. It is
  therefore **explicitly exempt** from the R8 refusal and MUST NOT be gated.
  The consequence MUST be stated rather than left implicit: the R25 boundary
  constrains an agent operating **through the Engram CLI and MCP surfaces**; it
  does not constrain an actor with arbitrary shell access and workspace write
  access, who can run `sync --direct` while the daemon lock is free or edit the
  persisted configuration. The boundary is a **correctness and reliability
  boundary against the in-session agent's own tool surface**, not a sandbox
  against a hostile local process. Documentation MUST state this scope so the
  guarantee is not oversold.
* **R33 — Refusals must be terminal and self-describing to an agent.** A
  refusal under R8 MUST carry enough structure for an agent to stop rather than
  retry: a stable non-retryable indicator, and a remediation hint naming the
  out-of-session refresh as the resolution. An agent that retries a refused
  mutation, or that falls back to raw filesystem scanning because the refusal is
  opaque, defeats the purpose of the feature.

### Revised assumptions

* **A9 — REVISED.** `flush_state` is **not** treated as merely
  state-persisting. It dispatches to the `write` module and writes code-graph
  dehydration output into the frozen artifact set defined by amended R26, so it
  is classified as **mutating and refused** in read-only snapshot mode. Q3 is
  resolved here rather than deferred.
* **A10 — REVISED.** `run_retrieval_eval` is **not** treated as read-only.
  `docs/compound/eval-recompute-must-match-index-time-persist-or-freshness-gate-2026-07-17.md`
  records that shipment 091-F added the `index_canonical_workspace_snapshot`
  Cozo relation together with guarded snapshot **writes**. It is therefore
  classified as mutating and refused. The same learning notes that under a
  frozen snapshot every edited file fails the per-file freshness gate, so
  evaluation would degrade to syntax-only counting even if permitted — a second,
  independent reason to refuse it. Q4 is resolved here rather than deferred.

### Resolved deferred questions

Q3 and Q4 are resolved above (revised A9, A10). Q8 is resolved as: no new
"index only" affordance is required on `sync --direct`; the existing
`DaemonLock` semantics plus R32's explicit exemption are sufficient, and the
launcher sequences indexing before daemon start. Q9 is resolved as: R26's
`SHOULD` for opening the index without mutation intent reduces to the
dispatch-seam guarantee plus the amended-R26 hash invariant; storage-engine open
modes are out of scope.

Remaining deferred questions: Q1, Q2, Q5, Q6, Q7, Q10.

`handoff_status` remains `ready_for_plan`.

## Revision 7 — Pre-session indexing authority

Date: 2026-09-02. Trigger: final plan validation found the original direct-sync
requirement conflicted with the separately distributed supervisor selected in
Revision 5.

### Amended requirements

* **R1 — AMENDED.** In read-server deployments, pre-session indexing MUST be
  performed by the separate `engram-indexer` supervisor before the agent
  session starts. The agent `engram` binary MUST NOT perform direct indexing in
  read-server mode.
* **R32 — AMENDED.** `engram sync --direct` and `engram index --direct` remain
  supported only in Managed mode. They are not agent read operations and MUST
  return the stable non-retryable read-server refusal when effective mode is
  `ReadServer`.

### New requirement

* **R54 — Acyclic generation layering.** Generation manifest/domain types and
  opened generation contexts MUST live below server state. Server state may
  hold generation contexts, and activation services may construct them, but
  generation services MUST NOT depend on `server::state`, and database
  infrastructure MUST NOT depend on generation-service types.

`handoff_status` remains `ready_for_plan`.

## Revision 6 — Availability-preserving activation

Date: 2026-09-02. Trigger: the final plan-review cycle identified that opening
or copying a generation on the request path would recreate the readiness and
latency failure this feature exists to prevent.

### Amended requirement

* **R45 — AMENDED.** Initial daemon startup MUST block until one valid
  generation context is open. After startup, request entry MUST first classify
  and authorize the method, perform only a bounded manifest-revision probe, and
  trigger at most one background activation for a newer revision. The request
  MUST immediately pin and use the current context. Health and non-read methods
  MUST NOT trigger activation. A failed or slow activation leaves the current
  context serving and records the rejected revision/fingerprint to prevent
  repeated work on every request.

### New requirements

* **R50 — Retired transport removal.** HTTP/SSE is no longer a supported Engram
  endpoint. The legacy HTTP MCP and SSE modules, feature flag, dependencies,
  tests, and operator documentation MUST be removed or superseded. Supported
  agent surfaces are direct daemon IPC, CLI over IPC, and stdio MCP through the
  shim.
* **R51 — Managed-mode compatibility.** Request context is mode-agnostic.
  Read-server mode constructs it from an immutable generation; managed mode
  constructs it from the existing workspace database and preserves current
  refresh semantics. Manifest reconciliation is a no-op in managed mode.
* **R52 — Bounded publication inputs.** Manifest bytes, inventory entries,
  generation bytes, runtime-copy bytes, validation time, and activation time
  MUST have explicit fail-closed limits. Exceeding a limit MUST retain the
  current context and surface a structured activation failure.
* **R53 — Separate supervisor distribution.** `engram-indexer` MUST be a
  separate workspace crate and release artifact excluded from the agent
  `engram` archive and install path. Packaging tests MUST prove the agent
  distribution contains no supervisor executable.

`handoff_status` remains `ready_for_plan`.

## Revision 5 — Durable-manifest reconciliation

Date: 2026-09-02. Trigger: the second plan review showed that a live privileged
control endpoint and hostile same-user filesystem threat model added
unnecessary implementation risk without advancing the operator's reliability
goal.

### Amended requirements

* **R10 — AMENDED.** In read-server mode, `set_workspace` is allowed only as an
  identity-equal, side-effect-free bind to the already admitted workspace. A
  request to retarget the daemon is a control operation and MUST be refused.
* **R39 — AMENDED.** Candidate build and publication belong to a separately
  distributed `engram-indexer` supervisor executable and MUST NOT appear in the
  agent `engram` CLI or MCP catalogs. No live generation-control endpoint is
  required. The threat model trusts processes running as the workspace owner;
  preventing a malicious same-user shell process from editing `.engram` is out
  of scope.
* **R43 — AMENDED.** The preferred read path opens the published database
  read-only. If Cozo cannot provide a proven read-only existing-database open,
  the daemon MUST copy the sealed generation into a generation-specific private
  runtime directory before opening it; the published generation remains
  unchanged and request pinning applies to the opened private copy.
* **R45 — AMENDED.** The read daemon MUST reconcile the durable active manifest
  at startup and synchronously at shared read dispatch before capturing the
  request context. It activates only a revision greater than the currently
  opened revision. No notification ordering or notification retry is part of
  the correctness contract.
* **R46 — AMENDED.** Generation access MUST enforce strict single-component IDs,
  canonical containment under a configured generation root, regular-file
  checks, sealed inventories, and digest revalidation. Resistance to deliberate
  same-user replacement after validation is outside the threat model; no claim
  of stable-handle or hostile-TOCTOU protection is made.

### New requirements

* **R48 — No generation control endpoint.** The read daemon MUST NOT expose a
  named or anonymous generation activation endpoint. The durable manifest is
  the sole publication authority, and request-entry reconciliation is the sole
  activation trigger.
* **R49 — Storage feasibility gate.** Before implementation of generation open
  or publication, a time-boxed spike MUST prove: Cozo read-only-open behavior or
  the private-runtime-copy fallback, Windows and Unix atomic
  replace-existing behavior, sidecar placement, and the safe-Rust dependency
  set. The plan MUST stop for revision if those experiments invalidate the
  selected design.

### Threat model

The feature protects reliability against crashes, partial builds, malformed
artifacts, stale publications, failed opens, and concurrent legitimate
publisher/read operations. It does not establish a security boundary against
arbitrary code execution under the workspace owner's OS identity. A deployment
requiring that boundary must isolate the supervisor and generation root under a
separate OS identity as a future deployment feature.

`handoff_status` remains `ready_for_plan`.

## Revision 3 — Separate indexer and live generation publication

Date: 2026-09-02. Trigger: the operator selected **Option 3 — separate indexer
process** after the initial requirements draft. This revision supersedes the
frozen-for-the-entire-session assumption while preserving read-only agent
access and watcher-independent serving.

Decision artifact:
`docs/decisions/2026-09-02-separate-indexer-read-server-deliberation.md`.

### Superseded requirement and assumption

* **R27 — SUPERSEDED.** The index is not frozen for the entire agent session.
  Instead, each published generation is immutable, and the read daemon serves
  one validated generation at a time. Source changes remain invisible until a
  separate non-agent indexer publishes a replacement generation.
* **A1 — SUPERSEDED.** Session-long snapshot freezing is replaced by immutable
  generation semantics. The agent still cannot invoke refresh operations.

The Out of scope entry that excluded every in-session refresh is also
superseded. Watcher-driven, agent-triggered, and same-database refresh remain
out of scope; supervisor-driven generation publication is in scope.

### New requirements

* **R34 — Separate indexer process.** A non-agent indexer process MAY refresh
  Engram during an agent session. It MUST run outside the read daemon and MUST
  NOT depend on recursive workspace watcher events.
* **R35 — Isolated generation build.** The indexer MUST build a candidate in a
  generation-specific directory that is not open for serving. It MUST NOT write
  to the database or dehydrated graph artifacts of the active generation.
* **R36 — Validated publication.** Before publication, the indexer MUST validate
  workspace identity, branch identity, schema version, database readability,
  and at least one representative index-derived read.
* **R37 — Atomic active-generation manifest.** Publication MUST atomically
  replace a durable manifest naming the validated active generation. A partial
  build or failed validation MUST NOT change the manifest.
* **R38 — Availability-preserving reload.** The read daemon MUST open and
  validate the published generation before swapping its active database
  handle. Existing requests MUST finish against the previous generation while
  new requests use the replacement. Reload failure MUST leave the previous
  generation serving reads.
* **R39 — Explicit supervisor control.** Generation publication and reload MUST
  be initiated by a non-agent supervisor through an internal control surface.
  The agent-facing CLI and MCP catalogs MUST remain read-only and MUST NOT
  expose generation build, publication, or reload authority.
* **R40 — Durable convergence.** The active-generation manifest is
  authoritative. If a reload notification is lost, a bounded retry or daemon
  restart MUST converge on the published generation without enabling watcher
  or automatic workspace sync behavior.
* **R41 — Bounded retention and rollback.** The active and immediately previous
  validated generations MUST be retained so publication can roll back without
  rebuilding. Cleanup of older generations MUST be explicit, bounded, and
  outside the agent read path.
* **R42 — Generation provenance.** Health and status responses MUST report the
  active generation identifier, publication timestamp, indexed source
  revision when available, and whether a newer publication is pending or
  failed.

### Revised success criteria

The following criteria supplement and, where they conflict, supersede the
original criteria:

1. A supervisor can build and publish a replacement generation while the read
   daemon continues to serve the prior generation.
2. No read request observes a partially built generation.
3. A successful reload changes both CLI and MCP reads to the same generation
   without restarting the agent session.
4. A failed build, validation, publication, or reload leaves the prior
   generation healthy and queryable.
5. Instrumentation confirms that neither recursive workspace watching nor
   same-database indexing occurs in read-server mode.

### Handoff status

`handoff_status` remains `ready_for_plan`. Requirements R34–R42 and the linked
decision artifact are authoritative for the separate-indexer architecture.

## Revision 4 — Pinned reads and supervisor authority

Date: 2026-09-02. Trigger: plan review found that path-only generation swaps
and catalog-hidden control methods do not satisfy the availability or authority
requirements.

### Amended requirements

* **R38 — AMENDED.** Validating a candidate before publication is the indexer's
  duty under R36; the read daemon MUST NOT open a candidate before it is
  published. The daemon discovers a generation only after the manifest
  publication that names it, and MUST then open and validate that published
  generation into a `GenerationReadContext` containing the database handle and
  typed provenance. That open-and-validate MUST complete successfully before the
  active request context is swapped; a failure leaves the previous context
  serving. Each request MUST capture one `Arc<GenerationReadContext>` at
  dispatch entry and use it for its entire lifetime. Publishing a path while
  handlers reopen databases independently is forbidden.
* **R39 — AMENDED.** Supervisor operations MUST use a distinct control plane
  and executable that are not part of the agent CLI or MCP catalogs. Catalog
  omission alone is not authorization. The control plane MUST require an
  ephemeral capability held by the supervisor process and unavailable through
  agent-facing process environment, command arguments, workspace files, or
  logs.
* **R41 — AMENDED.** The first release MUST retain all completed generations;
  destructive cleanup is deferred. It MUST expose generation lease/reference
  state and disk usage so a later, separately reviewed cleanup feature can
  delete only generations with no live reader, rollback, build, or publication
  lease.

### New requirements

* **R43 — Read-existing database open.** Read-server mode MUST use a database
  open path that refuses missing files or schema, performs no directory
  creation or schema bootstrap, and does not hydrate JSONL into the database.
  Engine-required transient sidecars MUST be characterized explicitly and
  excluded from immutable artifact hashes only with evidence.
* **R44 — Request-scoped generation pin.** Every database-backed read handler
  MUST consume the generation context captured once at shared dispatch. A
  single response MUST NOT combine workspace, database, registry, or provenance
  data from different generations.
* **R45 — Monotonic publication revision.** Every manifest publication MUST
  carry a monotonic revision separate from generation identity. Activation
  MUST compare the expected revision under the publication lock so delayed or
  reordered notifications cannot regress the daemon to an older publication.
* **R46 — Capability-rooted generation access.** Generation IDs MUST be strict
  single path components. Build, validation, activation, and rollback MUST use
  no-follow, capability-rooted access and verify stable object identity and
  sealed artifact digests so symlinks, junctions, reparse points, and
  validate-then-replace races cannot redirect access.
* **R47 — Shared response provenance.** Every agent-facing read response MUST
  carry compact provenance: mode, active generation ID, publication revision,
  indexed source revision, publication timestamp, and freshness/degraded
  state. Availability errors MUST additionally report stable code/classifier,
  retryability, whether the bounded restart was attempted, last known
  generation, consumed deadline, and operator remediation.

### Security scope clarification

The supervisor boundary assumes the agent process does not possess the
supervisor's inherited capability. Arbitrary code execution under the
supervisor process identity remains outside the boundary. Deployments that
need resistance to a hostile same-user agent MUST run the supervisor under a
separate OS identity and place its control endpoint and generation root under
that identity's ACL.

`handoff_status` remains `ready_for_plan`.
