---
title: "Sticky proxy readiness recovery"
date: 2026-08-26
type: session-memory
---

## Outcome

Completed task `136.001-T` under reliability feature `136-F`.

The root cause was a sticky stdio-proxy startup result. The proxy cached its
initial daemon readiness timeout and never probed the named-pipe daemon again,
even after that daemon became healthy. The proxy now keeps transient readiness
failures recoverable, publishes recovered state to every request handler, and
preserves fail-closed behavior for terminal startup failures.

## Files Modified

* `src/shim/mod.rs`
* `src/shim/transport.rs`
* `src/daemon/ipc_server.rs`
* `src/db/cozo_backend/mod.rs`
* `tests/contract/shim_stdio_initialize_test.rs`
* `docs/decisions/2026-08-26-large-multi-repo-workspace-scale-spike.md`
* `docs/troubleshooting.md`
* `.backlogit/archive/136-F.md`
* `.backlogit/archive/136.001-T.md`

## Decisions

* Retain the stdio proxy to workspace-scoped named-pipe daemon architecture
* Treat only `DaemonError::NotReady` as recoverable
* Continue bounded readiness monitoring after the initial startup budget
* Permit request-triggered recovery through a session-wide single-flight probe
* Apply a short cooldown after failed probes to avoid request amplification
* Abort unresolved startup work when the MCP client disconnects
* Emit retry metadata so agents can distinguish transient from terminal errors
* Add structured database startup timings before redesigning the transport

## Verification

* `cargo fmt --all -- --check` passed
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` passed
* `cargo dev-test` passed
* `cargo audit` passed with 14 pre-existing allowed warnings
* The shim contract suite passed all five startup, recovery, and teardown cases
* Backlog feature `136-F` and task `136.001-T` passed their completion gates
  and moved to `done`

## Failed Approaches

* Inspecting only the original MCP session suggested the daemon was still down
* Direct named-pipe health and a fresh shim proved the daemon had recovered
* Increasing the readiness timeout alone would not repair a session after a
  legitimate slow startup
* Backlog index refresh remains blocked by the pre-existing conflict markers in
  `.backlogit/stash.jsonl`

## Context Compaction

The `compact-context` assessment found 157 memory files totaling 499,475
bytes, including 60 files older than 14 days. No active task checkpoint was
compacted. Archival was not attempted because moving existing files requires
destructive-operation approval.

## Open Questions

* Post-ready daemon restart recovery remains separate work
* A controlled 5,000-file index-and-query release gate does not yet exist
* Multi-repository federation remains unsupported; separate repository roots
  still create separate daemons

## Next Steps

* Resolve the pre-existing `.backlogit/stash.jsonl` merge conflict before commit
* Commit and push the verified change set after the conflict is resolved
* Add follow-up work for post-ready daemon restart recovery
* Add a release-mode 5,000-file index-and-query benchmark
