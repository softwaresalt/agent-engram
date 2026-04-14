---
title: "Engram Effectiveness Metrics & Token Usage Tracking"
date: 2026-03-27
scope: deep
status: draft
---

# Engram Effectiveness Metrics & Token Usage Tracking

## Problem Frame

Engram exists to make AI coding assistants more efficient by returning precise, pre-indexed results instead of raw file content. There is currently no way to measure whether it achieves this goal. Without concrete data, the value proposition remains anecdotal: we believe engram saves tokens, but we cannot prove it or quantify the savings.

The daemon already tracks tool call counts and latency percentiles (`AppState.tool_call_count`, `query_stats.rs`), and every indexed symbol carries a pre-computed `token_count` field (`body.len() / 4`). The infrastructure for measurement partially exists but lacks the accounting layer that connects tool responses to context window impact and persists those measurements per feature branch for longitudinal analysis.

Proving savings requires a baseline: how many tokens do agents consume when searching without engram? This baseline must be captured from real work on real projects, not estimated from heuristics. The measurement methodology must be portable (deployable to any workspace), standardized (producing comparable numbers across projects), and non-intrusive (not changing how agents work).

## Requirements

### Core Accounting

1. Every MCP tool response MUST record a `UsageEvent` containing: tool name, timestamp, response payload size in bytes, estimated token count, number of symbols/results returned, and the active Git branch name.
2. Token estimation MUST use the existing `body.len() / 4` heuristic already established in the codebase (consistent with `token_count` on Function, Class, and Interface models).
3. Savings ratios MUST NOT use theoretical counterfactual estimates. Instead, the system MUST support a **two-phase approach**: Phase 1 tracks engram volume only (no savings claims). Phase 2 applies empirically calibrated multipliers derived from real Copilot CLI baseline data (see Baseline Measurement and Multiplier Calibration sections) to produce savings ratios with stated confidence levels.

### Per-Branch Persistence

4. Usage events MUST be persisted to `.engram/metrics/{branch-name}/usage.jsonl` as append-only JSONL (one JSON object per line), consistent with the existing `.engram/code-graph/*.jsonl` pattern.
5. Branch names MUST be sanitized for filesystem safety (replace `/` with `__`, strip special characters), matching the existing `sanitize_branch_for_path()` function in `src/db/workspace.rs`.
6. A summary file `.engram/metrics/{branch-name}/summary.json` MUST be computed on `flush_state` or on demand, containing aggregated statistics for the branch.
7. The `.engram/metrics/` directory tree MUST be Git-trackable (not in `.gitignore`) so metrics travel with the branch and are available for cross-branch comparison.
8. When the daemon detects a branch change (via workspace sync or explicit check), it MUST switch the active metrics path to the new branch folder.

### Aggregated Statistics

9. The branch summary MUST include:
    - Total tool calls (broken down by tool name)
    - Total tokens delivered to agents
    - Estimated counterfactual tokens (Phase 2 only; derived from calibrated multipliers, not theoretical estimates)
    - Overall savings ratio (Phase 2 only)
    - Average tokens per tool call (by tool type)
    - Top 10 most-queried symbols (by call count)
    - Time range covered (first event to last event)
    - Unique session count (if session IDs are available from SSE connections)
10. A cross-branch comparison view MUST be available via a new MCP tool (`get_branch_metrics`) that returns the summary for a specified branch or compares two branches.
11. An aggregate "all branches" summary MUST be computable for overall daemon effectiveness reporting.

### MCP Tool Surface

12. A new `get_branch_metrics` MCP tool MUST expose branch-level metrics to agents, accepting optional parameters: `branch_name` (defaults to current), `compare_to` (optional second branch for delta comparison).
13. The existing `get_health_report` tool SHOULD be extended to include a `metrics_summary` section with current-branch token savings data alongside existing latency and tool call stats.
14. A new `get_token_savings_report` MCP tool SHOULD provide a formatted summary optimized for agent consumption: a concise paragraph describing engram's effectiveness on the current branch with key numbers.

### Baseline Measurement (Non-Engram Capture via Copilot CLI)

