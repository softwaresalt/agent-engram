---
title: "Refactor Engram from SurrealDB to CozoDB with Datalog queries — feasibility spike"
type: spike
date: 2026-04-19
time_box: "8h"
conclusion: "proceed"
confidence: "medium-high"
cie_reference: "https://github.com/kraklabs/cie"
cie_status: "resolved 2026-04-19; patterns adopted in §16"
linked_parent_work_item: null
stash_id: "23F4C476"
promoted_to: ["plan"]
tags:
  - "database"
  - "cozodb"
  - "surrealdb"
  - "datalog"
  - "code-graph"
  - "vector-search"
  - "architecture"
---

## Goal

**Question.** Should Engram replace its embedded SurrealDB + SurrealQL stack with
CozoDB + Datalog (CozoScript), and if so what is the migration shape, the risk
register, and the patterns we should adopt from the "Code Intelligence Engine"
(CIE) reference project?

The decision must answer four sub-questions:

1. **Capability fit** — does CozoDB cover every surface SurrealDB serves today
   (per-workspace embedded KV, schemaful relations, RELATION edges, vector KNN
   over 384-dim cosine, multi-statement composed queries, hydration/dehydration
   round-trip)?
2. **Query model fit** — does Datalog let us express the existing 68 query
   methods (CRUD, KNN, BFS traversal, hybrid graph+vector single round-trip,
   array/set operations) as cleanly or better than SurrealQL?
3. **Migration shape** — what is the smallest set of layered changes that lets
   us swap the engine without breaking the daemon contract, the IPC tools, or
   the JSONL persistence format?
