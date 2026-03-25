---
id: decision-009
title: 'ADR-009: Vector Search Fallback for map_code'
date: '2025-02-16'
status: Accepted
source: docs/adrs/0009-vector-search-fallback.md
---
## Context

When `map_code` receives a symbol name that does not match any exact-name entry in the code graph tables (function, class, interface), the tool should not simply fail. Per FR-130, it should fall back to vector search to find semantically similar symbols, enabling fuzzy discovery even when the caller misspells a name or uses a conceptual description.

## Decision

When exact-name lookup returns zero matches, `map_code` attempts embedding-based vector search:

1. Embed the query string via `embed_text()`.
2. Search all symbol embeddings using cosine similarity.
3. Return matching symbols in a `fallback_used: true` response with `matches` array.
4. If the embedding model is not loaded (stub mode), return an empty fallback result rather than an error.

The response shape changes for fallback: `root` is null, `neighbors` and `edges` are empty, and `matches` contains the ranked symbol list.

## Consequences

- **Pro**: Graceful degradation — callers always get a useful response even with imprecise queries.
- **Pro**: Consistent with the tiered embedding strategy (ADR-0006) — reuses the same embedding infrastructure.
- **Con**: Fallback results lack graph neighborhood context (no BFS expansion on fallback matches).
- **Future**: Could extend to BFS-expand the top fallback match if confidence is high enough.