15. The measurement system MUST define a **standardized unit**: "search tokens per search sequence." A search sequence is one or more contiguous search-related tool calls (grep, glob, view-for-reading) within an agent turn, separated from other search sequences by non-search actions (edits, terminal commands, agent reasoning).
16. Each captured search sequence MUST record:
    - `sequence_id`: unique identifier
    - `timestamp`: when the sequence started
    - `turn_purpose`: classified as one of: `point_lookup` (find a specific symbol), `neighborhood` (understand related code), `impact_trace` (trace callers/dependencies), `broad_exploration` (understand a module or subsystem), `implementation` (search done as part of making changes), `verification` (search to confirm correctness)
    - `tools_used`: ordered list of tool calls with per-call token estimates (`[{tool: "grep", params_summary: "pattern=foo, -C 5", response_tokens: 340}, ...]`)
    - `total_search_tokens`: sum of all tool response tokens in the sequence
    - `total_tool_calls`: count of search tool calls in the sequence
    - `files_touched`: list of file paths accessed during the sequence
    - `codebase_context`: `{language, total_files, total_loc}` (captured once per session, referenced by all sequences)
17. The primary capture mechanism for non-engram baselines MUST be a **post-hoc analysis script** that reads the Copilot CLI session store (`session_store` SQLite database). The script parses assistant responses for search tool call patterns (grep, glob, view invocations and their results), estimates tokens from response content sizes, classifies turns by purpose, and outputs standardized JSONL. This captures real usage from any Copilot CLI session without requiring any prior setup in the target workspace.
18. The baseline JSONL schema MUST be identical to the engram metrics schema (from requirement 1) in its common fields so that records from engram and non-engram workspaces can be loaded into the same analysis pipeline. Engram records include additional fields (`engram_tool`, `symbols_returned`, `savings_ratio`) that non-engram records leave null.
19. Baseline measurements MUST include **codebase normalization metadata**: primary language, lines of code, file count, and a codebase size bracket (`small` < 10K LOC, `medium` 10-100K LOC, `large` > 100K LOC). This enables comparing "search tokens per sequence on a 50K-line Rust codebase" against "search tokens per sequence on a 200K-line TypeScript codebase."
20. The analysis script MUST accept a `--repository` filter to extract data for a specific project, and a `--since` date filter for time-bounded analysis. Output goes to stdout (JSONL) or a specified file path.

### Multiplier Calibration

21. Once baseline data exists from multiple non-engram workspaces, the system MUST support computing **empirical multipliers** per engram tool type. The multiplier for a given tool is: `median_baseline_tokens_per_sequence(matching_purpose) / median_engram_tokens_per_call(tool)`. For example, if non-engram `impact_trace` sequences average 4,200 tokens across 6 tool calls, and engram's `impact_analysis` averages 1,050 tokens in 1 call, the multiplier is 4.0x.
22. Multipliers MUST be stratified by `turn_purpose` to ensure like-for-like comparison:
    - `point_lookup` ↔ `list_symbols`, `map_code` (depth=0)
    - `neighborhood` ↔ `map_code` (depth=1+)
    - `impact_trace` ↔ `impact_analysis`
    - `broad_exploration` ↔ `unified_search`
    - `implementation` ↔ mixed (not directly comparable; track volume only)
23. A **calibration report** MUST be producible that shows: per-purpose baseline medians, per-tool engram medians, derived multipliers, sample sizes, and confidence indicators (low/medium/high based on sample count: <10 low, 10-50 medium, >50 high).

### Integration Points

24. Usage event recording MUST be non-blocking and MUST NOT add measurable latency to tool call responses. Events SHOULD be queued and written asynchronously.
25. The metrics subsystem MUST respect the existing flush/dehydration lifecycle: raw events are written continuously, summaries are recomputed on `flush_state`.
26. The `.engram/metrics/` structure MUST survive hydration/dehydration cycles. On startup, if metrics files exist for the current branch, the daemon MUST load them as the running baseline.

## Success Criteria

