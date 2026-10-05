---
title: "Archive verifier read-before-close repair (142-F, separate unit after F50)"
source: "docs/decisions/2026-09-25-archive-verifier-read-before-close-deliberation.md"
parent_feature: "142-F"
target_shipment: "142-S (pending explicit operator authorization to amend the active manifest)"
stash_refs: "4EE241DC (survivor); BE626470, F86074CD and 7 others archived as merged duplicates"
date: "2026-09-25"
---

## Problem Frame

`scripts/verify-release-archive.py::verify_mcp_stdio` (lines 179–265) starts `engram shim --workspace
<missing>` and sends `initialize`, `notifications/initialized`, and `tools/list` in a single
`Popen.communicate(payload, timeout=45)`. Because `communicate` closes stdin right after writing, the shim
reaches EOF and exits 10 before the `tools/list` response is fully out:

* On this Windows host, stdout stops at about 8192 characters, so the check fails with
  `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`.
* In Linux CI (F86074CD), the id 2 response never appears, so it fails with
  `ARCHIVE_SMOKE=FAIL: missing JSON-RPC response id 2`.

A client that reads each response before closing stdin receives the complete 12,837-byte `tools/list`
(21 tools) and exit 10.

This is a test-infrastructure (release smoke harness) defect. The product's MCP contract is not part of it.

## Requirements Trace

| # | Requirement (from the deliberation) | Implementation action |
|---|---|---|
| R1 | Read every JSON-RPC response to completion before closing stdin | U1: incremental write, then deadline-bounded line reads until ids 1 and 2 are parsed, then close stdin |
| R2 | Keep the classified admission exit 10 check and the stderr panic/backtrace checks | U1: keep the existing assertions and messages; collect stderr after exit |
| R3 | Keep every output and contract marker | U1: keep `MCP_PROTOCOL_VERSION=`, `MCP_TOOL_COUNT=`, `MCP_STDIN_CLOSE_EXIT=`, the `non-JSON stdout from MCP stdio` failure text, and the source markers asserted by `archive_verifier_is_version_generic_and_never_uses_cargo_run` |
| R4 | Work on Windows and Unix with no hang | U1: deadline-bounded reader (reader thread or equivalent), the existing 45-second overall budget, and terminate/kill on timeout |
| R5 | Deterministic regression proof on both OS families | U1: a fake stdio MCP server test run through Python on any host, plus the existing native smoke test |
| R6 | No change to F50 or to any other 142-S task's owned files | Scope guard: the owned files below only |

## Implementation Units

### U1 — Read-before-close MCP smoke in the release archive verifier (single task)

* **Parent:** feature `142-F`, as a direct child. This matches every existing 142-F task, for example
  `142.054-T`; 142-F uses no sub-epic level. The task references this plan and the deliberation.
* **Owned files (2):** `scripts/verify-release-archive.py` and
  `tests/integration/release_archive_smoke_workflow_test.rs` (already registered as
  `integration_release_archive_smoke_workflow`). Every fake server is **inline Python inside the Rust
  test**, written to a `TempDir`, following the existing `fixture_builder` pattern. No new fixture files.
* **Functions (<5):** `verify_mcp_stdio` (reworked), plus at most two private helpers: a
  deadline-bounded stream-drain reader, and a response collector.
* **Implementation constraints:**
  1. Write requests incrementally. Drain **both** stdout and stderr at the same time from the moment the
     process starts: one daemon reader thread per stream, feeding a queue. Use one overall deadline, set
     by a module-level constant `MCP_STDIO_TIMEOUT_SECONDS = 45`. `select` is not used, because it does
     not work on Windows pipes.
  2. Parse stdout lines until the responses with id 1 and id 2 are both present. Then close stdin, keep
     reading stdout to EOF, and send **every** stdout line (before and after close) through the existing
     `non-JSON stdout from MCP stdio` check.
  3. Wait for exit within the remaining deadline. On timeout: terminate, then kill, then reap, and raise a
     `SmokeFailure`. Keep the existing message `MCP stdio process hung after stdin closed` for a
     post-close hang, and add `MCP stdio response timeout before id 2` for a hang before id 2.
  4. A `BrokenPipeError` on write becomes a `SmokeFailure`.
  5. Keep: `returncode is None or < 0`, the exit-10 message, the `panicked at` and `stack backtrace:`
     checks, the response and shape checks, and the `MCP_PROTOCOL_VERSION=`, `MCP_TOOL_COUNT=` and
     `MCP_STDIN_CLOSE_EXIT=` prints.
  6. Read `MCP_STDIO_TIMEOUT_SECONDS` when `verify_mcp_stdio` runs, never as a default argument, so
     the harness override takes effect.
  7. Keep calling `subprocess.Popen` **through the module-level `subprocess` name**, never
     `from subprocess import Popen`. This is the test seam below. Keep the `shim` and `--workspace`
     literals, and every literal asserted by `archive_verifier_is_version_generic_and_never_uses_cargo_run`.
