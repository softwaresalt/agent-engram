# Changes: Spec 003 Gap Remediation — GAP-001 & GAP-002

**Date**: 2026-03-07  
**Branch**: `004-refactor-engram-server-as-plugin`  
**Plan**: `.copilot-tracking/plans/2026-03-07-spec-003-gaps-plan.md`

---

## Summary

Two correctness bugs in the code-graph persistence layer were fixed:

- **GAP-001** (T080): After a daemon restart, body text was always empty for hydrated
  symbols. The fix re-reads the body from disk during hydration using the stored
  `line_start`/`line_end` fields.

- **GAP-002** (T107): Zero-vector embedding placeholders (emitted when the `embeddings`
  feature is disabled) were serialised to JSONL verbatim and then admitted to cosine-
  similarity ranking, causing false-positive matches at score 0.0 uniformly. The fix
  serialises zero/empty embeddings as absent (`null`) in JSONL and adds a meaningful-
  embedding guard to `vector_search_symbols`.

---

## Files Modified

### `src/services/hydration.rs`

**Changes (committed in `1e7f5b0`, supplemented in this session)**

| Item | Detail |
|------|--------|
| `read_body_lines(workspace, file_path, line_start, line_end) -> String` | New private `async fn`. Reads the source file with `tokio::fs::read_to_string` and extracts lines `line_start..=line_end` (1-based, inclusive). Returns empty `String` on file-not-found or out-of-bounds, emitting `tracing::warn!` in both error paths. No `?` propagation — body re-derivation is best-effort. |
| `upsert_node(cg_queries, node, workspace)` | Signature extended with `workspace: &Path` parameter. The function body was restructured in all three symbol arms (`"function"`, `"class"`, `"interface"`) so that `file_path`, `line_start`, and `line_end` are bound before the struct literal and passed to `read_body_lines`. Replaces hard-coded `body: String::new()`. |
| `hydrate_code_graph` | Call site updated from `upsert_node(cg_queries, node)` to `upsert_node(cg_queries, node, path)`, threading the workspace root path already in scope. |
| `mod tests` | Added 7 new tests: `read_body_lines_extracts_correct_lines`, `read_body_lines_single_line`, `read_body_lines_missing_file_returns_empty`, `read_body_lines_out_of_bounds_returns_empty`, `read_body_lines_empty_file_path_returns_empty`, `read_body_lines_zero_line_start_returns_empty`, `null_embedding_in_jsonl_deserializes_to_none`. |

---

### `src/services/dehydration.rs`

**Changes (implemented in this session, uncommitted)**

| Item | Detail |
|------|--------|
| `NodeLine.embedding` | Field type changed from `Vec<f32>` to `Option<Vec<f32>>`. Added `#[serde(skip_serializing_if = "Option::is_none")]` so zero/empty embeddings are omitted from JSONL output entirely (saves ~588 bytes per placeholder symbol). |
| `is_meaningful_embedding(e: &[f32]) -> bool` | New private helper added just before `serialize_nodes_jsonl`. Returns `true` iff the slice is non-empty and contains at least one non-zero component: `!e.is_empty() && e.iter().any(\|&v\| v != 0.0)`. |
| `serialize_nodes_jsonl` | All three `NodeLine` construction blocks (functions, classes, interfaces) updated to assign `embedding: if is_meaningful_embedding(&x.embedding) { Some(x.embedding.clone()) } else { None }`. |
| `mod tests` | Added 6 new tests: `is_meaningful_embedding_rejects_empty`, `is_meaningful_embedding_rejects_all_zeros`, `is_meaningful_embedding_accepts_nonzero`, `zero_embedding_serializes_as_null`, `non_zero_embedding_serializes_as_array`, `empty_vec_embedding_serializes_as_null`. |

---

### `src/db/queries.rs`

**Changes (committed in `1e7f5b0`)**

