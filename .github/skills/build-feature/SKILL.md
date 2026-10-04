---
description: "Execute the code harness loop or the strict recorded-verification route for qualifying docs-only tasks"
---

# Build Feature

Implement a code task with the existing strict RED-harness loop, or execute the recorded verification gates for a strictly docs-only task. The docs route does not use the code harness loop.

## When to Use

Invoked by the ship agent only after validating the matching route: `harness-ready` for code/non-prose work or `harness-verification-gated` for strictly qualifying docs-only work. Not invoked directly by users.

## Inputs

* `task_id`: (Required) The backlog task ID to implement.
* `route`: (Required) Exactly `harness-ready` (code/non-prose) or `harness-verification-gated` (docs-only).
* Code route only: `harness_cmd` from the task's harness metadata (e.g., `cargo dev-test`).
* Docs route only: a durable Ship-owned verification-record reference created before docs implementation, plus the unchanged execution-baseline reference captured and persisted by Ship at Step 4.1 immediately after the queued task is moved to `active` or existing active status is verified. Capture it before Step 4.1a telemetry, pre-build reads, task verification commands, or docs implementation. Step 4.2 passes this reference unchanged; build-feature must not capture, refresh, or replace it. The baseline contains `HEAD` SHA and the complete workspace changed-path snapshot; it is not a recursive inventory of clean files. The docs route must not receive a substitute `harness_cmd`.

## Output

* Code route: all tests in the current task's targeted harness pass. Report full-suite `PASS` only for an unequivocally green run; an eligible interim non-green result is `EXPECTED_PENDING_RED` and remains subject to Ship's independent Step 4.3 verification. Preserve the existing code implementation and task-completion behavior.
* Docs route: every recorded executable command and auditable manual gate passed; all command/result/evidence records are durable; the baseline comparison is clear; no code harness loop was run. Return the result to Ship; do not independently mark the task `done`.

## Required Protocol

When the `agent-intercom` capability pack is installed, follow
`.github/instructions/agent-intercom.instructions.md` throughout the loop: establish heartbeat /
ping visibility up front, broadcast meaningful attempt transitions, and route any destructive
actions through the intercom approval path rather than improvising local-only approval.

When the `agent-engram` capability pack is installed, follow
`.github/instructions/agent-engram.instructions.md` throughout the loop: prefer indexed symbol and
impact lookup while diagnosing failures, verify the workspace is bound before trusting engram
results, and refresh stale indexes before concluding the code graph is wrong.

### Route Guard (before any implementation)

1. Re-read the task's declared `Owned files`, route label, and relevant durable
   task/run records. Apply P-002/P-004 strict qualification: docs-only requires
   every owned path to be exclusively prose Markdown under `docs/**`; any
   mixed, missing, ambiguous, code/config/test, `.github`, policy/config,
   schema, script, template, or build-artifact scope is not docs-only.
   Require exactly the matching label (`harness-ready` for code/non-prose or
   `harness-verification-gated` for docs-only), never both or a substitute.
2. For `harness-ready`, preserve the existing code path below. Do not convert
   code/test/config or other non-prose work to the docs route.
3. For `harness-verification-gated`, require a readable verification plan in
   an official backlogit task/run comment or other Ship-owned execution record
   that predates docs implementation. Record executable commands and manual
   gates separately. Require at least one exact, non-empty executable command
   with working directory, exact pass criteria, and result/evidence field.
   Each manual gate names source/target paths, exact pass/fail criteria, and
   required evidence. Prose cross-checks are not commands. Missing, empty,
   ambiguous, or unreadable records fail closed before implementation. If
   existing verification text needs supplementation, Ship may add a comment
   before implementation without changing task planning fields, scope,
   acceptance criteria, or `Owned files`; the architect may update only the
   route label after qualification.
4. Before docs implementation, verify each owned path against `HEAD` and the
   current worktree to establish that no unrecorded implementation delta
   exists. Any existing owned-path delta must be positively attributed to a
   durable pre-existing baseline that predates implementation; otherwise fail
   closed and do not retroactively qualify partial or completed docs work.
   Require a readable execution-baseline reference captured and persisted by
   Ship at Step 4.1, immediately after a queued task is moved to `active` or an
   existing active status is verified. This is the current task's execution
   baseline, not a required historical baseline from when an active task first
   became active. Confirm that the supplied reference is unchanged from the
   Step 4.1 capture; build-feature MUST NOT capture, refresh, or replace it.
   Do not edit docs or run recorded verification commands until this baseline
   reference is validated. Ship must capture it before Step 4.1a telemetry,
   pre-build reads, task verification commands, or docs implementation.
   Record `HEAD` SHA and the complete workspace changed-path set: tracked
   staged/unstaged changes, every untracked path individually, and deletions,
   with status and content hash/type for existing changed files, including
   symlink targets; record index and worktree type/hash separately when their
   states differ. Retain the last available HEAD/index type and hash for
   deleted paths. A status listing or
   `git diff` alone is insufficient. Do not recursively hash clean, ignored,
   or build-output files outside that changed-path set. Missing or unreadable
   baseline blocks implementation/completion. Unknown `HEAD` history blocks.

