---
title: "Plan Review: Engram Effectiveness Metrics & Token Usage Tracking"
date: 2026-03-27
plan: ".backlog/plans/2026-03-27-engram-effectiveness-metrics-plan.md"
gate: fail
reviewers: [constitution-reviewer, rust-safety-reviewer, architecture-strategist, scope-boundary-auditor]
---

# Plan Review: Engram Effectiveness Metrics & Token Usage Tracking

## Gate Decision: FAIL

2 P0 and 7 P1 findings require plan revision before proceeding to the backlog harvester.

## Summary

| Severity | Count | Action |
|----------|-------|--------|
| P0       | 1     | Must fix |
| P1       | 7     | Should fix |
| P2       | 8     | Advisory |
| P3       | 3     | Informational |
| **Total**| **19**| |

3 scope-boundary-auditor findings were dismissed as false positives (referenced requirements R18/R21 and Units 9-11 that do not exist in the plan or requirements).

## Findings

### P0: Critical (must fix before proceeding)

**F1. Branch-switch signaling gap in MetricsCollector**
The `tokio::sync::mpsc` channel carries `UsageEvent` items, but the background writer task has no mechanism to learn about branch changes. After a workspace sync detects a new branch, events will continue writing to the old branch's file until the writer is somehow notified.

Resolution: Define an enum `MetricsMessage { Event(UsageEvent), SwitchBranch(String), Shutdown }` as the channel item type. The dispatch path sends `Event`, workspace-sync sends `SwitchBranch`, and graceful shutdown sends `Shutdown` to drain the buffer. This also resolves the unspecified graceful shutdown semantics (see F5).

Affects: Unit 2, Unit 4

### P1: High (should fix before proceeding)

**F2. Error code mismatch between Unit 5 and Unit 6**
Unit 5 says "use error code 13001" for no-metrics-for-branch, but Unit 6 defines 13001 as `METRICS_WRITE_FAILED` and 13002 as `METRICS_NOT_FOUND`. Contract test references 13001 for the wrong condition.

Resolution: Unit 5 should reference 13002 (`METRICS_NOT_FOUND`). The no-workspace case should use existing 1001 (`WORKSPACE_NOT_SET`), not a metrics code.

Affects: Unit 5, Unit 6

**F3. HashMap produces non-deterministic JSON key ordering**
`MetricsSummary.by_tool: HashMap<String, ToolMetrics>` violates Constitution IX ("File formats MUST minimize merge conflicts (sorted keys, stable ordering)"). Every flush produces different key orderings in `summary.json`.

Resolution: Use `BTreeMap<String, ToolMetrics>` for deterministic alphabetical key ordering.

Affects: Unit 1

**F4. Tuple types produce opaque JSON arrays**
`top_symbols: Vec<(String, u32)>` and `time_range: (String, String)` serialize as positional arrays (e.g., `[["foo", 5]]`). Every other MCP response in this codebase uses named fields.

Resolution: Replace with `Vec<SymbolCount>` where `SymbolCount { name: String, count: u32 }` and `TimeRange { start: String, end: String }`.

Affects: Unit 1

**F5. Missing Complexity Tracking table**
Constitution Governance requires plans to document justified violations in a Complexity Tracking table. PD3 (append mode vs. atomic write for `usage.jsonl`) deviates from Principle IX. The justification is sound but not formally recorded.

Resolution: Add a Complexity Tracking table with PD3 entry: Principle IX deviation, justification (atomic write counterproductive for append-only log), and the rejected alternative (read-modify-write cycle).

Affects: Plan-level

**F6. Serialization error handling in dispatch instrumentation**
`serde_json::to_string(&value)` is fallible. The plan doesn't specify how the error is handled. Using `unwrap()` violates the constitution; using `?` makes a metrics failure kill the tool response.

Resolution: Use `value.to_string()` (infallible `Display` impl on `serde_json::Value`) or `serde_json::to_string(&value).map_or(0, |s| s.len())` for graceful fallback.

Affects: Unit 3

**F7. Double-sanitization of branch names**
`resolve_git_branch()` already calls `sanitize_branch_for_path()` before returning, so `WorkspaceSnapshot.branch` is already sanitized. Calling `sanitize_branch_for_path()` again on it would turn `feature__foo` into `feature____foo`.

Resolution: When reading branch from `WorkspaceSnapshot`, use it directly as the directory name. Only call `sanitize_branch_for_path()` on raw git branch names. Document this invariant.

Affects: Unit 2, Unit 4

**F8. No automated tests for PowerShell scripts**
Units 7 and 8 specify "manual validation" only. The constitution requires tests before implementation for every feature. The verification criteria (valid JSONL, correct filters, reasonable token estimates) are automatable.

Resolution: Either add Pester test files or a `-Validate` switch that runs against fixture data and asserts output schema conformance. Alternatively, document in Complexity Tracking table as a justified deviation with commitment to add validation before feature completion.

Affects: Unit 7, Unit 8

### P2: Moderate (advisory, user discretion)

**F9. MetricsCollector lifecycle not specified**
The plan doesn't specify how the background writer task is started, registered in AppState, or shut down gracefully. Risk of metrics not flushed on daemon shutdown.

