---
name: build-feature
description: "Usage: Build feature {task-id} with harness {harness-cmd}. Implements a requested feature by continuously looping a fast worker agent against a strict, compiling, but failing test harness until success is achieved."
version: 2.0
maturity: stable
input:
  properties:
    task-id:
      type: string
      description: "The unique Backlog.md task ID."
    harness-cmd:
      type: string
      description: "The cargo test command defining the strict compiler harness boundary."
  required:
    - task-id
    - harness-cmd
---

# Build Feature Skill

Implements a requested feature by continuously looping a fast worker agent against a strict, compiling, but failing test harness until success is achieved. The harness defines the contract; the compiler is the critic.

## Subagent Execution Constraint (NON-NEGOTIABLE)

This skill is a leaf executor. It MUST NOT spawn additional subagents via runSubagent, Task, or any other agent-spawning mechanism. Perform all work using direct tool calls (read, edit, search, terminal, MCP tools) and return results to the parent agent (build orchestrator). If you encounter work that seems to require a subagent, report it as a finding and let the parent decide.

## Agent-Intercom Communication (NON-NEGOTIABLE)

If agent-intercom is available (determined by the parent agent's intercom state), broadcast at every step. Broadcasting is not optional.

## Stall Detection

Every terminal command gets a watchdog timeout:

| Operation | Timeout | Action |
|---|---|---|
| cargo test/check/clippy | 45 minutes | Kill process, broadcast stall error, check for lock files, clean up |
| Non-cargo terminal commands | 5 minutes | Kill, broadcast, proceed with error handling |

If a command exceeds its timeout, broadcast `[STALL] {command} exceeded {timeout}`, kill the process, clean up any cargo lock files, and count toward the parent orchestrator's stall limit.

## Prerequisites

* The test harness defined by `${input:harness-cmd}` compiles (green compilation, red tests)
* The structural stubs in `src/` exist with `unimplemented!()` markers
* The project compiles before starting (`cargo check` passes)
* All `[[test]]` entries for new test files are registered in `Cargo.toml`

## Compile Time Warning

> ⚠️ **embeddings feature is enabled by default.** The `ort-sys` and `fastembed` crates compile native ONNX binaries. First-time `cargo test` on a changed source tree takes **20-40 minutes** in debug profile. Release builds (`cargo build --release`) take 5-10 minutes and cache separately.
>
> **Mitigation strategy**: Always use `--test {specific_test}` for the feedback loop. Run `cargo test` (full suite) only once before the final commit.

## Shell Session Hygiene

Before starting any test run, verify no previous cargo or rustc processes are still running from prior iterations. On Windows: `Get-Process -Name cargo,rustc -ErrorAction SilentlyContinue`. Stale processes hold the cargo lock file and cause silent hangs. Stop them before proceeding.

## Remote Operator Integration (agent-intercom)
When the agent-intercom MCP server is reachable, status updates and file modifications route through it so the remote operator can follow progress via Slack.
### Availability Detection
At the start of execution, call `ping` with a brief status message. If the call succeeds, agent-intercom is active and you must follow all remote workflow rules below, then verify messaging with the first `broadcast` before reading files or running the harness. If it fails or times out, print a prominent CLI warning that agent-intercom is unavailable and Slack status updates will not be delivered for this task, then fall back to local-only operation. Silent fallback is forbidden.
### Status Broadcasting
Use `broadcast` (non-blocking) throughout execution to keep the operator informed.
| When | Tool | Level | Message Pattern |
|---|---|---|---|
| Skill start | `broadcast` | `info` | `[BUILD] Starting task {task-id}: {harness-cmd}` |
| Each iteration start | `broadcast` | `info` | `[LOOP] Attempt {N}/5 � running harness` |
| File created | `broadcast` | `info` | `[FILE] created: {file_path}` � include full content in body |
| File modified | `broadcast` | `info` | `[FILE] modified: {file_path}` � include unified diff in body |
| Harness passes | `broadcast` | `success` | `[BUILD] Harness passed on attempt {N}` |
| Harness fails | `broadcast` | `warning` | `[LOOP] Attempt {N} failed � {error_summary}` |
| Circuit breaker hit | `broadcast` | `error` | `[BUILD] Circuit breaker � 5 attempts exhausted, task blocked` |
| Workspace test pass | `broadcast` | `success` | `[BUILD] Workspace tests pass � task {task-id} complete` |
| Task complete | `broadcast` | `success` | `[BUILD] Task {task-id} complete � commit {short_hash}` |
Post the first `broadcast` as a new top-level message and capture the returned `ts`. Use that `ts` as `thread_ts` for all subsequent messages. That first `broadcast` is an intercom verification gate and must happen before reading files, editing code, or running the harness. If it fails after a successful `ping`, print a prominent CLI warning, mark agent-intercom unavailable for the remainder of the task, and continue in local-only mode instead of assuming the operator received the update.
### File Change Workflow
File creation and modification proceed with direct writes. After each file write, call `broadcast` at `info` level with the change details.
For **destructive operations** (file deletion, directory removal), route through the approval workflow:
1. `auto_check` � Check if workspace policy allows the operation.
2. `check_clearance` � Submit proposal and block until operator responds.
3. `check_diff` � Execute only after `status: "approved"`.
## Execution Steps

### Step 1: Context Isolation

1. Read the test file targeted by the `${input:harness-cmd}`. Carefully read the embedded `// GIVEN`, `// WHEN`, `// THEN` BDD comments to fully internalize the human intent behind the test.
2. Use `engram` MCP tools to understand the codebase context before reading raw files:
   * Call `map_code` for each domain struct and function found in the test. This maps the exact source files in `src/` containing the `unimplemented!()` stubs that require attention, including their call graphs and relationships.
   * Call `unified_search` with the feature's key concepts to find related code, context records, and prior decisions that inform the implementation.
   * Call `list_symbols` filtered by file path if you need to discover available symbols in specific modules.
   * Fall back to grep/glob **only** when engram results are insufficient or you need exact text pattern matching.
3. Read `.github/copilot-instructions.md` and `.github/agents/rust-engineer.agent.md` for project coding standards and Rust-specific conventions.
4. `broadcast` at `info` level: `[BUILD] Starting task {task-id}: {harness-cmd}` with a summary of the test scenarios and stub files.

### Step 2: Mechanical Feedback Loop (Actor-Critic)

Execute the following loop with a **hard limit of 5 attempts**:
1. **Run** the targeted `${input:harness-cmd}`.
2. **If it passes** (exit code 0): proceed to Step 3.
3. **If it fails** (exit code != 0):
   a. Capture the raw `stderr` output (compiler errors, type mismatches, or panic traces).
   b. `broadcast` the failure summary at `warning` level.
   c. Analyze the error output and implement the fix:
      * **Compiler errors**: Fix type mismatches, missing imports, incorrect signatures in the `src/` stubs.
      * **Panic traces** (`unimplemented!()` or assertion failures): Implement the underlying logic inside the `src/` stubs to make the harness pass. Replace the `unimplemented!()` macros with real logic.
      * **Test assertion failures**: Fix the implementation logic (not the test itself, unless the test setup has a compilation error).
   d. Apply all project coding standards:
      * All fallible operations return `Result<T, AppError>` � never `unwrap()` or `expect()`.
      * Default visibility `pub(crate)` unless wider access is needed.
      * `///` doc comments on public items, `//!` on modules.
      * Run `cargo check` after each fix to verify compilation before re-running the harness.
   e. After each file write, `broadcast` the change at `info` level with the unified diff.
   f. **Do not modify the test file itself** unless fixing a compilation error in the test setup.
   g. Return to step 1 of this loop.

4. **Circuit breaker**: If 5 attempts are exhausted without the harness passing:
   * `broadcast` at `error` level: `[BUILD] Circuit breaker � 5 attempts exhausted, task blocked`.
   * Call `backlog-task_edit` with `id: ${input:task-id}` and add a note in the task description indicating it is blocked pending human review.
   * Halt execution. Do not retry automatically.
### Step 3: Verification & State Update

Once the isolated harness passes:
1. **Workspace verification — tiered strategy**: Do NOT run `cargo test` (full suite) after every harness pass in the feedback loop. Use this order:
   a. Run `cargo test --test {harness_test_name}` — confirms the harness still passes after any cleanup changes.
   b. Run `cargo test --lib` — fast check for library unit test regressions.
   c. Run `cargo test` (full suite) exactly once before committing. If ort/fastembed have not been compiled for the current source state, this takes 20-40 minutes — broadcast a warning and wait.
   * If new failures appear in the full suite, diagnose and fix them before committing.
   * `broadcast` at `success` level: `[BUILD] Workspace tests pass — task {task-id} complete`.
2. **Lint verification**: Run `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`. Fix any violations.
3. **Commit**: Stage and commit validated changes:
   * `git add -A`
   * `git commit -m "feat: implement passing harness for ${input:task-id}"`
   * `broadcast` at `success` level: `[BUILD] Task {task-id} complete — commit {short_hash}`.
4. **State update**: Mark the task complete in the backlog board:
   * Call `backlog-task_complete` with `id: ${input:task-id}`

## Troubleshooting

### Build fails on fastembed/ort-sys

The `fastembed` crate is enabled **by default** (`default = ["embeddings"]` in `Cargo.toml`). Every `cargo test` run after a source change recompiles ort-sys native binaries in debug profile — this takes 20-40 minutes on first compile and is normal. Do not assume a hang; check `Get-Process -Name rustc` to confirm compilation is active.

If you need to run tests without ort compilation overhead, use `cargo test --no-default-features` to skip the embeddings feature, but note that embedding-gated tests will be excluded.

### Feature guards: compile-time vs runtime

Two patterns exist for embedding availability checks:

* **Compile-time** (`#[cfg(feature = "embeddings")]`): Use in tool handlers and test files to gate code that should not compile without the feature. This is the correct pattern for blocking tool execution paths.
* **Runtime** (`embedding::is_available()`): Returns `false` until the model has been lazily loaded on the **first call**. Do NOT use this as a guard in tool request handlers — it fires on every cold start, including when the feature is compiled in. It is only appropriate for status/health reporting.

### Cargo.toml `[[test]]` registration missing

Every new external test file in `tests/` requires a `[[test]]` block in `Cargo.toml`. Without it, `cargo test` silently ignores the file — no error, no output. Always verify:

```toml
[[test]]
name = "{feature}_test"
path = "tests/integration/{feature}_test.rs"
```

### SurrealDB v2 SDK behavioral differences

Refer to the session memory at `.copilot-tracking/memory/` for documented workarounds including `Thing` deserialization, `<datetime>` casts, and raw SurrealQL over SDK methods.

Known test data requirements:
* `embed_type` must be `"explicit_code"` (not `"code"`) — the DB schema validates this field
* `embedding` must be `vec![0.0_f32; 384]` — SurrealDB enforces 384-dimensional vectors even for test fixtures that do not exercise vector search

### Global state in integration tests

`OnceLock`-backed singletons (e.g., `query_stats`) persist across parallel test threads. Use `tokio::sync::Mutex::const_new(())` as a test-level serialization lock when tests share global state. `std::sync::Mutex` cannot be held across `.await` points — clippy will deny this with `await_holding_lock`.

### Tests pass locally but fail in CI

Verify `rust-toolchain.toml` matches the CI configuration in `.github/workflows/ci.yml`. Check that all `[[test]]` entries in `Cargo.toml` include the new test files.

### Circuit breaker triggered (5 failed attempts)

When the 5-attempt hard limit is reached, the task is marked as blocked in the backlog board. Review the `stderr` output from each attempt to identify the root cause. Common issues include missing trait implementations, incorrect type signatures in stubs, or test assumptions that conflict with the codebase architecture.

---

Proceed by reading the harness test file and isolating context for the given task.


