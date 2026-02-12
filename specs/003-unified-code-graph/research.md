# Research: Unified Code Knowledge Graph

**Phase**: 0 — Outline & Research
**Created**: 2026-02-12
**Purpose**: Resolve all technology decisions and best practices for code graph implementation

## Research Tasks

### R1: Tree-Sitter Rust Integration

**Decision**: Use the `tree-sitter` crate (v0.24+) with the `tree-sitter-rust` grammar crate for AST parsing.

**Rationale**: `tree-sitter` is the standard incremental parsing library, providing concrete syntax trees for any language via grammar crates. The Rust API exposes `Parser::parse(source, None)` → `Tree`, `tree.root_node()` → `Node`, and `TreeCursor` for traversal. Node kinds like `"function_item"`, `"struct_item"`, `"trait_item"`, `"impl_item"` map directly to the spec's entity types. Each node exposes `start_position()`, `end_position()`, `start_byte()`, `end_byte()` for precise line range extraction. The `tree-sitter-rust` crate provides the grammar via `tree_sitter_rust::LANGUAGE` (or `tree_sitter_rust::language()`).

**API Pattern**:

```rust
use tree_sitter::{Parser, TreeCursor, Node};

let mut parser = Parser::new();
parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("Rust grammar");
let tree = parser.parse(source_code, None).expect("parse");
let root = tree.root_node();
// Walk with cursor or recursive descent
let mut cursor = root.walk();
```

**Node Kind Mapping** (Rust grammar):

| Spec Entity | tree-sitter Node Kind | Key Fields |
|---|---|---|
| function | `function_item` | name, parameters, return_type, body |
| class | `struct_item` | name, fields |
| interface | `trait_item` | name, methods |
| (method) | `function_item` inside `impl_item` | name, self param, body |
| (impl) | `impl_item` | trait name, type name |

**Edge Discovery**:
- `calls` edges: Walk function bodies for `call_expression` nodes; resolve callee name from `function` field.
- `imports` edges: Walk `use_declaration` nodes at file level; extract `scoped_identifier` path.
- `inherits_from` edges: Walk `impl_item` nodes with a `trait` field; link implementing struct to trait.
- `defines` edges: Walk top-level children of `source_file`; link file node to each function/struct/trait.

**Sync/Async Boundary**: tree-sitter parsing is CPU-bound synchronous work. Per Constitution II, parsing MUST run via `tokio::task::spawn_blocking` to avoid blocking async threads. The parser itself is `!Send` due to internal C state, so it must be created and used within a single `spawn_blocking` closure. For parallelism across files, spawn multiple blocking tasks, each creating its own parser instance.

**Alternatives Considered**:
- `syn` (Rust-specific): Full Rust macro resolution but much slower than tree-sitter; doesn't support multi-language extensibility. Rejected.
- `rust-analyzer` wrappers: Extremely high fidelity but heavyweight (starts an LSP server); overkill for structural extraction. Rejected.
- `tree-sitter-graph`: DSL for tree-sitter queries; adds complexity without clear benefit over direct traversal. Rejected.

### R2: Embedding Model Switch (bge-small-en-v1.5)

**Decision**: Switch from `AllMiniLML6V2` to `BGESmallENV15` via `fastembed` `EmbeddingModel` enum. Clean replacement — no migration needed.

**Rationale**: The spec mandates `bge-small-en-v1.5` (FR-118) as the unified embedding model for all regions. This model is the **default** in fastembed-rs v5 (`EmbeddingModel::BGESmallENV15`). It maintains **384 dimensions** (same as the v0 `all-MiniLM-L6-v2`), so vector indexes (`MTREE DIMENSION 384`) and the `EMBEDDING_DIM` constant require no changes. The model has a 512-token input limit, which aligns perfectly with the tiered embedding strategy (FR-141–FR-145).

**Code Change**:

```rust
// Before (v0):
let options = fastembed::InitOptions::new(fastembed::EmbeddingModel::AllMiniLML6V2)
// After (v1):
let options = fastembed::TextInitOptions::new(fastembed::EmbeddingModel::BGESmallENV15)
```

