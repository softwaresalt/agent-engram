---
type: compacted-memory
scope: chore/001-c-cozodb-datalog-migration
phases: Phase-0 → Phase-2 (003-S shipped)
date: 2026-04-20
status: complete
merge_commit: 0f195d37d6b018312aec61eb2974d23f3a1d83ae
sources:
  - docs/memory/2026-04-13/phase-1-u14-u16-complete-memory.md
  - docs/memory/2026-04-14/shipment-a-staging-memory.md
  - docs/memory/2026-04-19/cozodb-spike-memory.md
  - docs/memory/2026-04-19/stage-triage-cozodb-checkpoint.md
  - docs/memory/2026-04-19/stage-cozodb-shipment-handoff.md
  - docs/memory/2026-04-19/phase-2-harness-complete-memory.md
  - docs/memory/2026-04-20/phase-2-u21-done-u22-in-progress.md
  - docs/memory/2026-04-20/003-s-phase2-pr-ready-memory.md
  - docs/memory/2026-04-20/003-s-post-merge-memory.md
archived_to: docs/archive/memory/
---

# Compacted Memory — 003-S CozoDB Migration (Phase 0 → Phase 2)

## Shipment Scope

`003-S` — CozoDB + Datalog migration, root chore `001-C`.
Phase 2 (chore `001.003-C`) shipped. Phases 3–7 remain.

---

## Phase 0 Decisions (final, locked)

| Decision | Outcome | Rationale |
|----------|---------|-----------|
| Storage backend | SQLite (`cozo = { version = "0.7", features = ["storage-sqlite"] }`) | Pure-Rust, no RocksDB native dep, cross-platform CI |
| Embedding model | `bge-small-en-v1.5` (384-dim) | Baseline locked in U2.8; micro-benchmark deferred |
| API spike | `examples/cozo_api_spike.rs` — validated CozoDB 0.7 trait surface | Kept as research artifact, gated `required-features = ["cozo-backend"]` |
| HNSW benchmark | Deferred — `#[ignore]` tagged, will run in Phase 4 before vector work | Phase 4 gate |
| 003-F (vector parity) | Superseded by `001.005-C` Phase 4 | Closed as duplicate in Stage handoff |

---

## Phase 1 Key Decisions (committed in aa33891, 8d21edd)

| Decision | Outcome |
|----------|---------|
| CI matrix | Two axes: `surreal-backend` (required) + `cozo-backend` (advisory, `continue-on-error: true`). Cozo axis becomes required after Phase 4. |
| `cozo_queries.rs` | Feature-gated `#[path = "cozo_queries.rs"] pub mod queries` in `src/db/mod.rs`. Mutually exclusive with `queries.rs`. |
| Dual-backend helper macros | `assert_ok_or_stub!`, `assert_empty_count_or_stub!` in `tests/helpers/dual_backend.rs` |
| Inner attributes in `tests/helpers/mod.rs` | `//!` and `#![allow(dead_code)]` MUST precede `pub mod` items — Rust E0753 rule |
| `examples/cozo_api_spike.rs` gate | `required-features = ["cozo-backend"]` — prevents surreal-backend CI failure |

---

## Phase 2 Key Decisions (committed in 47d81d2, 715e849, e27a054)

| Decision | Outcome |
|----------|---------|
| `CozoHandle` stays unit struct | Harness constraint: `cozo_schema_test.rs` constructs it as `CozoHandle` with no args |
| `CozoDb(Arc<cozo::DbInstance>)` | Production handle; `type Db = CozoDb` |
| `SchemaTarget` trait | Dispatches `cozo_instance()`: unit struct → in-memory, CozoDb → Arc clone |
| `connect_db` idempotency | Matches "already"/"defined"/"conflicts"/"existing" to suppress duplicate-relation errors |
| `find_symbols_by_name` returns `Ok(vec![])` | `impact_analysis` empty-check contract — other graph stubs return `Err(backend_err())` |
| `run_schema_bootstrap` must be `pub` | External test crates in `tests/` cannot see `pub(crate)` — P3 visibility narrowing rejected |
| `record_query_metrics` in `cozo_queries.rs` | Pure tracing impl (backend-agnostic), mirrors `queries.rs` pattern |

