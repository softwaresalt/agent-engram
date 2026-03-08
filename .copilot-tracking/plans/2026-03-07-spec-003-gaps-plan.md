<!-- markdownlint-disable-file -->
# Implementation Plan: Spec 003 Gap Remediation — GAP-001 & GAP-002

**Date**: 2026-03-07  
**Branch**: `003-unified-code-graph`  
**Research**: `.copilot-tracking/research/2026-03-07-spec-003-audit.md`

---

## Problem Summary

### GAP-001: Body re-derivation on hydration (Critical — T080)

When the daemon restarts, code graph state is loaded from `.engram/code-graph/nodes.jsonl`. The `body`
field (full source text of each symbol) is intentionally excluded from JSONL to keep the file compact
(FR-133). However, `hydration.rs:upsert_node` currently hard-codes `body: String::new()` for every
`function`, `class`, and `interface` node — so after any restart, all tools that surface body text
(`map_code`, `get_active_context`, `impact_analysis`) return an empty string. This is a silent
correctness failure: the tools succeed but their most valuable field is blank.

The fix reads the source file and extracts the relevant lines (`line_start..=line_end`) during
hydration. Both `line_start` and `line_end` are already serialised to JSONL and available in
`ParsedNode`. The file path is workspace-relative and the workspace path is already passed to
`hydrate_code_graph`.

### GAP-002: Zero-vector null serialization (Medium — T107)

When the `embeddings` feature is disabled (or the model is unavailable), symbols receive a
`vec![0.0; 384]` embedding placeholder. This zero vector is currently serialised to JSONL verbatim
(588 bytes of `0.0`s per symbol). On the next restart the zero vector is re-loaded into the DB, and
`vector_search_symbols` guards only with `!embedding.is_empty()` — so the zero vector passes the
guard and participates in cosine-similarity ranking. Because `cosine_similarity` on a zero denominator
returns 0.0 uniformly, every query matches zero-embedding records at score 0.0, polluting results.

The fix serialises embeddings as JSON `null` whenever all components are zero (or the vec is empty),
and adds a meaningful-embedding guard in `vector_search_symbols` to exclude any all-zero or empty
vector from ranking.

---

## Implementation Steps

### Phase 1 — GAP-001: Body re-derivation in `hydration.rs`

**File**: `src/services/hydration.rs`

#### Step 1.1 — Add `read_body_lines` helper

Add a new private `async fn` **after** the `parse_edge_line` function (currently line 387):

```rust
/// Read the source body for a symbol from its file on disk.
///
/// Extracts lines `line_start..=line_end` (1-based, inclusive) from `file_path`
/// (workspace-relative). Returns an empty `String` if the file cannot be read or
/// the line range is out of bounds; logs a warning in both cases.
async fn read_body_lines(
    workspace: &Path,
    file_path: &str,
    line_start: u32,
    line_end: u32,
) -> String {
    if file_path.is_empty() || line_start == 0 {
        return String::new();
    }
    let abs = workspace.join(file_path);
    let content = match tokio::fs::read_to_string(&abs).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                file = %abs.display(),
                error = %e,
                "hydration: cannot read source file for body re-derivation"
            );
            return String::new();
        }
    };
    let lines: Vec<&str> = content.lines().collect();
    let start = (line_start as usize).saturating_sub(1); // convert to 0-based
    let end = (line_end as usize).min(lines.len());       // inclusive → exclusive
    if start >= lines.len() || start > end {
        tracing::warn!(
            file = %abs.display(),
            line_start,
            line_end,
            total_lines = lines.len(),
            "hydration: line range out of bounds during body re-derivation"
        );
        return String::new();
    }
    lines[start..end].join("\n")
}
```

**Error handling rationale**:
- File-not-found: warn + empty body (symbol still usable for search by name/hash).
- Line range out of bounds: warn + empty body (file may have been edited since last index).
- No `?` propagation — a missing body is a degraded-but-non-fatal condition; the node should still
  be hydrated.

