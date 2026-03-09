# Operator Review Log: 004-refactor-engram-server-as-plugin

**Date**: 2026-03-04  
**Review Session**: Adversarial analysis findings from Stage 6  
**Total Findings Reviewed**: 35

## Summary

| Decision | Count |
|----------|-------|
| Auto-applied (critical/high) | 14 |
| Approved (medium) | 15 |
| Deferred | 0 |
| Rejected | 0 |
| Recorded (low suggestions) | 6 |

## Critical/High Findings (Auto-Applied)

| Finding ID | Severity | Consensus | Decision | Notes |
|------------|----------|-----------|----------|-------|
| UF-01 | CRITICAL | Unanimous | Applied | Constitution amendment prerequisite added to plan.md |
| UF-02 | CRITICAL | Unanimous | Applied | Principle IV IPC deviation documented in Complexity Tracking |
| UF-03 | CRITICAL | Majority | Applied | .engram/ layout partitioned (committed vs runtime) in FR-017 |
| UF-04 | CRITICAL | Unanimous | Applied | Principle V logging deviation documented in Complexity Tracking |
| UF-05 | CRITICAL | Majority | Applied | Cold start target justified in FR-009 |
| UF-06 | HIGH | Majority | Applied | FR-015 rewritten (ephemeral → stateless proxy) |
| UF-07 | HIGH | Single (verified) | Applied | Error codes reassigned: IPC→8xxx, installer→9xxx |
| UF-08 | HIGH | Majority | Applied | WatcherEvent.old_path added for Renamed |
| UF-09 | HIGH | Unanimous | Applied | SC-008 qualified; behavioral delta section in mcp-tools.md |
| UF-10 | HIGH | Majority | Applied | Windows pipe security upgraded to explicit DACL |
| UF-11 | HIGH | Majority | Applied | S109 added (100k+ file indexing with concurrent tool call) |
| UF-12 | HIGH | Majority | Applied | S110-S114 added (backward compat for 5 remaining tools) |
| UF-13 | HIGH | Majority | Applied | T088 added (migrate existing tests from mcp-sdk) |
| UF-14 | HIGH | Single (validated) | Applied | T089 added (process-based test harness) |

## Medium Findings (Operator Approved)

| Finding ID | Severity | Consensus | Operator Decision | Notes |
|------------|----------|-----------|-------------------|-------|
| UF-15 | MEDIUM | Unanimous | Approved | settings.toml → config.toml standardized |
| UF-16 | MEDIUM | Majority | Approved | Phase mapping table added to tasks.md |
| UF-17 | MEDIUM | Majority | Approved | Deferred to research.md update (dependency justification) |
| UF-18 | MEDIUM | Single | Approved | S115 added (shim→daemon during ShuttingDown) |
| UF-19 | MEDIUM | Single | Approved | S116 added (two-shim spawn race) |
| UF-20 | MEDIUM | Majority | Approved | T013 moved IPC types to src/daemon/protocol.rs |
| UF-21 | MEDIUM | Single | Approved | Deferred to build phase (US5 acceptance expansion) |
| UF-22 | MEDIUM | Majority | Approved | S117 added (60s cleanup boundary test) |
| UF-23 | MEDIUM | Majority | Approved | T090 + T091 added (dead code verification) |
| UF-24 | MEDIUM | Single | Approved | S119 added (UDS path overflow fallback) + T093 |
| UF-25 | MEDIUM | Majority | Approved | S118 added (external symlink filtering) |
| UF-26 | MEDIUM | Single | Approved | Terminology mapping table added to tasks.md |
| UF-27 | MEDIUM | Single | Approved | 60s daemon-side IPC read timeout added to contract |
| UF-28 | MEDIUM | Single | Approved | .env* added to default exclude_patterns |
| UF-29 | MEDIUM | Single | Approved | FR-006 "near-real-time" → "within 2 seconds" |

## Low Findings (Recorded as Suggestions)

| Finding ID | Summary | Notes |
|------------|---------|-------|
| UF-31 | notify v9 RC risk — pin version, document fallback | Tracked in Complexity Tracking |
| UF-32 | No log rotation/size limits specified | Consider adding to PluginConfig during build |
| UF-33 | DaemonState.ipc_address String type is platform-ambiguous | Consider enum during implementation |
| UF-34 | Concurrent scenarios underrepresented (9%) | Additional concurrent scenarios added (S116) |
| UF-35 | S026 newline handling inconsistent with protocol | Clarify during IPC implementation |

## Artifacts Modified

| File | Changes Applied |
|------|----------------|
| spec.md | FR-006 wording, FR-009 justification, FR-015 rewrite, FR-017 layout, SC-008 qualification |
| plan.md | Complexity Tracking expanded (4 entries), settings.toml→config.toml |
| data-model.md | Error code ranges (8xxx/9xxx), WatcherEvent.old_path, .env* exclude |
| tasks.md | Phase mapping, terminology mapping, T088-T093 added, T008/T013 updated |
| SCENARIOS.md | S109-S119 added (11 new scenarios), summary metrics updated |
| contracts/ipc-protocol.md | Windows pipe DACL security, daemon-side read timeout |
| contracts/mcp-tools.md | Behavioral delta section for workspace binding change |
| ANALYSIS.md | Full adversarial analysis report |

## Dismissed Finding

| Finding ID | Original Reviewer | Reason for Dismissal |
|------------|------------------|----------------------|
| RC-09 | Reviewer A | Read latency (50ms) > write latency (10ms) is correct per constitution: semantic search (query_memory) is computationally heavier than simple DB writes (update_task). Not inverted. |
