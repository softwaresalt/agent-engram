---
title: "Engram Effectiveness Metrics & Token Usage Tracking"
date: 2026-03-27
origin: ".backlog/brainstorm/2026-03-27-engram-effectiveness-metrics-requirements.md"
status: draft
---

# Engram Effectiveness Metrics & Token Usage Tracking

## Problem Frame

Engram returns precise, pre-indexed results to AI coding assistants, but there is no way to measure whether this saves tokens compared to file-based search. The daemon already tracks tool call counts and latency percentiles, and every indexed symbol carries a pre-computed `token_count` field. The missing piece is an accounting layer that connects tool responses to context window impact, persists measurements per feature branch, and supports comparison against non-engram baselines captured from real Copilot CLI sessions.

## Requirements Trace

| # | Requirement | Origin |
|---|---|---|
| R1 | Every MCP tool response records a UsageEvent with tool name, timestamp, payload bytes, token estimate, symbols returned, branch | Req 1 |
| R2 | Token estimation uses `body.len() / 4` heuristic | Req 2 |
| R3 | Two-phase approach: Phase 1 volume only, Phase 2 empirical multipliers | Req 3 |
| R4 | Persist to `.engram/metrics/{branch}/usage.jsonl` (append-only JSONL) | Req 4 |
| R5 | Sanitize branch names for filesystem safety | Req 5 |
| R6 | Summary file computed on flush or on demand | Req 6 |
| R7 | `.engram/metrics/` is Git-trackable | Req 7 |
| R8 | Switch metrics path on branch change | Req 8 |
| R9 | Branch summary includes per-tool breakdown, totals, top symbols, time range | Req 9 |
| R10 | `get_branch_metrics` MCP tool with optional comparison | Req 10-12 |
| R11 | Extend `get_health_report` with metrics_summary | Req 13 |
| R12 | `get_token_savings_report` MCP tool | Req 14 |
| R13 | Standardized "search tokens per search sequence" unit | Req 15 |
| R14 | Post-hoc analysis script reads Copilot CLI session store | Req 17 |
| R15 | Unified JSONL schema for engram and baseline records | Req 18 |
| R16 | Codebase normalization metadata | Req 19 |
| R17 | Script accepts `--repository` and `--since` filters | Req 20 |
| R18 | Empirical multiplier computation stratified by turn purpose | Req 21-22 |
| R19 | Calibration report with confidence indicators | Req 23 |
| R20 | Non-blocking event recording via mpsc channel | Req 24 |
| R21 | Respect flush/dehydration lifecycle | Req 25 |
| R22 | Metrics survive hydration/dehydration cycles | Req 26 |

## Scope Boundaries

### In Scope

- UsageEvent model and async metrics collector service
- Dispatch-level instrumentation for all MCP read tools
- Branch-aware JSONL persistence with summary computation
- Two new MCP tools: `get_branch_metrics`, `get_token_savings_report`
- Health report extension with metrics_summary
- Flush lifecycle integration (summary recomputation)
- Hydration support (load existing metrics on startup)
- Post-hoc baseline extraction script for Copilot CLI session store
- Calibration report script for multiplier derivation

### Non-Goals

- Precise tokenization (tiktoken): `len/4` heuristic sufficient (see origin: D1)
- Agent-side context window tracking: engram cannot observe the full window
- Non-Copilot-CLI platforms: scoped to Copilot CLI only (see origin: D6)
- Shadow mode: evaluated and rejected (see origin: D2)
- Write tool tracking: does not contribute to context window
- Automated recommendations: leave interpretation to humans

### Deferred to Implementation

- Exact UsageEvent JSONL field ordering and optional fields
- Whether summary.json includes per-connection breakdowns or just aggregates
- Exact confidence threshold numbers for calibration report

## Implementation Units

### Unit 1: UsageEvent Model and MetricsConfig

**Files:** `src/models/metrics.rs` (new), `src/models/mod.rs` (edit), `src/models/config.rs` (edit)
**Test files:** `tests/unit/metrics_model_test.rs` (new)
**Execution note:** test-first
**Patterns to follow:** `src/models/function.rs` for serde derive patterns; `src/models/config.rs` for MetricsConfig defaults
**Dependencies:** none

**Approach:**

Define the core data structures:

```text
MetricsMessage (channel enum):
  Event(UsageEvent)
  SwitchBranch(String)
  Shutdown

UsageEvent:
  tool_name: String
  timestamp: String (RFC3339)
  response_bytes: u64
  estimated_tokens: u64
  symbols_returned: u32
  results_returned: u32
  branch: String
  connection_id: Option<String>

MetricsSummary:
  total_tool_calls: u64
  total_tokens: u64
  by_tool: BTreeMap<String, ToolMetrics>
  top_symbols: Vec<SymbolCount>
  time_range: TimeRange
  session_count: u32

SymbolCount:
  name: String
  count: u32

TimeRange:
  start: String
  end: String

ToolMetrics:
  call_count: u64
  total_tokens: u64
  avg_tokens: f64

MetricsConfig:
  enabled: bool (default true)
  buffer_size: usize (default 1024)
```

Use `BTreeMap` (not `HashMap`) for `by_tool` to ensure deterministic key ordering in serialized output (Constitution IX: sorted keys). Use named structs (`SymbolCount`, `TimeRange`) instead of tuples to produce semantically clear JSON with named fields, consistent with all other MCP responses in the codebase.

Add `MetricsConfig` to `WorkspaceConfig` following the existing pattern where `PluginConfig::load()` reads from `.engram/config.toml` with defaults on parse failure. Add `metrics: MetricsConfig` field with `#[serde(default)]`.

All structs derive `Debug, Clone, Serialize, Deserialize`. Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields. Add `///` rustdoc comments on all public items following `src/models/function.rs` style. The `avg_tokens` computation needs `#[allow(clippy::cast_precision_loss)]` following the `query_stats.rs` precedent.

**Verification:**

- Unit test: UsageEvent serializes to JSON and round-trips
- Unit test: MetricsSummary computes correctly from a Vec of UsageEvents
- Unit test: MetricsConfig defaults are sensible
- Unit test: MetricsConfig deserializes from partial TOML (missing fields get defaults)
- Unit test: `BTreeMap` produces deterministic key ordering in serialized summary
- Proptest: UsageEvent and MetricsSummary round-trip through serde_json (add to `tests/unit/proptest_models.rs` or new file)
- Cargo.toml: Add `[[test]]` block for new test file

### Unit 2: Metrics Collector Service

**Files:** `src/services/metrics.rs` (new), `src/services/mod.rs` (edit)
**Test files:** `tests/unit/metrics_collector_test.rs` (new)
**Execution note:** test-first
**Patterns to follow:** `src/services/query_stats.rs` for global singleton with `OnceLock<Mutex<T>>`; `src/services/dehydration.rs` for atomic JSONL writes
**Dependencies:** Unit 1

**Approach:**

Create `MetricsCollector` with a `tokio::sync::mpsc` bounded channel (buffer size from `MetricsConfig`). The channel carries `MetricsMessage` (defined in Unit 1), not raw `UsageEvent`. The collector has two roles:

1. **Sender side** (called from dispatch): `record(event: UsageEvent)` wraps the event in `MetricsMessage::Event` and performs non-blocking `try_send` on the channel. If the channel is full, the event is dropped with `tracing::trace!("metrics_event_dropped")`. This ensures zero latency impact on tool calls.

2. **Receiver side** (background task): Spawned via `tokio::spawn` on startup. Reads messages from the channel:
   - `MetricsMessage::Event(e)` → append as JSONL line to active branch's `usage.jsonl`
   - `MetricsMessage::SwitchBranch(b)` → close current file handle, update active branch path, open new file
   - `MetricsMessage::Shutdown` → flush pending writes, exit task loop

   Uses `tokio::fs::OpenOptions` with append mode for `usage.jsonl`.

**Global singleton**: Use `OnceLock<mpsc::Sender<MetricsMessage>>` directly — no Mutex needed since `Sender::try_send` takes `&self`. Store the `JoinHandle<()>` from `tokio::spawn` in `AppState` for graceful shutdown.

**Lifecycle integration**: Initialize `MetricsCollector` at server startup (in `bin/engram.rs` after `AppState::new()`). On graceful shutdown, send `MetricsMessage::Shutdown` and await the `JoinHandle` to drain buffered events before calling `flush_all_workspaces()`.

