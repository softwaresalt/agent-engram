# ADR 0001: Feature-Gated Embeddings with Keyword-Only Fallback

**Status**: Accepted  
**Date**: 2026-02-13  
**Phase**: 6 (US4: Semantic Memory Query)  
**Tasks**: T077-T086

## Context

Phase 6 introduces semantic search via `fastembed-rs` for embedding generation and hybrid (vector + keyword) scoring. The `fastembed` crate depends on `ort-sys` (ONNX runtime), which adds significant compile time, binary size, and a TLS dependency that complicates CI builds. Not all deployments need vector search — keyword matching alone is useful for many workflows.

## Decision

Gate the `fastembed` dependency behind a Cargo feature flag `embeddings` (not in default features). When disabled:

- `embed_text()` and `embed_texts()` return `QueryError::ModelNotLoaded`
- `hybrid_search()` gracefully degrades to keyword-only scoring (vector component is zero)
- `backfill_embeddings()` during hydration is a no-op (attempts embedding, silently continues on failure)

The hybrid scoring formula `0.7 * vector + 0.3 * keyword` naturally degrades: when vector scores are zero, results are ranked purely by BM25-inspired keyword matching.

## Consequences

**Positive**:
- Default builds compile faster without ONNX runtime
- CI/CD pipelines don't need network access for model downloads
- Keyword search provides useful results even without embeddings
- Users opt into vector search only when they need it

**Negative**:
- Two code paths (feature-gated stubs vs real implementation) require testing both
- Users must explicitly enable `--features embeddings` for semantic search quality

**Risks**:
- BM25-only search quality may not meet SC-010 (95% relevance) without vector component
- Model download on first query adds latency to the first `query_memory` call