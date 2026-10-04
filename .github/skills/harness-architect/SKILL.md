---
name: harness-architect
description: "Prepares the required code RED harness or a verification-gated plan for strictly docs-only tasks"
argument-hint: "feature=001-{SUFFIX_FEATURE} tasks=001.001-{SUFFIX_TASK},001.002-{SUFFIX_TASK}"
input:
  properties:
    feature:
      type: string
      description: "Feature or chore ID to scaffold harnesses for"
    tasks:
      type: string
      description: "Comma-separated task IDs to prepare or validate (optional; ship supplies its manifest-bounded queued and already-active task set)"
  required:
    - feature
---

# Harness Architect Skill

## When to Use

Use before implementation to prepare the applicable P-002/P-004 gate for
queued work, or to validate/reuse an already-active task's existing valid
gate. An already-active docs-only task may also be qualified if it is still
pre-implementation and passes the owned-path and verification-plan checks.
Code/non-prose work uses the existing RED harness; strictly qualifying docs-only
work uses the separate pre-implementation verification-plan route.

## Purpose

Prepare the P-002/P-004 gate for a feature's or chore's selected work items.
Code/non-prose tasks retain a compiling, expected-failure RED harness. Strictly
qualifying prose-only documentation tasks receive a durable
`harness-verification-gated` plan instead; they get no test, stub, or invented
RED phase.

It stops before implementation and does not implement production logic or
documentation.

## Agent-Intercom Communication

When the `agent-intercom` capability pack is installed, call `ping` at
session start. If reachable, broadcast at every step. If unreachable,
warn the operator that visibility is degraded and continue locally.

| Event | Level | Message prefix |
|---|---|---|
| Session start | info | `[HARNESS] Starting: feature={input.feature}` |
| Tasks loaded | info | `[HARNESS] Ready tasks: {task_count}` |
| Codebase analyzed | info | `[HARNESS] Context gathered: {module_count} modules` |
| Harness generated | info | `[HARNESS] Generated: {test_file} ({scenario_count} scenarios)` |
| Compilation check | info | `[HARNESS] Compilation: {result}` |
| Red phase check | info | `[HARNESS] Red phase: {result}` |
| Label applied | success | `[HARNESS] harness-ready: {task_id}` |
| Docs verification plan recorded | success | `[HARNESS] harness-verification-gated: {task_id}` |
| Complete | success | `[HARNESS] Complete: {task_count} tasks harnessed` |

## Inputs

* `${input:feature}`: (Required) Feature or chore ID such as `001-F`
* `${input:tasks}`: (Optional) Comma-separated task IDs to prepare or validate.
  When omitted, use the task set supplied by the caller under the feature. For a
  shipment caller, this is already artifact-type-filtered and manifest-bounded.
* For every task, load its durable `Owned files` declaration, status, labels,
  and existing harness/verification records. Ship must pass active manifest
  members as well as queued members; never re-claim an active task. Active code
  tasks retain the validation/reuse-only rule. An active docs-only task may be
  qualified while still pre-implementation under the same owned-path and
  durable-plan checks as queued work; its route label need not date from when
  the task first became active. Ship captures the current execution baseline at
  Step 4.1 after verifying active status.

## Output

* Code/non-prose queued task: valid `harness-ready` disposition plus the
  compiling and expected-failure RED harness manifest.
* Docs-only queued or already-active pre-implementation task: only
  `harness-verification-gated`, with a durable pre-implementation
  verification-plan reference and no code harness or RED test. Ship records
  the plan in an official task/run comment or other Ship-owned execution
  record; the architect may update only the route label after qualification.
* Already-active task: no duplicate active transition or rebuild of an
  existing valid RED harness. Active code tasks receive validation/reuse only.
  An already-active docs-only task may receive
  `harness-verification-gated` after successful pre-implementation
  qualification; the report notes that Ship captures the current execution
  baseline at Step 4.1 immediately after verifying active status, before
  telemetry, pre-build reads, task verification commands, or docs edits. A
  historical baseline from when the task first became active is not required.

## Required Protocol

### Step 1: Load the selected task set

1. Load the feature or chore and the exact caller-selected task set through
   backlog query or queue operations. Preserve shipment manifest boundaries;
   do not add unlisted descendants.
2. If `${input:tasks}` is present, restrict the scope to that explicit
   task set.