#### Step 1.2 — Thread `workspace: &Path` through `upsert_node`

Change the signature of `upsert_node` (line 392) from:
```rust
async fn upsert_node(cg_queries: &crate::db::queries::CodeGraphQueries, node: ParsedNode) -> bool {
```
to:
```rust
async fn upsert_node(
    cg_queries: &crate::db::queries::CodeGraphQueries,
    node: ParsedNode,
    workspace: &Path,
) -> bool {
```

#### Step 1.3 — Populate `body` for function / class / interface arms

Inside `upsert_node`, replace the three `body: String::new(), // body not persisted` lines with
calls to `read_body_lines`. Each arm has `node.file_path` and `node.line_start` / `node.line_end`
already unwrap-or-defaulted. **Pattern to apply in all three arms** (function at line 416, class at
line 433, interface at line 450):

```rust
// BEFORE:
body: String::new(), // body not persisted

// AFTER (in function arm — file_path/line_start/line_end are local let-bindings by this point):
body: read_body_lines(
    workspace,
    &node.file_path.clone().unwrap_or_default(),
    node.line_start.unwrap_or(0),
    node.line_end.unwrap_or(0),
).await,
```

> **Note**: `node` fields are `Option<T>` and are typically moved into local variables in this arm.
> Access them before the move, or clone as needed. Because `upsert_node` already does
> `node.file_path.unwrap_or_default()` etc., restructure so `file_path`, `line_start`, `line_end`
> are bound *before* the struct literal, then used in both the body call and the struct field.

**Concrete restructure for the `"function"` arm** (lines 407–423):

```rust
"function" => {
    let file_path = node.file_path.unwrap_or_default();
    let line_start = node.line_start.unwrap_or(0);
    let line_end   = node.line_end.unwrap_or(0);
    let body = read_body_lines(workspace, &file_path, line_start, line_end).await;
    let f = Function {
        id: node.id,
        name: node.name.unwrap_or_default(),
        file_path,
        line_start,
        line_end,
        signature: node.signature.unwrap_or_default(),
        docstring: node.docstring,
        body,
        body_hash: node.body_hash.unwrap_or_default(),
        token_count: node.token_count.unwrap_or(0),
        embed_type: node.embed_type.unwrap_or_default(),
        embedding: node.embedding.unwrap_or_default(),
        summary: node.summary.unwrap_or_default(),
    };
    cg_queries.upsert_function(&f).await.is_ok()
}
```

Apply the same restructure to the `"class"` and `"interface"` arms.

#### Step 1.4 — Update call site in `hydrate_code_graph`

At line 277:
```rust
// BEFORE:
if upsert_node(cg_queries, node).await {

// AFTER:
if upsert_node(cg_queries, node, path).await {
```

(`path` is the `workspace: &Path` parameter already in scope for `hydrate_code_graph`.)

---

### Phase 2 — GAP-002: Zero-vector null serialization

#### Step 2.1 — Add `is_meaningful_embedding` helper in `dehydration.rs`

Add a private helper just before `serialize_nodes_jsonl` (around line 622):

```rust
/// Returns `true` if `e` contains at least one non-zero component.
///
/// Used to distinguish a real embedding from the zero-vector placeholder that is
/// emitted when the `embeddings` feature is disabled or the model is unavailable.
fn is_meaningful_embedding(e: &[f32]) -> bool {
    !e.is_empty() && e.iter().any(|&v| v != 0.0)
}
```

#### Step 2.2 — Change `NodeLine.embedding` to `Option<Vec<f32>>` in `dehydration.rs`

In `NodeLine` (lines 573–591):
```rust
// BEFORE:
embedding: Vec<f32>,

// AFTER:
#[serde(skip_serializing_if = "Option::is_none")]
embedding: Option<Vec<f32>>,
```

#### Step 2.3 — Apply `is_meaningful_embedding` when building `NodeLine` in `serialize_nodes_jsonl`