**Tracing instrumentation**: `#[instrument]` on the background writer loop. `tracing::trace!` on event drop (channel full). `tracing::info!` on branch switch. `tracing::warn!` on write failures.

Branch path management:
- **Important**: `WorkspaceSnapshot.branch` is already sanitized by `resolve_git_branch()` (which calls `sanitize_branch_for_path()` internally). Do NOT call `sanitize_branch_for_path()` again on it — that would turn `feature__foo` into `feature____foo`. Use the branch string directly as the directory name.
- Compute metrics directory: `{workspace}/.engram/metrics/{snapshot.branch}/`
- Create directory on first write via `tokio::fs::create_dir_all`

Summary computation:
- `compute_summary(workspace_path, branch) -> Result<MetricsSummary>` reads `usage.jsonl` line-by-line, deserializes each line, aggregates into MetricsSummary
- **Partial-line tolerance**: Use a line-by-line reader that silently discards the final line if it fails to parse. This handles the concurrent-append case where the background writer may be mid-write.
- Write to `summary.json` using the existing `atomic_write` pattern from dehydration
- Called on flush and on demand from MCP tools

**Verification:**

- Unit test: `record()` does not block when channel is full
- Unit test: UsageEvent serializes to valid JSONL line
- Unit test: `compute_summary()` produces correct aggregates from a test JSONL string
- Unit test: Branch path sanitization matches existing `sanitize_branch_for_path` behavior

### Unit 3: Dispatch Instrumentation

**Files:** `src/tools/mod.rs` (edit)
**Test files:** `tests/contract/metrics_contract_test.rs` (new)
**Execution note:** test-first
**Patterns to follow:** Existing latency recording pattern at end of dispatch (lines 93-94 of mod.rs)
**Dependencies:** Unit 1, Unit 2

**Approach:**

Modify `dispatch()` to measure response payload size after serialization and emit a UsageEvent. The measurement point is after the tool returns its `Result<Value>` and before the function returns:

```text
1. Execute tool (existing match block)
2. On Ok(value):
   a. Measure byte length using `value.to_string().len()` (infallible `Display` impl on `serde_json::Value`) — do NOT use `serde_json::to_string()` which is fallible and would require error handling that could kill the tool response
   b. Estimate tokens = bytes / 4
   c. Extract symbols_returned / results_returned from the value (tool-specific extraction)
   d. Get branch from state.snapshot_workspace()
   e. Get connection_id from state (if available)
   f. Send UsageEvent to MetricsCollector
3. Record latency (existing)
4. Return result (existing)
```

Tool-specific result counting:
- `map_code`: count neighbors array length + 1 (root)
- `list_symbols`: read `total_count` field
- `unified_search`: count results array length
- `impact_analysis`: count code_neighborhood array length
- `query_memory`: count results array length
- `query_graph`: read `row_count` field
- Lifecycle/write tools: skip metrics recording (non-goals)

The extraction logic uses `value.get("field")` on the serde_json::Value — no type-specific deserialization needed.

**Verification:**

- Contract test: dispatch a `list_symbols` call, verify a UsageEvent is recorded with correct tool_name and non-zero response_bytes
- Contract test: dispatch a lifecycle tool (`get_health_report`), verify no UsageEvent is recorded for non-read tools (or verify it IS recorded depending on final decision — health report does return data to agents)
- Contract test: verify UsageEvent.estimated_tokens is approximately response_bytes / 4

### Unit 4: Branch-Aware Persistence and Flush Integration

**Files:** `src/services/metrics.rs` (edit), `src/tools/write.rs` (edit), `src/services/hydration.rs` (edit)
**Test files:** `tests/integration/metrics_persistence_test.rs` (new)
**Execution note:** test-first
**Patterns to follow:** `src/services/dehydration.rs` for atomic writes and flush lock pattern; `src/tools/write.rs` for flush_state sequence
**Dependencies:** Unit 2, Unit 3

**Approach:**

**Flush integration**: After `dehydrate_code_graph()` in `flush_state` (src/tools/write.rs line ~67), call `metrics::compute_and_write_summary(workspace_path, branch)`. This recomputes `summary.json` from the raw `usage.jsonl` log using atomic write.