Resolution: Store the `JoinHandle` in `AppState`. Before `flush_all_workspaces()` in shutdown path, send `Shutdown` message (per F1) and await the handle. Use `OnceLock<mpsc::Sender<MetricsMessage>>` for the global sender (no Mutex needed since `try_send` takes `&self`).

**F10. Concurrent read during append**
`compute_summary()` reading `usage.jsonl` while the background writer appends may encounter a half-written final line, causing parse failure.

Resolution: Line-by-line reader that silently discards the final line if it fails to parse. Document as intentional tolerance for concurrent append.

**F11. connection_id not available in dispatch**
`dispatch()` receives only `SharedState` and `method`/`params`. The SSE connection UUID is known at the handler level but not threaded through.

Resolution: Omit `connection_id` from Phase 1 (field is already `Option`). Threading it through dispatch would change the signature and all call sites.

**F12. Proptest coverage for new serializable models**
The codebase has `proptest_models.rs` establishing a pattern. New models (`UsageEvent`, `MetricsSummary`) should have proptest round-trip coverage.

Resolution: Add proptest cases to existing `tests/unit/proptest_models.rs` or a new file.

**F13. clippy::cast_precision_loss for avg_tokens computation**
`ToolMetrics.avg_tokens: f64` requires integer-to-float casts. The codebase handles this with `#[allow(clippy::cast_precision_loss)]` in `query_stats.rs`.

Resolution: Document that `compute_summary()` needs the same annotation.

**F14. METRICS_BUFFER_FULL error code may be dead code**
If channel-full is handled as `tracing::trace!()` only, the 13004 error code would never appear in a JSON-RPC response. Clippy may flag the enum variant as dead code.

Resolution: Remove METRICS_BUFFER_FULL as an error code. Handle channel-full as a trace event only.

**F15. Tracing instrumentation points not specified**
Units 2 and 4 don't specify where `#[instrument]` annotations or `tracing::info_span!` placements go. The constitution requires spans for lifecycle events.

Resolution: Add explicit instrumentation points: span on background writer loop, `tracing::trace!` on event drop, `tracing::info!` on branch switch, `tracing::warn!` on write failures.

**F16. Requirements doc says `--` but codebase uses `__` for branch sanitization**
The requirements doc (R5) says "replace `/` with `--`" but `sanitize_branch_for_path()` uses `__`. The plan correctly follows the codebase.

Resolution: Update requirements doc R5 to match existing convention (`__` not `--`).

### P3: Low (informational)

**F17. Cargo.toml [[test]] blocks**
Every new test file needs a `[[test]]` entry. The Learnings Applied section covers this but individual units don't call it out.

**F18. Rustdoc on all public items**
New public structs and methods need `///` comments to pass clippy pedantic.

**F19. Scripts must replicate branch sanitization**
PowerShell scripts must implement their own `/` → `__` replacement to construct correct directory paths.

## Reviewer Attribution

| Finding | Reviewer | Model |
|---------|----------|-------|
| F1 | Rust Safety Reviewer | Claude Sonnet 4 |
| F2 | Rust Safety Reviewer | Claude Sonnet 4 |
| F3 | Constitution Reviewer | Claude Sonnet 4 |
| F4 | Rust Safety Reviewer | Claude Sonnet 4 |
| F5 | Constitution Reviewer | Claude Sonnet 4 |
| F6 | Rust Safety Reviewer + Constitution Reviewer | Claude Sonnet 4 |
| F7 | Rust Safety Reviewer | Claude Sonnet 4 |
| F8 | Constitution Reviewer | Claude Sonnet 4 |
| F9 | Architecture Strategist + Rust Safety Reviewer | GPT-4.1 + Claude Sonnet 4 |
| F10 | Rust Safety Reviewer | Claude Sonnet 4 |
| F11 | Rust Safety Reviewer | Claude Sonnet 4 |
| F12 | Constitution Reviewer | Claude Sonnet 4 |
| F13 | Rust Safety Reviewer | Claude Sonnet 4 |
| F14 | Rust Safety Reviewer | Claude Sonnet 4 |
| F15 | Constitution Reviewer | Claude Sonnet 4 |
| F16 | Constitution Reviewer | Claude Sonnet 4 |
| F17 | Constitution Reviewer | Claude Sonnet 4 |
| F18 | Rust Safety Reviewer | Claude Sonnet 4 |
| F19 | Rust Safety Reviewer | Claude Sonnet 4 |

## Dismissed Findings

3 findings from the Scope Boundary Auditor (GPT-4.1) were dismissed as false positives:

1. "P0: Phase 2 scope creep, Units 9-11, UI" — The plan has 8 units and no UI. The auditor hallucinated content.
2. "P2: R18 CSV export, R21 opt-out not covered" — These requirements do not exist in the requirements document.
3. "P1: Two-phase reporting complexity in Unit 6" — Unit 6 is Error Codes, not reporting. No two-phase reporting exists.

## Next Steps

All P0 and P1 findings (F1-F8) have been resolved in the plan. The gate can be re-evaluated as **PASS**.

1. **Run `backlog-harvester`** to decompose this plan into backlog tasks for implementation
2. **Revise further** if any P2 advisory findings need addressing
