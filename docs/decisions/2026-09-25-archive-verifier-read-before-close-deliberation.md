---
title: "Archive verifier read-before-close repair: unblock route for F50 / 142-S"
description: "P-021 C6 deliberation for deferred entry BE626470 (reconciled into 4EE241DC). Decides how the release-archive MCP smoke repair can ship without expanding F50's active fix cycle."
topic: "scripts/verify-release-archive.py closes MCP stdin before reading tools/list; the unmapped failure blocks 142.054-T (F50) and the 142-S final gate"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-09-25-archive-verifier-read-before-close-plan.md"
  - "stash:4EE241DC (survivor)"
  - "stash:BE626470 (archived duplicate, Ship's C2 capture ID)"
tags:
  - "p-021"
  - "deferred-scope-expansion"
  - "142-S"
  - "release-archive-smoke"
---

> **Status (2026-10-04, PR #410 review): route superseded.** Option A's "add the repair to active 142-S as its final
> code task" (PA1) was never authorized. Under `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` Revision 18
> (frozen), `142.060-T` is Slot-01, its own one-task shipment claimed first (OD-1, 2026-10-04), with no task
> predecessors (E11), and 142-S is abandoned at H3. The root-cause analysis and repair design below still apply.

## Problem Frame

`integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary` fails on
this Windows host with `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`. The root cause is already
proven.

* `verify_mcp_stdio` in `scripts/verify-release-archive.py` (lines 207–217) sends all three requests with
  `Popen.communicate(payload, timeout=45)`. `communicate` closes stdin right after it writes.
* The shim sees EOF, takes its classified admission exit (10), and stdout is cut at about 8192 characters.
  The cut lands in the middle of the `tools/list` line.
* An interactive client that reads each response before closing stdin got the full 12,837-byte
  `tools/list` (21 tools) and exit 10.
* A Linux CI variant, merged from F86074CD (138-S, PR #391), failed with `missing JSON-RPC response id 2`.
  This is the same race on a host where the shim exits before it writes id 2 at all.

This matters now because F50 (`142.054-T`) passed its targeted harness, check, clippy, and fmt. But
`cargo dev-test --no-fail-fast` fails 13 tests:

* 12 map exactly to the recorded F51–F54 RED harnesses.
* The archive smoke failure maps to no RED harness.

Under Ship Step 4.3, F50 cannot get `EXPECTED_PENDING_RED` while that failure is unmapped. The final
142-S task and PR readiness run must be green with no pending-red allowance, so 142-S cannot ship until
the verifier is repaired. Every task that depends on F50 is blocked, including the operator's later F51
requirement for the `start.ps1` workspace debug binary.

**Success criteria.** The archive smoke passes for a real reason on Windows and Unix. Exit-10 and stderr
panic/backtrace checks are kept. F50 gets a legal interim verdict. The fix lands without any code in F50's
fix cycle, without suppressing or filtering tests, and without faking a green suite.

**Out of scope.** Any product/shim behavior change. The shim's EOF flush is captured separately below.
Also out of scope: F50 source or tests, and every other 142-S task.

## Research Findings

* **P-021 C1/C4.** The verifier is a different contract surface from F50's typed preflight state machine,
  so it is out of scope for F50's cycle. C4 is unconditional: no authorization, including the operator's,
  expands the cycle that found the problem. Authorization can only open a separate unit, after C2 capture
  (done: BE626470) and C6 deliberation (this artifact).
* **P-001.** Only one top-level release unit may be in flight. `142-F` is active; its shipment is `142-S`.
  A separate top-level chore cannot start until 142-F finishes post-merge closure.
* **P-016.** Exactly one implementation branch/worktree. The repair has to land on the active 142-S branch.
* **Ship Step 4.3 `EXPECTED_PENDING_RED`.** Each failing test must map to a previously recorded, compiling
  RED harness of a different, unstarted task that comes later in this shipment's dependency-resolved order.
  Exact test names and markers are required. A generic `Worker` substring is not enough.
* **Active-manifest immutability.** The Orchestrator's planning-overlap rules say "Stage must not modify
  the active Ship shipment manifest". The 2026-09-24 CLI-parity deliberation (Option C) followed this rule
  for 142-S. P-010 does allow Stage to update shipment manifests in general. Stage's own Step 5.5 covers
  only *queued* shipments, so no Stage rule explicitly allows amending an active one.
* **backlogit.** `shipment add` is idempotent and refuses an item that already belongs to another shipment.
  So putting the repair in a separate shipment would then make it impossible to add it to 142-S.
* **Consumers.** `.github/workflows/verify-release-assets.yml:242` runs the verifier on release targets.
  `tests/integration/release_archive_smoke_workflow_test.rs` checks the verifier's source markers
  (`tools/list`, `protocolVersion`, `--mcp`, and so on). The native test asserts `MCP_PROTOCOL_VERSION=`,
  `MCP_TOOL_COUNT=`, `MCP_STDIN_CLOSE_EXIT=10`, and `ARCHIVE_SMOKE=PASS`.
* **Learnings (`docs/compound/`).** Nothing directly relevant (confidence low). No prior pattern for
  read-before-close or for amending an active manifest.

## Options Evaluated

### Option A: Separate repair task under 142-F, added to active 142-S, sequenced after F50 (recommended)

Create a new task under `142-F` whose only job is the verifier repair plus its regression coverage. Its
sequencing-only edges sit on the new task alone. They put it after F50, and after the plan-review
refinement also after F51–F54, as the final code task. Add it to active shipment 142-S on
the same branch. Ship runs harness-architect for it the normal P-002 way: it becomes a recorded, compiling
RED harness for a later, unstarted task in this shipment. The archive smoke failure then maps honestly,
and F50 can earn `EXPECTED_PENDING_RED`. F50's record, code, and cycle are not touched.

* Pros: keeps P-021 C4 (separate unit, opened forward), P-001 (same release unit), and P-016 (same
  branch). Uses the existing interim-verdict mechanism exactly as designed. No test suppression and no
  misclassification. The final gate still requires real green.
* Cons: needs one explicit operator approval to amend an active manifest. Relies on the native smoke
  failing reliably on this host until the repair lands (see Risks).
* Effort: low (one task, two owned files).

### Option A2: Same task, but sequenced before F50

Make `142.054-T` depend on the repair task.

* Cons: this edits the dependency set of an active, already-implemented task in the middle of its gate.
  Ship would have to move off an active cursor, and F50's record would change. Rejected: more intrusive
  than A, with no gain.

### Option B: Separate queued shipment or chore sequenced after 142-S

* Cons: deadlock. 142-S's final gate needs the repair, and P-001 bars starting the chore before 142-S
  closes. Once the item is in that shipment, backlogit's single-assignment rule also blocks adding it to
  142-S later. Infeasible.

### Option C: Fix inside F50's active fix cycle

* Cons: P-021 C1 fails, and C4 forbids it absolutely. Rejected.

### Option D: Ignore, filter, or baseline the archive smoke test, or call it "pre-existing"

* Cons: Ship Step 4.3 forbids filters, suppression, and new ignores, and requires every failure to be
  mapped. This would fake green. Rejected.

### Option E: Fix the shim so it flushes pending stdout before its EOF exit

* Cons: this is a product runtime surface with larger blast radius. It still needs its own unit and the
  same manifest approval, so it does not avoid the exception. It also does not answer whether a client
  that closes stdin mid-request should be served. Deferred to separate investigation (captured below);
  it is not a prerequisite.

### Option F: Operator `skip_policy: P-001` and a parallel chore branch

* Cons: a much larger exception, and it also breaks P-016. Rejected.

## Trade-off Comparison

| Criterion | A (after F50) | A2 (before F50) | B (separate shipment) | C (in F50) | D (suppress) | E (shim) | F (skip P-001) |
|---|---|---|---|---|---|---|---|
| P-021 C4 | Yes | Yes | Yes | **No** | n/a | Yes | Yes |
| P-001 / P-016 | Yes | Yes | Yes, but deadlocks | Yes | Yes | Yes | **No** |
| Honest verdicts | Yes | Yes | n/a | No | **No** | Yes | Yes |
| Touches active F50 record | No | **Yes** | No | Yes | No | No | No |
| Exception needed | 1 manifest amendment | Manifest + dependency edit | None, but infeasible | Impossible | Impossible | 1 manifest amendment | skip_policy P-001 |
| Can unblock 142-S | **Yes** | Yes | No | No | No | Yes, riskier | Yes, unsafe |

## Decision

**Option A.** The operator delegated this with "Use your best judgement. Seek the best path based on the
available information." Stage takes that as confirmation of this deliberation outcome only. It is **not**
approval to amend the active manifest (see below).

* Create one repair task under `142-F`. It gets sequencing-only edges **on the new task only**:
  blocked by `142.054-T` through `142.058-T`, which makes it the final code task. This refinement came
  out of plan review attempt 1: sibling order was ambiguous. No active task record is edited. The plan is
  `docs/exec-plans/2026-09-25-archive-verifier-read-before-close-plan.md`.
* Repair approach: write the requests incrementally. Read stdout line by line with a single deadline until
  responses id 1 and id 2 are both parsed. Only then close stdin, wait for exit, and collect stderr. Keep
  the checks for exit 10, panic/backtrace, and the non-JSON line, and all `MCP_*` output markers. The
  reader must not deadlock on either OS (no blocking read without a deadline). A reader thread or
  equivalent handles this on Windows.
* Regression coverage: a deterministic fake stdio server that models "drops output still pending at
  EOF". It runs the verifier's `verify_mcp_stdio` on Windows and Unix via Python. Plus the existing native
  smoke test.

**Smallest required approval (not yet granted):** a one-time, explicit operator authorization to add the
new repair task to **active** shipment `142-S`. This is an exception to active-manifest immutability
(Orchestrator planning-overlap rule; 2026-09-24 precedent). Stage performs it with
`backlogit shipment add 142-S <task-id>`. Nothing else is excepted: P-001, P-016, P-021, and the Ship
Step 4.3 gates all stay in force.

## Rejected Alternatives

Options A2, B, C, D, E, and F were rejected for the reasons above. In short:

* A2 is intrusive.
* B deadlocks.
* C and D are forbidden.
* E is a riskier surface and is deferred.
* F needs a larger exception.

## Unresolved Questions

* Whether the shim *should* flush a response that is still in flight when stdin closes. This is a product
  question and does not block this work. Captured as stash `F1F3D9D7` (low, requires deliberation).
* Whether `backlogit shipment add` accepts an item into an `active` shipment. Stage did not test this,
  because testing it would mutate the manifest. If it refuses, halt and report. No workaround.

## Risks and Mitigations

* **The native smoke passes intermittently before the repair**, so a mapped pending-red test would not
  fail. The deterministic fake-server RED test fails every time. Ship records both, and should run the
  repair right after F50 to shorten the window. If the native test passes during F50's gate, Ship records
  exactly what it observed and does not invent a failure.
* **The verifier change hides the shim EOF-flush behavior.** Captured as stash `F1F3D9D7`. The verifier
  still checks the stdin-close exit (10) after reading.
* **Reads hang on Windows pipes.** The plan requires a deadline-bounded reader and the existing 45-second
  budget.