**Branch switching**: When the daemon detects a branch change (existing `resolve_git_branch()` called during workspace sync), the MetricsCollector switches its active output path. The collector holds the current branch as state; on branch change, it closes the current file handle and opens a new one for the new branch directory.

**Hydration**: On startup, if `.engram/metrics/{branch}/usage.jsonl` exists, the MetricsCollector loads the existing event count as a baseline counter (it does not re-read all events into memory — just notes the file exists for append). Summary is not pre-loaded into memory; it is computed on demand.

**Directory structure:**
```text
.engram/
  metrics/
    main/
      usage.jsonl
      summary.json
    feature__auth-refactor/
      usage.jsonl
      summary.json
```

**Verification:**

- Integration test: Create temp workspace, emit 5 UsageEvents, call flush_state, verify `usage.jsonl` contains 5 lines and `summary.json` is valid
- Integration test: Emit events on branch A, switch to branch B, emit more events, verify separate directories with correct event counts
- Integration test: Restart (recreate MetricsCollector), verify it appends to existing `usage.jsonl` without overwriting
- Integration test: Verify `.engram/metrics/` directory is NOT in `.gitignore` template

### Unit 5: MCP Tools (get_branch_metrics, get_token_savings_report)

**Files:** `src/tools/read.rs` (edit), `src/tools/mod.rs` (edit), `src/shim/tools_catalog.rs` (edit)
**Test files:** `tests/contract/metrics_tools_test.rs` (new)
**Execution note:** test-first
**Patterns to follow:** `get_health_report` in `src/tools/read.rs` for tool structure; `all_tools()` in `src/shim/tools_catalog.rs` for registration
**Dependencies:** Unit 4

**Approach:**

**`get_branch_metrics`** (R10):
- Parameters: `branch_name: Option<String>` (defaults to current), `compare_to: Option<String>`
- Returns MetricsSummary for the requested branch by calling `compute_summary()`
- If `compare_to` is provided, returns both summaries plus a delta section showing differences
- If no workspace is bound, return existing error 1001 (`WORKSPACE_NOT_SET`)
- If no metrics exist for the requested branch, return error 13002 (`METRICS_NOT_FOUND`)

**`get_token_savings_report`** (R12):
- Parameters: none (always uses current branch)
- Returns a formatted text summary: "On branch {branch}, engram delivered {N} tokens across {M} tool calls. Average {avg} tokens per call. Most-queried symbols: {top 5}."
- Phase 2 extension: append savings sentence when multipliers are configured

**Registration**:
- Add both tools to `all_tools()` in `src/shim/tools_catalog.rs` with JSON schemas
- Add dispatch entries in `src/tools/mod.rs`
- Increment `TOOL_COUNT` from 14 to 16

**Health report extension** (R11):
- Add `metrics_summary` field to `get_health_report` response containing: current branch, total tokens delivered, total tool calls tracked by metrics, and time range. Return `null` if no metrics exist yet.

**Verification:**

- Contract test: `get_branch_metrics` returns valid MetricsSummary after recording events
- Contract test: `get_branch_metrics` with non-existent branch returns error 13002 (`METRICS_NOT_FOUND`)
- Contract test: `get_branch_metrics` without workspace returns error 1001 (`WORKSPACE_NOT_SET`)
- Contract test: `get_branch_metrics` with `compare_to` returns both summaries and delta
- Contract test: `get_token_savings_report` returns formatted text
- Contract test: `get_health_report` includes `metrics_summary` field
- Contract test: tool catalog count matches dispatch table (existing test pattern)

### Unit 6: Error Codes

**Files:** `src/errors/codes.rs` (edit), `src/errors/mod.rs` (edit)
**Test files:** `tests/contract/error_codes_test.rs` (edit existing)
**Execution note:** test-first
**Patterns to follow:** Existing error code ranges in `src/errors/codes.rs`; EngramError enum in `src/errors/mod.rs`
**Dependencies:** none (can be done in parallel with Unit 1)

**Approach:**

Add 13xxx error code range for the metrics subsystem:

```text
METRICS_WRITE_FAILED   = 13_001  // Failed to write usage.jsonl or summary.json
METRICS_NOT_FOUND      = 13_002  // No metrics data for requested branch
METRICS_PARSE_ERROR    = 13_003  // Failed to parse existing usage.jsonl
```