**Breaking Changes**: None for the crate API. The `EmbeddingModel::BGESmallENV15` uses CLS pooling (not Mean pooling like `AllMiniLML6V2`), but this is handled internally by fastembed-rs. The `DEFAULT_MAX_LENGTH` in fastembed-rs is 512 tokens, matching the spec's token limit.

**Migration Path**: Per the spec clarification (2026-02-11), this is a clean replacement. No production users exist on v0 embeddings. All existing embeddings (if any) are discarded and regenerated with the new model on next hydration. No `model_version` field is needed.

**Shared Instance Pattern**: The existing `OnceLock<Result<TextEmbedding, String>>` pattern (FR-146) works unchanged. The model is loaded exactly once and shared across all connections and workspaces. The `BGESmallENV15` model is approximately 130 MB in memory (SC-111 budget: 150 MB).

**Alternatives Considered**:
- Keep `AllMiniLML6V2`: Lower quality embeddings for code; does not meet spec requirement FR-118. Rejected.
- Use `BGESmallENV15Q` (quantized): Smaller model but reduced precision; spec does not call for quantization. Rejected for initial release; can be a configuration option later.

### R3: JSONL Persistence Format

**Decision**: Use newline-delimited JSON (JSONL) for `.tmem/code-graph/nodes.jsonl` and `.tmem/code-graph/edges.jsonl`.

**Rationale**: JSONL is line-oriented, enabling Git-friendly diffs (one record per line, additions/deletions cleanly visible). It supports streaming reads/writes without loading the entire file into memory. Each line is a self-contained JSON object, making partial corruption recoverable (skip bad lines, log warning). This aligns with Constitution VI (Git-friendly, no binary files in `.tmem/`).

**Schema — nodes.jsonl**:

```json
{"id":"function:abc123","type":"function","name":"process_payment","file_path":"src/billing.rs","line_start":42,"line_end":78,"signature":"fn process_payment(amount: f64) -> Result<Receipt, Error>","docstring":"Processes a payment...","body_hash":"sha256:a1b2c3...","token_count":128,"embed_type":"explicit_code","embedding":[0.01,-0.02,...],"summary":"Payment processing function"}
```

**Schema — edges.jsonl**:

```json
{"type":"calls","from":"function:abc123","to":"function:def456","created_at":"2026-02-12T10:00:00Z"}
{"type":"concerns","from":"task:t1","to":"function:abc123","created_at":"2026-02-12T10:00:00Z","linked_by":"agent-1"}
```

**Code_file nodes** also go in `nodes.jsonl` with `"type":"code_file"` and subset fields (path, language, size_bytes, content_hash, last_indexed_at).

**Serialization**: Use `serde_json::to_string()` per record + `\n`. For reading, use `BufReader::lines()` + `serde_json::from_str()`. Atomic writes via temp+rename pattern (existing in dehydration service).

**Sorting**: Records are sorted by `id` for stable ordering and minimal merge conflicts (Constitution VI).

**Source Bodies NOT Included**: Per the source-canonical model (FR-132), node records contain metadata only — `body` field is omitted from JSONL. Bodies are populated at runtime from source files during hydration.

**Alternatives Considered**:
- Single JSON file: Not line-oriented; entire file changes on any edit; poor Git diffs. Rejected.
- TOML: Poor fit for arrays of heterogeneous records. Rejected.
- SurrealQL export: Ties persistence to SurrealDB format; not human-readable. Rejected.
- Separate files per node: Too many small files for large codebases. Rejected.
- MessagePack/CBOR: Binary formats violate Constitution VI. Rejected.

### R4: SurrealDB Graph Traversal Queries

**Decision**: Use recursive SurrealQL graph traversal with `->` edge syntax and LIMIT-based truncation.

**Rationale**: SurrealDB supports graph edge traversal natively via the `->edge_table->target_table` syntax. For multi-hop traversal required by `map_code` (FR-126), `impact_analysis` (FR-129), and `get_active_context` (FR-127), SurrealQL provides:

**1-Hop Traversal** (map_code depth=1):

```surql
SELECT *, ->calls->function AS callees, <-calls<-function AS callers,
         ->imports->code_file AS imports, <-defines<-code_file AS defined_in
FROM function WHERE name = $symbol_name
```

