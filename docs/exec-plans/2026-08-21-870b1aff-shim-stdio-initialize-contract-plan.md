---
title: Shim stdio initialize contract and early-child-exit diagnostics
date: 2026-08-21
type: implementation-plan
status: reviewed
source_stash_id: 870B1AFF
source: docs/decisions/2026-08-21-870b1aff-copilot-mcp-stdio-initialize-investigation.md
agent: stage
---

## Problem Frame

`src/shim/mod.rs::run` evaluates three fallible preconditions —
`db::workspace::canonicalize_workspace`, `shim::lifecycle::ensure_daemon_running`,
and `daemon::ipc_server::ipc_endpoint` — before `shim::transport::run_shim`
binds `rmcp::transport::io::stdio()`. Any `Err` propagates through
`src/bin/engram.rs::main` and terminates the process while an MCP client is
writing `initialize`, producing Windows `os error 232` on the client side and no
attributable engram diagnostic. The client's `tokio::process::ChildStdin` /
`ChildStdout` naming is a downstream symptom, not the defect.

Secondary hazard: `src/lib.rs::init_tracing` uses `fmt::layer()` whose default
writer is stdout. The shim path does not call it today, but nothing enforces
that invariant, and stdout is the MCP framing channel.

## Requirements Trace

| Requirement (stash 870B1AFF) | Implementation action |
|---|---|
| Determine whether the daemon/shim exits before MCP initialize | Resolved in investigation (E1–E3). Encoded as regression assertions in U1. |
| Capture child exit status and stderr | U1 harness asserts exit-code taxonomy and stderr attribution; U5 implements the taxonomy and durable record. |
| Distinguish Tokio transport symptoms from the actual startup root cause | U1/U2 assert client-visible `initialize` success under induced precondition failure; U5 record names the real cause. |
| Inspect `/mcp show engram` and engram logs | U6 documents the operator diagnostic path against the new record. |
| Add regression coverage for Windows stdio initialization | U1 (contract RED) and U2 (stdout purity RED), both Windows-executed. |
| Add early-child-exit diagnostics | U5 startup-failure record + exit-code taxonomy. |

## Implementation Units

### U1 — RED: stdio initialize contract harness

* Changes: new contract test asserting that a shim child spawned with a
  deliberately unsatisfiable precondition (unreachable/failing daemon readiness)
  still completes the MCP `initialize` handshake, and that when the shim does
  exit before `initialize` it exits with a documented distinct code and writes an
  attributable stderr line.
* Files: `tests/contract/shim_stdio_initialize_test.rs`, `Cargo.toml` (`[[test]]` entry).
* Tests: 3 scenarios — (a) initialize succeeds under failed daemon readiness,
  (b) `tools/list` returns the static catalog in that degraded state,
  (c) `tools/call` returns a structured JSON-RPC error naming the startup cause.
* Posture: test-first (RED). Must compile and fail.

### U2 — RED: stdout framing purity harness

* Changes: contract test asserting that every byte the shim writes to stdout
  parses as JSON-RPC framing, including when tracing is force-initialized via
  `RUST_LOG` / `ENGRAM_LOG_FORMAT` on the shim path.
* Files: `tests/contract/shim_stdout_purity_test.rs`, `Cargo.toml`.
* Tests: 2 scenarios — clean run, and run with logging env vars set.
* Posture: test-first (RED).

### U3 — GREEN: serve-first shim startup

* Changes: restructure `shim::run` to bind the stdio transport and serve
  `initialize` before daemon-dependent preconditions; represent startup outcome
  as a typed state carried into `ShimHandler` instead of an early `?`.
* Files: `src/shim/mod.rs`, `src/shim/transport.rs`.
* Tests: U1 scenarios (a) and (b) turn green.
* Posture: paired GREEN for U1.

### U4 — GREEN: degraded-session tool error surface

* Changes: `ShimHandler::call_tool` returns a structured `ErrorData` carrying the
  recorded startup failure cause when the session is in the degraded state;
  `list_tools` continues to serve the static catalog.
* Files: `src/shim/transport.rs`.
* Tests: U1 scenario (c) turns green.
* Posture: paired GREEN for U1.

### U5 — GREEN: stderr pinning, exit-code taxonomy, durable startup record

* Changes: pin the tracing writer to stderr for the shim path
  (`fmt::layer().with_writer(std::io::stderr)`); introduce documented exit codes
  distinguishing admission failure, readiness timeout, endpoint derivation
  failure, and transport failure; write a durable startup-failure record under
  the workspace `.engram/` diagnostics location containing timestamp, binary
  version, failure class, and message. No environment values, tokens, paths
  outside the workspace, or credentials are recorded.