1. After a coding session, `.engram/metrics/{branch}/usage.jsonl` contains a verifiable log of every tool call with token counts.
2. The summary.json shows an overall savings ratio for the branch (expected to be positive for typical usage patterns where `map_code` and `impact_analysis` dominate).
3. An agent can call `get_branch_metrics` and receive a structured comparison showing that engram delivered X tokens versus a Y-token file-based alternative, achieving Z% savings.
4. Metrics persist across daemon restarts via the `.engram/metrics/` directory.
5. The metrics subsystem adds less than 1ms overhead per tool call (non-blocking write).
6. Cross-branch comparison works: an agent can see that feature branch A consumed 50K tokens from engram at 72% savings while feature branch B consumed 120K tokens at 65% savings.
7. The post-hoc analysis script extracts baseline measurements from Copilot CLI session store data for any specified repository, producing valid JSONL with classified search sequences.
8. Baseline data from 3+ non-engram workspaces of varying size produces calibration multipliers with medium or high confidence (10+ samples per purpose category).
9. The calibration report clearly shows per-purpose multipliers (e.g., "impact tracing costs 4.2x more tokens without engram, based on 47 samples across 3 projects").

## Scope Boundaries

### In Scope

- Token accounting for all existing MCP read tools (`map_code`, `list_symbols`, `unified_search`, `impact_analysis`, `query_memory`, `query_graph`)
- Per-branch JSONL event logging and JSON summary persistence in `.engram/metrics/`
- New MCP tools for querying metrics
- Integration with existing `flush_state` and health report infrastructure
- Branch detection and automatic metric routing
- Post-hoc baseline extraction script for Copilot CLI session store data
- Multiplier calibration from baseline vs. engram comparison
- Codebase normalization metadata for cross-project comparison

### Non-Goals

- Precise token counting (e.g., tiktoken-level accuracy): the `len/4` heuristic is sufficient for comparative measurement
- Tracking agent-side token usage (engram cannot observe the full context window)
- Real-time dashboards or external metrics export (OTLP integration exists separately)
- Measuring embedding generation costs (one-time indexing cost, not per-query)
- Tracking write tool usage (`set_workspace`, `flush_state`, `index_workspace`): these do not contribute to context window size
- Automated recommendations ("you should use list_symbols instead of map_code"): leave interpretation to humans and agents
- Historical data migration: existing sessions before this feature will have no metrics
- Support for non-Copilot-CLI agent platforms (Claude Code, Cursor, Aider): Copilot CLI is the sole measurement platform for now

## Key Decisions

### D1: Token estimation heuristic

**Decision**: Use `byte_length / 4` consistently, matching the existing `token_count` field on all symbol models.

**Rationale**: Precise tokenization (e.g., via tiktoken or cl100k_base) would require a tokenizer dependency and per-model configuration. The `len/4` heuristic is within ~10-15% of actual token counts for English text and code. Since we are computing ratios (savings = actual/counterfactual), systematic bias cancels out. Adding a tokenizer dependency violates Constitution Principle VI (single-binary simplicity) without proportional benefit.

### D2: Counterfactual estimation approach

**Decision**: Use empirical multipliers calibrated from real Copilot CLI session data, not heuristic estimates.

**Approach**: Rather than computing theoretical counterfactuals per tool call, measure actual non-engram search costs by analyzing Copilot CLI session store data from workspaces that do not use engram. A post-hoc analysis script extracts search sequences from session history, classifies them by purpose (`point_lookup`, `neighborhood`, `impact_trace`, `broad_exploration`), and computes median tokens per sequence per purpose. These become the baseline. Engram metrics provide the treatment group. The multiplier per purpose is `baseline_median / engram_median`.

**Observed agent behavior** (from web research and system prompt analysis):

Real-world agents without a code graph follow a grep-then-view pattern, not a read-entire-files pattern:

1. **grep with context lines** (`-C 5` or `-A 10`): returns ~10-20 lines per match, plus filename/line-number overhead. Multiple matches per file expand this.
2. **Targeted view of specific ranges**: agents view the function or block identified by grep, not entire files. Typical range: 30-100 lines.
3. **Iteration**: agents typically need 2-3 search cycles (grep → view → refine → grep again) before they have equivalent coverage to one engram call. Complex tasks (impact analysis, call graph tracing) can require 10-40+ tool calls.
4. **Dead-end overhead**: searches that don't yield useful results still consume context window tokens with negative-value content.

**Why empirical over theoretical**: The grep-then-view pattern makes theoretical estimation fragile. The number of iterations, context line settings, and view range sizes vary by agent configuration, codebase structure, and task complexity. Real measurements from the Copilot CLI session store capture all of this variance without requiring assumptions.

