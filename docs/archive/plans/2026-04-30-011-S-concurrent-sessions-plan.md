---
title: "011-S Concurrent Session Validation Plan"
description: "Implementation plan for 001-F concurrent agent session characterization and 003-F closure"
source: "docs/decisions/2026-04-30-011-S-daemon-reliability-deliberation.md"
---

## Problem Frame

Multiple AI agent sessions (VS Code Copilot, CLI agents, Cursor, etc.) may
invoke the engram shim simultaneously against the same workspace daemon. The
architecture already supports this via stateless shim connections and tokio
async accept loop, but no tests prove concurrent safety and no documentation
describes the model. Feature 003-F (code-graph co-location) is already resolved
by schema 4.0.0 and needs formal closure.

Key code paths:
- `src/shim/ipc_client.rs` — stateless per-connection client
- `src/shim/lifecycle.rs` — daemon spawn race handling
- `src/daemon/ipc_server.rs` — concurrent accept loop
- `src/server/state.rs` — `AppState` with `RwLock`/atomic concurrency primitives

## Requirements Trace

| Requirement | Source | Implementation Action |
|---|---|---|
| Prove concurrent tool calls are safe | 001-F + deliberation | Integration tests with parallel shim invocations |
| Document concurrency model | 001-F + deliberation | Architecture docs section |
| Close 003-F with rationale | Deliberation decision | Update backlog item status + note |

## Implementation Units

### Unit 1: Concurrent tool-call characterization tests

**Domain**: tests
**Files**: `tests/integration/concurrent_sessions_test.rs` (new)
**Execution posture**: test-first (the tests ARE the deliverable)

Write integration tests that:
1. Spawn a daemon for a test workspace
2. Issue N parallel tool calls from separate tokio tasks (simulating concurrent shims)
3. Assert all calls return valid responses without corruption
4. Assert `indexing_in_progress` serializes concurrent index requests

Test scenarios (≤3):
- 3 concurrent `list_tools` calls succeed without response corruption
- 2 concurrent `call_tool` (get_daemon_status) calls return consistent state
- Concurrent `sync_workspace` + `call_tool` — index serialization holds

Note: `active_connections` and `RateLimiter` are SSE-transport concerns
(US5/T091, FR-025/T118) and are out of scope for IPC characterization.

**Verifies**: Architecture handles concurrent access without panics, data races,
or response corruption.

### Unit 2: Concurrency model documentation

**Domain**: docs
**Files**: `docs/architecture.md` (append section)
**Execution posture**: docs-only, no code changes

Document:
- The stateless shim connection model (one connection per tool call)
- Daemon's async accept loop and per-connection task spawning
- `AppState` concurrency primitives and their guarantees (RwLock, AtomicBool)
- `indexing_in_progress` serialization behavior
- Recommendations for multi-agent workspace configuration

Note: Rate limiter and `active_connections` are SSE-transport concerns and
belong in SSE documentation, not IPC concurrency documentation.

### Unit 3: Close 003-F with architectural rationale

**Domain**: config/backlog
**Files**: `.backlogit/queue/003-F.md` (status update)
**Execution posture**: backlog mutation only

Move 003-F to `done` status with a note explaining:
- Schema 4.0.0 made code-graph branch-aware
- Current separation of tracked JSONL vs untracked DB is intentional
- Co-location would break the persistence model

## Dependency Graph

```
Unit 1 (tests) ──→ Unit 2 (docs)
                       │
Unit 3 (closure) ─────┘ (independent, can run in parallel)
```

Unit 1 should run first because test results inform the documentation claims.
Unit 3 is independent and can execute any time.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Integration tests, not unit tests | Concurrency bugs manifest at the connection level, not within isolated functions |
| Use tokio tasks, not OS processes | Lighter weight, deterministic, sufficient to prove the daemon handles concurrent IPC connections |
| Close 003-F rather than defer | The work was completed by schema 4.0.0; leaving it open creates false impression of remaining work |
| Append to architecture.md, not new file | Concurrency model is an architectural property, not a standalone document |

## Risks and Caveats

| Risk | Likelihood | Mitigation |
|---|---|---|
| Concurrent tests are flaky in CI | Medium | Use synchronization barriers (channels/semaphores), not sleeps; ensure daemon is fully ready before parallel calls |
| Tests discover a real race condition | Low | If found, file a separate bug and block the test on a fix |
| Documentation claims become stale | Low | Tests enforce the claims — if architecture changes, tests fail first |

## Plan Hardening Signals

- Public API, schema, or contract change: **No** — tests and docs only
- Security, auth, permission, or compliance-sensitive: **No**
- Migration, backfill, destructive action: **No**
- External integration or operator checkpoint: **No**
- High runtime, rollout, or rollback risk: **No**

Requires plan hardening: no

## Runtime Verification and Closure

### Unit 1 (tests)
- **Changes runtime surface**: No — test-only
- **Verification**: `cargo test concurrent_sessions` passes locally and in CI
- **Closure**: Test results documented in shipment closure artifact

### Unit 2 (docs)
- **Changes runtime surface**: No — documentation only
- **Verification**: Architecture section renders correctly, factual claims match test evidence
- **Closure**: None required beyond review

### Unit 3 (closure)
- **Changes runtime surface**: No — backlog metadata only
- **Verification**: 003-F status shows `done` in backlog
- **Closure**: Note preserved for future reference

## Constitution Check

| Principle | Compliance |
|---|---|
| I. Safety-First Rust | N/A — no production code changes |
| II. Test-First Development | Unit 1 IS the test deliverable |
| III. Workspace Isolation | Tests use isolated temp workspaces |
| IV. CLI Containment | All outputs within repo tree |
| VI. Single Responsibility | Each unit targets one domain |

## Plan Review

**Gate Decision: PASS**

Review conducted 2026-04-30 with four always-on personas: Constitution Reviewer,
Rust Reviewer, Scope Boundary Auditor, Learnings Researcher.

### Initial Review (pre-revision)

The initial plan included SSE-transport concerns (rate limiter, `active_connections`
counter) in the IPC characterization scope. Three P1 findings identified scope leakage:

- **[P1] Rust Reviewer**: `active_connections` is only incremented via
  `register_connection()` (US5/T091 — SSE-only). IPC server does not call it.
- **[P1] Rust Reviewer**: `RateLimiter` (FR-025/T118) is enforced only in the SSE
  handler, not in IPC server.
- **[P1] Scope Boundary Auditor**: Plan leaked SSE-only concerns into an IPC-focused
  shipment.

### Revision Applied

Plan revised to remove SSE-transport concerns from Unit 1 and Unit 2:
- Removed connection counter assertion from test scenarios
- Removed rate limiter test scenario
- Removed rate limiter from documentation scope
- Added explicit out-of-scope notes for both SSE concerns

### Post-Revision Assessment

| Persona | Findings after revision |
|---|---|
| Constitution Reviewer | No findings. Plan complies with all principles. |
| Rust Reviewer | No findings. Scope limited to IPC concurrency primitives. |
| Scope Boundary Auditor | No findings. Units isolated to their domains. |
| Learnings Researcher | No findings. No contradictions with `docs/compound/`. |

### Plan Hardening

- Required: **No** (all hardening signals absent)
- Satisfied: N/A

### Summary

Plan is sound after revision. Three units stay within scope: IPC concurrency
characterization tests, architecture documentation, and backlog closure. No
runtime surfaces changed. Proceed to harvest.
