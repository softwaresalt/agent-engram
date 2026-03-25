---
id: decision-010
title: 'ADR-010: Code Graph Performance Criteria Validation (T074)'
date: '2025-02-16'
status: Accepted
source: docs/adrs/0010-code-graph-performance-validation.md
---
## Context

Phase 10 (T074) requires validating performance against success criteria SC-101 through SC-116 as defined in the 003-unified-code-graph spec. These criteria cover latency, throughput, and resource usage for all code graph operations.

Validation requires a representative workspace (500+ Rust files for SC-101) and the embedding model (`bge-small-en-v1.5`) loaded via the `embeddings` feature flag. The embedding feature is currently gated and has a known TLS dependency issue (`ort-sys`) that blocks compilation. Full-stack benchmarks therefore cannot run until the TLS issue is resolved.

## Decision

We categorize each success criterion as **covered**, **partially covered**, or **deferred**:

| Criterion | Description | Status | Coverage |
|-----------|-------------|--------|----------|
| SC-101 | `index_workspace` < 30s for 500 files | Deferred | Requires embeddings + large workspace fixture |
| SC-102 | `sync_workspace` < 3s for 10 changed files | Deferred | Requires embeddings + pre-indexed workspace |
| SC-103 | `map_code` 1-hop < 50ms | Partially covered | BFS traversal benchmarked, but via tool dispatch not yet isolated |
| SC-104 | `get_active_context` < 100ms | Partially covered | Query pathway tested, latency not yet measured under load |
| SC-105 | `unified_search` < 200ms | Deferred | Requires embeddings for vector search component |
| SC-106 | `impact_analysis` 2-hop < 150ms | Partially covered | BFS + concerns traversal works, not benchmarked at scale |
| SC-107 | Metadata round-trip preserves 100% | Covered | JSONL serialization tests verify field preservation (Phase 9) |
| SC-108 | Incremental sync re-embeds < 5% | Deferred | Requires embeddings + content-hash comparison |
| SC-109 | 80% fewer irrelevant tokens | Deferred | Requires comparative benchmark harness |
| SC-110 | Cross-region results in single response | Covered | `impact_analysis` and `get_active_context` return both regions |
| SC-111 | Model memory < 150 MB | Deferred | Requires embeddings feature |
| SC-112 | Batch embed 100 nodes < 2s | Deferred | Requires embeddings feature |
| SC-113 | Tier 2 summaries within 10% recall | Deferred | Requires embeddings + benchmark query set |
| SC-114 | In-memory footprint < 50 MB for 10K nodes | Deferred | Requires large-scale graph fixture |
| SC-115 | Hydration of 10K nodes < 10s | Deferred | Requires large JSONL fixture generation |
| SC-116 | Disk usage < 60 MB for 10K nodes | Deferred | Requires large JSONL fixture |

**Summary**: 2 covered, 3 partially covered, 11 deferred (blocked on embeddings feature or large-scale fixtures).

## Consequences

- Covered and partially covered criteria serve as regression guards during ongoing development.
- Deferred criteria form the acceptance test backlog for when the `embeddings` feature is unblocked.
- A future phase should generate large-scale fixtures (10K-node JSONL files) for SC-114, SC-115, and SC-116 benchmarks.

## Existing Benchmarks

The `integration_benchmark` test suite covers the original 001/002 spec criteria:

- T097: Cold start < 200ms (passes)
- T098: Hydration 1000 tasks < 500ms (passes in release, flaky in debug)
- T099: `query_memory` keyword search < 50ms (passes)
- T100: `update_task` < 10ms (passes)
- T101: Idle memory < 100 MB (passes)
- T119: `flush_state` 100 tasks < 1s (passes)

These remain valid baselines for the task management subsystem.
