---
title: TMDL Parser Crate Start Memory
type: session-memory
date: 2026-06-12
feature: 064-F
stash_id: 59039891
---

## Task IDs Completed

* `59039891` spike follow-on implementation slice — parser crate boundary plus semantic-model shell indexing

## Files Modified

| File | Change |
|---|---|
| `Cargo.toml` | Added the new internal `powerbi-tmdl-parser` workspace member and dependency |
| `Cargo.lock` | Recorded the new workspace package |
| `crates/powerbi-tmdl-parser/Cargo.toml` | Added the dedicated parser crate manifest |
| `crates/powerbi-tmdl-parser/src/lib.rs` | Implemented the initial safe fixture-driven TMDL parser and crate-local tests |
| `src/services/powerbi_tmdl.rs` | Replaced inline parsing with an adapter over the new crate |
| `src/services/powerbi_indexer.rs` | Added semantic-model summary records for JSON-backed and TMDL-backed semantic models |
| `tests/unit/powerbi_extract_tmdl_test.rs` | Added regression coverage for relationship blocks, multiline measure bodies, and ref-only `model.tmdl` shells |
| `tests/integration/powerbi_search_ingestion_test.rs` | Added semantic-model summary and ref-only `model.tmdl` indexing coverage |
| `docs/decisions/2026-06-12-tmdl-tree-sitter-spike.md` | Captured the spike findings and recommended crate boundary |

## Key Decisions

1. **Start with an internal crate boundary**: We created `powerbi-tmdl-parser` inside the workspace so TMDL parsing can evolve independently from Engram ingestion and Power BI graph mapping.

2. **Keep the first implementation safe and fixture-driven**: We did not introduce tree-sitter FFI yet because the workspace forbids `unsafe` in production code and the grammar approach still needs a constitution-compliant path.

3. **Target the highest-value fixture gaps first**: The current slice closes the known relationship and multiline-measure gaps, and it keeps canonical `model.tmdl` files indexable even when they only contain refs.

4. **Emit semantic-model records explicitly**: Semantic models now produce a searchable `powerbi_semantic_model` content record so JSON-backed and TMDL-backed models share a first-class model summary surface.

## Verification

* `cargo test --test unit_powerbi_extract_tmdl --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo test --test integration_powerbi_search_ingestion --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo clippy --all-targets --config "target.x86_64-pc-windows-msvc.linker='lld-link'" -- -D warnings -D clippy::pedantic`
* `cargo fmt --all -- --check`
* `cargo dev-test --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`
* `cargo test -p powerbi-tmdl-parser --config "target.x86_64-pc-windows-msvc.linker='lld-link'"`

## Open Items

* The new crate is still a line-and-indent parser, not a true tree-sitter grammar
* TMDL constructs such as partition blocks, top-level expressions, and richer data-source properties remain follow-on work
* A future tree-sitter-backed crate still needs a safe integration story for generated parser bindings

## Next Steps

1. Extend the parser crate to cover more semantic-model file shapes from the real PBIP fixture
2. Decide whether a true grammar should live in-workspace or in a separate external crate once the `unsafe` boundary question is resolved
3. Reuse the crate boundary for later DAX and PBIR parsing work instead of re-embedding parser logic into `engram`
