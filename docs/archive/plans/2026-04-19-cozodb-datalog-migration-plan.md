---
title: "CozoDB + Datalog migration — implementation plan"
type: exec-plan
date: 2026-04-19
source: "docs/decisions/2026-04-19-engram-cozodb-datalog-migration-spike.md"
stash_id: "23F4C476"
covering_kind: chore
hardening_required: yes
review_required: yes
tags:
  - database
  - cozodb
  - surrealdb
  - datalog
  - migration
  - hnsw
---

## Problem Frame

Engram persists code-graph and embedding state in an embedded SurrealDB 2.x instance under `.engram/db/{branch}/`. Surreal's MTREE vector index forces multi-statement query construction in `src/db/queries.rs` whenever a hybrid graph+vector lookup is needed, and Surreal's KNN cannot be combined with arbitrary `WHERE` filters in a single statement (verified in spike §1.4 and §8). The schema is a flat one-row-per-symbol model that conflates hot fields (id, name, file_path, kind) with cold fields (embedding payload, raw body, summary), forcing the planner to load the embedding column on every metadata read.

The CozoDB + Datalog migration replaces SurrealDB embedded with CozoDB embedded, swaps SurrealQL for CozoScript Datalog, and redesigns the schema along the patterns CIE (Code Intelligence Engine, https://github.com/kraklabs/cie) arrived at after three schema iterations: vertical partitioning per symbol type, entity-with-stable-id edges, and validation-at-ingest. The migration ships behind a feature flag (`cozo-backend`), runs both engines in parallel through a `CodeGraphBackend` trait, and cuts over only after dual-backend equivalence assertions and a Phase-0 HNSW recall + latency benchmark pass.

## Requirements Trace

Each requirement is a direct lift from the source spike. Each maps to one or more implementation units below.

| Req | Source | Implementation units |
|---|---|---|
| R1 — replace SurrealDB embedded with CozoDB embedded behind a feature flag | spike §1, §9 | U1.1, U1.2, U1.4 |
| R2 — promote `CodeGraphQueries` to a backend trait without behavior change | spike §9 | U1.3 |
| R3 — translate the schema to CozoScript Datalog with vertical partitioning per CIE §16.2 | spike §16.13 | U2.1, U2.2 |
| R4 — port symbol CRUD (function, class, interface) with dual-backend equivalence | spike §13 Phase 2 | U2.3, U2.4, U2.5, U2.6 |
| R5 — port edge CRUD with entity-with-stable-id IDs per CIE §16.3 | spike §16.13, §16.14 | U3.1, U3.2 |
| R6 — port concerns-edge specialty queries | spike §13 Phase 3 | U3.3, U3.4 |
| R7 — rewrite graph traversal (`bfs_neighborhood`, `graph_neighborhood`) as Datalog | spike §8.2 | U3.5 |
| R8 — port symbol identity / lookup queries | spike §13 Phase 3 | U3.6 |
| R9 — bring up HNSW index per spike §16.13 schema | spike §16.13 | U4.1 |
| R10 — rewrite `vector_search_*` and `hybrid_graph_vector_search` against the HNSW + Datalog combined-rule API | spike §8.1, §16.5 | U4.2, U4.3, U4.4 |
| R11 — embedding write-back + GC simplification | spike §13 Phase 4 | U4.5 |
| R12 — validation-at-ingest port from CIE `validateFunctionEmbedding` | spike §16.8, §16.14 (Phase 2.5) | U2.7 |
| R13 — content_record, commit_node + normalized commit_change, file_hash, hydration glue | spike §13 Phase 5 | U5.1, U5.2, U5.3, U5.4 |
| R14 — flip default backend to `cozo-backend` and update docs | spike §13 Phase 6 | U6.1, U6.2 |
| R15 — operational closure: monitoring plan, rollback trigger, observation window | spike §13 Phase 6, release-observability overlay | U6.3 |
| R16 — drop SurrealDB dep and dead row types in next minor release | spike §13 Phase 7 | U7.1, U7.2 |
| R17 — Phase-0 HNSW benchmark to gate Phase 1 | spike §12 | U0.2 |
| R18 — Phase-0 storage backend selection (RocksDB vs SQLite-backed Cozo) | spike §13 Phase 0, §14 | U0.1 |
| R19 — Phase-0 vector index parameters (`m`, `ef_construction`, `ef_query`) | spike §13 Phase 0 | U0.3 |
| R20 — Phase-0 Rust `cozo` crate API spike to confirm trait surface viability | spike §16.14 | U0.4 |
| R21 — Phase-0 embedding-model micro-benchmark (384 baseline vs. 768 candidates) | spike §17.6 | U0.5 |
| R22 — schema parameterization so future model swap requires only re-hydration | spike §17.7 | U2.8 |
| R23 — absorb queued 003-F (move code-graph under `db/{branch}/`) | this plan, §16.13 layout | U2.1 (note); 003-F closes as superseded after migration ships |

## Implementation Units

Units are grouped by phase. Each unit is sized to the 2-hour rule (fewer than 3 files, fewer than 5 functions, fewer than 4 test scenarios, single skill domain). Phase IDs are used as task-ID prefixes during harvest.

### Phase 0 — Spike close-out

* **U0.1 — Pick storage backend (RocksDB vs SQLite-backed Cozo).** Decision artifact at `docs/decisions/2026-04-{NN}-cozo-storage-backend.md`. Inputs: pure-Rust desire vs. RocksDB performance, build matrix impact on Linux/macOS/Windows. Posture: investigate-first.
  * Files: `docs/decisions/`
  * Verification: decision artifact exists, references both options and the rationale.
* **U0.2 — Run §12 HNSW benchmark and write findings.** Build a fresh fixture corpus (~5K symbols), run all four §12 workloads, compare against SurrealDB MTREE baseline. Pass criteria are §12's: recall@10 ≥ 0.95, p95 KNN ≤ 1.5× baseline, filtered KNN p95 ≤ baseline, hybrid p95 ≤ baseline, build time within 2× baseline. Output: `docs/decisions/2026-04-{NN}-cozo-hnsw-benchmark.md`.
  * Files: `tests/integration/cozo_hnsw_benchmark.rs` (new; gated `#[ignore]`), `docs/decisions/`
  * Verification: benchmark binary runs, decision artifact records each pass criterion outcome.
* **U0.3 — Decide vector index parameters.** From U0.2 results, pick `m`, `ef_construction`, default `ef_query`. Document in U0.2 artifact.
  * Files: `docs/decisions/`
  * Verification: parameters captured in benchmark artifact with rationale.
* **U0.4 — Rust `cozo` crate API spike.** ~2-hour prototype that exercises the methods needed for the §9 trait surface in the actual `cozo` crate API (transactions, `:put`, fixed rules, HNSW DDL). Confirms the §9 abstraction is achievable in idiomatic Rust before Phase 2 begins.
  * Files: `examples/cozo_api_spike.rs` (throwaway; deleted at Phase 1 start)
  * Verification: example compiles and runs against a fresh in-memory Cozo store.
* **U0.5 — Embedding-model micro-benchmark.** Per spike §17.6 — measure hydration wall time and a ~30-query golden set across `bge-small-en-v1.5` (baseline), `jina-embeddings-v2-base-code` (768), `nomic-embed-text-v1.5` (768). Single 5K-symbol fixture, single laptop-class machine. Output: `docs/decisions/2026-04-{NN}-embedding-model-benchmark.md`. **Does not block Phase 1.**
  * Files: `tests/integration/embedding_model_benchmark.rs` (new; `#[ignore]`), `docs/decisions/`
  * Verification: artifact contains a comparison table of wall time and MRR per model.

### Phase 1 — Parallel-DB scaffolding

* **U1.1 — Add `cozo` crate behind `cozo-backend` feature flag.** Add dep with chosen storage backend (from U0.1). No call sites yet. Confirm `cargo check --features cozo-backend` succeeds.
  * Files: `Cargo.toml`
  * Verification: `cargo check --no-default-features --features cozo-backend,embeddings` passes.
* **U1.2 — Wire `db::connect_db` to choose backend by feature.** No-op for `cozo-backend` initially (return `unimplemented!()`); exists to make the call site compile under both feature sets.
  * Files: `src/db/mod.rs`
  * Verification: `cargo check` passes under both `surreal-backend` (default) and `cozo-backend`.
* **U1.3 — Promote `CodeGraphQueries` to `CodeGraphBackend` trait; rename concrete struct to `SurrealBackend`.** Surgical, no behavior change. Trait surface from spike §9.
  * Files: `src/db/queries.rs`, `src/db/mod.rs`
  * Verification: existing tests pass unchanged under default features.
* **U1.4 — Add `tests/helpers/dual_backend.rs` and `assert_dual_eq!` macro.** Helper fixture that creates a `SurrealBackend` and a `CozoBackend` (the latter behind `cozo-backend`), runs the same operation against both, and compares results normalized for ordering. Macro shape: `assert_dual_eq!(method, args)`. CI matrix axis defers to U1.6.
  * Files: `tests/helpers/dual_backend.rs` (new), `tests/helpers/mod.rs`
  * Verification: helper compiles; smoke test asserts both backends agree on an empty-DB count query.
* **U1.5 — Stub `CozoBackend` skeleton implementing `CodeGraphBackend`.** All methods return `unimplemented!("phase 2+")`. Not exercised at runtime; needed so Phases 2–5 can fill methods one at a time without breaking the type system.
  * Files: `src/db/cozo_backend/mod.rs` (new)
  * Verification: `cargo check --no-default-features --features cozo-backend,embeddings` passes.
* **U1.6 — Add CI matrix axis for dual-backend test runs.** Update `.github/workflows/*.yml` to run the test suite under both `surreal-backend` (default) and `cozo-backend` feature sets. Initially expects most tests to be feature-gated `#[cfg(feature = "surreal-backend")]`; tests migrate as Phases 2–5 complete.
  * Files: `.github/workflows/ci.yml` (or equivalent)
  * Verification: CI runs both axes; the cozo axis can be advisory until Phase 4 completes.

### Phase 2 — Schema + symbol CRUD parity

* **U2.1 — Translate `src/db/schema.rs` → CozoScript bootstrap module.** Implements the spike §16.13 schema verbatim: vertically partitioned `function`, `function_code`, `function_embedding` (and the same triple for `class` and `interface`), plus `file`, `import`, `commit_node`. Embedding tables declare `<F32; EMBEDDING_DIM>` using the constant from U2.8. Includes the four `::hnsw create` index DDL statements. Notes that `code-graph/` directory layout consolidates under `.engram/db/{branch}/` per queued 003-F (003-F can close as superseded once Phase 6 ships).
  * Files: `src/db/cozo_backend/schema.rs` (new)
  * Verification: bootstrap succeeds against a fresh in-memory Cozo store; `::relations` lists all 12 expected relations.
* **U2.2 — Add the 6 vertically partitioned symbol tables to schema bootstrap.** Already covered by the literal §16.13 schema in U2.1; this task is the dehydrate/hydrate path update — every `:put` of a symbol writes to all three of its derived tables in one transaction.
  * Files: `src/db/cozo_backend/schema.rs`, `src/db/cozo_backend/symbol_writes.rs` (new)
  * Verification: a unit test inserts one function and asserts the row appears in all three of `function`, `function_code`, `function_embedding`.
* **U2.3 — `code_file` CRUD parity.** `put_code_file`, `select_code_file_by_path`, `delete_code_file`. Use `assert_dual_eq!`.
  * Files: `src/db/cozo_backend/files.rs` (new)
  * Verification: dual-backend tests for put / read / delete.
* **U2.4 — `function` CRUD parity.** Includes the three-table write fan-out from U2.2.
  * Files: `src/db/cozo_backend/functions.rs` (new)
  * Verification: dual-backend tests; vertical-partition write fan-out asserted.
* **U2.5 — `class` CRUD parity.** Same shape as U2.4.
  * Files: `src/db/cozo_backend/classes.rs` (new)
  * Verification: dual-backend tests.
* **U2.6 — `interface` CRUD parity.** Same shape.
  * Files: `src/db/cozo_backend/interfaces.rs` (new)
  * Verification: dual-backend tests.
* **U2.7 — Validation-at-ingest port (Phase 2.5).** Port CIE's `validateFunctionEmbedding` pattern into the Rust ingest path: reject NaN/Inf in embedding vectors, reject dimension mismatch, reject empty-string IDs. Demote `gc_corrupted_embeddings` to defense-in-depth backstop.
  * Files: `src/services/embedding.rs`, `src/services/code_graph.rs`
  * Verification: unit tests assert each rejection case; existing GC tests still pass.
* **U2.8 — Parameterize `EMBEDDING_DIM` and `EMBEDDING_MODEL` constants per spike §17.7.** Single source of truth for both values; schema bootstrap reads `EMBEDDING_DIM` rather than literal `384`. Daemon status output exposes `EMBEDDING_MODEL`.
  * Files: `src/services/embedding.rs`, `src/db/cozo_backend/schema.rs`, `src/tools/lifecycle.rs`
  * Verification: unit test asserts `EMBEDDING_DIM == 384`; `get_daemon_status` integration test asserts the model string is present in the response.
* **U2.9 — Aggregate counts (`count_*`).** Port the count queries.
  * Files: `src/db/cozo_backend/aggregates.rs` (new)
  * Verification: dual-backend test on a known fixture.
* **U2.10 — Dual-backend integration test sweep for Phase 2.** Bring the existing symbol-CRUD integration tests under the dual-backend macro.
  * Files: `tests/integration/cozo_symbol_parity_test.rs` (new), updates to existing tests
  * Verification: CI cozo axis turns from advisory to required for symbol-CRUD tests.

### Phase 3 — Edge + traversal parity

* **U3.1 — Edge ID derivation helper.** A small module that produces deterministic stable IDs for edges per spike §16.3 (e.g., `format!("call:{}|{}", caller_id, callee_id)`).
  * Files: `src/db/cozo_backend/edges/ids.rs` (new)
  * Verification: unit tests assert determinism and uniqueness for the five edge kinds.
* **U3.2 — Edge `:put` parity (calls / imports / defines / inherits_from / concerns).** Five edge kinds, each with put + delete.
  * Files: `src/db/cozo_backend/edges/mutations.rs` (new)
  * Verification: dual-backend tests per edge kind.
* **U3.3 — Concerns-edge specialty queries (Part 1).** `get_concerns_edges_for_file`, `delete_concerns_edges_for_symbol`, `concerns_edge_exists`.
  * Files: `src/db/cozo_backend/edges/concerns.rs` (new)
  * Verification: dual-backend tests.
* **U3.4 — Concerns-edge specialty queries (Part 2).** `list_concerns_for_task[s]`, `find_tasks_for_symbols`.
  * Files: `src/db/cozo_backend/edges/concerns.rs`
  * Verification: dual-backend tests.
* **U3.5 — Rewrite `bfs_neighborhood` and `graph_neighborhood` as Datalog programs per spike §8.2.** Recursive Datalog rule with depth bound, cycle handling, kind filter.
  * Files: `src/db/cozo_backend/traversal.rs` (new)
  * Verification: dual-backend tests at depths 1, 2, 3 on a fixture with cycles.
* **U3.6 — Symbol identity / lookup queries.** `find_symbols_by_name`, `find_symbols_by_name_and_hash`, `get_symbol_identities_for_file`, `resolve_symbol`, `list_symbols`.
  * Files: `src/db/cozo_backend/lookups.rs` (new)
  * Verification: dual-backend tests for each method.

### Phase 4 — Vector + hybrid parity

* **U4.1 — HNSW index creation in schema bootstrap.** Activate the four `::hnsw create` statements from U2.1's schema. Index parameters from U0.3.
  * Files: `src/db/cozo_backend/schema.rs`
  * Verification: integration test confirms indexes exist after bootstrap; insert + KNN smoke test returns expected nearest neighbor.
* **U4.2 — `vector_search_symbols_native` rewrite.** Single `?[id, score] := ~function_embedding:idx{...}` rule with combined filter.
  * Files: `src/db/cozo_backend/vector.rs` (new)
  * Verification: dual-backend test; recall@10 vs. exhaustive ground truth on fixture.
* **U4.3 — `vector_search_content_native` rewrite (kills the MTREE+WHERE workaround).** Replaces the multi-statement Surreal hack with a single Cozo rule.
  * Files: `src/db/cozo_backend/vector.rs`
  * Verification: dual-backend test; assert SurrealBackend's over-fetch+filter still works and CozoBackend's combined-rule path returns the same set.
* **U4.4 — `hybrid_graph_vector_search` rewrite (the spike §8.1 program).** Single Datalog program: KNN seed → graph traversal → score combination.
  * Files: `src/db/cozo_backend/hybrid.rs` (new)
  * Verification: dual-backend test on fixture; p95 latency captured for comparison.
* **U4.5 — Embedding write-back + GC simplification.** With validation-at-ingest in U2.7, the GC path simplifies to a backstop sweep. Update `gc_corrupted_embeddings` to a smaller, defensive form.
  * Files: `src/db/cozo_backend/embeddings.rs` (new), `src/services/code_graph.rs`
  * Verification: GC integration test still passes; corruption injection test still triggers cleanup.

### Phase 5 — Auxiliary surfaces

* **U5.1 — `content_record` CRUD.** Includes the dedicated `content_record:idx` HNSW index from U2.1.
  * Files: `src/db/cozo_backend/content.rs` (new)
  * Verification: dual-backend tests.
* **U5.2 — `commit_node` + new normalized `commit_change` table; rewrite `select_commits_by_file_path` as a join.** Separates per-commit metadata from per-file change rows; eliminates the JSONL `created_at` duplication issue (spike §14).
  * Files: `src/db/cozo_backend/commits.rs` (new), `src/db/cozo_backend/schema.rs` (add `commit_change` relation)
  * Verification: dual-backend test asserting same commit list per file path on fixture.
* **U5.3 — `file_hash` CRUD.** Direct port.
  * Files: `src/db/cozo_backend/file_hash.rs` (new)
  * Verification: dual-backend tests.
* **U5.4 — Hydration / dehydration glue updates.** JSONL on-disk format unchanged; only the upsert backend changes. Verify that a deleted-DB cold restart fully rehydrates from JSONL into the Cozo backend.
  * Files: `src/services/hydration.rs`
  * Verification: integration test deletes `.engram/db/{branch}/` and asserts both backends rehydrate identically (per memory `vector hydration`).

### Phase 6 — Cutover

* **U6.1 — Flip default feature to `cozo-backend`.** `surreal-backend` remains compilable behind a feature for one release cycle.
  * Files: `Cargo.toml`
  * Verification: `cargo build` (default) uses CozoBackend; `cargo build --no-default-features --features surreal-backend,embeddings` still compiles.
* **U6.2 — Update `docs/ARCHITECTURE.md`, `AGENTS.md`, copilot instructions to reference Cozo + Datalog.** Includes a migration note pointing at this plan and the spike. Closes queued **003-F** as superseded (the new layout is in `src/db/cozo_backend/schema.rs` and `db/{branch}/` is the canonical home).
  * Files: `docs/ARCHITECTURE.md`, `AGENTS.md`, `.github/copilot-instructions.md`
  * Verification: doc lint passes; `003-F` is moved to `done` with a comment referencing this plan.
* **U6.3 — Operational closure: monitoring plan, rollback trigger, post-deploy observation window (release-observability overlay).** Captures: SLI = HNSW p95 latency on `unified_search`; alert threshold = > 2× pre-cutover baseline; rollback trigger = sustained recall regression (manual MRR-on-fixture probe drops > 10%) within 24 h of cutover; rollback procedure = re-flip default to `surreal-backend` (one-line `Cargo.toml` change), invalidate the cozo `db/{branch}/` directory, restart daemon to rehydrate from JSONL into Surreal; observation window = 1 week, owner = repo maintainer.
  * Files: `docs/closure/2026-{NN}-cozo-cutover-closure.md`
  * Verification: closure artifact exists with all five fields populated.

### Phase 7 — Removal (next minor release)

* **U7.1 — Drop `surrealdb` dep and `surreal-backend` feature.** After one release cycle of dual-backend coexistence.
  * Files: `Cargo.toml`, `src/db/mod.rs`
  * Verification: `cargo build` succeeds; no `surrealdb::` references remain.
* **U7.2 — Delete `SurrealBackend` impl and dead row types.** Remove the trait impl, row types, and any leftover SurrealQL strings.
  * Files: `src/db/surreal_backend/` (delete)
  * Verification: `cargo build` + `cargo clippy -- -D warnings -D clippy::pedantic` pass.

## Dependency Graph

Phase ordering is enforced by feature-flag cohabitation; within a phase, units have intra-phase dependencies as noted.

```text
U0.1, U0.2, U0.3, U0.4, U0.5  (Phase 0; U0.3 depends on U0.2; U0.5 independent of U0.1–U0.4)
            │
            ▼
U1.1 → U1.2 → U1.3 → U1.4 → U1.5 → U1.6   (Phase 1; strict sequential)
                                  │
                                  ▼
U2.8 → U2.1 → U2.2 → {U2.3, U2.4, U2.5, U2.6}  (Phase 2; CRUD parallelizable after U2.2)
                          │            │
                          ▼            ▼
                       U2.7 (Phase 2.5; depends on U2.4/U2.5/U2.6 ingest paths)
                          │
                          ▼
                       U2.9 → U2.10
                                  │
                                  ▼
U3.1 → U3.2 → {U3.3, U3.4, U3.5, U3.6}   (Phase 3; parallelizable after U3.2)
                                       │
                                       ▼
U4.1 → {U4.2, U4.3, U4.4} → U4.5         (Phase 4)
                                       │
                                       ▼
{U5.1, U5.2, U5.3, U5.4}                  (Phase 5; independent)
                                       │
                                       ▼
U6.1 → U6.2 → U6.3                        (Phase 6; sequential)
                                       │
                                       ▼ (next minor release)
U7.1 → U7.2                               (Phase 7)
```

## Decisions and Rationale

| Decision | Rationale | Source |
|---|---|---|
| Use chore (not feature) as covering work item | Migration is internal infrastructure; no new user-visible capability | Stage Step 5.1 |
| Vertically partition symbol tables (3 tables per type) | CIE arrived at this after 3 schema iterations and documented 10× memory-footprint reduction | spike §16.2, §16.13 |
| Entity-with-stable-id edges instead of composite keys | CIE source comment explicit on composite-key footguns | spike §16.3 |
| Validation-at-ingest as Phase 2.5 (NEW) | Port CIE's `validateFunctionEmbedding`; demotes GC sweep from primary path to backstop | spike §16.8, §16.14 |
| Ship migration at 384-dim / `bge-small-en-v1.5` | Don't bundle two changes; embedding model swap is a separate decision with its own re-hydration UX surface | spike §17.6 |
| Phase-0 embedding micro-benchmark independent of Phase-0 HNSW benchmark | Decouples the model decision from the migration go/no-go gate | spike §17.6 |
| Parameterize `EMBEDDING_DIM` and `EMBEDDING_MODEL` from single constants | Future model swap is a re-hydration concern, not a code-change concern | spike §17.7 |
| Single-namespace v1; Glean-style prefix preserved as forward extension | CIE proves 5-language single namespace works; prefix is a future addition | spike §10 |
| Dual-backend trait + feature flag, not big-bang cutover | Risk reduction; allows recall and latency comparison on real workloads | spike §9 |
| Cozo + Datalog over Postgres + pgvector / Neo4j / SQLite + custom | CIE's documented rationale maps 1:1 to Engram | spike §16.10 |

## Risks and Caveats

The full risk register is in spike §14 + §16.12 + §17.8. Highlights with mitigation owners:

| Risk | Severity | Mitigation in this plan |
|---|---|---|
| HNSW recall regression vs MTREE | High | U0.2 benchmark is a hard go/no-go gate. Pass criterion: recall@10 ≥ 0.95. |
| Cozo write throughput under bulk hydration | Medium | U5.4 measures hydration time on existing fixture; tune `:put` batching with explicit transactions. |
| Test surface drift during multi-phase migration | Medium | U1.4 dual-backend macro + U1.6 CI matrix axis catch divergence per phase. |
| Engram diverges from CIE patterns and re-discovers footguns | Medium | U2.1, U3.1, U2.7 explicitly adopt CIE patterns. |
| Rust `cozo` crate API differs materially from CIE's CGo wrapper | Medium | U0.4 is the gating spike before Phase 2 begins. |
| RocksDB C++ dependency on every supported platform | Medium | U0.1 explicitly evaluates SQLite-backed Cozo as pure-Rust alternative. |
| Operator assumes higher-dim = better retrieval and ships 1.5B model | Medium | This plan explicitly recommends 384-dim ship and parks 1.5B as future GPU-only opt-in. U0.5 grounds the model decision. |
| Schema created with literal `384` | Low | U2.8 parameterizes `EMBEDDING_DIM`. |
| §17.7 dim-swap path drifts | Low | A small `tests/integration/embedding_dim_swap_test.rs` re-hydrates with two dim configs on a tiny fixture. |
| Schema version field bump | Low | Bump `SCHEMA_VERSION` to `"3.0.0"` at U6.1; hydration falls back to "rebuild from JSONL" on mismatch (already supported). |

## Plan Hardening Signals (REQUIRED)

* **public API, schema, or contract change** — **PRESENT**: `SCHEMA_VERSION` bump, full DB schema swap, `CodeGraphBackend` trait introduction.
* **security, auth, permission, or compliance-sensitive behavior** — absent.
* **migration, backfill, destructive data/config action, or irreversible step** — **PRESENT**: full DB engine swap with on-disk format change. Mitigated by JSONL durable ground truth (rebuildable index). Phase 6 default flip is reversible via feature-flag flip + DB directory invalidation.
* **external integration, operator checkpoint, or external dependency** — **PRESENT**: new `cozo` crate dependency (and possibly RocksDB C++ dep depending on U0.1).
* **high runtime, rollout, or rollback risk** — **PRESENT**: HNSW recall regression risk; Cozo write throughput unknown until U0.2.

**Conclusion: Requires plan hardening: yes**

## Runtime Verification and Closure

| Surface affected | Runtime verification needed | Closure artifact |
|---|---|---|
| Daemon DB layer (all read/write paths) | Dual-backend equivalence assertions per phase (U1.4 macro + U1.6 CI matrix) | none until cutover |
| `unified_search` MCP tool (HNSW path) | U0.2 benchmark + U4.2/U4.3 dual-backend tests; post-cutover MRR probe on fixture | U6.3 closure record |
| `hybrid_graph_vector_search` (most user-visible read path) | U4.4 dual-backend test + post-cutover p95 latency probe | U6.3 closure record |
| `set_workspace` / `flush_state` lifecycle | U5.4 hydration/dehydration test (delete-and-restart) | U6.3 closure record |
| Default backend feature flag (U6.1) | Build-time verification under both `--features` sets | U6.3 closure record |
| Operational metrics (`record_query_metrics`) | Wrap every `CozoBackend` method with the same `record_query_metrics` call in the same `query_type` taxonomy | U6.3 closure record |

---

## Plan Hardening

### Hardening required and why

This plan declares all five hardening signals (one absent — security). The combination of: full DB engine swap, schema redesign, on-disk format change, new third-party dependency with possible non-Rust transitive (RocksDB), and HNSW recall risk against an unverified vector backend make this the highest-blast-radius change in the recent backlog. Plan hardening is required.

### Reinforcing context consulted

* `docs/compound/build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` — informs the U1.1 dependency-add posture (carefully verify `cozo`'s transitive C deps don't conflict with `tree-sitter`'s ABI pinning).
* `.github/instructions/release-observability.instructions.md` — informs U6.3 closure scope (SLI, threshold, rollback trigger, observation window, owner).
* `.github/instructions/strict-safety.instructions.md` — risky actions classified below.
* spike memories (§14, §16.12, §17.8) — risk register source of truth.
* `docs/memory/2026-04-19/cozodb-spike-memory.md` — full spike continuity record, including the four research passes that produced this plan.

### Risky actions (ProposedAction / ActionRisk)

| ProposedAction | ActionRisk | Approval | ActionResult target |
|---|---|---|---|
| **U0.1 storage backend choice (RocksDB)** — adds a non-Rust C++ build dep on every platform | high | operator approval before merging Phase 0 if RocksDB is chosen over SQLite-backed Cozo | applied |
| **U2.1 schema bootstrap creating 12 new relations and 4 HNSW indexes in a fresh Cozo store** | moderate | none required (dev-time only; per-branch DB is disposable) | applied |
| **U6.1 flip default feature to `cozo-backend`** | high | operator approval; coordinated with U6.3 closure | approved → applied |
| **U6.3 cutover requires the post-deploy observation window to start before declaring the migration done** | high | operator approval to start the window; window owner named in closure | applied |
| **U7.1 drop `surrealdb` dep** | destructive | operator approval; gated on U6.3 observation window passing without rollback trigger fire | planned (next minor release) |
| **U7.2 delete `SurrealBackend` impl** | destructive | bundled with U7.1 approval | planned |
| **Rollback path: re-flip default to `surreal-backend`, invalidate cozo `db/{branch}/`, restart daemon to rehydrate from JSONL into Surreal** | high | requires explicit operator invocation; documented in U6.3 closure | planned (rollback only; should never run if U0.2 + U4.2/U4.3 + U4.4 pass) |

### Added verification

* **U0.2 acceptance gate is a hard go/no-go** — Phase 1 cannot start unless §12 pass criteria are met. Document failure as a P-005 broadcast and route back to spike for re-investigation.
* **U2.10 + U1.6** establish a CI matrix axis that runs every dual-backend test under both feature sets. The cozo axis is advisory through Phase 3, then required from Phase 4 onward.
* **U4.2 / U4.3 / U4.4** add MRR-on-fixture probes alongside dual-backend equality tests, because exact equality is not the right oracle for HNSW (approximate index).
* **U5.4** explicitly tests the cold-restart-after-DB-delete path per the `vector hydration` memory pattern (delete `.engram/db/<branch>` between daemon restarts to prove rehydration is from JSONL, not embedded reuse).

### Added closure detail

* **U6.3** specifies SLI, alert threshold, rollback trigger, rollback procedure, observation window duration, and owner. Without all five, Phase 6 cannot complete.
* **U6.3** explicitly references the rollback procedure in operator-runnable steps, not abstract description.
* The post-deploy observation window is 1 week, owned by the repo maintainer. During the window, the `surreal-backend` feature remains compilable as the rollback escape hatch.

### Unresolved operator decisions

* **U0.1** RocksDB vs. SQLite-backed Cozo — unresolvable without running the spike. Tasked.
* **U0.5** specific 768-dim model recommendation — deferred to U0.5 benchmark; not blocking the migration.
* Whether to ship U7.1 + U7.2 in the same release or stage them across two — operator decision deferred until U6.3 observation window closes.

---

## Plan Review

### Gate decision: **PASS**

Reviewed by Stage agent acting in plan-review capacity (single-reviewer mode; cross-model not invoked because no separate reviewer model surface is available in this session). Findings are merged into a single advisory list rather than spawned across separate persona subagents.

### Hardening verification

* `## Plan Hardening` section present — yes.
* Hardening signals match plan content — yes (five-signal declaration in line with the migration shape).
* Risky actions classified as ProposedAction with ActionRisk — yes (7 entries spanning moderate / high / destructive).
* Rollback procedure operator-runnable — yes (U6.3 + Hardening "Rollback path" entry).
* Runtime verification covers every changed surface — yes (Phase 1–6 all map to verification entries).
* Operational closure scope complete — yes (SLI, threshold, trigger, procedure, window, owner all named in U6.3).

### Findings

**P0 (block harvest):** none.

**P1 (block harvest):** none.

**P2 (record as backlog follow-up):**

* **PR-1.** U0.5 (embedding micro-benchmark) is independent of the migration but lives in Phase 0 of this plan. Strictly it could be its own follow-on chore. Keeping it here is a forcing function — operator gets the data with the migration's other Phase-0 outputs. Acceptable trade-off; recorded for awareness.
* **PR-2.** U7.1 + U7.2 are scheduled "next minor release" but do not have a concrete release-version anchor in the plan. Acceptable for now (operator anchors at the time of cutover) but worth noting in Stage's next session.

**P3 (advisory):**

* **PR-3.** Phase ordering uses Phase number as task-ID prefix during harvest for human readability. The backlog tool assigns its own typed IDs; the prefix is purely a sort key and not authoritative.
* **PR-4.** U6.2 closes 003-F as superseded. The harvest step should add a comment to 003-F referencing the new sub-chore or task. Stage will do this manually after harvest if backlogit lacks an idiomatic command.

### Constitution compliance

* I (Safety-First Rust) — preserved; no `unsafe`, no `unwrap`/`expect`; new `cozo` calls return `Result<T, EngramError>`.
* II (Test-First) — every implementation unit lists verification before implementation activity; dual-backend macro enables side-by-side assertions.
* III (Workspace Isolation) — no path-traversal surface introduced; per-branch DB pattern preserved (per memory `path normalization`).
* IV (CLI Containment) — n/a (server-internal change).
* V (Structured Observability) — `record_query_metrics` wrapper preserved per spike §14 risk row.
* VI (Single Responsibility) — one new dependency (`cozo`); justified by spike §16.10. RocksDB transitive evaluated in U0.1.
* VII (Destructive Approval) — U7.1, U7.2, U6.1, U6.3 cutover all flagged as ProposedAction with ActionRisk.
* VIII (Safety Modes) — plan implicitly enters careful mode through the dual-backend + feature-flag posture; investigate-first applies to U0.1 + U0.4.
* IX (Git-Friendly Persistence) — JSONL ground truth unchanged; only DB upsert backend changes.
* X (Context Efficiency) — vertical partitioning per CIE §16.2 directly improves hot-path token efficiency for the agent.

### Hardening signals re-check

The plan-review gate confirms the `Requires plan hardening: yes` declaration matches the plan's actual hardening footprint. The `## Plan Hardening` section is present and materially complete.

<!-- plan-review-attempt: 1 -->
<!-- plan-review-gate: PASS -->
