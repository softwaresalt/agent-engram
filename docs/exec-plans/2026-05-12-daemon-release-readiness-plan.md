---
title: "Daemon and CLI release-readiness hardening"
type: impl-plan
date: 2026-05-12
status: draft
source_documents:
  - docs/decisions/2026-05-12-daemon-release-readiness-deliberation.md
---

# Daemon and CLI release-readiness hardening

## Problem Frame

The operator states that `engram` cannot be released in its current state
because daemon and CLI behavior still fail in real workspace workflows. Recent
reliability fixes improved startup and indexing behavior, but release confidence
is still too low because the workflow-level validation surface does not yet
prove:

* stale lock/PID recovery and daemon restart behavior
* realistic workspace lifecycle behavior across bind, readiness, indexing,
  search/query, shutdown, restart, and recovery
* real command/subcommand flows from the CLI, not just isolated contract or
  helper-level assertions
* explicit release smoke gates and readiness signals for daemon + CLI behavior

This plan turns that concern into small, execution-ready units focused on test,
config, and documentation surfaces. It deliberately excludes direct source-code
feature changes and blocked upstream dependency work under `041-F`.

Execution owner for all implementation units: **Ship**. Stage owns planning,
review, and backlog decomposition only.

## Requirements Trace

| Requirement | Implementation |
|---|---|
| Capture the real-world daemon/CLI fragility problem statement | Root chore description + Unit 7 release artifact |
| Cover daemon lifecycle robustness and stale lock/PID recovery | Unit 1 |
| Cover realistic workspace lifecycle flows | Unit 2 |
| Cover CLI commands/subcommands from real developer usage | Units 3 and 4 |
| Add regression coverage for observed workspace failures | Unit 5 |
| Add release validation and readiness gates | Units 6 and 7 |
| Keep work release-oriented, not unit-only | All test units use workflow-level posture and real workspace fixtures |

## Implementation Units

### Unit 1: Add stale PID and dead-daemon recovery coverage

**Scope**: Extend integration coverage around stale PID recovery and daemon
startup after abnormal prior runtime state. Keep this unit bounded to exactly
two scenarios:

* stale PID is replaced during next daemon start
* dead-daemon runtime state no longer blocks clean startup

**Files affected**:

* `tests/integration/stale_pid_recovery_test.rs`
* `tests/integration/daemon_lifecycle_test.rs`
* `tests/helpers/mod.rs`

**Changes**:

* add workflow tests that begin from stale or inconsistent daemon runtime state
* keep assertions centered on externally visible recovery behavior

**Acceptance criteria**:

* stale PID recovery reaches Ready within the existing harness timeout
* dead-daemon runtime state does not require manual workspace cleanup before
  restart
* each scenario is expressed as a failing test before implementation changes

**Execution posture**: test-first

### Unit 2: Add restart-safe workspace lifecycle workflow coverage

**Scope**: Cover the operator-facing lifecycle chain in one realistic workspace:
bind, daemon start/readiness, indexing, one representative symbol/search/query
flow, shutdown, restart, and recovery. Keep this unit bounded to exactly two
workflow paths:

* bind → ready → indexing → representative query
* shutdown → restart → representative query

**Files affected**:

* `tests/integration/workspace_lifecycle_workflow_test.rs`
* `tests/helpers/mod.rs`

**Changes**:

* create or extend one workflow test that exercises the bounded lifecycle paths
  in the same workspace fixture
* validate readiness transitions and successful query use after restart

**Acceptance criteria**:

* bind → ready → indexing → representative query succeeds without manual cleanup
* shutdown → restart → representative query succeeds in the same workspace
* intermediate readiness transitions are asserted explicitly in the test

**Execution posture**: test-first

### Unit 3: Add core lifecycle CLI workflow coverage

**Scope**: Exercise CLI commands and subcommands as a real developer would use
them for the core lifecycle path. Keep this unit bounded to exactly four command
families:

* `bind`
* `daemon-status`
* `workspace-status`
* `flush`

**Files affected**:

* `tests/integration/cli_e2e_test.rs`
* `tests/integration/cli_command_matrix_test.rs`

**Changes**:

* define one bounded workflow-oriented CLI sequence for core lifecycle commands
* assert command exit behavior and state transitions in realistic order