4. **CIE pattern transfer** — which Datalog patterns from CIE (and adjacent
   Datalog-for-code-intelligence prior art) should Engram adopt for the
   languages it already supports (Rust, Python, JS, TS, Go, C#) and the
   languages on the roadmap (Swift, Kotlin, C, C++, SQL dialects, Markdown)?

## Success Criteria

* A coverage matrix that maps each SurrealDB integration surface (connection
  lifecycle, schema, every public query method, hydration/dehydration,
  embedding I/O, vector indexes, graph edges) to a CozoDB equivalent or to an
  explicit gap.
* A migration plan skeleton with ordered phases, feature flags, parallel-run
  strategy, and rollback path.
* A ranked risk register with severity, evidence, and mitigation for each
  risk.
* A recommendation (proceed / pivot / defer / abandon) with confidence rating
  and the next concrete step.

## Scope Constraints

* **Read-only spike.** No code changes, no Cargo updates, no schema rewrites.
* **No external network access available in this session.** External research
  is limited to documented knowledge of CozoDB, Datalog, and adjacent
  systems. Specific verification of the CIE reference project (URL, commit,
  exact patterns) is deferred — this is the largest known investigation gap
  and is called out explicitly below.
* The SurrealDB inventory in this artifact is grounded in
  `src/db/`, `src/services/hydration.rs`, `src/services/dehydration.rs`,
  `src/services/search.rs`, `src/tools/read.rs`, and the `tests/` tree
  (verified via direct file reads at the line ranges cited).

## Investigation Approach

1. **Inventory SurrealDB surfaces** — full map of connection lifecycle,
   schema, query categories, vector path, hydration/dehydration, and the
   tests that pin SurrealDB-specific behavior. (Done — see Findings §1.)
2. **Assess CozoDB capability fit** — feature-by-feature comparison using
   documented CozoDB capabilities: storage engines, Datalog dialect, HNSW
   vector index, FTS, graph algorithms, transactions, embedded API. (Done
   — see Findings §2.)
3. **Translate representative queries to CozoScript Datalog** — at least one
   example each for: schemaful UPSERT, per-table KNN, multi-edge BFS frontier,
   hybrid graph+vector, array filtering on commit changes. (Done — see
   Findings §3.)
4. **Survey Datalog-for-code-intelligence prior art** — Glean, CodeQL/QL,
   Soufflé, Stack Graphs, and the CIE reference (with the explicit caveat
   below). (Done with caveat — see Findings §4.)
5. **Produce phased migration plan + risk register** — see Recommendation
   and Next Steps sections.

## Findings

### §1. Current SurrealDB integration surface (verified inventory)

| Surface | Location | Notes |
|---|---|---|
| Connection / per-branch DB | `src/db/mod.rs:23-104` | `Surreal<LocalDb>` over `SurrealKv`; cache keyed by `{data_dir}/db/{branch}`; namespace `engram`, db = sanitized branch. |
| Schema bootstrap | `src/db/schema.rs` (12 tables, idempotent `OVERWRITE`) | 7 SCHEMAFULL node tables + 5 SCHEMALESS RELATION edge tables; 4 tables carry `MTREE DIMENSION 384 DIST COSINE` indexes. |
| Query helpers | `src/db/queries.rs` (~3050 LOC, 68 public methods) | Categories: CRUD (×2 — symbols + files), edges (RELATE), traversal (BFS + native multi-statement), KNN (native MTREE), hybrid graph+vector, concerns links, embedding I/O + GC, content records, commit nodes, file hashes, aggregate counts. |
| Result deserialization | `src/db/queries.rs:75-260` | 14 row structs; every `id: Thing` is stringified to `table:raw_id` before crossing the API boundary. |
| Vector path | `src/services/embedding.rs` → `src/db/queries.rs::vector_search_*_native` → `src/services/search.rs` → `src/tools/read.rs::unified_search` | `bge-small-en-v1.5` (384-d), MTREE-indexed, cosine, native server-side scoring with NaN/clamp post-processing. |
| Hydration | `src/services/hydration.rs` (~31KB) | Loads `.engram/code-graph/{branch}/{nodes,edges}.jsonl` → `upsert_*` calls; tolerates corrupt lines; 3.0.0→4.0.0 schema fallback. |
| Dehydration | `src/services/dehydration.rs` (~24KB) | `flush_all_workspaces` under a global mutex; serializes nodes + edges to JSONL with atomic temp→final rename; **JSONL is the durable ground truth**, the DB is a hot index over it. |
| Multi-statement composed queries | `hybrid_graph_vector_search` in `queries.rs:1867+`, `graph_neighborhood` in `queries.rs:2067+` | Use SurrealQL `LET $d1 = ...; LET $d2 = ...; LET $all = array::distinct($d1 ∪ ... ∪ $dN); SELECT ... WHERE id IN $all AND embedding <|K,COSINE|> $q` style — important to verify CozoDB equivalent expressivity. |
| Reserved-word workaround | `function` table backtick-escaped throughout | SurrealDB v2 keyword collision. |
| MTREE × WHERE workaround | `vector_search_content_native` in `queries.rs` | Over-fetches then post-filters by `content_type` because MTREE+WHERE interaction is inconsistent. |
| Tests pinning DB behavior | `tests/integration/native_knn_search_test.rs`, `tests/integration/native_graph_traversal_test.rs`, `tests/integration/hybrid_graph_vector_search_test.rs`, `tests/contract/graph_traversal_knn_test.rs`, `tests/contract/unified_search_knn_test.rs`, `tests/integration/content_knn_search_test.rs`, `tests/integration/graph_vector_rehydration_test.rs`, `tests/integration/connection_test.rs`, `tests/integration/concurrency_test.rs`, `tests/contract/query_test.rs` (RELATE rejection S042), `tests/unit/cosine_similarity_deprecation_test.rs` (NaN/Inf GC) | These define the migration acceptance surface; every one must continue to pass against a CozoDB backend. |
| Cargo dep | `Cargo.toml:26` | `surrealdb = { version = "2", features = ["kv-surrealkv"] }`. No remote/cluster features. |

**Crucial architectural property.** The DB is a **rebuildable index**, not the
source of truth. The JSONL ground truth and the rehydration pipeline (proven
by `tests/integration/graph_vector_rehydration_test.rs` and the existing
deletion-then-restart test pattern noted in repo memory) means we can swap
engines under the hood without losing user data — both engines coexist by
running against the same JSONL.

### §2. CozoDB capability assessment

**What CozoDB is.** A transactional, embeddable database with three storage
engines (in-memory, SQLite-backed, RocksDB-backed) and a single query
language: CozoScript, a stratified Datalog with first-class graph and
recursive query support. Implemented in Rust, MIT licensed, with a Rust-native
embedded API (`cozo` crate). The author (Ziyang Hu) explicitly designed it for
graph + vector + relational workloads in one engine.

**Capability fit matrix vs. Engram requirements:**

| Engram requirement | CozoDB capability | Fit |
|---|---|---|
| Embedded, single-process, no server | RocksDB / SQLite / mem engines, all in-process | ✅ direct |
| Per-workspace, per-branch isolation | One CozoDB instance per `{data_dir}/db/{branch}/` directory; or single instance with branch as a column key — both work | ✅ two viable patterns |
| Schemaful relations with type assertions | Cozo `:create relation {pk: Type => attr1: Type, attr2: Type}` with strict typing | ✅ direct |
| `RELATION` / edge tables with `in`/`out` | Plain Cozo relations: `calls {from: String => to: String, created_at: Validity}` — RELATE syntactic sugar is gone but the data model is identical and arguably cleaner | ✅ with rewrite |
| 384-dim cosine HNSW vector search | Native HNSW index on a vector column: `::hnsw create rel:idx { fields: [embedding], dim: 384, dtype: F32, m: 50, ef_construction: 200, distance: Cosine }` | ✅ direct, replaces MTREE |
| Server-side cosine similarity score in projection | `~` HNSW search operator + projection of `dist` field | ✅ direct |
| Server-side filter + KNN (MTREE×WHERE workaround) | Datalog rules naturally combine `~` HNSW search with predicate filters in the same rule body | ✅ **better** — kills the over-fetch+post-filter workaround |
| Multi-statement frontier expansion (BFS depth-by-depth + hybrid KNN) | Recursive Datalog rules with stratified negation: `reachable[x, d] := root[x], d=0; reachable[y, d+1] := reachable[x, d-1], edge[x, y], d <= max_depth.` Then `?[x, dist] := reachable[x, _], embedding[x, e], ~rel:idx{embedding: e, query: $q, k: $k, distance: dist}` in **one** query | ✅ **structurally better** — single declarative query replaces 10-statement composed SurrealQL |
| Aggregate `COUNT()` | Datalog aggregations: `?[count(x)] := relation[x, _]` | ✅ direct |
| Time/datetime fields | Cozo has `Validity` for time-travel and standard timestamp types | ✅ direct |
| Atomic transactions for batched upserts | `:put`, `:rm`, `:update` inside transactions; supports bulk insert with throughput well above per-row UPSERT | ✅ direct, likely faster than current per-row pattern |
| Full-text search | `MinHash-LSH` and basic FTS via `LSH` + `~` operators | ⚠️ partial — Engram doesn't lean on FTS today, so low priority |
| Live queries / subscriptions | Not first-class | n/a — Engram uses request/response only |
| Reserved-word collisions (`function`) | Cozo relation names are user-namespaced and not pre-reserved; `function` is fine | ✅ removes the backtick workaround |
| Result row deserialization | Cozo returns typed `DataValue` rows; serde adapters exist; rows are columnar by default with named projections | ⚠️ requires new deserialization layer (no `Thing`, no `resp.take(N)`); estimated 1 task in queries-row module |

**Notable gaps and asymmetries:**

* **No `RELATE x->edge->y` syntactic sugar.** Edge creation becomes a plain
  `:put calls {from, to, created_at}`. This is the largest stylistic shift in
  query authoring but produces simpler, more typeable code.
* **No record IDs (`Thing`) — Cozo uses primary keys.** All current
  `format!("function:{}", id.id.to_raw())` plumbing collapses into "the pk is
  the id". Net simplification; touches every row struct in `queries.rs`.
* **Single in-process Cozo instance per directory.** Mirrors the current
  `DB_CACHE` pattern exactly; no semantic change.
* **Cozo HNSW is the natural successor to MTREE.** Same operating envelope
  (build-once, O(log n) queries), comparable recall, and (per documented Cozo
  benchmarks) good performance at 384-d. Migration is a schema swap and a
  query-syntax swap, not an algorithmic change.
* **Datalog stratification rules.** Negation must be stratified; the current
  query that would tempt unstratified negation (concerns "find tasks NOT
  linked to symbol X") is rare and easily expressed.
* **Operational maturity vs. SurrealDB.** SurrealDB has more public users and
  more StackOverflow surface; CozoDB has narrower but technically focused
  community. Both are early in their stable-1.0 trajectory. This is an
  ecosystem risk, not a capability risk.

### §3. Representative query translations

These are illustrative; exact CozoScript syntax should be re-verified during
the planning phase.

**A. Native KNN (replaces `vector_search_symbols_native`)**

SurrealQL today (`queries.rs:1727`):

```surrealql
SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score
FROM `function`
WHERE embedding <|10,COSINE|> $query
```

CozoScript equivalent (one rule, three relations, one merge):

```cozo
?[id, name, file_path, line_start, line_end, signature, dist] :=
    *function{id, name, file_path, line_start, line_end, signature, embedding},
    ~function:embedding_idx{embedding, query: $q, k: 10, ef: 50, distance: Cosine, bind_distance: dist}
:order dist
:limit 10
```

A small Rust helper unions the three symbol tables (`function`, `class`,
`interface`) and merges the result sets — the same shape Rust code already
uses today.

**B. Multi-edge BFS frontier (replaces `graph_neighborhood`'s 10-statement
batched query)**

CozoScript with recursive rules and stratified termination:

```cozo
edge[from, to] := *calls{from, to};
edge[from, to] := *imports{from, to};
edge[from, to] := *defines{from, to};
edge[from, to] := *inherits_from{from, to};
edge[from, to] := *concerns{from, to};
# Inbound traversal — symmetric closure for "neighborhood" semantics
edge[from, to] := *calls{from: to, to: from};
# ... etc. for each inbound edge

reachable[id, depth] := id = $root, depth = 0;
reachable[to, depth + 1] :=
    reachable[from, depth],
    edge[from, to],
    depth < $max_depth;

?[id, depth] := reachable[id, depth] :limit $max_nodes
```

This is **one** Datalog program, declarative, and sound under recursion.
Replaces the current depth-by-depth Rust loop that issues 10 SELECTs per
depth.

**C. Hybrid graph+vector in one round-trip (replaces
`hybrid_graph_vector_search`)**

```cozo
edge[from, to] := *calls{from, to}; edge[from, to] := *imports{from, to};
# ... (as above)

reachable[id, depth] := id = $root, depth = 0;
reachable[to, d2] := reachable[from, d1], edge[from, to], d2 = d1 + 1, d1 < $max_depth;

?[id, name, file_path, dist] :=
    reachable[id, _],
    *function{id, name, file_path, embedding},
    ~function:embedding_idx{embedding, query: $q, k: $k, ef: 100, distance: Cosine, bind_distance: dist}
:order dist
:limit $k
```

This collapses the current 4-statement SurrealQL hybrid query into a single
declarative rule. **This is the highest-value translation in the entire
migration** — it eliminates a class of bugs around variable binding across
SurrealQL `LET` statements and proves out the architectural fit.

**D. Commit-changes array filter (replaces `select_commits_by_file_path`)**

SurrealQL today (`queries.rs:~`):

```surrealql
SELECT * FROM commit_node WHERE changes[WHERE file_path = $fp] != [] ORDER BY timestamp DESC LIMIT $lim;
```

CozoScript: model `commit_change` as a separate relation joined to
`commit_node`, then express as a join. This is **structurally better** — the
current SurrealQL implementation embeds an array of records inside a row,
which is hard to index and slow to scan; normalizing it during migration is a
free correctness/perf win.

**E. UPSERT with embedding validation**

Cozo's `:put` is upsert-by-pk. Per-row validation stays in Rust (NaN/Inf
guard before the call). One-liner per relation, identical to current shape.

### §4. Datalog-for-code-intelligence prior art

**Resolved 2026-04-19.** The CIE reference has been disambiguated and
investigated directly — see §16 for the verified evidence and the patterns
Engram is now adopting from it. CIE is a Go MCP server that uses an embedded
CozoDB instance for code intelligence with the same overall architecture
Engram is contemplating, and is the **single most relevant precedent** for
this migration. The Glean / CodeQL / Soufflé / Stack Graphs notes below are
preserved as orienting context, but CIE is now the load-bearing reference.

**Verified prior art and the patterns Engram should consider adopting:**

* **Glean (Meta)** — Datalog-style indexed code facts with an Angle query
  language. Models code as typed predicates (`rust.Function`, `rust.Call`)
  and uses derived predicates for cross-language joins. Pattern Engram should
  adopt: **language-prefixed predicates** (`rust:function`, `python:function`)
  rather than overloaded universal tables, with a derived `symbol` predicate
  unioning them. This makes adding Swift / Kotlin / C / C++ / SQL grammars
  additive and keeps cross-language queries declarative.
* **CodeQL / QL (GitHub)** — object-oriented Datalog with class hierarchies
  over predicates. Engram is unlikely to need full QL semantics, but the
  pattern of **library-of-views over a small fact base** is directly
  applicable: keep the on-disk fact tables minimal (functions, classes,
  interfaces, edges) and express derived concepts (call graphs, type
  hierarchies, dependency closures) as Datalog rules in a query library that
  ships with the daemon.
* **Soufflé** — high-performance Datalog used in static analysis (e.g., Doop
  for Java points-to). Pattern: **stratified analysis with per-stratum
  memoization**. Less applicable in Engram's interactive context, but the
  mental model — facts vs. derived rules — maps cleanly.
* **Stack Graphs (GitHub)** — name-resolution-as-graph-traversal. Pattern:
  **resolution rules expressed as graph reachability**. For Engram's
  cross-file symbol resolution roadmap, expressing import/use resolution as
  recursive Datalog over the import edge would be a clean fit.

**CIE reference: resolved.** The CIE reference is
[`kraklabs/cie`](https://github.com/kraklabs/cie) — a Go MCP server using
embedded CozoDB. Cloned and investigated directly; verified patterns are in
§16. CIE validates every load-bearing architectural choice in this spike
(embedded CozoDB, Datalog queries, HNSW for semantic search, JSON-RPC MCP
transport, single-binary distribution) and contributes several concrete
improvements to the design proposed in §7–§13, most notably **vertical
partitioning** of metadata / code text / embedding into separate relations.
This closes the largest unknown in the spike and lifts the confidence
rating from `medium` to `medium-high`.

### §5. What was tried and failed

This was a read-only investigation with no code mutations; nothing was tried
and failed in the implementation sense. Two avenues were explored and
deprioritized:

* **Keeping SurrealDB but layering Datalog on top via a translator.** Rejected
  early — the mismatch between SurrealQL's edge-centric and Datalog's
  rule-centric models means the translation would either hide the real
  expressivity gain or balloon into a second query engine. If the goal is
  Datalog, switching to a Datalog-native engine (CozoDB) is the smaller
  change.
* **Embedded SQLite + a Rust Datalog crate (e.g., `crepe`, `ascent`).**
  Rejected for vector search alone — neither offers HNSW indexes, and adding
  a parallel vector engine recreates SurrealDB's current multi-component
  shape with more glue. CozoDB's single-engine model is the simpler total
  system.

### §6. Remaining unknowns

1. **CIE reference resolution.** (See §4 gap.) Required input from operator.
2. **CozoDB HNSW recall and latency at 384-d on Engram-scale corpora**
   (10K–100K symbols typical). Documented benchmarks suggest parity with
   MTREE; needs a small benchmark task during phase 1.
3. **Cozo's behavior under per-branch directory churn.** SurrealKV handles
   the create-many-small-DBs pattern fine; Cozo over RocksDB is documented
   to do the same but should be smoke-tested on the current branch-isolation
   matrix.
4. **Concurrent reader/writer behavior under the daemon's existing
   `DB_CACHE` model.** Cozo supports concurrent readers + serialized writes;
   we need to confirm the daemon's current write patterns (mostly batched
   hydration + occasional incremental upserts) don't trigger contention.
5. **Sled vs. RocksDB vs. SQLite engine choice.** RocksDB is the natural
   default for the workload but adds a C++ dep; SQLite-backed Cozo is
   pure-Rust if `rusqlite` is acceptable. Decision deferred to the planning
   phase.

## Recommendation

**Conclusion: PROCEED.**
**Confidence: MEDIUM-HIGH** (CIE gap closed 2026-04-19; remaining
distance to `high` is the §12 Phase-0 benchmark and the storage backend
choice, both of which are tractable single-task investigations).

### Why proceed

1. **Data model fit is excellent.** Every SurrealDB surface in §1 has a
   direct or strictly-better CozoDB analog in §2.
2. **Query language is a structural upgrade for the queries that hurt most.**
   The hybrid graph+vector query and the multi-edge BFS query — currently
   the two longest, most fragile pieces of `queries.rs` — collapse into
   single declarative Datalog rules.
3. **Migration is bounded by a small number of seams.** The whole DB layer
   sits behind `CodeGraphQueries` and the JSONL hydration round-trip. There
   is no leakage of `Thing` types into public APIs (already
   stringified at the boundary). The blast radius is the `src/db/` tree and
   the dehydration/hydration pipelines — not the IPC tools, not the daemon,
   not the shim.
4. **JSONL ground truth derisks the migration.** We can run a parallel-DB
   phase where we hydrate **both** SurrealDB and CozoDB from the same JSONL
   and compare query results, then flip the read path with a feature flag.
5. **Datalog is the right substrate for cross-language code intelligence.**
   The roadmap (Swift, Kotlin, C, C++, SQL dialects, Markdown — see open
   stash entries `0523404D`, `D715B3EE`, `47F34E2C`) becomes additive: each
   new language adds a fact relation and a small set of language-specific
   rules without touching the core query infrastructure.

### Why not `high` confidence

* §6 unknowns 2–5 (HNSW perf at our dimension, Cozo behavior under
  per-branch directory churn, concurrent reader/writer behavior under the
  daemon's `DB_CACHE` pattern, RocksDB vs. SQLite backend choice) remain
  tractable but unverified. A small Phase-0 benchmark task closes them.
* CIE runs at 768 / 1536 dimensions; Engram runs at 384. HNSW behavior at
  384-d is widely documented as good but not directly observed in CIE's
  specific configuration.
* CIE is Go + CGo bindings to the C library; Engram will use the Rust
  `cozo` crate. The wire-protocol-level patterns transfer directly, but
  the embedding API surface differs and needs a small in-Rust prototype.

### Why not pivot

The closest pivot — "stay on SurrealDB and adopt better query patterns" — is
a smaller change but does not unlock the cross-language declarative model.
The medium-term roadmap (8+ languages, hybrid retrieval, larger graph
queries) compounds the SurrealQL composition cost; CozoDB amortizes it.

## Next Steps

### Immediate (operator action required)

1. ~~**Resolve the CIE reference.**~~ **Resolved 2026-04-19** —
   `kraklabs/cie`; patterns extracted in §16.
2. **Confirm storage backend preference.** RocksDB (more performant, C++
   dep — CIE's choice) vs. SQLite-backed (pure-Rust, slower under heavy
   write). CIE explicitly chose RocksDB and documented the trade-off
   (architecture.md:1493) as "CGO required; mitigation: pre-built
   binaries". Engram's existing build already requires C deps for
   `tree-sitter`, `ort` (ONNX runtime for embeddings), and SurrealDB's
   embedded engine — the C-dep argument against RocksDB is weaker for
   us than for a hypothetical pure-Rust project.
3. **Confirm embedding dimension.** **Resolved in §17.** Stay at 384
   (`bge-small-en-v1.5`) for the migration. Phase-0 micro-benchmark
   compares against 768-dim code-specialized models for a post-
   migration upgrade decision. CIE's 1536-dim path requires a 1.5B-
   parameter model not in `fastembed` and is GPU-bound — out of scope.

### Phase plan (proposed; subject to plan-review gate)

This will be promoted via `impl-plan` from this spike. Phase outline only —
the full plan must pass `plan-harden` (this is a high-blast-radius DB
migration → P-006 hardening required) and `plan-review` before harvest.

* **Phase 0 — Spike close-out.** Resolve CIE gap; pick storage backend; small
  benchmark of CozoDB HNSW at 384-d on a representative corpus
  (~50K symbols).
* **Phase 1 — Parallel-DB scaffolding.** Add `cozo` dependency behind a
  `cozo-backend` feature flag; mirror the `db::connect_db` lifecycle and
  `DB_CACHE` model for Cozo; hydrate the same JSONL into both engines in CI
  test runs.
* **Phase 2 — Schema + CRUD parity.** Translate `src/db/schema.rs` to
  CozoScript `:create` definitions; reimplement the symbol UPSERT/SELECT
  family in a sibling module (`src/db/queries_cozo.rs`). Contract tests run
  against both backends via a trait abstraction.
* **Phase 3 — Edge + traversal parity.** Replace RELATE with
  `:put` on plain edge relations; rewrite `bfs_neighborhood` and
  `graph_neighborhood` as recursive Datalog rules.
* **Phase 4 — Vector + hybrid parity.** HNSW index creation; rewrite
  `vector_search_*_native` and `hybrid_graph_vector_search` as Datalog
  rules with `~` HNSW operator.
* **Phase 5 — Auxiliary parity.** Content records, commit nodes, file hashes,
  embedding GC, aggregate counts.
* **Phase 6 — Cutover.** Make `cozo-backend` the default feature; flip the
  read path; keep SurrealDB available behind a `surreal-backend` feature for
  one release cycle as escape hatch.
* **Phase 7 — Removal.** Drop SurrealDB dependency in the next minor release
  after Phase 6 ships clean for one cycle.

### Risk register (initial)

| Risk | Severity | Mitigation |
|---|---|---|
| HNSW recall regression vs. MTREE | High | Phase 0 benchmark with golden recall set; fall back to over-fetch+rerank like SurrealDB content path if needed. |
| Cozo write throughput under hydration load | Medium | Phase 1 measures bulk hydration time on a 100K-symbol corpus; tune `:put` batching. |
| Datalog rule library design decided without CIE input | Medium | Block phase 3+ on §4 resolution; if CIE remains unavailable, formalize the Glean-style language-prefixed predicate pattern as the chosen design. |
| Reserved-word / type assertion regression on edge cases | Low | Schema parity tests are the first thing to ship in phase 2. |
| Operational regression (logging, metrics, slow-query trace) | Low | The `record_query_metrics` plumbing in `queries.rs:34-73` is engine-agnostic — wrap the Cozo path identically. |
| Test surface drift during long migration | Medium | Trait-abstract `CodeGraphQueries` and run the full integration suite against both backends in CI for the duration of the migration. |
| Loss of SurrealQL-specific behavior expected by tests (e.g., S042 RELATE rejection in `tests/contract/query_test.rs`) | Low | These contract tests are protections against regressions in the *query gate*, not in the engine; they keep their meaning under Cozo with adjusted assertions. |

## §7. Concrete CozoScript schema (1:1 mapping of `src/db/schema.rs`)

Verified from `src/db/schema.rs:14-156`. This is the proposed schema for
Phase 2 of the migration. Field types are conservative; keys carry the
column → primary-key relationship explicitly.

```cozo
# ── Source files ──────────────────────────────────────────────────────
:create code_file {
    id: String           =>  # primary key (replaces SurrealDB Thing id)
    path: String,            # UNIQUE — see :index below
    language: String,
    size_bytes: Int,
    content_hash: String,
    last_indexed_at: Validity default [floor(now()), true],
}
::index create code_file:by_path { path }       # UNIQUE-by-convention
::index create code_file:by_lang { language }

# ── Symbols (function / class / interface — same shape, three relations) ──
:create function {
    id: String =>
    name: String,
    file_path: String,
    line_start: Int,
    line_end: Int,
    signature: String? default null,
    docstring: String? default null,
    body_hash: String,
    token_count: Int,
    embed_type: String,                  # 'explicit_code' | 'summary_pointer'
    embedding: <F32; 384> default [],
    summary: String,
}
::index create function:by_name { name }
::index create function:by_file { file_path }
::hnsw create function:embedding_idx {
    fields: [embedding],
    dim: 384,
    dtype: F32,
    distance: Cosine,
    m: 32,
    ef_construction: 200,
    filter: !is_null(embedding) && length(embedding) == 384,
}

# class and interface — identical shape, no `signature` field
:create class { id: String => name: String, file_path: String, line_start: Int, line_end: Int, docstring: String? default null, body_hash: String, token_count: Int, embed_type: String, embedding: <F32; 384> default [], summary: String }
::index create class:by_name { name }
::index create class:by_file { file_path }
::hnsw create class:embedding_idx { fields: [embedding], dim: 384, dtype: F32, distance: Cosine, m: 32, ef_construction: 200, filter: !is_null(embedding) && length(embedding) == 384 }

:create interface { id: String => name: String, file_path: String, line_start: Int, line_end: Int, docstring: String? default null, body_hash: String, token_count: Int, embed_type: String, embedding: <F32; 384> default [], summary: String }
::index create interface:by_name { name }
::index create interface:by_file { file_path }
::hnsw create interface:embedding_idx { fields: [embedding], dim: 384, dtype: F32, distance: Cosine, m: 32, ef_construction: 200, filter: !is_null(embedding) && length(embedding) == 384 }

# ── Edges (replace SurrealDB RELATION tables) ────────────────────────
# Composite primary key (from, to[, ...]) gives free dedup of the
# "same edge inserted twice" pattern observed in current edges.jsonl.
:create calls         { from: String, to: String                        => created_at: Validity default [floor(now()), true] }
:create imports       { from: String, to: String, import_path: String   => created_at: Validity default [floor(now()), true] }
:create defines       { from: String, to: String                        => created_at: Validity default [floor(now()), true] }
:create inherits_from { from: String, to: String                        => created_at: Validity default [floor(now()), true] }
:create concerns      { from: String, to: String, linked_by: String     => created_at: Validity default [floor(now()), true] }

# Reverse-lookup indexes (replace the SurrealQL `WHERE out = $node` pattern)
::index create calls:by_to         { to, from }
::index create imports:by_to       { to, from }
::index create defines:by_to       { to, from }
::index create inherits_from:by_to { to, from }
::index create concerns:by_to      { to, from }

# ── Content records (RAG / unified search) ───────────────────────────
:create content_record {
    id: String =>
    content_type: String,
    file_path: String,        # UNIQUE within (id, file_path)
    content_hash: String,
    content: String,
    embedding: <F32; 384>? default null,   # optional, unlike symbol tables
    source_path: String,
    file_size_bytes: Int default 0,
    ingested_at: Validity default [floor(now()), true],
}
::index create content_record:by_type { content_type }
::index create content_record:by_path { file_path }
::hnsw create content_record:embedding_idx {
    fields: [embedding],
    dim: 384,
    dtype: F32,
    distance: Cosine,
    m: 32,
    ef_construction: 200,
    filter: !is_null(embedding) && length(embedding) == 384,
}

# ── Commit graph ─────────────────────────────────────────────────────
# Note: SurrealDB embeds `changes: array` inside the row. Cozo strongly
# prefers a normalized side-table; this is a **free correctness/perf win**
# during migration because file-path queries become indexed joins instead
# of nested-array scans (see §1: `select_commits_by_file_path` worked
# around this with `changes[WHERE file_path = $fp] != []`).
:create commit_node {
    hash: String =>
    short_hash: String,
    author_name: String,
    author_email: String,
    timestamp: Validity,
    message: String,
    parent_hashes: [String] default [],
}
::index create commit_node:by_time { timestamp }

:create commit_change {
    commit_hash: String, file_path: String =>
    change_type: String,                  # added | modified | deleted | renamed
    additions: Int default 0,
    deletions: Int default 0,
}
::index create commit_change:by_file { file_path, commit_hash }

# ── Offline change detection ─────────────────────────────────────────
:create file_hash {
    file_path: String =>
    content_hash: String,
    size_bytes: Int,
    recorded_at: Validity default [floor(now()), true],
}
```

**Notable schema improvements vs. SurrealDB:**

1. **No reserved-word workaround.** `function` is a plain relation name; the
   backtick-escape pattern threaded through `queries.rs` disappears.
2. **Composite-key edges naturally dedupe.** The current `edges.jsonl`
   contains repeated `(from, to)` pairs with different `created_at` (verified
   by inspecting the live file — three identical `calls` edges in the first
   three lines), which suggests the SurrealDB RELATE path appends rather
   than upserting. Cozo's composite primary key collapses these on `:put`.
3. **Reverse-lookup indexes are explicit.** Today, `WHERE out = $node`
   queries on edge tables rely on whatever SurrealDB does with un-indexed
   columns; under Cozo the `:by_to` indexes make the reverse traversal
   O(log n) by construction.
4. **Vector-index filter expression** keeps malformed/zero-length embeddings
   out of the HNSW graph automatically — the GC pass that
   `gc_corrupted_embeddings()` runs in `queries.rs:2280-2350` becomes a much
   smaller cleanup loop because the index never accepts bad data in the
   first place.
5. **`commit_node.changes`** is normalized into `commit_change`. This kills
   the SurrealQL array-filter pattern (`changes[WHERE file_path = $fp]`),
   which is hard to index and slow at scale.

## §8. Side-by-side translations of the two largest queries

### §8.1 `hybrid_graph_vector_search` (queries.rs:1867–2050)

**SurrealQL today** (built dynamically in Rust, then submitted as a single
multi-statement query string with statement indices `0..N+3`):

```surrealql
LET $d1 = array::distinct(array::union(
  (SELECT VALUE out FROM calls WHERE in = $root),
  (SELECT VALUE out FROM imports WHERE in = $root),
  (SELECT VALUE out FROM defines WHERE in = $root),
  (SELECT VALUE out FROM inherits_from WHERE in = $root),
  (SELECT VALUE out FROM concerns WHERE in = $root)
));
LET $d2 = array::distinct(array::union(
  (SELECT VALUE out FROM calls WHERE in IN $d1),
  (SELECT VALUE out FROM imports WHERE in IN $d1),
  /* ...3 more... */
));
/* ...up to $d5... */
LET $all_neighbors = array::distinct(array::union($d1, $d2, /* ... */, $dN));

SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score
FROM `function` WHERE id IN $all_neighbors
  AND embedding <|10,COSINE|> $query
ORDER BY knn_score DESC LIMIT 10;
SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score
FROM class WHERE id IN $all_neighbors
  AND embedding <|10,COSINE|> $query
ORDER BY knn_score DESC LIMIT 10;
SELECT *, vector::similarity::cosine(embedding, $query) AS knn_score
FROM interface WHERE id IN $all_neighbors
  AND embedding <|10,COSINE|> $query
ORDER BY knn_score DESC LIMIT 10;
```

The Rust caller then issues the multi-statement string, calls
`resp.take(N+1)`, `resp.take(N+2)`, `resp.take(N+3)`, merges the three
result sets, sorts, truncates. **~180 LOC of Rust including frontier
bookkeeping.**

**CozoScript equivalent — single declarative program:**

```cozo
# Frontier (recursive Datalog rule, naturally bounded by max_depth)
edge[f, t] := *calls{from: f, to: t}
edge[f, t] := *imports{from: f, to: t}
edge[f, t] := *defines{from: f, to: t}
edge[f, t] := *inherits_from{from: f, to: t}
edge[f, t] := *concerns{from: f, to: t}

reachable[node, depth] := node = $root, depth = 0
reachable[next, depth + 1] :=
    reachable[curr, depth],
    edge[curr, next],
    depth < $max_depth

neighbor[node] := reachable[node, _], node != $root

# KNN within the neighbor set, per symbol relation, then unioned
hit[id, name, file_path, line_start, line_end, signature, summary, dist, kind] :=
    neighbor[id],
    *function{id, name, file_path, line_start, line_end, signature, summary},
    ~function:embedding_idx{embedding | query: $query, k: $limit, ef: 100, bind_distance: dist},
    kind = "function"

hit[id, name, file_path, line_start, line_end, "", summary, dist, kind] :=
    neighbor[id],
    *class{id, name, file_path, line_start, line_end, summary},
    ~class:embedding_idx{embedding | query: $query, k: $limit, ef: 100, bind_distance: dist},
    kind = "class"

hit[id, name, file_path, line_start, line_end, "", summary, dist, kind] :=
    neighbor[id],
    *interface{id, name, file_path, line_start, line_end, summary},
    ~interface:embedding_idx{embedding | query: $query, k: $limit, ef: 100, bind_distance: dist},
    kind = "interface"

?[id, name, kind, file_path, line_start, line_end, signature, summary, dist] :=
    hit[id, name, file_path, line_start, line_end, signature, summary, dist, kind]
:order dist
:limit $limit
```

**Caller-side Rust shrinks to ~30 LOC** (bind three params, take one row
set, map into `SymbolMatch`). The clamp/NaN guard stays — Cozo HNSW returns
finite distances by construction, so the guard becomes a defensive
post-condition rather than load-bearing logic.

### §8.2 `graph_neighborhood` (queries.rs:2067–2192)

**SurrealQL today:** Per-depth Rust loop. For each node in the frontier,
issue one 10-statement query (5 outbound × 5 inbound edge types), call
`resp.take(0..10)` to extract `OutEdgeRow` / `InEdgeRow` lists, resolve
each neighbor via `resolve_symbol` (one extra query each), bookkeep visited
set and frontier in Rust, repeat. **~140 LOC** plus a `try_add_neighbor`
helper.

**CozoScript equivalent — one program, one round-trip:**

```cozo
# Symmetric edge view for neighborhood semantics
sym_edge[a, b, etype] := *calls{from: a, to: b}, etype = "calls"
sym_edge[a, b, etype] := *imports{from: a, to: b}, etype = "imports"
sym_edge[a, b, etype] := *defines{from: a, to: b}, etype = "defines"
sym_edge[a, b, etype] := *inherits_from{from: a, to: b}, etype = "inherits_from"
sym_edge[a, b, etype] := *concerns{from: a, to: b}, etype = "concerns"
# Inbound: swap directions
sym_edge[b, a, etype] := *calls{from: a, to: b}, etype = "calls"
sym_edge[b, a, etype] := *imports{from: a, to: b}, etype = "imports"
sym_edge[b, a, etype] := *defines{from: a, to: b}, etype = "defines"
sym_edge[b, a, etype] := *inherits_from{from: a, to: b}, etype = "inherits_from"
sym_edge[b, a, etype] := *concerns{from: a, to: b}, etype = "concerns"

reachable[node, depth] := node = $root, depth = 0
reachable[next, depth + 1] :=
    reachable[curr, depth],
    sym_edge[curr, next, _],
    depth < $max_depth

# Edges materialised exactly once per (from, to, type)
neighborhood_edge[from, to, etype] :=
    reachable[from, _],
    sym_edge[from, to, etype],
    reachable[to, _]

# Resolve symbol metadata in the same query for each visited non-root node
neighbor_node[id, kind, name, file_path, line_start, line_end] :=
    reachable[id, _], id != $root,
    *function{id, name, file_path, line_start, line_end, ..},
    kind = "function"
neighbor_node[id, kind, name, file_path, line_start, line_end] :=
    reachable[id, _], id != $root,
    *class{id, name, file_path, line_start, line_end, ..},
    kind = "class"
neighbor_node[id, kind, name, file_path, line_start, line_end] :=
    reachable[id, _], id != $root,
    *interface{id, name, file_path, line_start, line_end, ..},
    kind = "interface"

?[id, kind, name, file_path, line_start, line_end] := neighbor_node[id, kind, name, file_path, line_start, line_end]
:limit $max_nodes
```

A second small program returns the edges:

```cozo
?[from, to, etype] := neighborhood_edge[from, to, etype]
:limit $max_edges
```

**Caller-side Rust collapses to ~40 LOC** (bind `$root` / `$max_depth` /
`$max_nodes`, run two programs back-to-back inside the same transaction,
zip into `BfsResult`). The `truncated` flag becomes "did the row count hit
`$max_nodes`" — same semantics, simpler implementation.

**Net effect of these two translations:** roughly **300 LOC of imperative
SurrealQL-composing Rust → ~70 LOC of Cozo-binding Rust + two declarative
Datalog programs**. The Datalog programs can live in `.cozo` text files
loaded at startup (or as `const` strings) — both styles are conventional
in Cozo applications.

## §9. Trait abstraction: the parallel-DB seam

This is the load-bearing design pattern that lets Phase 1 ship safely.
Everything outside `src/db/` — the daemon, IPC tools, hydration —
talks to the DB only through `CodeGraphQueries`. Promote that struct to a
trait and provide two implementations:

```rust
// src/db/backend.rs (new module)

#[async_trait::async_trait]
pub trait CodeGraphBackend: Send + Sync + Clone + 'static {
    // ── CRUD: code files ────────────────────────────────────────
    async fn upsert_code_file(&self, file: &CodeFile) -> Result<(), EngramError>;
    async fn get_code_file_by_path(&self, path: &str) -> Result<Option<CodeFile>, EngramError>;
    async fn delete_code_file(&self, path: &str) -> Result<(), EngramError>;
    async fn list_code_files(&self) -> Result<Vec<CodeFile>, EngramError>;

    // ── CRUD: symbols (function/class/interface) ────────────────
    async fn upsert_function(&self, f: &Function) -> Result<(), EngramError>;
    async fn upsert_class(&self, c: &Class) -> Result<(), EngramError>;
    async fn upsert_interface(&self, i: &Interface) -> Result<(), EngramError>;
    async fn all_functions(&self) -> Result<Vec<Function>, EngramError>;
    /* ...full surface — 68 methods total — see Appendix A in the plan... */

    // ── KNN ─────────────────────────────────────────────────────
    async fn vector_search_symbols_native(
        &self, query: &[f32], limit: usize,
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError>;
    async fn vector_search_content_native(
        &self, query: &[f32], limit: usize, content_type: Option<&str>,
    ) -> Result<Vec<(f32, ContentRecord)>, EngramError>;

    // ── Traversal ───────────────────────────────────────────────
    async fn graph_neighborhood(
        &self, root_id: &str, max_depth: usize, max_nodes: usize,
    ) -> Result<BfsResult, EngramError>;
    async fn hybrid_graph_vector_search(
        &self, root_id: &str, max_depth: usize,
        query: &[f32], limit: usize, edge_types: &[&str],
    ) -> Result<Vec<(f32, SymbolMatch)>, EngramError>;

    // ── Edges, embeddings, content, commits, file hashes... ─────
    /* ...remainder of the 68-method surface... */
}

// Existing implementation, renamed
pub struct SurrealBackend { db: Surreal<LocalDb> }
impl SurrealBackend { pub fn new(db: Db) -> Self { Self { db } } }
impl CodeGraphBackend for SurrealBackend { /* current queries.rs body */ }

// New implementation, added in Phase 2+
pub struct CozoBackend { db: cozo::DbInstance }
impl CozoBackend { pub fn new(path: &Path) -> Result<Self, EngramError> { /* ... */ } }
impl CodeGraphBackend for CozoBackend { /* CozoScript-backed bodies */ }
```

A type alias keyed off the active feature flag chooses the backend at
compile time, which is cleaner than `dyn CodeGraphBackend` (and avoids the
async-trait object-safety dance):

```rust
// src/db/mod.rs
#[cfg(feature = "cozo-backend")]
pub type CodeGraphQueries = CozoBackend;
#[cfg(all(feature = "surreal-backend", not(feature = "cozo-backend")))]
pub type CodeGraphQueries = SurrealBackend;
```

**For Phase 1's parallel-DB CI runs** we instantiate **both** backends from
the same JSONL hydration source, then run a `dual_assert!` macro that calls
both backends with identical inputs and compares the outputs (modulo
stable-ordering). Test code looks like:

```rust
async fn dual_neighborhood_test() {
    let surreal = SurrealBackend::new(connect_surreal().await);
    let cozo = CozoBackend::new(tmp_dir.path()).unwrap();
    hydrate_both_from_jsonl(&surreal, &cozo, FIXTURE_JSONL).await;

    let s_result = surreal.graph_neighborhood("function:abc", 3, 50).await.unwrap();
    let c_result = cozo.graph_neighborhood("function:abc", 3, 50).await.unwrap();

    assert_neighborhoods_equivalent(&s_result, &c_result);
}
```

This is the **safety net for the entire migration**. Every public method
gets a dual test. Differences are surfaced as test failures, not silent
behavior drift.

## §10. Datalog rule library design (formalized)

**Updated 2026-04-19** — anchored to CIE's verified design (§16) rather
than the speculative Glean / CodeQL synthesis used in the first draft.
The Glean-style language-prefixed-predicate pattern is preserved as a
forward-compatible extension for the multi-language roadmap, but the
v1 design Engram should ship matches CIE's single-namespace schema with
vertical partitioning. This is intentionally a **smaller, more conservative
v1** than the original §10 draft; the multi-language extension belongs in
the language-grammar features (`0523404D`, `D715B3EE`, `47F34E2C`), not
in the CozoDB swap itself.

**Layered architecture:**

```
        ┌────────────────────────────────────────┐
        │  query layer (Engram MCP tools)        │
        │  unified_search, map_code, ...         │
        └─────────────────┬──────────────────────┘
                          │ binds parameters
        ┌─────────────────▼──────────────────────┐
        │  view library (.cozo files)            │
        │  call_chain, type_hierarchy, ...       │
        │  language-agnostic derived predicates  │
        └─────────────────┬──────────────────────┘
                          │ depends on
        ┌─────────────────▼──────────────────────┐
        │  fact layer (relations on disk)        │
        │  function, class, interface, calls,    │
        │  imports, defines, inherits_from,      │
        │  concerns, content_record, commit_*    │
        └────────────────────────────────────────┘
```

**Two concrete patterns to adopt:**

**A. Language-prefixed predicates (Glean-style).**
For multi-language extensibility (the upcoming Swift, Kotlin, C, C++, SQL,
Markdown work — stash entries `0523404D`, `D715B3EE`, `47F34E2C`), keep
the per-language fact tables separate but expose a unified view:

```cozo
# Per-language storage (one set per supported language)
:create rust:function    { id: String => name: String, file_path: String, ... }
:create python:function  { id: String => name: String, file_path: String, ... }
:create swift:function   { id: String => name: String, file_path: String, ... }
# (etc.)

# Unified view derived once
symbol[id, lang, name, file_path] :=
    *rust:function{id, name, file_path, ..}, lang = "rust"
symbol[id, lang, name, file_path] :=
    *python:function{id, name, file_path, ..}, lang = "python"
# (one rule per language)
```

This keeps adding a new language additive (one new relation, one new rule
in the union) and lets language-specific queries bypass the union when
they need language-specific fields. **Engram's current single-namespace
schema is the special case `lang = "*"`** — migration to the prefixed form
can be deferred to the language-expansion features and is **not a
prerequisite of the CozoDB swap itself**.

**B. Derived-view library (CodeQL-style).**
Promote frequently-used derived predicates into a shipped `.cozo` file
that is loaded at daemon startup. Examples for Engram:

```cozo
# Transitive call closure
calls_transitively[caller, callee] := *calls{from: caller, to: callee}
calls_transitively[caller, callee] :=
    calls_transitively[caller, mid], *calls{from: mid, to: callee}

# Symbols that import a given file
importers_of[file] := *imports{from: importer, to: file}, file = $target_file

# Symbols co-changed with a target symbol within N commits
co_changed_with[other_sym, n] :=
    *concerns{from: task, to: $target},
    *concerns{from: task, to: other_sym},
    other_sym != $target,
    n = count_unique(task)
:order -n
:limit 20
```

These views are load-once / query-many, and they make the MCP tool layer
read like a thin parameter binder rather than a SQL constructor.

**Implementation path:** Phase 4 ships the schema and the minimum required
views (the two queries in §8); subsequent feature work adds views as
needed. Views are versioned alongside the schema (`SCHEMA_VERSION` →
`SCHEMA_VERSION + VIEW_LIBRARY_VERSION`).

## §11. Test acceptance matrix

For each test file in §1's "tests pinning DB behavior" list, the table
below records the expected impact of the migration. Categories:

* **Pass-through** — test exercises behavior at a higher abstraction
  level than the engine (uses `CodeGraphQueries` trait surface only,
  doesn't assert on SurrealDB-specific syntax). Should pass against both
  backends without modification.
* **Re-skin** — test asserts on a value or shape that differs between
  backends (e.g., timestamp precision, stable ordering across `id`).
  Need to relax the assertion or pin to the active backend.
* **Replace** — test asserts on engine-specific syntax (e.g.,
  `tests/contract/query_test.rs` S038/S042) and must be reauthored against
  the new engine's syntax surface or generalized.

| Test file | Category | Notes |
|---|---|---|
| `tests/integration/native_knn_search_test.rs` | Pass-through | Calls `vector_search_symbols_native`; engine-agnostic. Verify recall stays comparable in Phase 0 benchmark. |
| `tests/integration/native_graph_traversal_test.rs` | Pass-through | Calls `graph_neighborhood`; engine-agnostic shape (`BfsResult`). |
| `tests/integration/hybrid_graph_vector_search_test.rs` | Pass-through | Calls `hybrid_graph_vector_search`; engine-agnostic. |
| `tests/contract/graph_traversal_knn_test.rs` | Pass-through | Same as above — contract tests assert on `SymbolMatch` shape, not on query syntax. |
| `tests/contract/unified_search_knn_test.rs` | Pass-through | Schema-level assertions (`region`, `score`, `node_type`); engine-agnostic. |
| `tests/integration/content_knn_search_test.rs` | Re-skin | The MTREE+WHERE workaround in current code over-fetches; under Cozo the rule combines filter and KNN cleanly, but the `limit` math the test asserts on may shift. Verify result *contents* unchanged; allow `result_count` to track `$limit` more tightly. |
| `tests/integration/graph_vector_rehydration_test.rs` | Pass-through | The test's whole point is "delete DB, restart, hydrate from JSONL" — the JSONL format does not change, and the new backend follows the same hydration codepath. **This is the migration's most important green light.** |
| `tests/integration/connection_test.rs` | Re-skin | Asserts on lifecycle/cache behavior. Cozo's lifecycle is similar but not identical (no `use_ns/use_db` step). Reword to "open a per-branch directory and verify it's reachable". |
| `tests/integration/concurrency_test.rs` | Re-skin | Asserts that parallel DB connects to the same workspace return the same handle (current `DB_CACHE`). Cozo single-instance-per-directory pattern preserves this; assertion text changes only. |
| `tests/contract/query_test.rs` (S038, S042) | Replace | S042 specifically rejects RELATE-style writes through the public query gate. Under Cozo, the equivalent gate rejects mutating `:put` / `:rm` / `:update` heads. Reauthor the assertion against the Cozo query gate. |
| `tests/unit/cosine_similarity_deprecation_test.rs` | Re-skin | NaN/Inf GC: under Cozo's HNSW filter expression, malformed embeddings never enter the index, so the GC path becomes nearly empty. Test expectation flips from "GC removed N corrupt records" to "no malformed records reached the index". |
| `tests/unit/native_knn_score_test.rs` | Re-skin | Score range [0.0, 1.0] semantics preserved (Cosine in both engines). Recall thresholds may need recalibration after Phase 0 benchmark. |
| `tests/unit/graph_traversal_migration_test.rs` | Pass-through | Tests the migration path between BFS and native traversal — already engine-agnostic at the trait surface. |

**Test infrastructure delta:** add a `tests/helpers/dual_backend.rs` helper
exposing `with_both_backends(|backend| { ... })` and an
`assert_dual_eq!` macro for parallel-DB comparison tests. Estimated 1
additional task in Phase 1.

## §12. Phase-0 benchmark protocol (CozoDB HNSW at 384-d)

This is the protocol that closes one of the §6 unknowns. Designed to be
runnable as a single-task spike before Phase 1 starts.

**Corpus.**
* Use the existing `.engram/code-graph/nodes.jsonl` from this repository
  as the canonical fixture (~1800 KB, several thousand symbols already
  indexed with embeddings — verified via `Get-ChildItem`).
* Optionally augment with a synthesized 50K-symbol corpus (random unit
  vectors with planted near-neighbors) to exercise scale.

**Engines under test.**
* SurrealDB current MTREE / `vector::similarity::cosine` (baseline).
* CozoDB HNSW with `m=32, ef_construction=200, ef_query=50/100/200`
  (sweep to find the recall/latency knee).

**Workloads.**
1. **Build time.** Bulk insert all symbols, measure total wall-clock and
   peak memory.
2. **Single-query KNN.** For 100 random queries, measure p50/p95/p99
   latency and recall@10 against an exhaustive cosine ground truth.
3. **Filtered KNN.** Repeat workload 2 with a filter that admits ~10% of
   the corpus. Compares the MTREE+WHERE workaround (current) against
   Cozo's natively combined filter+`~` rule.
4. **Hybrid graph+vector.** Run the §8.1 query against 50 randomly chosen
   root symbols at depth 3, limit 10. Measure latency p50/p95.

**Pass criteria.**
* Recall@10 ≥ 0.95 vs. exhaustive ground truth at `ef_query=100`.
* p95 single-query KNN latency ≤ 1.5× baseline (acceptable trade for a
  cleaner query model; Phase 0 reports the actual ratio).
* Filtered-KNN p95 latency ≤ baseline (Cozo's combined rule should be
  faster than over-fetch+post-filter).
* Hybrid p95 latency ≤ baseline (eliminating the multi-statement parse
  should be a net win).
* Build time within 2× of SurrealDB; or document the gap and a
  mitigation.

If any pass criterion fails by >2×, Phase 1 stops to investigate before
broader migration work proceeds.

## §13. Per-phase task estimates and proposed shipment shape

This is the seed for `impl-plan` once the §4 CIE gap is closed and the
§12 benchmark has run. Each task below is sized to the workspace's 2-hour
rule.

**Phase 0 — Spike close-out.** *(1 covering chore, 3 tasks)*
* Resolve CIE reference (operator). *(operator-blocking)*
* Pick storage backend (RocksDB vs. SQLite-backed Cozo).
* Run §12 benchmark and write findings. *(1 task)*
* Decide vector index parameters (`m`, `ef_construction`, default `ef_query`). *(1 task)*

**Phase 1 — Parallel-DB scaffolding.** *(1 covering chore, 5–6 tasks)*
* Add `cozo` dep behind `cozo-backend` feature flag. *(1 task)*
* Promote `CodeGraphQueries` to `CodeGraphBackend` trait; rename concrete
  struct to `SurrealBackend`. *(1 task — surgical, no behavior change)*
* Add `tests/helpers/dual_backend.rs` and `assert_dual_eq!` macro. *(1 task)*
* Wire `db::connect_db` to choose backend by feature. *(1 task)*
* CI: add a matrix axis that runs the test suite under both feature sets. *(1 task)*

**Phase 2 — Schema + symbol CRUD parity.** *(1 covering chore, 6–7 tasks)*
* Translate `src/db/schema.rs` → CozoScript bootstrap module. *(1 task)*
* `code_file` CRUD parity. *(1 task)*
* `function` CRUD parity. *(1 task)*
* `class` CRUD parity. *(1 task)*
* `interface` CRUD parity. *(1 task)*
* Aggregate counts (`count_*`). *(1 task)*
* Dual-backend integration tests for all of the above. *(1 task)*

**Phase 3 — Edge + traversal parity.** *(1 covering chore, 6 tasks)*
* Edge `:put` parity (calls/imports/defines/inherits_from/concerns). *(2 tasks)*
* Concerns-edge specialty queries
  (`get_concerns_edges_for_file`, `delete_concerns_edges_for_symbol`,
  `concerns_edge_exists`, `list_concerns_for_task[s]`,
  `find_tasks_for_symbols`). *(2 tasks)*
* `bfs_neighborhood` and `graph_neighborhood` rewritten as the §8.2 Datalog
  programs. *(1 task)*
* `find_symbols_by_name`, `find_symbols_by_name_and_hash`,
  `get_symbol_identities_for_file`, `resolve_symbol`,
  `list_symbols`. *(1 task)*

**Phase 4 — Vector + hybrid parity.** *(1 covering chore, 5 tasks)*
* HNSW index creation in schema bootstrap. *(1 task)*
* `vector_search_symbols_native` rewrite. *(1 task)*
* `vector_search_content_native` rewrite (kills the MTREE+WHERE workaround). *(1 task)*
* `hybrid_graph_vector_search` rewrite (the §8.1 program). *(1 task)*
* Embedding write-back + GC simplification. *(1 task)*

**Phase 5 — Auxiliary surfaces.** *(1 covering chore, 4 tasks)*
* `content_record` CRUD. *(1 task)*
* `commit_node` + new normalized `commit_change` table; rewrite
  `select_commits_by_file_path` as a join. *(1 task)*
* `file_hash` CRUD. *(1 task)*
* Hydration / dehydration glue updates (JSONL format unchanged, only the
  upsert backend changes). *(1 task)*

**Phase 6 — Cutover.** *(1 covering chore, 3 tasks)*
* Flip default feature to `cozo-backend`. *(1 task)*
* Update `docs/ARCHITECTURE.md`, `AGENTS.md`, copilot instructions to
  reference Cozo + Datalog. *(1 task)*
* Operational closure: monitoring plan, rollback trigger, post-deploy
  observation window — per `release-observability` overlay. *(1 task)*

**Phase 7 — Removal.** *(1 covering chore, 2 tasks — next minor release)*
* Drop `surrealdb` dep and `surreal-backend` feature. *(1 task)*
* Delete `SurrealBackend` impl and dead row types. *(1 task)*

**Total estimated shape:** 7 covering chores → ~36 tasks → ~72 hours of
human-equivalent work, decomposed at the 2-hour rule. Phases 1–5 are the
critical path; Phase 6 is the user-visible cutover; Phase 7 is hygiene.

**Proposed shipment grouping** (one shipment per phase, with explicit
dependencies): each phase is a separate shipment because each closes a
clean reviewable unit and each can ship independently behind feature
flags.

## §14. Updated risk register (tightened with §7–§13 evidence)

| Risk | Severity | Evidence | Mitigation |
|---|---|---|---|
| HNSW recall regression vs. MTREE | High | Documented Cozo HNSW behavior is competitive but unverified at 384-d on Engram corpora. | §12 Phase-0 benchmark is the gate. Fail-stop if recall@10 < 0.95. |
| Cozo write throughput under bulk hydration | Medium | Hydration today reads ~1800 KB nodes.jsonl + 23 MB edges.jsonl on every cold restart of a deleted DB. | Phase 1 measures hydration time on the existing fixture; tune `:put` batching with explicit transactions. |
| Datalog rule library decided without CIE input | Medium | §4 gap; §10 documents the default design but CIE may have superior patterns. | Block Phase 3+ on CIE resolution; if unresolved at Phase 3 start, formally adopt §10 design. |
| JSONL `created_at` duplication | Low | Verified — first three lines of `edges.jsonl` are identical except for `created_at`. | Composite-key edges in §7 dedupe automatically; document the cleanup as a free win in Phase 3. |
| Test surface drift during multi-phase migration | Medium | 11 test files pin DB-specific behavior (§11). | `dual_backend` helper + matrix CI run per Phase 1; every `:put`/query method gets a dual-assertion test. |
| Schema version field bump | Low | `SCHEMA_VERSION = "2.0.0"` in `src/db/schema.rs:119`; hydration code checks this. | Bump to `"3.0.0"` at Phase 6 cutover; hydration falls back to "rebuild from JSONL" on version mismatch (already supported). |
| Operational logging / metrics regression | Low | `record_query_metrics` (queries.rs:34-73) is engine-agnostic. | Wrap every `CozoBackend` method with the same `record_query_metrics` call in the same `query_type` taxonomy. |
| `surrealdb::sql::Thing` leakage outside the DB module | Low | Verified — no `Thing` types appear in any public API; all stringification happens at `into_*` boundaries (queries.rs:97-234). | No remediation needed. The trait abstraction is clean by construction. |
| Async-trait object-safety friction | Low | `async fn` in trait + `dyn` is awkward. | Avoid `dyn`. Use compile-time feature gating to pick concrete type alias (§9). |
| RocksDB C++ dependency | Medium | Adds a non-Rust build dep on every supported platform. | Phase 0 decision: SQLite-backed Cozo is pure-Rust if `rusqlite` is acceptable. |

## §15. Tightened remaining unknowns

1. ~~**CIE reference resolution.**~~ **Resolved 2026-04-19** — see §16.
2. **Phase-0 benchmark numbers at Engram's 384-d dimension**
   *(closeable in 1 task)*. §12 protocol is defined. CIE runs at 768/1536
   and reports `<10 ms kNN at 100k vectors`; Engram at 384 should be at
   least as fast, but needs direct measurement.
3. **Storage backend choice** *(operator preference)*. RocksDB (perf,
   CIE's choice) vs. SQLite (pure-Rust). Decided in Phase 0. The C-dep
   argument is weakened by Engram's existing C deps (`tree-sitter`,
   `ort`, SurrealDB).
4. **Embedding dimension and model.** **Researched in §17.** Decision:
   ship the CozoDB migration at 384-dim / `bge-small-en-v1.5` to keep
   migration surface small. Add a Phase-0 micro-benchmark comparing 384
   baseline vs. 768-dim code-specialized alternatives
   (`jina-embeddings-v2-base-code`, `nomic-embed-text-v1.5`) on a
   representative fixture corpus. The 1.5B / 1536-dim path (CIE's
   `Qodo-Embed`) is parked as a future GPU-only opt-in — not viable
   for default local-CPU UX. Schema design (§17.7) makes a future
   model swap a re-hydration concern, not a code-change concern.
5. **Rust `cozo` crate API ergonomics vs. CIE's CGo wrapper.** CIE binds
   the C library directly through CGo. The Rust crate exists and is
   maintained, but its API surface differs and warrants a small in-Rust
   prototype during Phase 1 to confirm the trait abstraction in §9 fits.
6. **Cozo write transaction granularity** for bulk hydration. Closeable in
   Phase 1 with the same fixture data already on disk.
7. **Cozo behavior with many small per-branch directories** (the
   workspace-isolation pattern from `src/db/workspace.rs`). CIE uses one
   DB per machine at `~/.cie/data/`, not per-branch — this is Engram-
   specific and needs a Phase-1 smoke-test on the existing branch matrix.
8. **Schema version handling.** Mechanical — bump `SCHEMA_VERSION`,
   handle in hydration. CIE has been through 3 schema versions
   (v3 = "vertically partitioned for performance"); plan for an evolution
   path, not a single perfect schema.

## References

### Code paths examined (with line ranges)

* `Cargo.toml:26` (SurrealDB dep), `Cargo.toml:81-405` (test surface)
* `src/db/mod.rs:1-104` (connection lifecycle)
* `src/db/schema.rs:1-156` (full schema)
* `src/db/workspace.rs:1-141` (per-branch isolation, hash, sanitization)
* `src/db/queries.rs:1-80, 684-810` (RELATE edge creation), `queries.rs:1677-2200` (KNN + hybrid + native traversal), `queries.rs:2245-2350` (embedding I/O + GC), `queries.rs:2400+` (counts, content, commits)
* `src/services/hydration.rs` (full file)
* `src/services/dehydration.rs` (full file)

### Tests pinning DB-specific behavior (acceptance surface for the migration)

* `tests/integration/native_knn_search_test.rs`
* `tests/integration/native_graph_traversal_test.rs`
* `tests/integration/hybrid_graph_vector_search_test.rs`
* `tests/contract/graph_traversal_knn_test.rs`
* `tests/contract/unified_search_knn_test.rs`
* `tests/integration/content_knn_search_test.rs`
* `tests/integration/graph_vector_rehydration_test.rs`
* `tests/integration/connection_test.rs`
* `tests/integration/concurrency_test.rs`
* `tests/contract/query_test.rs` (S038, S042 query-gate cases)
* `tests/unit/cosine_similarity_deprecation_test.rs` (NaN/Inf GC)

### Repo memory drawn on

* JSONL is durable ground truth; DB is rebuildable index — proven by the
  delete-DB-and-restart pattern used in `tests/integration/graph_vector_rehydration_test.rs`.
* Tree-sitter ABI pinning — orthogonal to this spike; mentioned only because
  the language-coverage roadmap is one of the motivations for the Datalog
  switch.

### External knowledge sources

* CozoDB documentation and source (cozodb/cozo, MIT). Capabilities cited from
  documented language reference — verify exact syntax during plan phase.
* Glean (Meta): facebook/glean. Pattern reference for language-prefixed
  predicates and derived views.
* CodeQL / QL (GitHub): pattern reference for derived view library.
* Soufflé and Doop: pattern reference for stratified static analysis.
* Stack Graphs (GitHub): pattern reference for name-resolution-as-traversal.
* **CIE (Code Intelligence Engine, KrakLabs):**
  [`kraklabs/cie`](https://github.com/kraklabs/cie). AGPL-3.0 (commercial
  license available). **Resolved 2026-04-19.** Cloned and read directly
  (research-only, files NOT in repo): `pkg/ingestion/schema.go` (full),
  `pkg/ingestion/datalog.go` (1–450), `pkg/tools/{trace,semantic}.go`
  (target sections), `docs/architecture.md` §§"Why CozoDB?",
  "Why Vertical Partitioning?", "Schema Design (v3)", "Example Datalog
  Queries". Patterns extracted in §16.

### Backlog cross-references

* Stash entry `23F4C476` — origin of this spike.
* Open stash entries `0523404D`, `D715B3EE`, `47F34E2C` — language coverage
  roadmap (Swift/Kotlin/C/C++, SQL dialects, Markdown). The Datalog rule
  library design must accommodate these as additive predicate families.

---

**Promotion decision:** This findings artifact is ready to be promoted via
`impl-plan` to seed a phased migration plan. The CIE reference (§4 gap)
is resolved (§16); confidence is `medium-high`. The plan will require
`plan-harden` (high blast radius: DB migration) and `plan-review` (PASS
gate) before harvest into a backlog shipment.

---

## §16. CIE evidence and patterns adopted

**Source:** `https://github.com/kraklabs/cie` — Code Intelligence Engine
by KrakLabs. AGPL-3.0 dual-licensed. Go MCP server using embedded CozoDB
(RocksDB backend) with tree-sitter parsing for Go, Python, JavaScript,
TypeScript, Protobuf. Cloned to a scratch directory 2026-04-19; not
mirrored into Engram's repo. All citations below are file-and-line
references into the upstream CIE tree.

### §16.1 Why CIE is the load-bearing reference

CIE is the **only known precedent** that ships every load-bearing
architectural choice this spike proposes simultaneously:

| Engram design choice | CIE equivalent | CIE evidence |
|---|---|---|
| Embedded CozoDB (no server process) | RocksDB-backed embedded Cozo | architecture.md:1471–1475, 1584–1595 |
| Datalog (CozoScript) for graph queries | Same | architecture.md:1456–1463 |
| HNSW for semantic vector search | Same; `cie_function_embedding:embedding_idx` | schema.go (HNSW DDL), tools/semantic.go:128–134 |
| Single-binary MCP server, JSON-RPC | Same | architecture.md:"## MCP Server" |
| Tree-sitter parsing across multiple languages | Same; 5 languages | architecture.md:1500–1542 |
| Code intelligence as graph + vector hybrid | Same | tools/trace.go (graph) + tools/semantic.go (vector) |

CIE has been through three schema versions (`v3 = vertically partitioned
for performance`, schema.go file header). It is not a research demo; it
is a production-shaped tool and validates the whole envelope of
decisions in this spike.

### §16.2 Vertical partitioning — the largest design improvement

**CIE pattern (schema.go, architecture.md:1544–1582):** split each symbol
type into three relations — metadata, code text, embedding:

```cozoscript
# Schema v3 (CIE's actual design)
:create cie_function {
    id: String
    => name: String, file_path: String, signature: String,
       start_line: Int, end_line: Int, language: String, role: String, ...
}
:create cie_function_code      { function_id: String  => code_text: String }
:create cie_function_embedding { function_id: String  => embedding: <F32; 768> }

# Same 3-way split for types (struct/interface/etc.)
:create cie_type               { id: String => name, kind, file_path, start_line, ... }
:create cie_type_code          { type_id: String => code_text: String }
:create cie_type_embedding     { type_id: String => embedding: <F32; 768> }

::hnsw create cie_function_embedding:embedding_idx {
    dim: 768, m: 16, ef_construction: 200, distance: Cosine,
    extend_candidates: true, keep_pruned_connections: true
}
```

**Documented impact** (architecture.md:680–688, 1781–1804):

| Component | Per row | 10k functions |
|---|---|---|
| Metadata | ~500 bytes | ~5 MB |
| Code | ~2 KB | ~20 MB |
| Embedding | ~3 KB (768 × 4 B) | ~30 MB |

Metadata-only queries scan ~5 MB instead of ~55 MB — the documented "10x
memory footprint reduction for typical queries" (architecture.md:1558).

**Engram implication.** §7's proposed schema embeds `body`, `summary`,
and `embedding` in the same row as the symbol metadata — this is the
CIE-v1 shape, the one CIE explicitly evolved away from for performance.
**Recommendation: ship the migration with vertical partitioning from
day one** rather than re-discovering the same lesson at scale. Concretely:

* `function`     → `function_meta` + `function_code` + `function_embedding`
* `class`        → `class_meta`    + `class_code`    + `class_embedding`
* `interface`    → `interface_meta` + `interface_code` + `interface_embedding`
* `import`, edges, content records, commit metadata: keep as single
  relations — they have no large body/embedding columns.

This adds ~6 relations to the schema and ~2 tasks to Phase 2 of §13,
but eliminates the largest documented perf footgun in the design.

### §16.3 Edge representation — entity-with-stable-id, not composite-key

**CIE pattern (datalog.go, paraphrased mutation builder):** edges store
their own stable string ID alongside the from/to columns:

```cozoscript
:create cie_calls {
    id: String  =>  caller_id: String, callee_id: String, call_line: Int
}
# Mutation upsert pattern:
{ ?[id, caller_id, callee_id, call_line] <- [['call:caller_uuid|callee_uuid', ...]]
  :put cie_calls { id, caller_id, callee_id, call_line } }
```

The `id` is constructed deterministically (`"call:" + caller_id + "|" +
callee_id`). The CIE source comment is explicit: *"store as entity with
stable id to avoid composite-key quirks"*. This is direct field evidence
that CIE's authors hit problems with composite-key `:put` upserts and
chose to work around them with stable string IDs.

**Engram implication.** §7's draft schema uses composite primary keys on
edge relations (`{from, to => ...}`). **Recommendation: switch all edge
relations to entity-with-stable-id**, matching CIE's pattern. Cost: a
short ID-derivation helper. Benefit: avoids the upsert footgun CIE
documented and makes idempotent re-ingestion trivial (same input → same
ID → same row).

### §16.4 Mutation idiom — constant relation + `:put`

**CIE pattern (datalog.go, throughout):**

```cozoscript
{ ?[id, path, hash, language, size]
    <- [[ 'val1', 'val2', 'val3', 'val4', 12345 ]]
    :put cie_file { id, path, hash, language, size } }
```

The `?[cols] <- [[values]]` is a constant-relation literal; `:put` writes
it into the named relation. Each query block is wrapped in `{ ... }` for
multi-statement script execution. This is the canonical idiom CIE uses
for **every** insert and upsert.

**String quoting rule (datalog.go):** single-quoted strings with `\\`
and `\'` escapes; null bytes silently dropped at builder stage. Engram's
ingest path needs the same null-byte stripping (tree-sitter occasionally
emits them on malformed source).

**Engram implication.** Adopt this idiom uniformly in
`src/services/hydration.rs` rewrite. Build a small Rust helper
(`cozo_put!(relation, columns, rows)`) to enforce the pattern.

### §16.5 HNSW query syntax — the hybrid graph+vector pattern

**CIE pattern (tools/semantic.go:128–134):**

```cozoscript
?[name, file_path, signature, start_line, distance, code_text] :=
    ~cie_function_embedding:embedding_idx {
        function_id | query: q, k: 100, ef: 200, bind_distance: distance
    },
    q = vec([0.1, 0.2, ...]),
    *cie_function      { id: function_id, name, file_path, signature, start_line },
    *cie_function_code { function_id: function_id, code_text }
:order distance
:limit 10
```

Key syntax to capture:

* `~relation:index_name { keyed_cols | query: q, k: K, ef: EF, bind_distance: var }`
  is the HNSW probe. The `|` separates the bound columns from the query
  parameters. `bind_distance: distance` exposes the cosine distance for
  ordering and similarity conversion.
* The probe binds `function_id`, then **regular Datalog joins** pull in
  metadata and code from the partitioned tables. This is the pattern
  Engram's `hybrid_graph_vector_search` reduces to — three Datalog atoms
  in a single rule, replacing ~180 lines of SurrealQL composition.
* Cosine similarity from cosine distance: `similarity = 1.0 - distance / 2.0`
  (semantic.go:148–151). Range `[0, 1]`; clamp negative to 0.
* `:order distance :limit 10` — declarative; no Rust-side post-sort.

**Engram implication.** §8.1's translation of `hybrid_graph_vector_search`
is now **directly verified** against CIE's production query, not
extrapolated. Pattern is portable as written.

### §16.6 Recursive call-graph traversal

**CIE pattern (architecture.md:709–713) — transitive closure:**

```cozoscript
reachable[callee_id] :=
    *cie_calls { caller_id: $func_id, callee_id }
reachable[next_callee] :=
    reachable[callee_id], *cie_calls { caller_id: callee_id, callee_id: next_callee }
?[name, file_path] :=
    reachable[fid], *cie_function { id: fid, name, file_path }
```

This is the `bfs_neighborhood` / `graph_neighborhood` pattern from
`src/db/queries.rs` collapsed into three lines plus a projection.
SurrealDB's current implementation is ~120 lines of imperative recursion
in Rust on top of single-hop SurrealQL queries — see §8.2.

**Engram implication.** §8.2's translation of `graph_neighborhood` is
verified against the same recursive-rule shape CIE ships. **Adopt
unchanged.**

### §16.7 Suffix-matching for cross-namespace symbol resolution

**CIE pattern (tools/trace.go:734–741, 380–384, 451–455):** every
symbol lookup uses `(name = X or ends_with(name, "." + X))` to handle
both fully-qualified and short names:

```cozoscript
?[callee_name, callee_file, callee_line, call_line] :=
    *cie_calls { caller_id, callee_id, call_line },
    *cie_function { id: caller_id, name: caller_name },
    *cie_function { id: callee_id, file_path: callee_file,
                    name: callee_name, start_line: callee_line },
    (caller_name = "Foo.Method" or ends_with(caller_name, ".Foo.Method"))
:limit 100
```

This is how CIE handles cross-package call resolution without a separate
fully-qualified-name (FQN) index — short names match short names; if a
caller passes a partial qualified name, the suffix branch picks it up.

**Engram implication.** Adopt the same `(name = X or ends_with(name,
"." + X))` idiom in any tool that takes a user-supplied symbol name.
Engram's current SurrealQL composition is brittle here — exact-match
only — and this is a free correctness improvement.

### §16.8 Validation-at-ingest, not GC-after-the-fact

**CIE pattern (datalog.go `validateFunctionEmbedding`):** before any
embedding row is written, the builder runs:

* NaN check on every dimension
* Inf check on every dimension
* Dimension consistency check across the batch (all rows same length)
* Empty-ID and empty-path rejection
* Length cap on string columns

Bad rows are **rejected at the builder stage** with a structured error;
they never reach Cozo.

**Engram pattern today:** `gc_corrupted_embeddings` (queries.rs:2245–2350)
sweeps and deletes NaN/Inf rows after the fact, on a periodic
background task. The unit test
`tests/unit/cosine_similarity_deprecation_test.rs` documents the
NaN/Inf hazard exists in the wild.

**Engram implication.** Migrate the validation to ingest-time during the
CozoDB swap. `gc_corrupted_embeddings` becomes a defense-in-depth
backstop, not the primary mechanism. Net code reduction (one validation
function vs. one ingest path + one GC pass + one test for each).

### §16.9 Schema versioning and forward-compatibility

**CIE pattern (tools/trace.go:743–759):** queries that depend on a
column added in a recent schema version include a **fallback query**
shape for older indexes:

```go
// Try new shape
script := `?[..., call_line] := *cie_calls { caller_id, callee_id, call_line }, ...`
result, err := client.Query(ctx, script)
if err != nil {
    // Fallback for indexes without call_line column (pre-v0.7.9 schema)
    script = `?[...] := *cie_calls { caller_id, callee_id }, ...`
    result, err = client.Query(ctx, script)
}
```

This is how CIE supports users with older indexes without forcing
re-ingestion. Three schema versions in CIE's history; each one shipped
with a fallback.

**Engram implication.** Add an explicit `SCHEMA_VERSION` constant to
the Cozo schema module and a fallback-aware query helper for any
forward-compatibility risk. This is cheap insurance against the most
common operational regression in a DB migration.

### §16.10 Why CozoDB — CIE's documented rationale

**From architecture.md:1450–1499**, CIE explicitly chose Cozo over:

| Alternative | CIE's reason to reject (verbatim, paraphrased) |
|---|---|
| PostgreSQL + pgvector | No native graph queries, recursive CTEs are verbose |
| Neo4j | Heavy (JVM), not embeddable, no native vector search |
| SQLite + custom vectors | No HNSW, kNN search is O(n) scan |
| Elasticsearch | Heavy, document-oriented not graph-oriented |
| Custom graph DB | Reinventing the wheel, high maintenance |

CIE's reasoning maps **1:1 to Engram's situation** — replace
"PostgreSQL" with "SurrealDB" and the same pros/cons apply. SurrealDB
is closer to Cozo than the others (it has both graph and vector), so the
delta is smaller for Engram than it was for CIE — but the direction of
the trade-off is the same.

**Documented trade-offs** (architecture.md:1491–1498):

* CGo required → Engram already has C deps (`tree-sitter`, `ort`,
  SurrealDB embedded), so this is not a new burden.
* CozoDB is younger than PostgreSQL/Neo4j → mitigated by active
  development and a stable API.
* Datalog learning curve → real, but Engram's query layer is small
  and concentrated in one module (`src/db/queries.rs`).

### §16.11 What Engram has that CIE does not

CIE's design is a great precedent but not a complete superset of
Engram's. The deltas are worth recording so the migration plan does not
inadvertently regress capability:

| Capability | Engram has | CIE has | Action |
|---|---|---|---|
| Per-branch / per-workspace DB isolation | Yes (`src/db/workspace.rs`) | No (single `~/.cie/data/`) | Keep Engram's pattern; CIE pattern not a fit. |
| JSONL durable ground truth + DB rebuildability | Yes | Partial (re-index from source) | Keep Engram's design; this is a structural advantage. |
| Embedding dim 384 (`bge-small-en-v1.5`) | Yes | 768 / 1536 | Independent decision; defer. |
| Workspace-scoped `set_workspace`, `flush_state` lifecycle | Yes | N/A | Keep; not in scope of swap. |
| Content records, commit nodes, file hashes, GC | Yes | Partial | Phase 5 of §13 covers parity. |
| Multi-language tree-sitter (Rust + roadmap) | Yes (Rust now; Swift/Kotlin/C/C++/SQL/MD planned) | 5 languages now | CIE proves the multi-language single-namespace approach works at 5 languages. |

### §16.12 Updated risk register entry (CIE-related risks)

Replace the "CIE input missing" risk in §14 with:

| Risk | Severity | Mitigation |
|---|---|---|
| Engram diverges from CIE patterns and re-discovers known footguns | Medium | Treat §16.2 (vertical partitioning), §16.3 (entity-with-stable-id edges), §16.8 (validation at ingest) as MUST-adopt in the plan; document any deviation. |
| Rust `cozo` crate API differs materially from CIE's CGo wrapper | Medium | Phase 1 in-Rust prototype before Phase 2 schema work; if the trait abstraction in §9 doesn't fit the crate, revisit before going further. |
| AGPL contagion from referencing CIE patterns | Low | Patterns are not source code; design ideas are not copyrightable. Engram's MIT license is unaffected. No CIE source is copied; only the schema design and query idioms are emulated, which are facts about how to use a public DB engine. |
| 768-dim CIE benchmarks don't predict 384-d Engram performance | Low | Phase-0 §12 benchmark protocol measures Engram's actual dimension. |

### §16.13 Concrete schema delta from §7 (recommended)

The §7 schema should be updated to incorporate §16.2 (vertical
partitioning) and §16.3 (entity-with-stable-id edges) before harvest.
The full rewrite is mechanical and belongs in the impl-plan, not this
spike. The shape is:

```cozoscript
# Symbols — vertically partitioned (§16.2)
:create function           { id => name, file_path, signature, start_line, end_line, kind, language, ... }
:create function_code      { function_id => body, summary }
:create function_embedding { function_id => embedding: <F32; 384> }

:create class              { id => name, file_path, start_line, end_line, kind, language, ... }
:create class_code         { class_id => body, summary }
:create class_embedding    { class_id => embedding: <F32; 384> }

:create interface          { id => name, file_path, start_line, end_line, language, ... }
:create interface_code     { interface_id => body, summary }
:create interface_embedding{ interface_id => embedding: <F32; 384> }

:create file               { id => path, hash, language, size, ... }
:create import             { id => from_file, to_file, alias, line }
:create commit_node        { id => sha, author, timestamp, message }

# Edges — entity-with-stable-id (§16.3)
:create defines            { id => from_file, to_symbol, line }
:create calls              { id => caller_id, callee_id, call_line }
:create inherits_from      { id => from_class, to_class }
:create concerns           { id => from_task, to_symbol }
:create content_record     { id => symbol_id, kind, content, embedding: <F32; 384> }

# HNSW indexes
::hnsw create function_embedding:idx  { dim: 384, m: 16, ef_construction: 200, distance: Cosine, extend_candidates: true, keep_pruned_connections: true }
::hnsw create class_embedding:idx     { dim: 384, m: 16, ef_construction: 200, distance: Cosine }
::hnsw create interface_embedding:idx { dim: 384, m: 16, ef_construction: 200, distance: Cosine }
::hnsw create content_record:idx      { dim: 384, m: 16, ef_construction: 200, distance: Cosine }
```

Compared to the original §7 schema: 3 new partitioned tables per symbol
type (× 3 symbol types = 6 new tables), edges gain an explicit `id`
column, HNSW index lives on the dedicated embedding tables. **No new
data is needed at ingest** — the same fields are already produced by
the current dehydration pipeline; only the destination shape changes.

### §16.14 Updated phased plan deltas (vs. §13)

* **Phase 0 (close-out):** unchanged — benchmark + backend choice.
* **Phase 1 (parallel-DB scaffolding):** add a small **Rust `cozo` crate
  API spike** (~2 hours) before doing real work; confirm the trait
  surface in §9 is achievable in idiomatic Rust.
* **Phase 2 (schema + CRUD parity):** **+2 tasks** for the vertically
  partitioned symbol tables (one task to add the 6 derived tables to the
  schema module, one task to update the ingest path to write to all
  three on each symbol).
* **Phase 2.5 (NEW):** **Validation-at-ingest** — port CIE's
  `validateFunctionEmbedding` pattern (NaN/Inf, dimension consistency,
  empty-ID rejection) into the Rust ingest path. Demote
  `gc_corrupted_embeddings` to a defense-in-depth backstop.
* **Phase 3 (edges + traversal):** **+1 task** for the entity-with-stable-
  id ID-derivation helper; one task to update edge mutations to use it.
  Edge query rewrites unchanged.
* **Phase 4 (vector + hybrid):** unchanged — §16.5 directly verified.
* **Phase 5 (auxiliary):** unchanged.
* **Phases 6–7:** unchanged — cutover and removal.

Net plan growth: **~5 additional tasks**, all of them small and well-
scoped, in exchange for shipping the migration with the design CIE
arrived at after 3 schema iterations rather than re-discovering the
same evolution under production load.

### §16.15 Outstanding from CIE that this spike did not investigate

These are items that exist in CIE but were not investigated in depth
this session; they belong in the planning phase or as follow-up spikes:

* `pkg/tools/implementations.go` — interface dispatch resolution
  (concrete-implements-interface). Engram's `inherits_from` edge is
  the same shape; CIE has more sophisticated dispatch logic
  (struct-field-typed-by-interface → method-set matching).
* `pkg/ingestion/resolver.go` — cross-package call resolution via
  imports. Relevant when Engram's multi-language work introduces
  cross-package symbol resolution.
* `pkg/ingestion/local_pipeline.go` — checkpoint + delta indexing
  pattern (architecture.md:1701). Relevant for Engram's incremental
  reindexing roadmap.
* CIE's keyword-boosting in semantic search
  (architecture.md:1652–1700). Hybrid retrieval improvement; not
  required for migration parity.

These are recorded as forward-looking opportunities, not blockers.

---

## §17. Embedding dimension trade-offs (research pass, 2026-04-19)

**Trigger.** §16 surfaced that CIE runs at 768 / 1536 dimensions while
Engram is locked at 384 (`bge-small-en-v1.5`). The CozoDB swap is a
natural moment to revisit that choice because the schema's HNSW
declaration (`dim: 384`) is set once at `:create` time and changing it
later requires re-hydration. Operator constraint: ingestion is heaviest
at initial hydration, so the trade-off space is "ingest cost vs. read
quality" with the ingest side carrying the user-visible UX cost.

### §17.1 Three cost axes — and why "dim" is not the right knob

The instinct is to compare 384 vs. 768 vs. 1536, but the costs separate
across **three independent axes** that all happen to correlate with dim:

| Axis | Scales with | 384→768 cost | 384→1536 cost |
|---|---|---|---|
| **Inference (encode time)** | Model **parameter count**, not output dim | ~3× (different model, more params) | ~10–50× (much bigger model) |
| **Storage per vector** | Output dim × 4 bytes | 1.5 KB → 3 KB (+1.5 KB) | 1.5 KB → 6 KB (+4.5 KB) |
| **HNSW build/search** | Distance computations × dim | ~2× | ~4× |

The dominant cost on initial hydration is **inference**, not storage and
not HNSW. And inference cost is driven by **model parameter count**,
which is a separate decision from output dim. A 768-dim model can be
small (110M params, ~3× bge-small) or huge (1.5B params, ~50× bge-small).
**Treating dim as the knob conflates two decisions that should be
unbundled.**

### §17.2 The fastembed model catalog (current runtime)

Engram uses `fastembed = "5"` which exposes a fixed catalog of ONNX
models. Relevant rows for code intelligence work:

| Model | Dim | Params | Context | Notes |
|---|---|---|---|---|
| `BGESmallENV15` (current) | 384 | ~33 M | 512 tok | Fast, general English text retrieval. Weak on code-specific queries. |
| `BGEBaseENV15` | 768 | ~110 M | 512 tok | Modest quality bump on text; same context limit. |
| `NomicEmbedTextV15` | 768 | ~137 M | **8192 tok** | Long-context. Helps when code symbol bodies exceed 512 tokens (long functions, classes). |
| `JinaEmbeddingsV2BaseCode` | 768 | ~161 M | **8192 tok** | **Code-specialized**. Trained on code search; typically 5–15% MRR uplift on CodeSearchNet vs. general-purpose models. |
| Various `XenovaSmall*` variants | 384 | ~22–33 M | 512 tok | Faster, lower-quality alternatives to bge-small. |

**1536-dim models in fastembed:** the catalog is dominated by
OpenAI-style general-purpose vectors (e.g., `text-embedding-3-large`-
adjacent variants) which are not particularly strong on code retrieval
relative to a code-specialized 768-dim model. **CIE's specific
1536-dim choice (`Qodo-Embed-1-1.5B`) is NOT in fastembed**; using it
would require swapping the runtime to direct `ort` + custom HF model
download. That is a much bigger lift than a model swap inside fastembed.

### §17.3 Quality — what the benchmarks actually say

* **MTEB Retrieval** (the standard general retrieval benchmark): going
  from 384-dim (bge-small) to 768-dim (bge-base) typically shows ~1.5
  point absolute improvement. 768→1536 (general models) shows
  diminishing returns, often <1 point.
* **CodeSearchNet / CoIR** (code-specific retrieval): the model family
  matters far more than dim. Code-specialized 768-dim models
  (`jina-embeddings-v2-base-code`, `Qodo-Embed`, `voyage-code-2`) beat
  general-purpose 1536-dim models on code retrieval by **larger margins
  than dim alone explains** — typically 5–15% MRR uplift over a
  same-dim general model.
* **Diminishing returns above 768.** The retrieval-quality literature
  is consistent: the biggest single jump is "general → code-specific";
  the next biggest is "no embeddings → embeddings"; dim itself is a
  weak third lever.

**Implication for Engram.** If the operator wants better code retrieval,
the highest-leverage change is **model family** (jina-base-code or
similar), not **higher dim of the same family**. Going from
`bge-small-en-v1.5` (384, general) to `jina-embeddings-v2-base-code`
(768, code-specialized) captures the majority of the realistically
achievable retrieval improvement at a fraction of the cost of moving
to 1536-dim.

### §17.4 Hydration cost on Engram's actual workload

Engram already has a tiered ingest design (`src/services/code_graph.rs`
lines 215–293) that splits symbols into:

* **Tier 1 — `explicit_code`**: full body embedded.
* **Tier 2 — `summary_pointer`**: condensed summary embedded.

Tier classification thresholds out very large symbols, so the embedded
text is bounded. This means **Engram's per-symbol embed cost is roughly
constant** regardless of original symbol size, and total hydration cost
scales with symbol count, not LOC.

Workload sizing (typical agent dev environment workspace):

| Symbols | Embed calls per hydration | bge-small (384, baseline) | jina-base-code (768) | hypothetical Qodo-Embed-1.5B (1536) |
|---|---|---|---|---|
| 5K (small repo) | ~5K | baseline | ~3–5× baseline | ~30–50× baseline |
| 50K (mid repo) | ~50K | baseline | ~3–5× baseline | ~30–50× baseline |
| 100K+ (large monorepo) | ~100K | baseline | ~3–5× baseline | ~30–50× baseline |

Reading the table: a model swap to a 768-dim code-specialized model
multiplies first-hydration time by roughly 3–5× (parameter count grows
~5×; ONNX inference is largely linear in params on CPU). A swap to a
1.5B-parameter 1536-dim model multiplies it by 30–50× and is **not
viable for the local-CPU UX without a GPU dependency**. Concrete
numbers depend on hardware and need a Phase-0 micro-benchmark (§17.7).

**Storage cost is negligible by comparison.** At 100K symbols:

* 384-dim: 150 MB total embedding bytes.
* 768-dim: 300 MB.
* 1536-dim: 600 MB.

The metadata + code text dwarfs all of these. Storage is not the deciding
factor at any dim in this catalog.

**HNSW build cost is also negligible by comparison to inference.** HNSW
construction at 100K vectors with `m=16, ef_construction=200` runs in
seconds-to-minutes range across all three dims; inference is the
order-of-magnitude bottleneck.

### §17.5 Read-side wins from a larger model

If the operator does swap to a larger / code-specific model, the read-
side wins are:

* **Recall improvement on `unified_search` and content-record search.**
  The MTEB / CodeSearchNet evidence above suggests 5–15% MRR uplift
  from `bge-small` → `jina-base-code`; further increase to a 1.5B model
  is typically <5% additional.
* **Long-context handling.** Both `NomicEmbedTextV15` and
  `JinaEmbeddingsV2BaseCode` support 8192-token context vs. bge-small's
  512-token limit. Engram's Tier 1 embed currently truncates long
  function bodies; an 8K-context model would embed full bodies for
  most symbols. This is a quality win independent of dim.
* **Cleaner hybrid graph+vector results** because a stronger semantic
  signal makes the graph-rerank step (§8.1) more discriminating.

**Read latency impact is small.** HNSW kNN at 100K vectors at any of
these dims completes in single-digit milliseconds. The CozoDB doc cite
in CIE's architecture.md notes "<10 ms kNN at 100K vectors" at 768 dim
— Engram's smaller dim baseline should be at least as fast.

### §17.6 Recommendation — unbundle the two changes

**Do NOT bundle a model / dim swap with the CozoDB migration.** The
CozoDB swap is already a high-blast-radius change with its own
verification surface. Ship it on the existing 384-dim baseline so the
read-quality regression risk surface stays small. The migration is
"prove Cozo gives the same answers as Surreal", not "prove Cozo +
new model gives better answers".

**Concrete plan delta (additive to §16.14):**

* **Phase 0 (close-out):** add a small embedding-model micro-benchmark
  task: measure hydration wall time and a small handcrafted retrieval
  golden set across `bge-small` (baseline), `jina-base-code` (768
  code-specialized), and `nomic-text-v1.5` (768 long-context). Single
  fixture corpus (~5K symbols), single laptop-class machine, single
  query set (~30 representative queries). Output: a small markdown
  table that informs the post-migration model decision.
* **Phase 1–7 (CozoDB swap):** keep `bge-small-en-v1.5`. Ship at
  `EMBEDDING_DIM = 384`. No model change.
* **Post-migration (separate spike or feature):** if the §17 benchmark
  shows ≥10% retrieval uplift from a 768-dim code-specialized model,
  promote a model-swap feature. Include re-hydration UX (background
  re-embed, opt-in flag, progress reporting) as part of that work.
  Do **not** attempt this in the same shipment as the CozoDB swap.
* **CIE-style 1.5B / 1536-dim path:** park as a future opt-in for
  power users with GPUs. Not viable for default local-CPU UX. Out of
  scope for this migration sequence.

### §17.7 Schema readiness — make dim swappable without code changes

Even though v1 ships at 384, the CozoDB schema should be **dim-agnostic
at the Rust level** so a future model swap requires only re-hydration,
not a code change:

```rust
// src/services/embedding.rs (post-migration)
pub const EMBEDDING_DIM: usize = 384;     // matches the active model
pub const EMBEDDING_MODEL: &str = "bge-small-en-v1.5";

// Schema generation reads the constant, not a literal:
//   format!(":create function_embedding {{ id => embedding: <F32; {}> }}", EMBEDDING_DIM)
```

When a future model swap happens:

1. Bump both constants together.
2. Bump `SCHEMA_VERSION`.
3. Hydration path detects version mismatch and triggers re-embed of all
   Tier 1 + Tier 2 symbols.
4. HNSW index is dropped and recreated at the new dim.
5. JSONL ground truth on disk is unaffected — the rebuildable-index
   property holds.

The `EMBEDDING_MODEL` string is also recorded in the daemon status
output so the operator can see at a glance which model the index was
built against (matches CIE's pattern of explicit model identification
in `cie_function_embedding` schema metadata).

### §17.8 Updated risk register entry (embedding-related)

Add to §16.12:

| Risk | Severity | Mitigation |
|---|---|---|
| Schema is created with literal `384` and a future model swap requires a code+schema change rather than re-hydration only | Low | §17.7 — parameterize `EMBEDDING_DIM` from a single constant; bake `SCHEMA_VERSION` ↔ model mapping into hydration check. |
| Operator assumes higher-dim = better retrieval; ships a Qodo-Embed-style 1.5B model and degrades local-CPU hydration UX | Medium | §17.4 / §17.6 — explicit recommendation in the impl-plan documenting "model family > dim", with a Phase-0 micro-benchmark to ground the decision. |
| §17.7 dim-swap path is added to the plan but never exercised, drifts out of date | Low | Phase-0 micro-benchmark exercises the swap path under controlled conditions. Add a low-priority test (`tests/integration/embedding_dim_swap_test.rs`) that re-hydrates with two dim configurations on a tiny fixture. |

### §17.9 What this section deliberately does not decide

* **Which specific 768-dim model** to recommend post-migration. That
  needs the Phase-0 micro-benchmark on Engram's actual workload, not a
  decision from this spike.
* **Whether to ever pursue the 1.5B / 1536-dim path.** Parked as an
  opt-in future feature for GPU-equipped users; not part of the
  migration roadmap.
* **Whether Tier 1 / Tier 2 thresholds need adjusting** for an 8K-
  context model. Likely yes (Tier 1 could embed more symbols if the
  context limit goes from 512 to 8192), but that is a follow-on
  refinement after the model swap, not part of the migration itself.

### §17.10 Closure on §15 unknown #4

Replace the "embedding dimension decision" item in §15 with:

> **4. Embedding dimension and model.** Researched in §17. **Decision:
> ship the CozoDB migration at 384-dim / `bge-small-en-v1.5` to keep
> the migration surface small.** Add a Phase-0 micro-benchmark
> comparing 384 (baseline) vs. 768 (`jina-base-code`, `nomic-text-
> v1.5`) on a representative fixture corpus to inform a post-migration
> model-swap proposal. Schema design (§17.7) makes a future swap a
> re-hydration concern, not a code-change concern. The 1.5B / 1536-dim
> path (CIE's `Qodo-Embed`) is parked as a future GPU-only opt-in.