* **Test seam, valid before and after the repair:** the Python harness loads the verifier with
  `runpy.run_path`. It replaces `verify_mcp_stdio.__globals__["subprocess"]` with a thin namespace whose
  `Popen` rewrites argv to `[sys.executable, <tempdir>/fake_server.py]` and forwards everything else to the
  real `subprocess.Popen`. Every other name, including `PIPE` and
  `TimeoutExpired`, falls through to the real module through a `__getattr__` fallback. The seam works against the
  **current** `communicate()`-based code, so the RED test fails for the right reason before any verifier
  change.
* **New test scenarios (3), plus the unchanged existing native smoke as characterization:**
  1. **`archive_verifier_reads_mcp_responses_before_closing_stdin`: deterministic RED → GREEN.** The fake
     server answers `initialize`. For `tools/list`, it builds a single line longer than 8192 characters
     (at least 12 KiB, matching the real 12,837-byte catalog) and writes and flushes the first 8192
     characters. It then calls `eof_event.wait(timeout=2.0)`, where a stdin reader thread sets `eof_event`
     on EOF.
     * If EOF has arrived, it exits 10 without writing the rest. This models "pending output dropped at
       EOF".
     * Otherwise it writes the rest, flushes, waits for EOF, and exits 10.
     * Against the current verifier, `communicate()` has already closed stdin, so the result is always the
       truncated line. **RED marker:** `SmokeFailure` whose message contains
       `non-JSON stdout from MCP stdio`. The harness prints `RED: F-ARCHIVE-U1 read-before-close` and
       exits non-zero when it observes that failure, so the Rust test fails with that marker.
     * **GREEN** after the repair: `MCP_TOOL_COUNT=` is at least 1, and `MCP_STDIN_CLOSE_EXIT=10`.
     * It runs on Windows and Unix and needs only Python.
  2. **`archive_verifier_keeps_exit_and_stderr_checks_after_reading_first`: parameterized guard, green
     before and after.** Three fake variants each send full responses:
     * (a) exits 3 → exit-10 message;
     * (b) writes `panicked at` to stderr **and exits 10** → panic message;
     * (c) writes `stack backtrace:` to stderr **and exits 10** → panic message.
     The current code checks the exit code before stderr, so (b) and (c) must exit 10.
     This is **not** a RED test. It must pass on the current code and after the repair.
  3. **`archive_verifier_bounds_an_unresponsive_mcp_server`: guard, green before and after.** The fake
     never answers id 2 and ignores EOF. It does a **bounded** sleep of about 90 seconds, which is longer
     than the 45-second budget, so the current code also times out. The bound means no process is left
     behind. Prefer `getattr(sys, "_base_executable", sys.executable)` for the fake interpreter. The harness sets `MCP_STDIO_TIMEOUT_SECONDS = 2`
     through the globals seam; the current code ignores this, and its guard pass may take up to its
     45-second budget. It asserts a `SmokeFailure` whose message contains either `hung after stdin closed`
     or `response timeout before id 2`, and that the child process was reaped. After the fix, it also
     asserts the guard finishes well under 45 seconds. This covers R4.
  * **Existing:** `archive_verifier_runs_the_unpacked_native_binary`, unchanged. On this Windows host it is
    currently a deterministic characterization RED: the 12,837-byte catalog exceeds the 8192 cutoff, and
    every 2026-09-24/25 run reproduced it. **Marker:**
    `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`.
* **Execution posture:** characterization-first, then test-first.
* **Size / complexity:** Size S | Complexity medium. These are independent axes. Complexity is medium
  because of cross-platform pipe and deadline semantics. Recorded as prose, because backlogit task
  creation here defines no structured sizing fields.

### U1 Acceptance Criteria

1. `verify_mcp_stdio` reads the id 1 and id 2 responses completely before closing stdin, drains stdout and
   stderr concurrently under one deadline, and never uses `select`.
2. Scenario 1 is recorded RED (exact marker) before any verifier change, and is GREEN after it.
3. Scenarios 2 and 3 pass both before and after the change. Exit-10, `panicked at`, `stack backtrace:` and
   hang/timeout failures are all still detected.
