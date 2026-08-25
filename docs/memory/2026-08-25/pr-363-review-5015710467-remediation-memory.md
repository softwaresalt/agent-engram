---
title: PR 363 review 5015710467 Stage memory
type: session-memory
doc_type: memory
source: PR 363 reviews 5015636140 and 5015710467
date: 2026-08-25
agent: stage
branch: stage/dark-factory-cycle2-20260824-1540
pull_request: 363
starting_head: a45ffb3035cf08698c33fc22445a58eb409842cb
status: remediation-in-progress
---

# PR 363 review 5015710467 Stage memory

## Completed planning work

- Retrieved exact review `5015710467`, its provider-lifecycle thread, all four currently unresolved Copilot threads, affected paths, and the suppressed daemon-key edge finding.
- Verified `Cargo.lock` pins `opentelemetry_sdk` and `tracing-opentelemetry` 0.26.0, then read their local registry source without running Cargo build/check/test/lint.
- Corrected the provider RED: layer-held tracer/provider retention is an already-GREEN baseline; missing explicit application lifecycle/flush control and source-owned export timeout are the RED owned by 131.006-T.
- Corrected the endpoint RED: a parent relaunches child tests with `Command::env`/`env_remove`; child code only reads inherited environment. 131.008-T owns endpoint GREEN and 131.009-T owns attachment/lifecycle-handle retention.
- Corrected daemon-key fan-in from four to exactly three prerequisite edges while preserving all blocked states and dependencies.
- Preserved tasks `131.001-T` through `131.013-T`, the fourteen-item `125-S` roster, twelve OTLP task edges, 45-115 minute widths, sole queued/unclaimed `125-S`, and blocked `126-S` through `129-S`.
- Withdrew stale eight-task/nine-item/1,121-artifact metadata and produced a copy-ready title/body with the exact thirteen-task/fourteen-item/twelve-edge/1,126-artifact facts.

## Decisions

1. SDK 0.26 `TracerProvider::library_tracer` clones the provider into `Tracer`; `OpenTelemetryLayer` stores the tracer. Dropping only the constructor-local binding does not stop processing.
2. A separate application provider handle is still required for explicit force-flush/shutdown invocation and result reporting, but not for layer liveness.
3. Safe endpoint precedence tests isolate environment before child process startup. `set_var`, `remove_var`, unsafe blocks, serial environment locks, and process-global test mutation are forbidden.
4. Every RED command compiles first; provider RED fails on `LifecycleUnavailable`/missing timeout for 131.006-T, while daemon RED fails at runtime on parser/resolution/handoff/attachment behavior for 131.008-T/131.009-T.
5. The daemon-key graph is exactly `U1 -> U3`, `U2 -> U3`, `U3 -> U4`; the missing safe create-and-retain primitive still blocks implementation.

## Files modified

Only active backlog planning artifacts `125-S`, `131-F`, and `131.003-T` through `131.008-T`; the two reviewed plans; the OTLP decision; directly coupled PR metadata/remediation records; this memory; and backlogit tool-managed metadata. No source, test, Cargo, lockfile, config, workflow, or PR #362 file/state changed.

## Tooling and failed approaches

- Root MCP and CLI sync failed on 19 parse errors in the dirty main worktree; target-worktree CLI sync succeeded with 1,126 artifacts and zero parse failures.
- Engram semantic search was healthy but did not surface the planning records needed for exact text; targeted file reads followed the required fallback order.
- `rg` was unavailable; targeted PowerShell `Select-String` was used only after Engram proved insufficient.
- One exploratory SQL query used a nonexistent `type` column; dependency-aware CLI reads still confirmed the daemon-key edges.

## Pending closure

- Verify all edited artifacts, exact safe semantics, source citations, graphs, widths, roster, docs/frontmatter/references, and planning-only diff.
- Run target/full doctor and final target index sync.
- Commit and push normally with trailers, reply to and resolve four threads, record the suppressed finding, then update this memory and closure with exact publication/thread evidence.
- Do not claim or close any shipment and do not alter PR #362.