**Acceptance criteria**:

* `bind`, `daemon-status`, `workspace-status`, and `flush` all exit successfully
  in the bounded workflow
* command output reflects the expected daemon/workspace state after each step
* each scenario is written as a failing test before implementation changes

**Execution posture**: test-first

### Unit 4: Add indexed workflow CLI coverage

**Scope**: Cover the representative indexed workflow commands a developer uses
after lifecycle setup. Keep this unit bounded to:

* one indexing command path (`sync` or `index`)
* one representative search or query path
* one failure-message assertion when daemon/workspace state blocks normal use

**Files affected**:

* `tests/integration/cli_command_matrix_test.rs`
* `tests/integration/cli_e2e_test.rs`

**Changes**:

* extend the CLI matrix to include one representative indexed workflow path
* keep the query surface intentionally narrow so the unit remains backlog-sized

**Acceptance criteria**:

* the chosen indexing command completes in the real workflow fixture
* one representative search/query command succeeds after indexing
* one blocked-state failure path returns actionable messaging

**Execution posture**: test-first

### Unit 5: Add named regression coverage for the top observed failures

**Scope**: Convert the top three failures already observed in this workspace
into named release-regression scenarios:

1. watcher/startup ordering regression
2. inherited `ENGRAM_DATA_DIR` environment contamination
3. lock contention or stale-state recovery regression

Do not expand beyond these three scenarios in this unit.

Priority sources for these three scenarios:

* `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`
* `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`
* `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`

**Files affected**:

* `tests/integration/release_regression_workflow_test.rs`
* `tests/helpers/mod.rs`

**Changes**:

* encode the three named regressions as repeatable workflow scenarios
* verify environment isolation explicitly rather than assuming helper behavior

**Acceptance criteria**:

* a named regression test exists for each of the three prioritized failures
* the environment-isolation test proves daemon subprocesses ignore ambient
  `ENGRAM_DATA_DIR`
* lock/stale-state regression fails safely and recovers predictably

**Execution posture**: test-first

### Unit 6: Add release smoke validation gate for daemon and CLI workflows

**Scope**: Create a focused release-smoke entry point that proves the daemon and
CLI workflow surface is release-ready before a cut. Keep the smoke gate fixed to
exactly four scenarios:

1. daemon reaches Ready
2. core lifecycle command sequence succeeds
3. representative indexed query flow succeeds
4. stale-state recovery scenario succeeds

**Files affected**:

* `tests/integration/release_smoke_daemon_cli_test.rs`
* `Cargo.toml`

**Changes**:

* define the fixed four-scenario smoke suite
* expose the suite through a repeatable entry point based on
  `cargo test --test release_smoke_daemon_cli_test`

**Acceptance criteria**:

* the smoke suite contains only the four fixed scenarios above
* failure in any smoke scenario is release-blocking
* the smoke entry point can be invoked repeatably by Ship during release work
  via `cargo test --test release_smoke_daemon_cli_test`

**Execution posture**: test-first

### Unit 7: Add release-readiness checklist and rollback signals

**Scope**: Capture the non-code release gate: what must be observed, who owns
the signal, and what rollback trigger blocks release.

**Files affected**:

* `docs/closure/2026-05-12-daemon-release-readiness-checklist.md`

**Changes**:

* define the release checklist tied to Units 1-6
* record monitoring signals, rollback triggers, and manual validation window
  expected before release

**Acceptance criteria**:

* checklist references the smoke gate and the bounded workflow suites created
  above
* rollback triggers, owner, and validation window are explicit
* the checklist records named SLIs, baseline expectations, and alert thresholds
  for the four smoke scenarios
* the artifact names which failures are release-blocking versus advisory

**Execution posture**: docs-first

## Dependency Graph