In each of the three `NodeLine` construction blocks (functions ~line 643, classes ~line 675,
interfaces ~line 697), change the `embedding` field:

```rust
// BEFORE:
embedding: f.embedding.clone(),

// AFTER:
embedding: if is_meaningful_embedding(&f.embedding) {
    Some(f.embedding.clone())
} else {
    None
},
```

Apply the same pattern for classes (`c.embedding`) and interfaces (`i.embedding`).

#### Step 2.4 — Add zero-vector guard to `vector_search_symbols` in `queries.rs`

Add a private helper near the bottom of `queries.rs` (alongside the existing `cosine_similarity`
function at line ~3261):

```rust
/// Returns `true` if `e` is a non-empty vector with at least one non-zero component.
///
/// Excludes the zero-vector placeholder used when embeddings are unavailable.
fn has_meaningful_embedding(e: &[f32]) -> bool {
    !e.is_empty() && e.iter().any(|&v| v != 0.0)
}
```

In `vector_search_symbols` (lines 2867, 2897, 2927), change each guard:

```rust
// BEFORE:
if !f.embedding.is_empty() {

// AFTER:
if has_meaningful_embedding(&f.embedding) {
```

Apply to all three blocks (functions, classes, interfaces).

---

## Test Plan

### GAP-001 Tests

#### Unit test: `read_body_lines` — add inside `mod tests` in `hydration.rs`

```rust
#[tokio::test]
async fn read_body_lines_extracts_correct_lines() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let content = "line1\nline2\nline3\nline4\n";
    tokio::fs::write(tmp.path().join("src.rs"), content).await.unwrap();
    let body = read_body_lines(tmp.path(), "src.rs", 2, 3).await;
    assert_eq!(body, "line2\nline3");
}

#[tokio::test]
async fn read_body_lines_missing_file_returns_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let body = read_body_lines(tmp.path(), "nonexistent.rs", 1, 5).await;
    assert!(body.is_empty());
}

#[tokio::test]
async fn read_body_lines_out_of_bounds_returns_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    tokio::fs::write(tmp.path().join("f.rs"), "only one line\n").await.unwrap();
    let body = read_body_lines(tmp.path(), "f.rs", 99, 100).await;
    assert!(body.is_empty());
}
```

#### Integration test: body survives dehydrate-hydrate round-trip

Add to `tests/integration/code_graph_test.rs` (or a new `hydration_roundtrip_test.rs`):

```rust
#[tokio::test]
async fn body_is_repopulated_after_hydration_roundtrip() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();

    write_sample_file(
        ws,
        "src/lib.rs",
        "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n",
    );

    let config = CodeGraphConfig::default();
    let ws_id = workspace_hash(ws);

    // Step 1: index the workspace (body is populated from source).
    code_graph::index_workspace(ws, &ws_id, &config, false)
        .await
        .expect("index");

    // Step 2: flush (dehydrate) to JSONL.
    let engram_dir = ws.join(".engram");
    std::fs::create_dir_all(&engram_dir).unwrap();
    // Use the dehydration service to write nodes.jsonl.
    let cg_queries = /* build CodeGraphQueries for the indexed workspace */ ...;
    dehydration::dehydrate_code_graph(ws, &cg_queries, &engram_dir)
        .await
        .expect("dehydrate");

    // Step 3: create a fresh DB (simulating daemon restart) and hydrate.
    let fresh_db = /* new in-memory CodeGraphQueries */ ...;
    hydration::hydrate_code_graph(ws, &fresh_db)
        .await
        .expect("hydrate");

    // Step 4: query for the function and assert body is non-empty.
    let results = fresh_db
        .find_symbols_by_name("add")
        .await
        .expect("query");
    assert!(!results.is_empty(), "function should be present after hydration");
    let func = &results[0];
    assert!(
        !func.body.is_empty(),
        "body must be re-derived from source on hydration"
    );
    assert!(func.body.contains("a + b"), "body should contain function source");
}
```