3. Exclude blocked, done, archived, or otherwise ineligible work items.
   Queued tasks may be prepared or have an invalid/missing route record
   repaired. Never transition an already-active task's status or recreate a
   valid RED harness. An already-active docs task may be qualified only while
   still pre-implementation: do not require its route label to predate active
   status; verify its owned paths are unchanged from `HEAD` in the
   current worktree, or positively attribute every existing delta to a
   recorded pre-existing baseline that predates docs implementation. An
   unexplained owned-path delta, or implementation already in progress without
   that positive attribution, fails closed; do not retroactively qualify
   partial or completed docs work. Record the verification plan before the
   first docs edit. Ship captures the execution baseline at Step 4.1
   immediately after a queued task is moved to `active` or existing active
   status is verified, before Step 4.1a telemetry, pre-build reads, task
   verification commands, or docs implementation. Step 4.2 passes that
   unchanged reference; it does not capture or replace it. The baseline is
   `HEAD` SHA plus the complete workspace changed-path set
   (tracked staged/unstaged, every untracked path individually, and deletions),
   with status and
   content hash/type for existing changed files, including symlink targets;
   record index and worktree type/hash separately when their states differ.
   Do not recursively hash clean, ignored, or build-output files. Capturing
   this execution baseline does not provide provenance for existing
   owned-path implementation deltas. After
   successful docs qualification, correct only its route label if needed;
   code-route conflicts or any other active-task ambiguity halt for
   Ship/operator disposition.
4. Preserve the work-item-to-task mapping so each harness can be traced
   back to the correct backlog item.

### Step 2: Read task intent

1. Read each selected task's title, description, acceptance criteria,
   declared `Owned files`, labels/status, and existing harness or verification
   records. Do not alter acceptance criteria or `Owned files`.
2. Pull in feature-level acceptance criteria when task text depends on
   broader feature behavior.
3. Classify the verification route before writing:
   * **Docs-only** qualifies only when every explicitly declared owned path is
     prose Markdown under `docs/**`. Inspect path, file type, and content;
     `.github`, policy/configuration, tests, schemas, scripts, templates,
     build artifacts, code/config/test paths, mixed scope, non-prose Markdown,
     missing ownership, or ambiguity disqualifies it. If ownership is
     ambiguous, halt rather than infer docs-only.
   * **Code/non-prose** is every other sufficiently understood scope and keeps
     the existing `harness-ready` compile-and-RED requirements. Mixed docs and
     code remains code/non-prose.
4. For docs-only, inspect the durable task/run verification plan in an
   official backlogit task/run comment or other Ship-owned execution record.
   It must exist before implementation, be readable, and contain at least one
   exact, non-empty executable command with working directory, exact pass
   criteria, and a result/evidence field. Record executable commands
   separately from manual gates; each manual gate names source and target
   paths, exact pass/fail criteria, and required evidence. Do not describe a
   prose cross-check as executable. If any gate is missing or ambiguous,
   qualification fails. Ship owns creating or supplementing this record;
   existing verification text may be supplemented by a comment before
   implementation without changing task planning fields, scope, acceptance
   criteria, or `Owned files`. The architect does not edit those fields and
   may update only the route label after qualification. An active task does not
   need a historical baseline from when it first became active; Ship captures
   the current execution baseline at Step 4.1 after status verification and
   before telemetry, pre-build reads, task verification commands, or docs
   implementation.
5. Translate code acceptance criteria into named test scenarios and identify
   the correct module, test tier, and affected files for each code harness.

When the `agent-engram` capability pack is installed, prefer indexed
symbol lookup and code-graph tools over broad grep when surveying
existing modules, test patterns, and import paths.

### Step 3: Determine execution posture

For each task, select the appropriate harness strategy:

| Posture | When to use | Harness pattern |
|---|---|---|
| **test-first** | New functionality with clear inputs/outputs | Write failing tests for expected behavior |
| **characterization-first** | Modifying existing behavior | Write tests that capture current behavior then modify |
| **migration-first** | Moving code between modules | Write tests at the destination, verify source behavior |
| **spike** | Exploratory with uncertain approach | Write minimal integration test, implement spike, expand tests |
| **docs-verification-gated** | Strictly prose-only `docs/**` ownership with a qualifying pre-implementation evidence plan | Record and apply the docs verification gate; no test, stub, or RED phase |

### Step 4: Prepare the route-specific gate

#### Code/non-prose route (unchanged)

1. Create test files that express the task intent as compilable tests.
2. Prefer table-driven or parameterized tests when the task describes
   multiple scenarios.
3. Create matching production stubs with unimplemented!("Worker: ...") bodies
   so the module compiles while the tests still fail for the intended
   reason.
4. Keep signatures, types, and module names aligned with the current
   codebase.

### File placement rules

Write harness files into the module that matches the work item's scope:

* **Unit harnesses**: colocated with the production code in the
  appropriate src/ subdirectory
* **Integration harnesses**: in `tests/integration/` when the
  task spans modules or runtime boundaries
* **Contract harnesses**: in `tests/contract/` when the task
  defines API, CLI, or schema behavior

Write companion stub files into the production module that the tests
exercise. Do not place scaffolding in unrelated modules.

#### Docs-only route

1. Before any docs implementation, ensure Ship has recorded a durable,
   readable verification plan in an official backlogit task/run comment or
   other Ship-owned execution record. If absent or incomplete, return the
   required plan details to Ship to record or supplement before implementation;
   do not write task planning fields, scope, acceptance criteria, or `Owned
   files`. Existing verification text may be supplemented by a comment before
   implementation without changing those fields. Retain the stable reference
   and preparation order.