### Docs-only Verification Route (no code harness loop)

For `harness-verification-gated` only:

1. Before editing, load the complete Ship-owned verification record and
   confirm it predates docs implementation. Require at least one exact,
   non-empty executable command, its working directory, exact pass criteria,
   and result/evidence field. Keep manual gates separate; each must specify
   source and target paths, exact pass/fail criteria, and required evidence.
   If any part is missing, ambiguous, or unreadable, stop as `BLOCKED` before
   editing; return requested plan additions to Ship rather than changing task
   planning fields.
2. Implement only the task's declared prose-Markdown files under `docs/**`.
   Do not create code/test stubs, tests, or a RED harness; do not run the code
   harness loop. Do not alter task acceptance criteria or `Owned files`.
3. Execute every recorded command exactly as written, from its recorded
   working directory, and evaluate it against its exact criteria. Run every
   recorded command even if an earlier command fails, unless execution itself
   is unsafe or impossible; in that case record the reason and block. Preserve
   the literal command, working directory, start/end, exit status, result,
   stdout/stderr or evidence reference, and pass/fail evaluation in the
   durable run/task record. Do not replace or reinterpret a failed command.
4. Perform every auditable manual gate against its named source/target paths.
   Record the observed facts, exact criteria evaluation, and each required
   evidence item in the durable run/task record. A prose cross-check is never
   reported as an executable command. Missing evidence, unreadable inputs, or
   a criteria mismatch is a failure.
5. Before reporting completion, compare current `HEAD` and the complete
   changed-path set—including status and the content hash/type of existing
   changed files, with symlink targets and separate index/worktree type/hash
   where their states differ—against the unchanged execution baseline. Do not
   recursively hash clean, ignored, or build-output files
   outside that set. Classify every added, modified, renamed/type-changed, or
   deleted delta. Keep unchanged pre-existing dirty entries as baseline state,
   not task-attributed deltas. Account for every changed or removed baseline
   entry.
   All task-attributed implementation changes must be within the declared
   `Owned files` and remain prose Markdown under `docs/**`. Any
   task-attributed non-prose delta blocks completion. A docs change outside
   the declared `Owned files`, an unexplained delta, or unknown provenance
   also fails closed; only positively evidenced unrelated changes may be
   classified as not task-attributed, and each classification must be
   recorded. Never blanket-ignore paths. Require `HEAD` to remain the
   baseline SHA until Ship commits the verified docs changes; for an already-
   active task, any intervening `HEAD` change must be positively attributed
   to manifest-bounded prior task commits, never presumed safe.
6. If any command fails, any manual criterion mismatches, evidence is missing,
   or the baseline comparison fails/is unreadable, preserve all results and
   return `BLOCKED` to Ship. Never report success or permit task completion.
   A later authorized repair must preserve the failure history and rerun every
   recorded command and manual gate plus the complete changed-path-set
   comparison before success.

The docs route returns its verification record and baseline-comparison
references to Ship; it does not commit changes or move the task to `done`.

### Code Harness Loop (5-Attempt Circuit Breaker; unchanged)

Code route only. **Before entering the loop**: Read coding standards once — constitution Principle I
and `rust.instructions.md`. These apply to all fix attempts.
Do not re-read the full standards on every iteration; only do a targeted re-read
if working on a file in an unfamiliar module or if the error pattern changes.

This loop is a skill-managed exception to the universal 3-retry circuit breaker
(per `circuit-breaker.instructions.md`). The 5-attempt limit governs within this
loop scope. However, if the **same error** recurs on attempts 3+, the universal
circuit breaker applies: stop and escalate.

```text
Attempt 1..5:
  1. Run harness_cmd → capture stdout/stderr
  2. If all tests pass → SUCCESS → exit loop
  3. Parse failure output → identify failing tests and error messages
  4. If error is substantially identical to previous attempt → check same-error recurrence limit
  5. Fix the code to address the specific failure
  6. Verify compilation: cargo check
  7. If compilation fails → fix compilation errors first
  8. Loop back to step 1

After 5 failures → mark task as BLOCKED → exit
```

The following loop and fix steps apply only to `harness-ready` code tasks.
`harness-verification-gated` docs tasks must use the route above and must not
enter this loop.

### Step-by-Step Detail

#### Step 1: Run the Harness

Execute `harness_cmd` and capture the full output. Record execution time.

**Stall timeouts**:

* Build/test commands: 45 minutes
* Other commands: 5 minutes

If the command exceeds the timeout, terminate and count it as a failed attempt.

#### Step 2: Evaluate Results