> **Note**: Wire up the DB helper using the same pattern as existing `code_graph_test.rs`
> (`connect_db`, `workspace_hash`, `write_sample_file`). The exact `dehydrate_code_graph` call
> signature may need adjustment once the service API is confirmed.

---

### GAP-002 Tests

#### Unit test: zero embedding serialises as null — add in `dehydration.rs` `mod tests`

```rust
#[test]
fn zero_embedding_serializes_as_null() {
    use crate::models::Function;
    let f = Function {
        id: "function:test".into(),
        name: "f".into(),
        file_path: "src/lib.rs".into(),
        line_start: 1,
        line_end: 3,
        signature: "fn f()".into(),
        docstring: None,
        body: String::new(),
        body_hash: "abc".into(),
        token_count: 0,
        embed_type: "summary_pointer".into(),
        embedding: vec![0.0; 384],   // all zeros
        summary: "fn f()".into(),
    };
    let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
    // The embedding field must not appear (null is serialised as omitted by skip_serializing_if)
    assert!(!out.contains("\"embedding\""), "zero embedding must be omitted from JSONL");
}

#[test]
fn non_zero_embedding_serializes_as_array() {
    use crate::models::Function;
    let mut emb = vec![0.0f32; 384];
    emb[0] = 0.5;
    let f = Function {
        id: "function:test2".into(),
        name: "g".into(),
        file_path: "src/lib.rs".into(),
        line_start: 5,
        line_end: 7,
        signature: "fn g()".into(),
        docstring: None,
        body: String::new(),
        body_hash: "def".into(),
        token_count: 0,
        embed_type: "explicit_code".into(),
        embedding: emb,
        summary: "fn g()".into(),
    };
    let out = serialize_nodes_jsonl(&[], &[f], &[], &[]);
    assert!(out.contains("\"embedding\""), "non-zero embedding must be present in JSONL");
}
```

#### Unit test: null embedding deserialises back as empty vec in `hydration.rs` `mod tests`

```rust
#[test]
fn null_embedding_in_jsonl_deserializes_to_empty_vec() {
    let line = r#"{"id":"function:x","type":"function","name":"x","file_path":"src/lib.rs","line_start":1,"line_end":2,"body_hash":"abc","token_count":0,"embed_type":"summary_pointer","summary":"fn x()"}"#;
    let node: ParsedNode = serde_json::from_str(line).expect("parse");
    // embedding absent (null) → unwrap_or_default → empty vec
    assert!(node.embedding.is_none());
}
```

#### Unit test: `has_meaningful_embedding` logic — add in `queries.rs` `mod tests`

```rust
#[test]
fn meaningful_embedding_excludes_zero_vectors() {
    assert!(!has_meaningful_embedding(&[]));
    assert!(!has_meaningful_embedding(&vec![0.0f32; 384]));
    let mut e = vec![0.0f32; 384];
    e[100] = 0.01;
    assert!(has_meaningful_embedding(&e));
}
```

#### Integration test: vector search excludes zero-embedding records

Add in `tests/integration/code_graph_test.rs` or a new `vector_search_test.rs`:

```rust
#[tokio::test]
async fn vector_search_excludes_zero_embedding_symbols() {
    // Build a workspace with two functions.
    // Function A: gets a real (non-zero) embedding.
    // Function B: has a zero-vector embedding (embeddings feature disabled simulation).
    // Perform vector search with a query embedding similar to Function A.
    // Assert: only Function A appears in results, Function B is excluded.
    // (Use a hand-crafted embedding — no model needed.)
    ...
}
```

---

## Definition of Done

### GAP-001

- [ ] `read_body_lines` correctly extracts `line_start..=line_end` from the file (1-based).
- [ ] File-not-found and out-of-bounds produce `warn!` logs and return empty `String` (no panic).
- [ ] `upsert_node` accepts `workspace: &Path` and calls `read_body_lines` for symbol arms.
- [ ] `hydrate_code_graph` passes `path` to `upsert_node`.
- [ ] All three unit tests for `read_body_lines` pass.
- [ ] Integration test demonstrates `body` is non-empty and correct after dehydrate → hydrate.
- [ ] `cargo test` — all 262 existing tests still pass plus new tests.
- [ ] `cargo clippy -- -D warnings` — clean.