```text
Unit 1 (stale PID / dead-daemon recovery)
  ├──→ Unit 2 (restart-safe workspace lifecycle)
  ├──→ Unit 5 (named regressions)
  └──→ Unit 6 (release smoke gate)

Unit 2 (restart-safe workspace lifecycle)
  ├──→ Unit 4 (indexed workflow CLI coverage)
  ├──→ Unit 5 (named regressions)
  └──→ Unit 6 (release smoke gate)

Unit 3 (core lifecycle CLI coverage)
  └──→ Unit 6 (release smoke gate)

Unit 4 (indexed workflow CLI coverage)
  └──→ Unit 6 (release smoke gate)

Unit 5 (named regressions)
  └──→ Unit 6 (release smoke gate)

Unit 6 (release smoke gate)
  └──→ Unit 7 (release checklist and rollback signals)
```

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Frame the root item as a chore | This is release hardening and validation work, not a net-new end-user capability |
| Separate workflow coverage from regression coverage | Keeps tasks backlog-sized and makes observed failures explicitly traceable |
| Split CLI work into core lifecycle coverage and indexed workflow coverage | Prevents the CLI matrix from becoming too large for a single task |
| Create an explicit smoke gate instead of relying on scattered tests | Release readiness needs one named handoff target |
| Keep blocked CozoDB work out of this plan | `041-F` remains upstream-blocked and should not contaminate release-hardening scope |
| Add a docs/checklist unit after the smoke gate | Release-observability requirements need an explicit artifact, not only passing tests |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Test scope expands into a new daemon reliability program | Keep every unit tied to release validation, observed failures, or smoke gating |
| Cross-platform daemon edge cases exceed the 2-hour rule | Split platform-specific follow-ups instead of widening Units 1-6 |
| Smoke gate becomes too broad and flaky | Keep it narrow, deterministic, and derived from Units 1-5 rather than re-testing everything |
| Existing helpers may need minor extension across multiple suites | Keep helper edits small and shared; avoid large helper rewrites |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | No | Planned work is centered on tests, config, and release artifacts |
| Security, auth, permission, or compliance-sensitive behavior | No | No auth or sensitive-data scope |
| Migration, backfill, destructive data/config action, or irreversible step | No | No destructive migration or data rewrite |
| External integration, operator checkpoint, or external dependency | Yes | Release go/no-go requires explicit operator checkpoint and documented readiness gate |
| High runtime, rollout, or rollback risk | Yes | The work defines the release-blocking proof for daemon and CLI runtime behavior |

**Requires plan hardening: yes**

## Runtime Verification and Closure

### Unit 1

* **Runtime surface**: daemon startup and stale-state recovery
* **Verification**: reproduce the two bounded stale-state scenarios and confirm
  clean recovery
* **Closure**: record which stale-state cases are release-blocking

### Unit 2

* **Runtime surface**: workspace bind, readiness, indexing, representative query
* **Verification**: full bounded workflow test in a real workspace fixture
* **Closure**: record whether workflow remains usable after restart/recovery

### Unit 3

* **Runtime surface**: core lifecycle CLI commands used in sequence
* **Verification**: bounded command matrix for `bind`, status, and `flush`
* **Closure**: record which lifecycle commands are covered by release smoke

### Unit 4

* **Runtime surface**: indexed CLI workflow after lifecycle setup
* **Verification**: one indexing path, one representative query path, and one
  blocked-state failure path
* **Closure**: record the indexed command path covered by release smoke

### Unit 5

* **Runtime surface**: previously observed daemon/CLI failure paths
* **Verification**: three named regression scenarios mapped to prior incidents
* **Closure**: list the incident classes now protected

### Unit 6

* **Runtime surface**: pre-release validation gate
* **Verification**: smoke suite fails closed on daemon/CLI workflow regressions
* **Closure**: gate becomes the explicit pre-release check

### Unit 7

* **Runtime surface**: operator release decision process
* **Verification**: checklist names signals, owner, observation window, and
  rollback triggers
* **Closure**: final release artifact is explicit and reusable

## Constitution Check

| Principle | Status |
|---|---|
| I. Safety-First Rust | Yes — implementation work stays in Rust test/config/doc surfaces |
| II. Test-First | Yes — Units 1-6 are explicitly test-first and Unit 7 is release documentation |
| III. Workspace Isolation | Yes — all planned work remains inside repo-managed fixtures |
| IV. CLI Containment | Yes — no out-of-workspace actions required |
| V. Structured Observability | Yes — release signals and closure artifacts are explicit deliverables |
| VI. Single Responsibility | Yes — no new dependency is assumed in the plan |
| VII. Destructive Approval | N/A — no destructive action planned |
| VIII. Safety Modes | Yes — release gating is handled as elevated-risk planning |
| IX. Git-Friendly Persistence | Yes — artifacts live in tracked tests/config/docs |
| X. Context Efficiency | Yes — reuse prior learnings instead of rediscovering failures |
| XI. Merge Commit History Preservation | Yes — standard repository merge policy remains unchanged |