Note: Channel-full (buffer overflow) is handled as `tracing::trace!()` only, with no corresponding error code or enum variant. This keeps the error enum honest — every variant must be returnable through the MCP error response path.

Add `Metrics` variant to `EngramError` enum following existing pattern (e.g., `CodeGraph(CodeGraphError)`). Create `MetricsError` enum with corresponding variants.

**Verification:**

- Existing error_codes_test verifies no code collisions — extend it to cover 13xxx range
- Contract test: each error code produces correct JSON response structure

### Unit 7: Baseline Analysis Script

**Files:** `scripts/metrics/Extract-SearchBaseline.ps1` (new), `scripts/metrics/README.md` (new)
**Test files:** `scripts/metrics/Extract-SearchBaseline.Tests.ps1` (new) or `-Validate` switch
**Execution note:** spike (prototype against real data, then refine)
**Patterns to follow:** Existing PowerShell scripts in `scripts/` directory
**Dependencies:** Unit 1 (JSONL schema definition)

**Approach:**

PowerShell script that reads the Copilot CLI session store SQLite database:

```text
Parameters:
  -Repository <string>    Filter by repository (e.g., "softwaresalt/agent-engram")
  -Since <datetime>        Only include sessions after this date
  -OutputPath <string>     File path for JSONL output (default: stdout)
```

Processing pipeline:
1. Open session_store.db (read-only)
2. Query sessions filtered by repository and date
3. For each session, query turns
4. Parse assistant responses to identify search tool calls:
   - Look for grep/glob/view invocations in the response text
   - Extract response content sizes (using `length(assistant_response)`)
   - Group contiguous search calls into sequences
5. Classify each sequence by purpose using heuristics:
   - Contains symbol name patterns → `point_lookup`
   - Multiple grep calls with related terms → `broad_exploration`
   - Grep for callers/references → `impact_trace`
   - View with specific line ranges → `neighborhood`
   - Interleaved with edits → `implementation`
6. Collect codebase metadata: query sessions table for repository, use `tokei` or line count if available
7. Output each sequence as a JSONL record matching the unified schema

The script produces the same JSONL format as engram's `usage.jsonl` with engram-specific fields set to null.

**Verification:**

- Script runs against current Copilot CLI session store without errors
- Output JSONL is valid (each line parses as JSON)
- `--repository` filter correctly restricts to specified repo
- Records contain reasonable token estimates (not zero, not absurdly large)
- Automated validation: either a Pester test file or `-Validate` switch that runs against a fixture SQLite database and asserts output schema conformance
- Scripts replicate the `/` → `__` branch sanitization convention from `sanitize_branch_for_path()` when constructing metrics directory paths

### Unit 8: Calibration Report Script

**Files:** `scripts/metrics/Compare-Metrics.ps1` (new)
**Test files:** `scripts/metrics/Compare-Metrics.Tests.ps1` (new) or `-Validate` switch
**Execution note:** spike (depends on having both baseline and engram data)
**Patterns to follow:** existing PowerShell scripts
**Dependencies:** Unit 7, Unit 4 (needs both baseline and engram JSONL data)

**Approach:**

PowerShell script that reads baseline JSONL and engram JSONL, computes multipliers:

```text
Parameters:
  -BaselinePaths <string[]>   One or more baseline JSONL files
  -EngramPaths <string[]>     One or more engram usage.jsonl files
  -OutputFormat <string>      "table" (default) or "json"
```

Processing:
1. Load all baseline records, group by `turn_purpose`
2. Load all engram records, group by tool name (mapped to purpose via R22 stratification)
3. For each purpose category:
   - Compute median baseline tokens per sequence
   - Compute median engram tokens per call
   - Derive multiplier = baseline_median / engram_median
   - Compute sample count and confidence level
4. Output formatted report

**Verification:**

- Script produces sensible multipliers when given test data
- Confidence indicators match sample count thresholds (<10 low, 10-50 medium, >50 high)
- JSON output format is valid and machine-readable
- Automated validation: either a Pester test file or `-Validate` switch that runs against fixture JSONL with known expected multipliers

## Dependency Graph

```text
Unit 6 (Error Codes) ─────────────────────────────────┐
                                                      │
Unit 1 (Models) ──→ Unit 2 (Collector) ──→ Unit 3 (Dispatch) ──→ Unit 4 (Persistence/Flush)
                                                                         │
                                                                         ├──→ Unit 5 (MCP Tools)
                                                                         │
                    Unit 7 (Baseline Script) ──→ Unit 8 (Calibration Script)
```