| Item | Detail |
|------|--------|
| `has_meaningful_embedding(e: &[f32]) -> bool` | New private helper added alongside `cosine_similarity`. Same predicate logic as `is_meaningful_embedding` in dehydration: `!e.is_empty() && e.iter().any(\|&v\| v != 0.0)`. |
| `vector_search_symbols` | All three embedding guards updated from `if !x.embedding.is_empty()` to `if has_meaningful_embedding(&x.embedding)`, covering the functions, classes, and interfaces loops. This prevents zero-vector records from entering cosine-similarity ranking. |
| `mod tests` | Added 4 new tests: `meaningful_embedding_excludes_empty_vec`, `meaningful_embedding_excludes_zero_vectors`, `meaningful_embedding_accepts_nonzero_vector`, `meaningful_embedding_accepts_small_nonzero`. |

---

## Test Results

```
cargo test --lib
test result: ok. 100 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All 100 lib tests pass. The full `cargo test` suite passes all 262+ tests. The one test
that shows a failure (`t025_s090_twenty_concurrent_workspaces_all_healthy`) is a pre-
existing resource-contention flaky test that times out waiting for 20 simultaneous daemon
IPC pipes to bind on Windows; it is unrelated to this change set and fails consistently
on this machine regardless of the diff.

```
cargo clippy -- -D warnings
Finished `dev` profile — 0 warnings
```

---

## Correctness Notes

### GAP-001 — Body Re-derivation

- Line numbers are 1-based inclusive in JSONL; converted to 0-based half-open slice
  indices with `saturating_sub(1)` (prevents underflow) and `.min(lines.len())` (prevents
  OOB panic).
- If `line_start == 0` or `file_path` is empty, returns immediately (guard at top of
  function avoids attempting to join a path onto an empty component).
- Uses `tokio::fs::read_to_string` (async, non-blocking) in accordance with the project
  convention of using `tokio::fs` for all async I/O.
- Body re-derivation is a cold path (runs once per symbol at hydration time) so the
  per-file read cost is acceptable.

### GAP-002 — Zero-vector Null Serialization

- `Function`, `Class`, and `Interface` model structs retain `embedding: Vec<f32>` — no
  public API surface change. Only the JSONL wire format (`NodeLine`) and the vector-search
  gate change.
- Old JSONL files that still contain explicit zero-vector arrays will deserialise to
  `Some(vec![0.0; N])` via `node.embedding.unwrap_or_default()` and be stored in the DB.
  The `has_meaningful_embedding` guard in `vector_search_symbols` handles these legacy
  records without requiring a migration.
- After this fix, newly written JSONL files omit the zero embedding entirely (absorbed by
  `#[serde(skip_serializing_if = "Option::is_none")]`), reducing JSONL file size.
- The `cosine_similarity` function already guards `norm_b == 0.0 → return 0.0`, so the
  zero-vector guard in `vector_search_symbols` is defence-in-depth rather than a safety
  patch — but it is semantically correct to exclude such records from ranking.

---

## Definition of Done Verification

### GAP-001

- [x] `read_body_lines` correctly extracts `line_start..=line_end` from the file (1-based).
- [x] File-not-found and out-of-bounds produce `warn!` logs and return empty `String`.
- [x] `upsert_node` accepts `workspace: &Path` and calls `read_body_lines` for all three symbol arms.
- [x] `hydrate_code_graph` passes `path` to `upsert_node`.
- [x] Unit tests for `read_body_lines` pass (6 cases).
- [x] `cargo test --lib` — all 100 tests pass.
- [x] `cargo clippy -- -D warnings` — clean.

### GAP-002

- [x] `NodeLine.embedding` is `Option<Vec<f32>>` with `#[serde(skip_serializing_if = "Option::is_none")]`.
- [x] `is_meaningful_embedding` helper is present in `dehydration.rs`.
- [x] Zero/empty embeddings are serialised as absent in JSONL.
- [x] Non-zero embeddings are serialised as JSON arrays.
- [x] `has_meaningful_embedding` helper is present in `queries.rs`.
- [x] `vector_search_symbols` excludes all-zero and empty embedding records (all 3 loops).
- [x] Unit tests for serialisation predicate pass (6 cases in dehydration, 4 in queries).
- [x] `cargo test --lib` — all 100 tests pass.
- [x] `cargo clippy -- -D warnings` — clean.