If all tests pass, proceed to quality gates.

#### Step 3: Parse Failures

Extract from the test output:

* Which tests failed
* The assertion or error message
* The file and line where the failure occurred
* The expected vs. actual values (if applicable)

When the `agent-engram` capability pack is installed, use engram-first lookup to inspect symbols,
callers, and affected regions before expanding into broader file-based searches.

#### Step 4: Re-read Standards

Before writing any fix, re-read the relevant coding standards:

* Constitution Principle I (safety-first language practices)
* Technology-specific instructions (`rust.instructions.md`)
* Any instruction files matching the files being modified

#### Step 5: Fix the Code

Apply targeted fixes to address the specific test failure. Do NOT:

* Modify the test to make it pass (tests are the specification)
* Add unrelated changes
* Refactor code not related to the failure
* Skip error handling to shortcut a fix

If the root cause is still unclear after repeated attempts, or the task touches a risky subsystem, invoke **safety-modes** in `investigate-first` or `freeze-scope` mode before continuing.

#### Step 6: Verify Compilation

Run `cargo check` to confirm the fix compiles. If compilation fails, fix compilation errors before returning to the harness loop.

### Post-Loop Quality Gates (code route only)

After the current task's targeted harness passes, run every gate after **each** code task:

1. **Lint**: `cargo clippy -- -D warnings -D clippy::pedantic`
2. **Format**: `cargo fmt --all -- --check`
   * If violations found: `cargo fmt --all` and re-check
3. **Full test suite**: `cargo dev-test --no-fail-fast` — run every configured test target even after an earlier target fails; retain the complete command output. Plain `cargo dev-test` may stop after the first failing target and is diagnostic only, not a complete per-task gate. Do not use command-line filters, omit targets, or newly apply ignore/disable/suppression. Report pre-existing ignored tests as reported by the harness, and require each mapped pending-red test to have actually executed and failed with its recorded marker.

The full-suite result is exactly one of:

* `PASS`: the complete command succeeds with no failing tests, compile errors, or warnings.
* `EXPECTED_PENDING_RED`: a non-green interim result for a non-final task only. This verdict is eligible only when the targeted harness, compilation, lint, and format pass; `cargo dev-test --no-fail-fast` executed every configured target and produced no compile errors, warnings, runner errors, or failures other than exact test-name/expected-marker matches to previously recorded compiling RED harnesses of still-unstarted later tasks in the shipment's dependency-resolved order; and the recorded Owned-file baseline (including each RED test file's content/type) is unchanged. Record the exact failures, expected markers, task IDs, baseline evidence, and planned later-task resolution. Shipment-wide `active` status alone does not mean implementation started. Ship independently verifies every condition in its Step 4.3 and is the only authority that accepts this verdict for task completion.
* `BLOCKED`: any other nonzero result or any missing, ambiguous, changed, or unreadable evidence.

Never call `EXPECTED_PENDING_RED` a pass or report the full suite as green. Do not fix, edit, disable, or suppress a different task's pending RED harness, or start that task ahead of dependency order. Return the complete failure evidence to Ship for P-021 scope classification; do not make an out-of-scope fix in this task. The final code task and final task/PR readiness run require `PASS`; no pending-red allowance applies there.

### Commit (code route only)

If all quality gates pass:

1. Stage all changes
2. Create a conventional commit message referencing the task ID
3. Report success to the caller

## Behavioral Constraints

* No subagent spawning (leaf executor)
* Never modify test files (tests are the specification)
* Maximum 5 attempts before circuit breaker trips (skill-managed exception; see `circuit-breaker.instructions.md`)
* Same-error recurrence at attempt 3+ triggers the universal circuit breaker
* Read coding standards once at task start; targeted re-read for unfamiliar modules
* One file change per tool call; broadcast after each write
* When the `agent-intercom` capability pack is installed, use intercom broadcasts for attempt milestones and file-write visibility

## Quality Criteria

* Code route: all current-task harness tests pass; no lint/format violations;
  every code task receives an unfiltered full-suite run. That run is either
  unequivocally `PASS` or is independently accepted by Ship as a precisely
  evidenced interim `EXPECTED_PENDING_RED`; the final task/PR readiness run is
  always `PASS`. Changes remain scoped to the task requirements.
* Docs route: every exact recorded executable command and every auditable
  manual gate is executed and passes; command/result/evidence and manual
  observations/evidence are durably preserved separately; every delta in the
  complete changed-path set is classified against the execution-baseline
  `HEAD` and snapshot; unchanged pre-existing dirty work is not misattributed;
  no task-attributed non-prose path changes or unowned docs changes exist;
  unknown provenance, missing evidence, failed criteria, or unreadable records
  block success.

## Model Routing

This skill operates at **Tier 2 (Standard)** — routine build loop execution and quality verification.

Generated by autoharness | Template: build-feature/SKILL.md.tmpl