## Plan Hardening

**Hardening required**: Yes. This work defines the evidence used to decide
whether `engram` is releasable. The blast radius is operational rather than
architectural: if the validation surface is incomplete, a bad release can still
ship even when individual tests are green.

### Reinforcing context consulted

* `docs/compound/bugs/daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`
* `docs/compound/test-failures/engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md`
* `docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md`
* `docs/closure/2026-05-09-034-S-daemon-startup-reliability-closure.md`
* `.github/instructions/strict-safety.instructions.md`
* `.github/instructions/release-observability.instructions.md`

### Protected invariants

* release validation must exercise real daemon + CLI workflows, not only unit
  helpers
* stale lock/PID and restart behavior must be covered before release
* release smoke checks must fail closed when core daemon/CLI workflows regress
* readiness artifacts must name owner, observation window, and rollback triggers

### ProposedAction records

**ProposedAction**
* summary: Add or extend workflow-level integration tests that simulate daemon
  stale-state, restart, indexing, and CLI command chains
* targets: `tests/integration/*`, `tests/helpers/mod.rs`
* change_kind: local edit
* rollback: revert the new suites or narrow failing scenarios without touching
  production runtime behavior
* approval_required: no

**ActionRisk**: moderate

**ActionResult**: planned

**ProposedAction**
* summary: Introduce a release smoke gate entry point for daemon and CLI
  workflow validation
* targets: `tests/integration/release_smoke_daemon_cli_test.rs`, `Cargo.toml`
* change_kind: config change
* rollback: remove the gate wiring while preserving underlying tests if it
  proves too broad or flaky
* approval_required: no

**ActionRisk**: moderate

**ActionResult**: planned

**ProposedAction**
* summary: Publish a release-readiness checklist with explicit monitoring and
  rollback signals
* targets: `docs/closure/2026-05-12-daemon-release-readiness-checklist.md`
* change_kind: local edit
* rollback: revise the checklist without affecting runtime behavior
* approval_required: no

**ActionRisk**: low

**ActionResult**: planned

### Added verification, monitoring, and rollback detail

* **Release-blocking signals**
  * daemon reaches Ready during the validated workflow path
  * the fixed four-scenario smoke gate passes
  * stale-state recovery scenarios succeed without manual workspace cleanup
* **Observation window**
  * pre-release validation window owned by the operator during release prep
* **Rollback triggers**
  * smoke gate fails on daemon readiness or recovery scenario
  * CLI workflow coverage fails on a core lifecycle or indexed command sequence
  * regression suite reproduces an already-known workspace failure
* **Fallback path**
  * if a platform-specific case exceeds the 2-hour rule, split it into a
    follow-up backlog item while keeping the main release gate focused on the
    cross-platform core workflow

## Plan Review

**Reviewed**: 2026-05-12
**Gate decision**: **PASS**
**Plan hardening required**: Yes — satisfied by the `## Plan Hardening` section.

### Gate rationale

The revised plan is harvest-ready. Implementation units are now explicitly
bounded, test-first, dependency-aware, and aligned to the operator's
release-hardening goal without reopening blocked upstream work.

### Findings

#### P3 Advisory — Shared helper edit coordination

Units 1, 2, and 5 may all touch `tests/helpers/mod.rs`. Ship should serialize
those helper edits or route them through one owner to avoid unnecessary merge
conflicts during execution.

### Persona summary

* **Constitution review**: PASS after converting all runtime-facing units to
  explicit test-first posture and bounding them to 2-hour-sized scope
* **Scope boundary audit**: PASS after splitting CLI coverage into core and
  indexed workflow units and fixing the smoke-gate boundary
* **Learnings cross-check**: PASS after naming the top three regression sources
  and adding explicit release-observability expectations to Unit 7