**Multi-Hop Traversal** (map_code depth=N):
SurrealDB does not natively support variable-depth recursive traversal. Two approaches:

- **Application-Level BFS**: Start from seed node, query 1-hop neighbors, collect results, repeat for each depth level. This is predictable, auditable, and respects the `max_nodes` limit naturally.
- **Subquery Chaining**: Nest subqueries for fixed depths (depth 2 = query-of-query). Less flexible but avoids round-trips.

**Decision**: Use application-level BFS for correctness and controllability. Each BFS level is a single SurrealQL query. The traversal loop runs in the `Queries` struct with `max_nodes` (default 50, configurable) as the stopping condition.

**Cross-Region Query** (impact_analysis):

```surql
-- Find tasks linked to code nodes reachable from target symbol
SELECT * FROM task
WHERE id IN (
  SELECT in FROM concerns WHERE out IN $code_node_ids
)
AND ($status_filter IS NULL OR status = $status_filter)
```

The `$code_node_ids` are collected from the code graph BFS traversal first, then used to query cross-region `concerns` edges in a second pass.

**Vector Search** (unified_search, map_code fallback):

```surql
SELECT * FROM function, class, interface
WHERE embedding <|$limit|> $query_embedding
ORDER BY embedding <-> $query_embedding
```

Combined with task region search for unified results when `region = "all"`.

**Alternatives Considered**:
- Pure SurrealQL recursive CTE: SurrealDB v2 does not yet support recursive CTEs. Rejected.
- GraphQL overlay: Adds unnecessary complexity; SurrealDB's native graph syntax is sufficient. Rejected.
- Neo4j or DGraph: Different database entirely; violates the embedded SurrealDB architecture. Rejected.

### R5: Token Counting for Tiered Embedding

**Decision**: Use the fastembed/ONNX tokenizer's output token count for tier classification.

**Rationale**: The spec defines a 512-token boundary (FR-143, FR-144) for tiered embedding. The precise token count must match the tokenizer used by `bge-small-en-v1.5`. However, fastembed-rs v5 does not expose a standalone tokenization API — it tokenizes internally during `embed()`.

**Approach**: Use a **character-based estimation** as a fast pre-filter, then rely on fastembed's built-in truncation behavior for correctness:

1. **Fast Estimate**: Approximate 1 token ≈ 4 characters for code (conservative for English code with symbols). If `body.len() / 4 < 400` (safe margin below 512), classify as Tier 1 without tokenizing.
2. **Boundary Cases**: For bodies in the 400–600 estimated token range, classify as Tier 2 (summary_pointer) to be safe. This overestimates slightly but avoids truncation.
3. **Store Token Count**: After embedding, the actual token count is recorded in the node's `token_count` field. For Tier 1, this is the actual count consumed by the model. For Tier 2, this is the count of the full body (used for analytics).

**Alternative (More Precise)**: Add `tokenizers` crate (used internally by fastembed) as a direct dependency. Load the `bge-small-en-v1.5` tokenizer and call `tokenizer.encode(text).len()`. This gives exact counts but adds a dependency and initialization cost.

**Decision**: Use the character-based fast estimate for tier classification. The 512-token limit with 4 chars/token heuristic gives a ~400 character safe threshold and a ~2048 character upper bound. Code typically has higher token density than prose (more symbols), so the 4 chars/token ratio is conservative. Store the character count as `token_count` for the estimate, with a note that it's approximate. This is sufficient for the v0 tiering decision and can be refined later with an exact tokenizer if needed.

**Alternatives Considered**:
- `tiktoken-rs`: Implements OpenAI tokenizers, not the BERT tokenizer used by BGE models. Rejected.
- `tokenizers` crate: Correct but heavyweight dependency; initialization cost shared with fastembed. Deferred to refinement phase.
- Fixed character cutoff (2000 chars): Too simplistic; doesn't account for token density variation. Rejected for primary strategy but used as the character-based fast path.

### R6: Two-Level Content Hashing for Incremental Sync

**Decision**: Use SHA-256 for both file-level and symbol-level content hashing, leveraging the existing `sha2` crate.