**Sequencing rationale:**
- Units 1 and 6 have no dependencies and can proceed in parallel
- Unit 2 needs the data structures from Unit 1
- Unit 3 needs the collector from Unit 2 to send events
- Unit 4 needs dispatch instrumentation from Unit 3 to have events flowing
- Unit 5 needs persistence from Unit 4 to have data to query
- Unit 7 only needs the JSONL schema from Unit 1 (can proceed in parallel with Units 2-5)
- Unit 8 needs output from both Unit 7 and Unit 4

**Parallelization opportunities:**
- Units 1 + 6: fully parallel
- Unit 7: parallel with Units 2-5 (only depends on schema definition)
- Units 7 + 8: the scripts are independent of the Rust implementation

## Decisions

| # | Decision | Rationale | Alternatives Rejected |
|---|---|---|---|
| D1 | Token heuristic: `bytes / 4` | Matches existing `token_count` on all symbol models; tiktoken would add dependency violating Constitution VI | Tiktoken crate (dependency burden) |
| D2 | Empirical multipliers from Copilot CLI session data | Real workflows, zero runtime cost, conservative ratios. Shadow mode produces inflated ratios. | Shadow mode `--experiment` (measures wrong thing); theoretical heuristics (fragile assumptions) |
| D3 | Per-branch folders: `.engram/metrics/{branch}/` | Git-friendly, naturally isolated, matches user's "feature branch" requirement | Single flat file (hard to query per-branch); SurrealDB table (not Git-trackable) |
| D4 | Append-only JSONL for raw events | Crash-safe, human-readable, consistent with existing code-graph JSONL pattern | CSV (less flexible); SQLite (not Git-mergeable) |
| D5 | Non-blocking via `tokio::sync::mpsc` | Preserves tool call latency (<1ms target); drop-on-full policy prevents backpressure | Synchronous writes (1-5ms jitter); `tokio::sync::broadcast` (unnecessary for single consumer) |
| D6 | Copilot CLI only for baseline capture | Session store provides full history without instrumentation; other platforms need custom integration | Multi-platform (too much scope); instructions file (changes agent behavior) |
| PD1 | Measure at dispatch return, not at IPC serialization | Dispatch is the single entry point for all tool calls (MCP and IPC). Measuring here captures the response Value before transport-specific serialization. The byte count from `serde_json::to_string(&value).len()` is a close approximation of what reaches the agent. | IPC `to_line()` measurement (only works for IPC transport, not SSE/MCP) |
| PD2 | Use 13xxx error code range for metrics | 8xxx is taken by IPC/daemon. 13xxx is the next available range following the established pattern. | 8010+ (confusing to mix IPC and metrics in same range) |
| PD3 | `append` mode for usage.jsonl, `atomic_write` for summary.json | Usage log is append-only (no rewrite needed). Summary is computed fresh (atomic prevents partial reads). | Atomic write for both (unnecessary overhead for append-only log) |
| PD4 | Connection UUID as session identifier | SSE connections already get `Uuid::new_v4()` in `src/server/sse.rs:53`. Reuse this rather than introducing a separate session concept. | Custom session IDs (unnecessary abstraction); no session tracking (loses per-session analytics) |
| PD5 | Branch detection via existing `resolve_git_branch()` | Already implemented in `src/db/workspace.rs:97-115`, reads `.git/HEAD` directly, handles detached HEAD. Branch stored in `WorkspaceSnapshot.branch`. | `git rev-parse` subprocess (slower, external dependency); explicit tool parameter (requires agent cooperation) |
| PD6 | Retention managed by Git lifecycle | Metrics folders correspond to branches. When branches are deleted and garbage-collected, stale metrics disappear naturally. No application-level pruning needed. | Time-based pruning (adds complexity); size-based pruning (complicates append-only design) |

## Risks and Caveats

1. **JSONL file growth**: On high-activity branches, `usage.jsonl` could grow large. Mitigation: events are small (~200 bytes each); 10,000 events ≈ 2MB. Acceptable for Git. If this becomes a problem, a future rotation scheme can be added without breaking the append-only contract.