2. For every executable check, record the exact command as a literal command
   (not explanatory prose), working directory, exact success/pass criteria,
   and where stdout/stderr, exit status, and other required evidence will be
   retained in that Ship-owned record. Keep executable commands separate from
   manual gates. Require at least one non-empty, unambiguous executable
   command; do not invent a weak token-presence or unrelated command merely to
   qualify.
3. For each manual gate, record source path(s), target path(s), exact
   pass/fail criteria, and required evidence. Keep it explicitly labeled
   **manual** and separate from executable commands.
4. Validate that the complete declared ownership is only prose Markdown under
   `docs/**`, and that the Ship-owned verification record is durable/readable.
   If any criterion is absent or ambiguous, halt without a docs label.
5. Apply only `harness-verification-gated` after qualification, for queued or
   already-active pre-implementation work. Never apply `harness-ready`; do not
   create a test, production stub, or RED test for this route.

### Step 5: Verify harness

#### Step 5.1: Compilation check

Code route only: run `cargo check --all-targets`. The harness MUST compile.

If compilation fails, fix the harness until it compiles. Do not proceed
with a non-compiling harness.

#### Step 5.2: Red phase check

Code route only: run `cargo dev-test` for the harness tests. ALL tests MUST fail with
the expected failure marker (unimplemented!("Worker: ...")).

If any test passes (false positive) or fails with an unexpected error
(compilation vs runtime), fix the harness.

Docs route: no compilation or RED phase is fabricated. Reconfirm that the
pre-implementation record is durable/readable and all required command/manual
gate fields qualify before applying the docs label.

### Step 6: Apply the matching disposition

For a queued code task, after both code checks pass (P-004 gate satisfied):

1. Leave exactly the `harness-ready` route label using the backlog tool's
   update operation; ensure `harness-verification-gated` is absent.
2. Add an implementation note with the harness command.
3. Record the harness manifest: `Compilation: PASS`,
   `Red Phase: CONFIRMED`.

For a queued docs-only task, after the pre-implementation plan has been
recorded and validated:

1. Apply only `harness-verification-gated`; ensure `harness-ready` is absent.
2. Return the Ship-owned verification-plan reference, preparation time/order,
   exact command(s), manual gate(s), and required evidence locations. Do not
   modify task planning fields to store the plan.
3. Do not implement the docs or report command/manual-gate execution as passed
   here; build-feature executes them after implementation.

For an already-active docs-only task that passes the same pre-implementation
qualification, set only `harness-verification-gated` (correcting a conflicting
route label if needed) and return the Ship-owned plan reference. Do not change
task status or any other task field. A historical baseline from when the task
first became active is not required; Ship captures the current execution
baseline at Step 4.1 immediately after verifying active status, before
telemetry, pre-build reads, task verification commands, or docs implementation.
If implementation may already have started and cannot be positively attributed
to a recorded pre-existing baseline, halt without retroactively qualifying it.
Already-active code tasks remain validation/reuse-only.

## Quality Criteria

The code route is complete only when selected queued tasks have:

* harness files in the correct modules
* structural stubs with intentional not-implemented behavior
* a successful `cargo check --all-targets` result after scaffolding
* all harness tests failing with the expected marker
* clear mapping from backlog task to harness command

The docs-only route is complete only when selected queued tasks and any
already-active tasks qualified before implementation have:

* explicit, exclusively prose-Markdown `docs/**` ownership validated
* owned paths verified unchanged from `HEAD` in the current worktree, or
  existing deltas positively attributed to a recorded pre-existing baseline
* a durable, readable Ship-owned verification plan recorded before docs
  implementation, with executable commands and manual gates separate, at
  least one exact non-empty executable command, and auditable manual gates
  where applicable
* only `harness-verification-gated` applied, never `harness-ready`
* no code harness, test, stub, or RED test created
* the stable verification-plan reference and preparation order reported to
  Ship; the execution baseline is captured at Step 4.1 immediately after the
  queued task is moved to `active` or its existing active status is verified,
  before telemetry, pre-build reads, task verification commands, or docs edits;
  the Step 4.2 handoff passes the unchanged reference

Already-active code tasks are complete for this skill only when their matching
disposition and supporting records were already valid; reuse them without
re-claiming or rebuilding. Already-active docs tasks may be qualified only
while pre-implementation under the checks above.

## Guardrails

* Do not implement production logic — code route stubs only.
* Do not skip code-route compilation or RED verification.
* Do not apply `harness-ready` until both code checks pass; docs-only tasks
  receive only `harness-verification-gated` after qualification and never get
  an invented RED test.
* Do not qualify ambiguous/mixed ownership as docs-only.
* Keep code test scenarios traceable to acceptance criteria and docs gates
  traceable to their source/target, exact criteria, and evidence.
* Do not edit task planning fields, scope, acceptance criteria, or file
  ownership to record a docs plan; Ship stores the full plan and evidence in
  an official task/run comment or other Ship-owned execution record. The
  architect may update only the route label after qualification.

## Model Routing

This skill operates at **Tier 2 (Standard)** — test scaffolding is structured but routine.
