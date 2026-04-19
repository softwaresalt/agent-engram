---
title: "U0.1 — CozoDB storage backend: SQLite vs RocksDB"
type: decision
date: 2026-04-19
status: decided
task_id: "001.001.001-T"
plan_ref: "docs/exec-plans/2026-04-19-cozodb-datalog-migration-plan.md"
spike_ref: "docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md"
tags:
  - cozodb
  - database
  - storage-backend
  - build-matrix
---

# U0.1 — CozoDB storage backend selection

## Decision

**Chosen backend: SQLite-backed Cozo** for Phase 1–6 of the migration.

**Cargo feature for production users seeking higher throughput: `cozo-rocksdb` (Phase 7 optional).**

## Alternatives considered

### Option A: SQLite-backed Cozo (`storage-sqlite`)

| Property | Value |
|---|---|
| Cargo feature | `storage-sqlite` |
| Rust crate | `cozo` with `features = ["storage-sqlite"]` |
| C/C++ build deps | None — uses `rusqlite` which bundles libsqlite3 as pure C |
| Windows support | Well-tested; pre-built binaries available |
| Write throughput | ~3K–8K `:put` / sec on typical laptop hardware |
| Read throughput (HNSW KNN) | Comparable to RocksDB for small-to-medium corpora |
| Build matrix complexity | Low — works on all three target platforms without extra LLVM/clang config |

**Limitation:** At 50K+ symbols with vertical partitioning (3 tables per symbol), initial
hydration would issue ~150K+ `:put` operations. At 5K `:put`/s that is ~30 seconds. Acceptable
for initial hydration on a laptop-class machine; not a blocking concern.

### Option B: RocksDB-backed Cozo (`storage-rocksdb`)

| Property | Value |
|---|---|
| Cargo feature | `storage-rocksdb` |
| Rust crate | `cozo` with `features = ["storage-rocksdb"]` |
| C/C++ build deps | `libclang`, LLVM headers, `cmake` (required by `rocksdb-sys`) |
| Windows support | Requires LLVM toolchain; CI setup is non-trivial |
| Write throughput | ~20K–80K `:put` / sec (10-20× SQLite) |
| Read throughput (HNSW KNN) | Comparable to SQLite for random-read KNN patterns |
| Build matrix complexity | High — requires additional OS-level packages on every CI runner |

**CIE's choice:** CIE uses RocksDB (architecture.md:1471–1475) and documents the trade-off:
*"CGO required; mitigation: pre-built binaries"*. CIE's authors accepted the C-dep cost
because their target deployment environment controls the build toolchain.

## Decision rationale

### Why SQLite for Phase 1–6

1. **Build matrix safety.** Engram's current CI runs on `ubuntu-latest`,
   `windows-latest`, and `macos-latest`. `rocksdb-sys` requires `libclang` and `cmake` which
   must be explicitly installed on every runner. `rusqlite` bundles its dependency cleanly
   on all three platforms with zero extra CI config.

2. **Acceptable hydration throughput.** The primary concern is initial-hydration latency
   for a large workspace (~50K–100K symbols). At 5K `:put`/s, 150K operations
   complete in ~30 seconds. For a daemon that runs persistently and caches the open DB
   handle, this is a one-time cost at first start. Subsequent operations are HNSW KNN queries
   (read-heavy) which are equally fast on both backends.

3. **HNSW performance is engine-agnostic.** CozoDB's HNSW index is implemented in the
   Cozo layer, not in the storage engine. Both backends use the same in-memory HNSW index
   structure; the only difference is how index changes are persisted. For a read-heavy
   workload (typical agent sessions), the HNSW query path is identical.

4. **Sequencing benefit.** The initial implementation can ship without requiring developers
   to install LLVM/clang. Once the CozoDB migration is complete and behaviorally verified,
   the RocksDB path can be added as an opt-in `cozo-rocksdb` cargo feature behind a
   conditional compilation gate.

5. **Engram's C-dep argument is weaker than for a pure-Rust project**, but not zero.
   Engram already requires `tree-sitter` (C) and `ort`/ONNX (C++). Adding RocksDB would
   add a third C++ dep with more complex CI requirements. The risk/reward ratio favors
   deferring it.

### Why not start with RocksDB

- CI complexity on Windows is disproportionately high for the throughput gain.
- SQLite per-database file isolation (`.engram/db/{branch}/`) is well-understood.
- Initial validation work is faster with the simpler stack.

## Implementation notes

### Cargo.toml changes (U1.1)

```toml
[dependencies]
# Primary CozoDB embedded backend
cozo = { version = "0.7", features = ["storage-sqlite"], optional = true }

[features]
cozo-backend = ["cozo"]
# Optional future: cozo-rocksdb = ["cozo", "cozo/storage-rocksdb"]
```

### DbInstance construction (U1.5)

```rust
use cozo::DbInstance;
// Connects to or creates the SQLite-backed database at the given path
let db = DbInstance::new("sqlite", db_path.to_str().unwrap(), Default::default())
    .map_err(|e| EngramError::from(SystemError::DatabaseError { reason: e.to_string() }))?;
```

For tests and the U0.4 API spike, use `DbInstance::new("mem", "", Default::default())` to
avoid disk I/O.

## Acceptance criteria (verify before closing U0.1)

- [x] Decision artifact exists at `docs/decisions/2026-04-19-cozo-storage-backend.md`
- [x] Both options and rationale documented
- [x] Cargo.toml snippet provided for U1.1 implementer
- [x] `DbInstance` construction snippet provided for U1.5 implementer
- [x] SQLite chosen; RocksDB deferred to post-Phase-6 optional feature

## Related decisions

- U0.3: HNSW index parameters (depends on this choice — parameters tuned for SQLite-backed HNSW)
- U1.1: Add cozo crate behind feature flag (uses this decision's Cargo snippet)
- U1.5: CozoBackend skeleton (uses this decision's DbInstance snippet)

## Known dependency issue (discovered U0.4)

`cozo 0.7.6` transitively depends on `graph → graph_builder 0.4.1`, which fails
to compile against `rayon 1.11.0` due to a breaking change in
`IntoParallelIterator` for boxed slices (`into_par_iter().copied()` now returns
`Item = T` rather than `Item = &T`).

**Impact:** `examples/cozo_api_spike.rs` does not compile. The lib target is unaffected.

**Resolution for U1.1:** when cozo moves from `[dev-dependencies]` to `[dependencies]`,
add one of the following to `Cargo.toml`:

```toml
# Option A — pin rayon below the breaking change
[dependencies]
rayon = ">=1.0, <1.11"

# Option B — patch graph_builder with a local fix
[patch.crates-io]
graph_builder = { path = "vendor/graph_builder" }
```

The API surface itself (`DbInstance`, `ScriptMutability`, `DataValue`, `Num`,
`NamedRows`) was verified correct by direct source inspection of the cozo 0.7.6
registry cache during Phase 0.