4. `archive_verifier_runs_the_unpacked_native_binary` is GREEN on Windows. The ubuntu CI job (`ci.yml`,
   `cargo test --all-targets`) is GREEN for the same target, and it runs the native `--mcp` path on
   `x86_64-unknown-linux-gnu`.
5. `archive_verifier_is_version_generic_and_never_uses_cargo_run` and every other existing test in the
   target stay GREEN. No test is ignored, filtered, or weakened.
6. No file outside the two owned files is modified.

### U1 Harness-ready record (what harness-architect must record, P-002/P-004, Step 4.3 rules 3–4)

* Task ID, and the exact pending-red set: scenario 1 (test name and marker) **and**
  `archive_verifier_runs_the_unpacked_native_binary` (marker
  `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`). Guards 2 and 3 are listed as green-required, not
  pending-red.
* Baseline content hashes for **both** owned files, captured after the RED commit and before any
  implementation commit.
* `cargo check --all-targets` PASS, and targeted `cargo test --test integration_release_archive_smoke_workflow`
  failing with exactly those two markers.

## Dependency Graph

* **U1 is blocked by `142.054-T`, `142.055-T`, `142.056-T`, `142.057-T` and `142.058-T`** (all other 142-S
  code tasks). These are sequencing-only edges, and they sit **only on the new task**. No active 142-S
  task's record or dependency set is edited, so Ship keeps ownership of the existing topology.
  * U1 is unambiguously the **final code task** in 142-S's dependency-resolved order. Ship Step 3 sorts by
    dependency; the edges remove sibling ambiguity.
  * Every earlier code-task gate (F50 through F54) can map the archive-smoke failure and scenario 1 to U1,
    which comes later (rule 3).
  * U1's own gate is the final code gate. It has no pending-red allowance and must be fully green. By then
    F51–F54 are done.
* `142.059-T` (F55, docs route) is unaffected. Its order relative to U1 does not matter to the code-route
  pending-red rules. The final PR readiness run must be green.
* **Optional variant (needs a separate authorization, not requested):** sequence U1 right after F50.
  This is a single **atomic edge swap**. First **remove** U1's edges on `142.055-T`, `142.056-T`,
  `142.057-T` and `142.058-T`, leaving only `U1 blocked by 142.054-T`. Then add `blocked by U1` edges on
  unstarted active tasks `142.055-T`, `142.057-T` and `142.058-T`. Applying the added edges without the
  removal creates a cycle and deadlocks Step 3. The swap cuts native-mapping exposure from five gates to
  one, but it edits active task records. It is not the default. If it is applied, Ship re-runs Step 3.

## Constitution Check

* **I** Safety-First Rust: not affected (no Rust product code; test code follows repo patterns).
* **II** Test-First: RED recorded before implementation.
* **III** Workspace Isolation: fake servers run in `TempDir`.
* **IV** CLI Containment: unchanged.
* **V** Observability: failure messages are kept or extended.
* **VI** Single Responsibility: one unit, one surface.
* **VII** Destructive Command Approval: no destructive commands.
* **VIII** Safety Modes: Careful mode for PA1 (operator checkpoint); Freeze-scope to the two owned
  files during U1.
* **IX** Git-Friendly Persistence: no persistence change.
* **X** Context Efficiency: 2 files.
* **XI** Merge Commits: unaffected.
* Task Granularity: fewer than 3 files, fewer than 5 functions, 3 new scenarios (fewer than 4). Width
  isolation: test-infrastructure domain only.

## Decisions and Rationale

* **Read-before-close, not a shim change.** The client pattern is the verifier defect. Shim EOF-flush
  semantics are a separate product surface, captured as stash **F1F3D9D7** (low, requires deliberation).
* **Keep the stdin-close check.** The verifier still closes stdin and asserts exit 10, after reading.
* **Test seam through the `subprocess` module global.** It works on the current code, so the RED test is
  honest. It needs no pre-change helper commit. Constraint 7 keeps it stable.
* **Final code task (edges on the new task only), not right after F50.** This keeps the existing topology
  under Ship ownership. The immediate-successor variant is offered separately.
* **Prior art:** `docs/compound/feature-test-flake-fix-in-scope-not-shared-infra-2026-07-11.md`. Fix inside
  the owned boundary, and stash the shared or product fix separately.

## Risks and Caveats

* **The native smoke passes in some interim gate before U1 lands.** Step 4.3 requires a mapped pending-red
  test to actually fail. If it passes, that gate verdict is **blocked**, and Ship records the observed
  output. Ship must not rerun just to get a failure, and must not drop the test from the mapping. It
  escalates to Stage/operator, and Stage evaluates the optional immediate-successor variant. Current
  evidence says the failure is deterministic on this host, because the 12,837-byte catalog exceeds the
  8192 cutoff.