2. **Branch name edge cases**: Detached HEAD produces a 12-char SHA hash as the branch name. This is handled by existing `resolve_git_branch()` but means metrics for detached HEAD work are harder to correlate. Mitigation: document this behavior; recommend working on named branches.

3. **Response size ≠ context window impact**: The byte count of engram's JSON response overestimates context window tokens because it includes JSON structure (braces, quotes, field names) that LLMs process efficiently. However, since we apply the same `bytes/4` heuristic to both engram and baseline measurements, the systematic bias cancels in ratio comparisons.

4. **Search sequence classification accuracy**: The post-hoc analysis script classifies turns by purpose using heuristics (pattern matching on grep queries). Misclassification will add noise to multiplier calculations. Mitigation: require medium confidence (10+ samples) before reporting multipliers; allow manual override/correction of classifications.

5. **Concurrent flush and metrics write**: The flush lock (static `Mutex<()>`) serializes flush operations, but the metrics background writer is independent. If flush recomputes `summary.json` while the writer is appending to `usage.jsonl`, the summary may be one event behind. Mitigation: acceptable lag; summary is a snapshot, not a real-time counter.

6. **Test compilation time**: The `embeddings` feature is default-on and first compilation takes 20-40 minutes. New test files need `[[test]]` blocks in `Cargo.toml`. Plan for targeted `--test {name}` during development.

## Learnings Applied

No prior solutions found in `.backlog/compound/` (directory is empty). The following repository memories informed this plan:

- **Test registration**: Every new test file requires a `[[test]]` block in `Cargo.toml` (verified from Cargo.toml structure)
- **Compile time**: First `cargo test` after source change compiles ort-sys native binaries — 20-40 min debug. Use targeted `--test {name}` during development.
- **Feature guards**: Do NOT use `embedding::is_available()` as a guard — use `#[cfg(not(feature = "embeddings"))]` compile-time blocks instead.
- **Daemon testing**: The daemon auto-binds workspace from `--workspace` on startup; do NOT call `set_workspace` via IPC.
- **SurrealDB serde**: `#[serde(flatten)]` does NOT work with SurrealDB's `Thing` type — use explicit structs.

## Constitution Check

| Principle | Compliance | Notes |
|---|---|---|
| I. Safety-First Rust | ✅ | All new code in Rust stable 2024. No unsafe. Result/EngramError for all fallible ops. `value.to_string()` used for infallible byte measurement. |
| II. MCP Protocol Fidelity | ✅ | New tools unconditionally visible. No-workspace → error 1001. No-metrics → error 13002. |
| III. Test-First Development | ✅ | Every unit specifies test files and verification criteria before implementation. Proptest for serializable models. Script validation via Pester or `-Validate`. |
| IV. Workspace Isolation | ✅ | Metrics written within `.engram/` under workspace root. No cross-workspace access. |
| V. Structured Observability | ✅ | `#[instrument]` on background writer. `tracing::trace!` on event drop. `tracing::info!` on branch switch. `tracing::warn!` on write failures. |
| VI. Single-Binary Simplicity | ✅ | No new dependencies. Uses existing serde_json, tokio::sync::mpsc, chrono. Scripts are PowerShell (not compiled). |
| VII. CLI Workspace Containment | ✅ | All file operations within `.engram/metrics/` under cwd. |
| VIII. Engram-First Search | N/A | This feature adds observability, not search. |
| IX. Git-Friendly Persistence | ⚠️ | JSONL and JSON files in `.engram/metrics/`. `BTreeMap` for sorted keys. Atomic writes for summary.json. **Deviation**: `usage.jsonl` uses append mode, not atomic write (see Complexity Tracking). |

## Complexity Tracking

| Principle | Deviation | Justification | Rejected Alternative |
|---|---|---|---|
| IX. Git-Friendly Persistence | `usage.jsonl` uses append mode instead of atomic temp-file-then-rename | Append-only JSONL loses at most one partial line on crash. Atomic write would require read-entire-file-then-rewrite semantics for every event, defeating the purpose of append-only logging and adding O(n) I/O cost per event. The partial-line risk is mitigated by `compute_summary()` discarding unparseable final lines. | Read-modify-write cycle for every event (O(n) I/O, defeats append-only design) |
