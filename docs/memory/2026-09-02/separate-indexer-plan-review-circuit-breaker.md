---
type: circuit-breaker
timestamp: 2026-09-02T23:42:18Z
agent: orchestrator
skill: plan-review
breaker_type: skill-managed
operation: separate-indexer plan review and remediation
attempts: 5
---

# Separate Indexer Plan Review Circuit Breaker

## Failure Chain

### Attempt 1

The original plan failed on an agent-invocable privileged indexer and a
path-only generation swap that did not pin database handles.

### Attempt 2

Revision 1 failed on control-plane authorization, immutable database open,
publication ordering, reader lifetime, error propagation, and oversized units.

### Attempt 3

Revision 2 failed on incomplete read-path migration, request-path activation
latency, supervisor packaging, publication bounds, and incomplete HTTP/SSE
retirement.

### Attempt 4

Revision 3 failed on generation context layering, missing error-foundation
dependencies, supervisor crate wiring, and missing RED harness declarations.

### Attempt 5

Revision 5 confirmation retained two P1 findings:

* the supervisor RED harness precedes workspace-member wiring;
* `src/daemon/ipc_server.rs` has five unordered unit owners, including two
  competing request-admission changes.

## Context

* Requirements:
  `docs/research/2026-09-02-engram-read-only-snapshot-mode-requirements.md`
* Decision:
  `docs/decisions/2026-09-02-separate-indexer-read-server-deliberation.md`
* Plan:
  `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`
* Diagnostic script:
  `scripts/diagnose-engram.ps1`
* Files modified in this session: requirements and plan
* Production source modified: none
* Tests run: none; planning artifacts only
* Resolution: circuit breaker triggered; implementation and harvest blocked
* Resume choice A: wire workspace membership before supervisor RED tests and
  serialize `ipc_server.rs` ownership as
  `F04 -> F18 -> F20 -> F40 -> F44`
* Resume choice B: split `ipc_server.rs` by startup, request entry, error
  transport, and lifecycle policy before harvesting
* Recommended next step: choose B for clearer module ownership, then run one
  bounded Rust and architecture confirmation before harvest