**Rationale**: The spec requires two-level hashing (FR-122): file-level `content_hash` on `code_file` nodes to identify changed files, and per-symbol `body_hash` on function/class/interface nodes to identify changed symbols within changed files. SHA-256 is already used in the codebase for workspace hashing (`sha2` crate in dependencies), so no new dependency is needed.

**Strategy**:

1. **File-Level Hash**: Compute `SHA-256(file_contents)` and store as `code_file.content_hash`. During sync, compare current file hash against stored hash. If equal, skip the file entirely (no re-parse, no re-embed). If different, re-parse the file.
2. **Symbol-Level Hash**: For each AST node (function/class/interface), compute `SHA-256(source_body)` where `source_body` is the exact text from `start_byte` to `end_byte`. Store as `body_hash`. After re-parsing a changed file, compare each symbol's new `body_hash` against the persisted hash. If equal, skip re-embedding (reuse persisted embedding). If different, re-embed.

**Hash Format**: `hex::encode(sha256_digest)` → 64-character lowercase hex string. No prefix needed (unlike the `sha256:` prefix in the JSONL examples, which is for human readability; actual storage uses raw hex).

**Performance**: SHA-256 of a 1 MB file takes <1ms on modern hardware. Hash comparison is O(1) string equality. This makes the two-level strategy nearly free compared to parsing or embedding costs.

**Alternatives Considered**:
- File modification time (mtime): Unreliable across Git operations (clone, checkout, stash). Rejected per spec (FR-122 uses content hash).
- CRC32 or xxHash: Faster but no collision resistance. SHA-256 is already in dependencies and collisions are unacceptable for correctness. Rejected.
- BLAKE3: Faster than SHA-256 but adds a new dependency; `sha2` is already present. Deferred.

### R7: Hash-Resilient Identity Matching for Concerns Edges

**Decision**: Use `(name, body_hash)` tuple as resilient identity for re-linking `concerns` edges across sync operations.

**Rationale**: FR-124 requires that `concerns` edges survive file moves (symbol name + body unchanged, only file path changed). During `sync_workspace`:

1. Before removing old nodes from a changed/deleted file, collect all `concerns` edges targeting those nodes as `(task_id, symbol_name, body_hash)` tuples.
2. After re-indexing, search for new nodes matching `(name, body_hash)` anywhere in the workspace.
3. If a match is found at a new file path, re-link the `concerns` edge to the new node and create a context note on the affected task recording the path change.
4. If no match is found (name or body changed), treat the edge as orphaned: remove it and create a context note on the affected task recording the broken link.

**Implementation**:

```rust
struct PendingConcernsRelink {
    task_id: String,
    symbol_name: String,
    body_hash: String,
    old_file_path: String,
}
```

After all files are re-indexed, execute a single query to resolve pending re-links:

```surql
SELECT * FROM function, class, interface
WHERE name = $symbol_name AND body_hash = $body_hash
```

