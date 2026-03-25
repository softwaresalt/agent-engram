---
id: TASK-008.04
title: '008-04: Embedding Feature Flag Hardening'
status: Done
assignee: []
created_date: '2026-03-21'
labels:
  - feature
  - '008'
  - embeddings
  - observability
dependencies: []
references:
  - src/services/embedding.rs
  - src/tools/read.rs
  - src/tools/lifecycle.rs
  - Cargo.toml
parent_task_id: TASK-008
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Harden the `embeddings` feature flag behavior so users get clear feedback when semantic search is degraded.

**Beads ID**: agent-engram-dxo.4

### Current State (the problem)
- When the `embeddings` feature is disabled, symbols store zero vectors (`vec![0.0; 384]`)
- `embed_text()` returns `QueryError::ModelNotLoaded`
- `vector_search_symbols()` silently returns empty results (zero vectors fail `has_meaningful_embedding()`)
- Users get no indication that semantic search is degraded

### Target State
- `unified_search` returns a clear message when embeddings are unavailable, not empty results
- `get_health_report` / `get_workspace_statistics` reports embedding status (enabled/disabled, model loaded, % of symbols with real embeddings vs zero vectors)
- `embeddings` made a default feature so the common case works out of the box
- `--no-embeddings` CLI flag added as the opt-out rather than requiring opt-in

### Files Modified
- `src/services/embedding.rs` — `is_available()` and `status()` functions
- `src/tools/read.rs` — informative error in `unified_search`
- `src/tools/lifecycle.rs` — embedding status in workspace statistics
- `Cargo.toml` — `default = ["embeddings"]`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `is_available()` returns true when embeddings feature is enabled AND model is loaded
- [x] #2 `is_available()` returns false when embeddings feature is disabled (compile-time cfg gate)
- [x] #3 `unified_search` returns a descriptive error message when `is_available()` is false
- [x] #4 `get_health_report` includes `EmbeddingStatus` with coverage metrics
- [x] #5 `get_workspace_statistics` includes embedding status (enabled, loaded, coverage %)
- [x] #6 `embeddings` is a default Cargo feature; `--no-embeddings` is the opt-out
- [x] #7 Unit tests pass for `is_available()` and `status()` in all configurations
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Tasks (Beads: agent-engram-dxo.4.*)

### dxo.4.1 — Add is_available() and status() functions to embedding service

**File**: `src/services/embedding.rs`

Add `EmbeddingStatus` struct:
```rust
pub struct EmbeddingStatus {
    pub enabled: bool,
    pub model_loaded: bool,
    pub model_name: Option<String>,
    pub symbols_with_embeddings: usize,
    pub total_symbols: usize,
    pub coverage_percent: f64,
}
```

Add two functions:
- `is_available()` — returns `true` when embeddings feature enabled AND model loaded
- `status()` — returns full `EmbeddingStatus` with coverage metrics from `CodeGraphQueries`

**Leaf task — dxo.4.1.1**: Implement `status()` in `src/services/embedding.rs` to return `EmbeddingStatus` with coverage metrics from `CodeGraphQueries`.

**Leaf task — dxo.4.1.2**: Implement `is_available()` in `src/services/embedding.rs` to return `true` when embeddings feature enabled AND model loaded.

### dxo.4.2 — Return informative error in unified_search when embeddings unavailable

**File**: `src/tools/read.rs`

- Check `is_available()` before attempting vector search
- If embeddings are unavailable, return a descriptive `McpError` message explaining the situation and how to enable embeddings
- Do NOT silently return empty results

**Dependencies**: dxo.4.1

### dxo.4.3 — Include embedding status in workspace statistics and health report

**Files**: `src/tools/lifecycle.rs`

- Include `EmbeddingStatus` from `embedding::status()` in the `get_health_report` response
- Report: enabled/disabled, model loaded, model name, symbols with real embeddings, total symbols, coverage %
- Surface this in `get_workspace_statistics` as well
- Consider making embeddings a default feature in `Cargo.toml` (`default = ["embeddings"]`)
- Add `--no-embeddings` CLI flag as opt-out

**Dependencies**: dxo.4.1

### dxo.4.4 — Unit tests for embedding status functions

**Files**: `tests/unit/` or `src/services/embedding.rs` inline tests

Test coverage:
- `is_available()` returns `true` when feature enabled and model loaded
- `is_available()` returns `false` when feature disabled (cfg feature gate)
- `is_available()` returns `false` when model fails to load
- `status()` returns `EmbeddingStatus` with correct field values
- `status().coverage_percent` is `0.0` when no symbols have embeddings
- `status().coverage_percent` is `100.0` when all symbols have real (non-zero) embeddings
- Division by zero handled when `total_symbols == 0`

**Dependencies**: dxo.4.1
<!-- SECTION:PLAN:END -->