---

## CI / Build Lessons (Phase 2)

1. **`--no-default-features --features cozo-backend`** is required — `--features cozo-backend` alone adds cozo on top of surreal default, triggering mutual exclusion compile_error.
2. **Rust 1.95 CI vs 1.85 local** — 4 lint classes caught only by 1.95: `useless_conversion`, `doc_markdown`, `unnecessary_hashes`, `uninlined_format_args`. Caused 4 fix iterations.
3. **`String + &String` → `format!`** — `String::Add` is `Add<&str>`, not `Add<&String>`; `.repeat()` returns `String`, borrow is `&String`.
4. **P3 visibility revert** — `pub(crate)` on items called from `tests/` crates fails with E0603. Always grep `tests/` before applying visibility narrowing.

---

## Compound Learnings Captured

| File | Category |
|------|----------|
| `docs/compound/best-practices/pub-visibility-for-external-test-harness-2026-04-20.md` | best-practices |
| `docs/compound/workflow-issues/mutually-exclusive-features-no-default-features-2026-04-20.md` | workflow |
| `docs/compound/build-errors/string-add-string-ref-type-error-2026-04-20.md` | build-errors |
| `docs/compound/workflow-issues/ci-rust-version-gap-clippy-lints-2026-04-20.md` | workflow |

---

## Commits Shipped (Phase 2, PR #15)

| SHA | Summary |
|-----|---------|
| `e27a054` | feat(build): implement U2.1 schema bootstrap |
| `47d81d2` | feat(build): implement U2.2–U2.10 CozoDB CRUD and connection layer |
| `715e849` | feat(build): implement U2.7 validate_cozo_embedding |
| `9523487` | fix(build): apply review findings P2/P3 for CozoDB Phase 2 |
| `ec3cee9` | fix(build): fix CI clippy failures on Rust 1.95 |
| `2659594` | fix(build): fix remaining CI clippy failures (examples + doc_markdown) |
| `6e2cd27` | style(build): cargo fmt after map_or_else refactor |
| `be38ad2` | docs(docs): add session memory and operational closure for 003-S phase 2 |
| `c668d19` | docs(docs): capture compound learnings from 003-S CozoDB Phase 2 |

---

## PR Status → MERGED

- **PR #15:** https://github.com/softwaresalt/agent-engram/pull/15
- **Merge commit:** `0f195d37d6b018312aec61eb2974d23f3a1d83ae`
- **CI:** ✅ both legs
- **Copilot review:** ✅ replied + resolved
- **State:** merged to `main`

---

## Post-Merge Closure (Step 6)

| Step | Outcome |
|------|---------|
| `backlogit_ship_shipment` | ✅ 50 items archived |
| P-007 archive integrity | ✅ restored via `git restore .backlogit/archive/` |
| Backlogit commit | `d663b77` |
| `docs/architecture.md` update | ✅ Dual-Backend section added — `e4a378e` |
| Compound refresh | No stale entries found — all 7 compound learnings remain accurate |
| compact-context | ✅ this pass |

### main HEAD after closure
`e4a378e` — docs(adrs): add CozoDB dual-backend architecture section

---

## Tests Passing at Phase 2 Completion

`unit_cozo_schema` 8/8, `unit_cozo_validation` 6/6, `integration_cozo_crud` 11/11,
`integration_cozo_dual_backend_sweep` 4/4, `integration_dual_backend_smoke` 3/3,
`contract_graph_traversal` 6/6 ✅

---

## Follow-Up Stashed

| ID | Title |
|----|-------|
| `68E3719F` | Phase 3: CozoDB graph edge CRUD, BFS traversal, vector KNN, bulk reads |
| `83B6BC5A` | Update local Rust toolchain to 1.95+ |
| `ED646C92` | Replace CozoDB idempotency error string-matching with version-locked constants |
