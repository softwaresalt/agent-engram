---
type: compacted-summary
date: 2026-03-30
source_count: 6
source_date_range: "2026-02-16 to 2026-03-08"
---

# Compacted Summary: research

Compacted from 6 research files spanning 2026-02-16 to 2026-03-08. All research is for completed features.

## Key Decisions

* **MCP server test plan research (2026-02-16)** — Confirmed mcp-sdk 0.0.3 JSON-RPC 2.0 over SSE; tool dispatch is synchronous within the handler; SSE keepalive must be 15s with configurable 60s timeout.
* **Items 3-4-5 research (2026-03-07)** — Investigated SurrealDB native KNN (`NEAREST` operator), hybrid graph+vector approach, and content record ingestion pipeline. Native KNN confirmed viable without cosine_similarity.
* **Lockfile test gap (2026-03-07)** — Identified missing test: daemon second-instance launch must fail with error 8007 (LOCK_ALREADY_HELD). stale PID cleanup requires existence check before removal.
* **Shim daemon fixes research (2026-03-07)** — Analyzed IPC address collision: Windows named pipe uses `\\.\pipe\engram-{SHA256[:16]}` where SHA256 is of the canonical workspace path. Confirmed pipe readiness check via CreateNamedPipe.
* **Spec-002 audit (2026-03-07)** — Found gaps: missing `language` field on code_file, missing import/type_alias edge types, missing content↔symbol linkage in schema.
* **Spec-003 audit (2026-03-07)** — Confirmed tree-sitter-rust 0.24 covers: function, struct, impl, trait, enum, type_alias, use declarations. Missing: macro_rules (deferred).

## Outcomes

* All research incorporated into implementation. No pending research items.
* Native KNN (`NEAREST`) replaces cosine_similarity in `src/services/search.rs`.
* IPC address format: `\\.\pipe\engram-{first_16_hex_of_sha256(canonical_path)}`.

## Preserved Context

* tree-sitter-rust 0.24 node types: function_item, struct_item, impl_item, trait_item, enum_item, type_alias, use_declaration, macro_definition (last deferred).
* SurrealDB DEFINE INDEX ... MTREE DIMENSIONS 384 TYPE F32 — embedding vector index.