* **Scenario 1 timing:** it uses a 2-second EOF wait margin, and only the failure path depends on it.
  Validate it under the loaded ubuntu CI job.
* **Windows pipe blocking:** concurrent drain threads plus a deadline, with terminate, kill, and reap.
* **Source-marker coupling:** constraint 7 keeps every asserted literal.
## Plan Hardening Signals (REQUIRED)

* Public API, schema, or contract change: **absent**. The verifier's CLI flags and output markers are
  unchanged.
* Security, auth, permission, or compliance-sensitive behavior: **absent**.
* Migration, backfill, destructive data/config action, or irreversible step: **absent**.
* External integration, operator checkpoint, or external dependency: **present**.
  `.github/workflows/verify-release-assets.yml` runs this verifier on the release targets
  (Linux, Windows, macOS). The work also depends on one operator checkpoint: approval to amend active
  shipment 142-S.
* High runtime, rollout, or rollback risk: **absent**. Release tooling only; `git revert` is enough.

Requires plan hardening: yes

## Runtime Verification and Closure

* **Runtime surface:** release tooling only (a Python script driven by CI and by the Rust integration
  test). No product runtime change.
* **Verification:**
  * `cargo test --test integration_release_archive_smoke_workflow` is GREEN on Windows. This covers
    scenarios 1–3 and the native smoke.
  * On Unix, the same target runs in the ubuntu CI job (`ci.yml`, `cargo test --all-targets`), which
    includes the native `--mcp` path for `x86_64-unknown-linux-gnu`.
  * U1 is the final code task, so its full `cargo dev-test --no-fail-fast` must be **unequivocally green**,
    with no pending-red allowance.