* Files: `src/lib.rs`, `src/shim/mod.rs`, `src/errors.rs`.
* Tests: U2 turns green; U1 exit-code assertion turns green.
* Posture: paired GREEN for U2.

### U6 — Docs: operator diagnostic path

* Changes: document the exit-code taxonomy, the startup-failure record location,
  the stdout purity invariant, and the `/mcp show engram` + record correlation
  procedure.
* Files: `docs/troubleshooting.md`.
* Tests: none (documentation unit).
* Posture: docs-only, width-isolated.

### U7 — Runtime verification and closure

* Changes: runtime verification evidence that a real Copilot CLI stdio session
  initializes against a workspace with an intentionally unavailable daemon, plus
  the operational closure record.
* Files: `docs/closure/2026-08-21-870b1aff-runtime-verification.md`.
* Tests: manual runtime verification, recorded.
* Posture: verification-only.

## Dependency Graph

```text
U1 ─┐
U2 ─┼─> U3 ─> U4 ─┐
    └─> U5 ────────┼─> U6 ─> U7
```

* U1 and U2 are parallel RED units with no dependencies.
* U3 depends on U1. U4 depends on U3. U5 depends on U2 (and consumes U1's exit-code assertion).
* U6 depends on U4 and U5. U7 depends on U6.

## Decisions and Rationale

1. **Serve-first over fail-fast.** MCP has no pre-`initialize` channel for
   startup errors. Answering `initialize` first is the only way to make a
   startup failure attributable to the client.
2. **Degraded session over silent success.** `tools/call` must fail loudly with
   the real cause; a shim that appears healthy but cannot serve is worse than
   one that fails clearly.
3. **stdout purity as an enforced invariant, not a convention.** U2 makes the
   invariant testable so future logging changes cannot silently regress framing.
4. **Exit-code taxonomy retained.** Even with serve-first, some failures (stdio
   unavailable) still terminate early; those must remain distinguishable.
5. **No Tokio or rmcp change.** Investigation E1–E3 attribute the pipe close to
   child exit. Changing the transport stack would be unfounded.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Serve-first masks a genuinely dead workspace | U4 guarantees every `tools/call` fails with the recorded cause; degraded state is never silent. |
| Startup-failure record leaks sensitive data | U5 restricts fields to timestamp, version, failure class, and sanitized message; no env dump, no paths outside workspace. Asserted in U1. |
| Reordering startup changes daemon spawn timing | U3 keeps `ensure_daemon_running` semantics unchanged; only its position relative to transport binding moves. |
| Windows-only regression coverage may not run in Linux CI | U1/U2 are cross-platform by construction; Windows-specific assertions are `#[cfg(windows)]` gated and required in the runtime verification of U7. |
| Version skew (E4) recurs | U5's record includes binary version so skew is immediately visible; operator upgrade guidance in U6. |

## Plan Hardening Signals (REQUIRED)

* Public API, schema, or contract change: **present** — the MCP stdio startup
  contract and a new exit-code taxonomy are externally observable contracts.
* Security, auth, permission, or compliance-sensitive behavior: **present
  (bounded)** — the startup-failure record writes diagnostics to disk and must
  not capture credentials, tokens, or environment values.
* Migration, backfill, destructive data/config action, or irreversible step:
  **absent** — no migration, no data mutation, no deletion.
* External integration, operator checkpoint, or external dependency: **present**
  — GitHub Copilot CLI is the external MCP client whose behavior defines success.
* High runtime, rollout, or rollback risk: **present** — this reorders process
  startup for the primary agent-facing entry point.

Requires plan hardening: **yes**

## Runtime Verification and Closure

| Unit | Runtime surface | Verification | Closure artifact |
|---|---|---|---|
| U3, U4 | MCP stdio entry point | Real Copilot CLI session initializes and lists tools against a workspace with a failing daemon | U7 closure record |
| U5 | Process exit + on-disk diagnostics | Induced failure produces the documented exit code and a record with no sensitive fields | U7 closure record |
| U6 | Operator documentation | Operator can go from `os error 232` to root cause using only the documented path | U7 closure record |

Monitoring plan, pre-deploy audit, observation window, and rollback triggers are
specified in the Plan Hardening section below.

## Plan Hardening

Triggered by four hardening signals (contract change, bounded security surface,
external integration, high runtime risk).

### Protected Invariants

1. stdout on the shim path carries JSON-RPC framing bytes and nothing else.
2. A spawned shim either answers `initialize` or exits with a documented,
   attributable code plus a stderr line.
3. No `tools/call` succeeds while the session is in a degraded startup state.
4. The startup-failure record never contains credentials, tokens, environment
   variable values, or paths outside the workspace root.

### Risky Actions

| ProposedAction | targets | change_kind | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|---|
| Reorder shim startup so transport binds before daemon preconditions | `src/shim/mod.rs`, `src/shim/transport.rs` | local edit to primary agent entry point | high | revert U3/U4 commits; startup order is a single function's control flow | no (non-destructive, reversible) | planned |
| Pin tracing writer to stderr | `src/lib.rs` | config change affecting all binaries | moderate | revert; daemon logging target changes from stdout to stderr | no | planned |
| Write startup-failure diagnostics to `.engram/` | `src/shim/mod.rs` | local file write inside workspace | moderate | delete record; feature-gated by failure path only | no | planned |
| Introduce non-zero exit-code taxonomy | `src/bin/engram.rs`, `src/errors.rs` | contract change | moderate | revert; consumers currently only distinguish zero/non-zero | no | planned |

No `destructive` action is present in this plan. No operator approval gate is required.

### Reinforced Verification

* U1 MUST assert the *absence* of sensitive fields in the startup record, not
  only the presence of expected fields.
* U2 MUST run with `RUST_LOG=engram=debug` set to prove the stderr pin holds
  under the configuration most likely to regress it.
* U3 MUST NOT change `ensure_daemon_running` timeout semantics; a diff touching
  `src/shim/lifecycle.rs` is out of scope for this release unit.
* U7 runtime verification MUST be performed on Windows, since the reported
  failure is Windows-pipe-specific.

### Monitoring Plan

| SLI | Where observed | Baseline | Alert threshold | Owner |
|---|---|---|---|---|
| Shim `initialize` success rate | Copilot CLI session start; absence of `os error 232` | 100% on healthy workspace | any occurrence of pre-initialize pipe close | Ship agent during validation window |
| Startup-failure record count | `.engram/` diagnostics record | 0 on healthy workspace | >0 records referencing admission or readiness | Ship agent |
| stdout framing violations | U2 contract test in CI | 0 | any failure | CI |

### Pre-Deploy Audit

* No feature flag required; the change is behavioral on a single entry point.
* Rollback procedure: revert the U3–U5 commits; startup returns to fail-fast
  ordering. No data or schema state to unwind.
* No migration, no schema change, no backward-compatibility concern.
* Dependent surface: `start.ps1` launcher pre-warm is unaffected because it uses
  CLI subcommands, not the shim path.

### Post-Deploy Observation Window

Duration: one full operator Copilot CLI session plus 24 hours. Owner: Ship agent
during runtime verification, then operator. Outcome recorded in the U7 closure
artifact as healthy, degraded, or rolled back.

### Rollback Triggers

1. Any Copilot CLI session reports `failed to initialize MCP client` after the
   change — revert immediately.
2. Any non-JSON-RPC byte observed on shim stdout — revert immediately.
3. A `tools/call` succeeds while the daemon is known unavailable (false-healthy)
   — revert U4.

## Plan Review

Gate: **PASS**

Personas dispatched: Architecture Lens, Security Lens, Test Strategy Lens,
Operational Readiness Lens.

### Findings

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| F1 | Architecture | P2 | Serve-first introduces a shim session state machine; without a named type the degraded state risks being represented as a loose `Option<String>`. | Accepted into U3 acceptance criteria: startup outcome MUST be a named enum, not an ad-hoc option. |
| F2 | Security | P1 | Writing a startup-failure record to disk is a new data-egress surface; the original plan named the fields but did not make their absence testable. | Resolved in hardening: U1 now asserts absence of sensitive fields. Gate-clearing. |
| F3 | Security | P2 | Pinning tracing to stderr changes daemon log destination, which could break an operator's log capture. | Accepted; U6 documents the destination change. |
| F4 | Test Strategy | P1 | Original U1 asserted only that `initialize` succeeds; a shim that answers `initialize` then silently accepts tool calls would pass. | Resolved: U1 scenario (c) and invariant 3 require `tools/call` to fail in degraded state. Gate-clearing. |
| F5 | Test Strategy | P2 | `#[cfg(windows)]` assertions will not execute in the Linux CI runner, so the Windows-specific contract is unverified by CI alone. | Accepted; U7 makes Windows runtime verification mandatory rather than optional. |
| F6 | Operational Readiness | P2 | No rollback trigger originally distinguished "false-healthy" from "still broken". | Resolved: rollback trigger 3 added. |
| F7 | Architecture | P3 | `Duration::from_secs(60)` IPC timeout in `run_shim` is unexplained. | Advisory only; out of scope for this release unit. |

No P0 findings. Both P1 findings were resolved during hardening before the gate
decision. Decomposition satisfies the 2-hour rule and width isolation: U1/U2 are
test-only, U3/U4/U5 are production code in a single subsystem, U6 is docs-only,
U7 is verification-only.

Review-fix cycles used: 1 of 3.
