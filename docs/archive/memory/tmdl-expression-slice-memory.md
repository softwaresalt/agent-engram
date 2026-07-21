---
title: TMDL Expression Slice Memory
type: session-memory
date: 2026-06-12
feature: 064-F
stash_id: 59039891
---

## Task IDs Completed

* `064-F` expression slice - top-level TMDL expressions, JSON parity, and Power BI indexing/graph wiring

## Files Modified

| File | Change |
|---|---|
| `crates/powerbi-tmdl-parser/src/lib.rs` | Added top-level expression parsing and parser-crate coverage |
| `src/models/powerbi.rs` | Added first-class semantic-model expression entities |
| `src/models/powerbi_graph.rs` | Added the `expression` node kind |
| `src/services/powerbi_tmdl.rs` | Mapped parsed TMDL expressions into the shared Power BI model |
| `src/services/powerbi_extract.rs` | Added JSON semantic-model expression extraction |
| `src/services/powerbi_indexer.rs` | Emitted `powerbi_expression` summaries and expression graph nodes |
| `src/db/cozo_queries.rs` | Recognized the `expression` Power BI node kind in DB parsing |
| `tests/unit/powerbi_extract_tmdl_test.rs` | Added regression coverage for `expressions.tmdl` |
| `tests/unit/powerbi_extract_json_test.rs` | Added JSON semantic-model expression parity coverage |
| `tests/unit/powerbi_graph_models_test.rs` | Added node-kind stability coverage for expressions |
| `tests/integration/powerbi_search_ingestion_test.rs` | Added searchable record and graph-node coverage for TMDL expressions |

## Key Decisions

1. **Model expressions as shared semantic-model entities**: We kept top-level expressions in the same Power BI entity model used by both JSON and TMDL extraction so downstream indexing and graph code stay format-agnostic.

2. **Expose expressions on both search surfaces**: The indexer now emits `powerbi_expression` content records and `expression` graph nodes so parameter-query definitions are searchable and traversable like other semantic-model entities.

3. **Preserve the Power BI-specific parser boundary**: Expression parsing lands in `powerbi-tmdl-parser` and is adapted through `src/services/powerbi_tmdl.rs`, not through the generic code-graph parser pipeline.

## Verification

* `cargo test --test unit_powerbi_extract_tmdl --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo test --test unit_powerbi_extract_json --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo test --test unit_powerbi_graph_models --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo test --test integration_powerbi_search_ingestion --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo test -p powerbi-tmdl-parser --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets --config "target.x86_64-pc-windows-msvc.linker='lld-link'" -- -D warnings -D clippy::pedantic`
* `cargo dev-test --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`

## Open Items

* Partition blocks and M payload parsing still need dedicated coverage
* Data-source extraction is still shallow and does not preserve richer properties
* A true tree-sitter-backed implementation still needs a constitution-compliant safety story

## Next Steps

1. Decompose `064-F` into queue-ready tasks around partition blocks, richer data-source parsing, and remaining TMDL metadata
2. Reconcile the TMDL parser track with `062-F` and `062.003-T` so PBIP semantic-model work does not fork
3. Revisit a safe grammar-backed parser strategy once the `unsafe`/FFI boundary has an approved design