* **Closure (P-021 C3, threadless build finding):** cite deferred IDs `4EE241DC` (survivor) and `BE626470`
  (Ship's original capture) in:
  * the F50 task-level record (comment on `142.054-T`) and the F50 run-level record, when Ship records
    the `EXPECTED_PENDING_RED` verdict;
  * U1's task and run records;
  * the 142-S PR/closure residual-risk record.
  Also record stash `F1F3D9D7` (shim EOF flush) as residual risk.

## Plan Hardening

**Hardening required:** yes, triggered by the external CI consumer and the operator checkpoint.

**Consulted:**

* P-001, P-016, P-021 (C1–C6), and Ship Steps 2, 3 and 4.3 (`EXPECTED_PENDING_RED` rules 1–6).
* The Orchestrator planning-overlap rules and the 2026-09-24 CLI-parity deliberation (active-manifest
  precedent).
* `docs/compound/workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` (shipment-reconcile
  after a manifest change).
* `docs/compound/test-failures/truncated-test-output-review-hid-call-site-regression-2026-09-12.md`
  (complete suite output).
* `docs/compound/workflow-issues/narrow-scope-expansions-require-stage-2026-08-18.md`.

**Protected invariants**

1. F50 (`142.054-T`) code, owned tests, dependencies and fix cycle are untouched. Its record changes only
   through Ship's own verdict and C3 comments.
2. No active 142-S task's dependency set or owned file is edited by Stage.
3. No test is filtered, ignored, suppressed, or weakened.
4. Exit 10, the panic/backtrace checks, and every `MCP_*` and `ARCHIVE_SMOKE=*` marker are kept.
5. One branch and one worktree, and no second shipment claim.
6. A full-suite result is never reported as green unless it is.

**Risky actions**

| ProposedAction | ActionRisk | Approval |
|---|---|---|
| PA1: Stage runs `backlogit shipment add 142-S <U1-task-id>` (amend an **active** manifest; adds a single item) | medium: overrides the active-manifest immutability default; reversible by Ship `shipment return-blocked` or by operator action | **Explicit operator authorization required. Not yet granted.** |
| PA1b: Right after PA1, Ship runs shipment-reconcile intake (`mode: pre`, `expected_status: active`) and confirms that the manifest is exactly the 12 prior items plus U1 | low | Part of PA1 |
| PA1c: After PA1b and PA2, Ship re-runs Step 3 (items 1–3). It confirms that U1 carries `harness-ready` and is the last code task in dependency order. | low | Part of PA1 |
| PA2: Harness-architect records the U1 RED harness (the U1 Harness-ready record above) in a separate commit `test(<U1-task-id>): …`, touching only U1-owned files | low: separate unit, no F50-owned file | Normal P-002 flow for a queued task after PA1 |
| PA3: Ship reruns F50 Step 4.3 with complete output captured (every per-binary `FAILED` line) and claims `EXPECTED_PENDING_RED` | medium: gate integrity | Only if rules 1–6 all hold and every failure maps exactly once. Expected: 12 failures → F51–F54 at their recorded markers; `archive_verifier_runs_the_unpacked_native_binary` and scenario 1 → U1 at the markers above. Guards 2 and 3 must pass. Anything else keeps F50 blocked |
| PA4: The verifier change reaches release CI | low | Normal local review, PR review and CI |

**Blocked-path handling**

* **`shipment add` refuses an active shipment:** halt and report to the operator. Do **not** create a
  second shipment; backlogit's single-assignment rule would then make 142-S inclusion impossible.
* **A mapped pending-red test passes in an interim gate** (for example, an intermittent native smoke):
  that verdict is blocked. Ship records the observed output. It does not rerun to shop for a failure, and
  does not drop the test from the mapping. It escalates to Stage, and Stage evaluates the immediate-
  successor variant with the operator.
* **Any unmapped failure appears:** the gate stays blocked. Do a new P-021 C1 classification and C2
  capture. Never fix it inside the discovering task.

**Rollback:** revert U1's commits. The verifier returns to its old behavior, and the failure becomes
unmapped again.

**Owner:** Ship (execution) and Stage (manifest amendment PA1).

**Validation window:** the 142-S PR CI run plus the next release-asset verification.

**Unresolved operator decision that blocks safe execution:** PA1 authorization.

<!-- plan-review-attempt: 1 -->

## Plan Review — attempt 1 (2026-09-25): FAIL, revised in place

Personas: Constitution, Rust, Scope Boundary, Learnings (medium confidence), and Architecture. The revision
above addresses these findings.

* **P1 (Rust, Constitution):** the RED seam didn't exist before the change → now uses the
  `subprocess`-global seam, which works on the current code.
* **P1 (Rust):** collecting stderr after exit could deadlock → now drains both streams concurrently, with
  no `select`.
* **P1 (Architecture):** sibling ordering was ambiguous, so U1 was not guaranteed to come before the final
  gate → now uses edges on the new task only, making U1 the final code task.
* **P2 findings addressed:**
  * Scenario 2 was not RED → relabeled as a guard.
  * Deterministic EOF wait added (`eof_event.wait(2.0)`).
  * R4 hang test added (scenario 3).
  * Harness-ready record and baselines made explicit.
  * Blocked-path rule for a native smoke that passes corrected to follow the Step 4.3 "must fail" rule.
  * C3 task/run records added.
  * Constitution Check added.
  * Acceptance criteria added.
  * Shim stash ID F1F3D9D7 recorded.
  * shipment-reconcile after PA1 added.
* **P3 findings addressed:** inline fake servers, post-close stdout validation, `BrokenPipeError`,
  complete suite output, and prior-art citation.
* **P-003 note:** 142-F has no sub-epic level. U1 is a direct child of 142-F, consistent with all existing
  142-F tasks.

<!-- plan-review-attempt: 2 -->

## Plan Review — attempt 2 (2026-09-25): FAIL (one P1), revised in place

* **Constitution: PASS.** No P0/P1. The P3s are applied: safety modes named, timeout read at call time.
* **Rust: PASS.** No P0/P1. The seam and determinism were confirmed against the current code. The P2s are
  applied: guard (b)/(c) exit 10, and guard 3 uses a bounded ~90 s sleep with `_base_executable`. The P3s
  are applied: `__getattr__` fallthrough, both hang messages accepted, and the RED harness exits non-zero.
* **Architecture: FAIL.**
  * P1: the optional immediate-successor variant, applied as an add-only change, would have created a
    cycle with U1's default edges. Fixed: the variant is now defined as an atomic edge swap.
  * P2: the ready queue was not rebuilt after the manifest amendment. Fixed: PA1c re-runs Ship Step 3.

## Plan Review — attempt 3 (2026-09-25): PASS

* **Architecture re-review:** no blocking findings. The attempt-2 P1 (edge-swap cycle) and P2 (ready-queue
  rebuild) are both resolved.
* Constitution and Rust passed at attempt 2 with no P0/P1. Scope Boundary and Learnings had no P0/P1 at
  attempt 1, and their P2/P3 items were applied.
* **Plan hardening:** required, and satisfied by the `## Plan Hardening` section.
* **Gate: PASS.** Ready for harvest.
* **Open operator decision:** PA1, the authorization to amend active shipment 142-S. This is an execution
  prerequisite, not a plan defect.