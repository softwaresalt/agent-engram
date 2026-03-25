---
id: decision-006
title: 'ADR-006: Switch embedding model from all-MiniLM-L6-v2 to bge-small-en-v1.5'
date: '2025-02-16'
status: Accepted
source: docs/adrs/0006-embedding-model-bge-small.md
---
## Context

The unified code knowledge graph (spec 003) requires semantic embeddings for code
symbols, documentation, and natural-language queries. The existing embedding
service used `AllMiniLML6V2` (all-MiniLM-L6-v2, 384 dimensions). Research
conducted during spec 003 planning found that `BGESmallENV15` (bge-small-en-v1.5)
achieves higher retrieval accuracy on code-related benchmarks while producing
embeddings of the same dimensionality (384).

The fastembed SDK also updated its initialization API, replacing `InitOptions`
with `TextInitOptions` for text embedding models.

## Decision

Switch the embedding model from `EmbeddingModel::AllMiniLML6V2` to
`EmbeddingModel::BGESmallENV15` in `src/services/embedding.rs`. Update the
initialization call from `InitOptions::new()` to `TextInitOptions::new()` to
match the current fastembed 3.x API.

Both models produce 384-dimensional vectors, so no schema changes are required
for existing embedding storage or cosine-similarity computations.

## Consequences

### Positive

- Improved retrieval accuracy for code-related semantic search queries.
- Same 384-dimension output preserves compatibility with existing vector storage
  and search infrastructure.
- Aligns with fastembed 3.x API conventions (`TextInitOptions`).

### Negative

- Any previously generated embeddings (if any exist) would need re-indexing since
  vectors from different models are not comparable. In practice, no production
  embeddings existed yet, so this is a non-issue.

### Risks

- Model download size and inference latency may differ slightly, though both
  models are in the "small" category. Benchmarking in Phase 8 (Semantic Search)
  will validate acceptable performance.

## Phase/Task

Spec: 003-unified-code-graph, Phase 1 (Setup), Task T013