**Shadow mode considered and rejected**: An alternative approach was evaluated: intercept each engram tool call in `dispatch()`, spawn an equivalent `grep` command in parallel via `tokio::spawn`, log both response sizes, and return only the engram result. This is architecturally feasible (single dispatch entry point in `src/tools/mod.rs`, params extractable, non-blocking via async spawn). However, it measures the wrong thing. A shadow grep returns every substring match with context lines (a massive, noisy blob), but a real agent without engram wouldn't consume that blob — it would iteratively grep, view specific ranges, refine, and grep again across 2-6 tool calls. The shadow approach compares engram's precise output against grep's unfiltered firehose, producing inflated ratios (estimated 100-500x) that nobody trusts. Additionally, tools like `unified_search` have no grep equivalent (grep cannot do semantic/vector search). The session analysis approach captures the actual multi-step agent workflow cost, producing conservative, evidence-based ratios. It also has zero runtime cost (offline analysis) versus spawning a process per tool call.

**Phase 1 (before baseline data exists)**: Track engram volume only, no savings claims.
**Phase 2 (after baseline collection)**: Apply calibrated multipliers to produce savings ratios with stated confidence levels.

### D3: Per-branch folder structure

**Decision**: `.engram/metrics/{sanitized-branch-name}/` with `usage.jsonl` + `summary.json`.

**Rationale**: Alternatives considered:
- **(a) Single flat file** with branch field per event: simpler but makes branch-level queries require scanning everything
- **(b) Database table** in SurrealDB: queryable but not Git-trackable (binary DB files)
- **(c) Branch folders** (selected): naturally isolates data, Git-friendly, supports the user's stated goal of "feature branch folders for feature level tracking", consistent with how `.engram/code-graph/` organizes data

### D4: Append-only JSONL for raw events

**Decision**: Use JSONL (one JSON object per line) for the event log, consistent with `.engram/code-graph/nodes.jsonl` and `edges.jsonl`.

**Rationale**: JSONL is append-only (no read-modify-write), crash-safe (partial writes lose at most one event), and human-readable. Summary statistics are computed from the log on demand or at flush time, keeping the write path trivially simple.

### D5: Non-blocking event recording

**Decision**: Use a `tokio::sync::mpsc` channel to queue events, with a background task writing to disk.

**Rationale**: Tool call latency is a critical performance metric (p50 target < 50ms). Synchronous file I/O on every tool call would add 1-5ms of jitter. A bounded channel with a background writer preserves tool call latency while guaranteeing eventual persistence. If the channel fills (e.g., disk stall), events are dropped rather than blocking tool responses.

### D6: Copilot CLI as sole measurement platform

**Decision**: Scope all baseline measurement to GitHub Copilot CLI. Do not build capture mechanisms for Claude Code, Cursor, Aider, or other agent platforms.

**Rationale**: Copilot CLI provides a session store database with full turn history, making post-hoc analysis possible without any instrumentation in target workspaces. Other platforms would each require custom integration. Starting with one well-understood platform produces trustworthy data faster. The standardized JSONL schema leaves the door open for adding other platforms later if needed, but the analysis script, session store schema, and calibration workflow are Copilot CLI specific.

## Outstanding Questions

### Resolve Before Planning

1. **Branch detection mechanism**: Should the daemon detect the current Git branch by running `git rev-parse --abbrev-ref HEAD` at startup and on sync, or should the branch be passed explicitly via a tool parameter? The implicit approach is simpler but adds a `git` process dependency; explicit requires agent cooperation.

2. **Retention policy**: Should old metrics be pruned (e.g., after N days or N MB), or should they accumulate indefinitely since they are Git-tracked and can be managed through normal Git operations (branch deletion)?

3. **Session identity**: The daemon tracks SSE connections but does not currently assign session IDs that correlate across reconnections. Should the metrics subsystem introduce session IDs, or is per-connection tracking sufficient?

### Deferred to Implementation

4. **Exact JSONL schema**: The precise fields in each `UsageEvent` JSON object (beyond the requirements listed above) can be finalized during implementation.

5. **Summary computation trigger**: Whether summary.json is recomputed on every flush, on explicit request, or both.

6. **Atomic write strategy**: Whether summary.json uses the existing temp-file-then-rename pattern from dehydration (likely yes, for consistency).