**Edge Cases**:
- Symbol renamed but body unchanged: Orphaned (name doesn't match). Correct per spec.
- Symbol body changed but name unchanged: Orphaned (hash doesn't match). Correct per spec — the old `concerns` link targeted a specific version of the symbol.
- Same name+hash at multiple new paths: Link to all matches (ambiguous, but correct per FR-110 which links to all matching nodes).

**Alternatives Considered**:
- File path-based identity: Breaks on file moves. Explicitly rejected by spec.
- UUID-based identity: Requires persisting UUIDs across parse cycles; tree-sitter doesn't produce stable IDs. Rejected.
- Fuzzy matching (edit distance on body): Over-engineered; hash equality is the spec-mandated threshold. Rejected.

### R8: Tree-Sitter Call Site Resolution

**Decision**: Use name-based call site resolution with best-effort matching against indexed functions.

**Rationale**: tree-sitter parses call expressions as `call_expression` nodes containing a `function` field that holds the callee name (or path). For Rust:
- Simple calls: `process_payment(x)` → function field = `process_payment` → match against indexed function names.
- Method calls: `x.process_payment()` → `method_call` node → extract method name `process_payment` → match against functions in `impl` blocks.
- Qualified calls: `billing::process_payment(x)` → `scoped_identifier` → extract final segment `process_payment`.
- Macro calls: `println!("...")` → `macro_invocation` → skip (macros are not indexed as functions).

**Limitations**: Name-based resolution is approximate. It cannot resolve:
- Trait method dispatch (which impl is called depends on runtime type).
- Function pointers or closures.
- Cross-crate calls where the callee is not in the indexed workspace.

This is acceptable for v0. The `calls` edges represent "this function references that function name" rather than "this function definitely invokes that specific implementation." The structural information is still valuable for dependency neighborhood construction.

**Alternatives Considered**:
- rust-analyzer for full name resolution: Requires cargo project compilation context; heavyweight. Deferred to post-v0.
- No call resolution (edges from `imports` only): Misses intra-file and intra-module calls. Rejected.

### R9: Embedding Dimension and Vector Index Compatibility

**Decision**: No vector index changes needed — `bge-small-en-v1.5` produces 384-dimensional vectors, same as `all-MiniLM-L6-v2`.

**Rationale**: Both models produce 384-dimensional float vectors. The existing `MTREE DIMENSION 384 DIST COSINE` index definitions in `schema.rs` work unchanged for code graph node tables. New tables (`function`, `class`, `interface`) need their own `MTREE` indexes with the same dimension.

**Schema Addition**:

```surql
DEFINE INDEX function_embedding ON TABLE function COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
DEFINE INDEX class_embedding ON TABLE class COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
DEFINE INDEX interface_embedding ON TABLE interface COLUMNS embedding MTREE DIMENSION 384 DIST COSINE;
```

**Potential Future Change**: If the model is ever changed to one with different dimensions, the `EMBEDDING_DIM` constant and all `MTREE DIMENSION` declarations must be updated simultaneously. The constant already serves as the single source of truth for dimension size.

### R10: Cargo Dependency Additions

**Decision**: Add `tree-sitter` and `tree-sitter-rust` as required dependencies; `ignore` crate for `.gitignore` pattern matching.

**New Dependencies**:

| Crate | Version | Purpose | Justification |
|---|---|---|---|
| `tree-sitter` | `0.24` | AST parsing core | Required for code structure extraction (FR-115) |
| `tree-sitter-rust` | `0.23` | Rust grammar | Required for Rust language support (FR-114) |
| `ignore` | `0.4` | `.gitignore` pattern matching | Required for exclusion patterns (FR-116); more robust than hand-rolled glob matching |

**Existing Dependencies Leveraged**:
- `sha2` + `hex`: Content hashing (already in Cargo.toml)
- `fastembed`: Embedding generation (existing, model switch only)
- `serde` + `serde_json`: JSONL serialization (existing)
- `tokio`: Async runtime and `spawn_blocking` (existing)
- `toml`: Config parsing (added in 002)
- `chrono`: Timestamps (existing)

**Alternatives Considered**:
- `globset` for pattern matching: Lower-level than `ignore`; doesn't understand `.gitignore` semantics (negation patterns, directory-only patterns). Rejected.
- No `.gitignore` crate (hand-rolled): Error-prone for edge cases in gitignore syntax. Rejected per Constitution IX (dependency justified).

## Summary of Decisions

| # | Topic | Decision |
|---|---|---|
| R1 | AST Parsing | `tree-sitter` + `tree-sitter-rust`, sync parsing via `spawn_blocking` |
| R2 | Embedding Model | `EmbeddingModel::BGESmallENV15` (384-dim, 512-token limit), clean replacement |
| R3 | Persistence Format | JSONL for nodes and edges, sorted by ID, source bodies excluded |
| R4 | Graph Traversal | Application-level BFS with per-level SurrealQL queries |
| R5 | Token Counting | Character-based estimation (4 chars/token), classify at ~2000 char boundary |
| R6 | Content Hashing | SHA-256 at file and symbol levels, existing `sha2` crate |
| R7 | Concerns Re-linking | `(name, body_hash)` tuple identity, orphan cleanup with context notes |
| R8 | Call Resolution | Name-based matching, best-effort for v0 |
| R9 | Vector Indexes | 384-dim MTREE COSINE, unchanged from v0 |
| R10 | New Dependencies | `tree-sitter`, `tree-sitter-rust`, `ignore` |