### GAP-002

- [ ] `NodeLine.embedding` is `Option<Vec<f32>>` with `#[serde(skip_serializing_if = "Option::is_none")]`.
- [ ] `is_meaningful_embedding` helper is present in `dehydration.rs`.
- [ ] Zero/empty embeddings are serialised as absent (null) in JSONL.
- [ ] Non-zero embeddings are serialised as JSON arrays.
- [ ] `has_meaningful_embedding` helper is present in `queries.rs`.
- [ ] `vector_search_symbols` excludes all-zero and empty embedding records.
- [ ] Unit tests for serialisation and the meaningful-embedding predicate pass.
- [ ] Integration test confirms zero-embedding records are excluded from vector search results.
- [ ] `cargo test` — all 262+ tests pass.
- [ ] `cargo clippy -- -D warnings` — clean.

---

## Constitution Check

### Async Correctness

- `read_body_lines` uses `tokio::fs::read_to_string` (already imported in `hydration.rs`). It is
  `async fn` and is `await`-ed inside `upsert_node` which is already `async`.
- No blocking I/O on the async executor.
- File reads happen once per symbol during hydration (cold path — acceptable cost).

### Safety

- No `unsafe` code added.
- `String::from_utf8_lossy` is not needed here: `tokio::fs::read_to_string` returns `io::Error`
  on invalid UTF-8 — callers receive the warn path cleanly. If lossy handling is preferred, use
  `tokio::fs::read` + `String::from_utf8_lossy`.
- Integer arithmetic: `line_start.saturating_sub(1)` and `.min(lines.len())` prevent underflow
  and out-of-bounds slice panics.

### Error Handling

- Both helpers return gracefully (not `Result`) — body re-derivation is best-effort.
- All error paths emit structured `tracing::warn!` with `file`, `error`, and line-range fields.
- No `unwrap()` or `expect()` added.

### Correctness of GAP-002

- Cosine similarity on a zero vector produces a zero score uniformly; excluding zero embeddings
  prevents false positives but does not silence legitimate low-score matches.
- Old JSONL files that still contain the zero-vector arrays will deserialise to `Some(vec![0.0; N])`
  via `node.embedding.unwrap_or_default()` → stored in DB. The `has_meaningful_embedding` guard in
  `vector_search_symbols` handles these legacy records without needing a migration.
- After the fix, newly written JSONL files omit the zero embedding entirely, saving ~588 bytes per
  symbol.

### No Cascading Model Changes

- `Function`, `Class`, `Interface` model structs keep `embedding: Vec<f32>` — no public API
  change. Only the JSONL wire format (`NodeLine`) and the vector-search gate change.
- `FunctionRow`, `ClassRow`, `InterfaceRow` in `queries.rs` are not changed.

---

## Dependencies

- No new Cargo dependencies.
- `tokio::fs` — already in scope in `hydration.rs`.
- `tracing` — already in scope.
- `serde` `skip_serializing_if` — already used in `NodeLine` for `signature` and `docstring`.

---

## File Change Summary

| File | Change |
|------|--------|
| `src/services/hydration.rs` | Add `read_body_lines` helper; update `upsert_node` signature + 3 arms; update call site |
| `src/services/dehydration.rs` | Add `is_meaningful_embedding`; change `NodeLine.embedding` to `Option<Vec<f32>>`; update 3 `NodeLine` construction blocks |
| `src/db/queries.rs` | Add `has_meaningful_embedding`; update 3 guards in `vector_search_symbols` |
| `tests/integration/code_graph_test.rs` | Add body round-trip test + vector-search exclusion test |
| (inline `mod tests`) | Unit tests for `read_body_lines`, serialisation, and the embedding predicates |
