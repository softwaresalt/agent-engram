# Ship session — 2026-07-11 — 078-S rec1-calledges (cross-file & method-call resolution)

**Agent:** Ship · **Repo:** softwaresalt/agent-engram · **Branch:** `feat/078-rec1-calledges` (off `main` @ `eaa9086`)
**Shipment:** **078-S** — rec1-calledges: cross-file & method-call resolution with durable provenance
**Feature:** 082-F · **Tasks:** 10 (all reached `done`) · **Backlog:** CLI `C:\Tools\backlogit.exe` (MCP unavailable)
**Authoritative design:** `docs/exec-plans/2026-07-10-callgraph-cross-file-resolution-plan.md`

## Outcome

All 10 tasks implemented test-first (red→green), each a coherent buildable conventional commit with the Copilot
co-author trailer, using explicit per-file `git add` (never `-A`/`.`). The two protected uncommitted deletions
(`.github/agents/auto-mergeinstall.agent.md`, `auto-tune.agent.md`) were left untouched throughout.

### Tasks → commits (DAG order)

| Task | Role | Commit |
|---|---|---|
| 082.001-T | parsing: capture method/receiver calls | `aa089eb` |
| 082.002-T | staging CAPTURE (`staged_call` + record-unresolved) | `e762a2d` |
| 082.003-T | provenance STORAGE (`calls_edge.resolution` migration + writer) | `ee86d62` |
| 082.011-T | MODEL + EXPORT projection (`CodeEdge.resolution`) | `08b7fba` |
| 082.012-T | REHYDRATE provenance on restart | `cb4ec6a` |
| 082.008-T | POST-PASS unambiguous cross-file resolution (full index only) | `82ef66b` |
| 082.009-T | staging LIFECYCLE (clear / delete cleanup / retract stale) | `1aea9d1` |
| 082.010-T | ROLLBACK down-migration LOGIC | `51da87a` |
| 082.013-T | ROLLBACK CLI TRIGGER (`engram migrate-down calls-resolution`) | `05e58db` |
| 082.004-T | ACCEPTANCE (manifest target-correctness + eval) | `0bee21e` |

Plus `7809abd` (chore: shipment/feature active-state backlog markdown).

## Canonical invariants (held)

- Provenance stored value is EXACTLY `calls_resolved_singleton` (never shortened); in-file direct edges use `direct`.
- Provenance survives the full path DB → export (JSONL) → rehydrate → DB (proven by 082.012 round-trip test).
- Every commit is buildable: struct-field additions fixed all construction sites in the same commit.

## Key engineering decisions

1. **082.011 export projection (deviation from literal task text).** The task said set `resolution: None` on the
   3 generic edge-read `CodeEdge` constructors. Doing so would make the production dehydrate→rehydrate path
   silently DROP singleton provenance (violating the canonical invariant AND 082.011's own Contract). Instead the
   `calls_edge` branch of `edges_from_table` PROJECTS `resolution` (Some); imports/concerns/defines/inherits set
   `None`. 082.012's round-trip test confirmed this was necessary and correct.

2. **Additive writer, stable API.** `create_calls_edge_with_resolution(from,to,resolution)` is new; the 2-arg
   `create_calls_edge` delegates to it with `direct`, so the ~30 existing callers are untouched.

3. **Post-pass runs on full/`--force` index only** (`code_graph.rs:~553`), never on incremental sync
   (`:~1171`). Singleton edges are created only for exactly-one-definition names; ambiguous and unmatched names
   produce no edge (no false edges).

4. **Rollback ordering (082.010).** `rollback_calls_resolution` retracts ALL `calls_resolved_singleton` edges
   WHILE the `resolution` column still exists, THEN drops the column — so every provenance query runs before the
   schema reverts. Idempotent: a re-run retracts nothing and finds no column to drop.

5. **CLI trigger (082.013)** is a deliberate destructive maintenance subcommand: local, holds the daemon lock,
   never auto-runs, never a daemon MCP tool. Unknown target → exit 2. Thin wrapper — all logic stays in 082.010.

6. **Acceptance (082.004) primary gate = ground-truth manifest.** The 081-F `false_edge_rate` is a conservative
   LOWER BOUND (dangling callees only; cannot detect mis-resolution to an existing-but-wrong function —
   follow-up D07F0919), so the PRIMARY gate is exact match of produced `calls_resolved_singleton` edges against a
   hand-authored expected-edges manifest. "Pre-change" recall = sync-only (no post-pass); "post-change" = full index.

## Migration facts (Cozo 0.7.6)

- Up: `?[from,to,created_at,resolution] := *calls_edge{from,to,created_at}, resolution="direct"` →
  `:replace calls_edge { from, to => created_at, resolution }`.
- Down: `?[from,to,created_at] := *calls_edge{from,to,created_at}` → `:replace calls_edge { from, to => created_at }`.
- Column probe: `::columns calls_edge`; `row.first()` is `DataValue::Str(name)`; use `name.as_str()`
  (NOT `as_ref()` — ambiguous with `SmartString`).
- **connect_db bootstrap re-runs the up-migration**, so a fresh DB open after a rollback re-adds the `resolution`
  column (defaulting survivors to `direct`). Operationally the rollback is run right BEFORE deploying reverted code
  (which no longer carries the up-migration) — documented in the CLI test comments.

## Test infrastructure notes

- New tests registered as `[[test]]` in `Cargo.toml` under the `# ── 082-F:` block (name + path + required-features).
  Integration crates need `#![allow(clippy::needless_raw_string_hashes)]` + `#![allow(clippy::doc_markdown)]`.
- Schema-internal scenarios that need a raw legacy-schema CozoDB (migration/rollback column state, legacy-writer
  round-trip) live in `src/db/cozo_backend/schema.rs` `#[cfg(test)] mod tests` (runs under `cargo test --lib`).
- The 082.013 CLI test drives the real binary via `CARGO_BIN_EXE_engram` as a subprocess (mirrors
  `cli_direct_test.rs`), pre-populating the DB through `CodeGraphQueries` then invoking `migrate-down`.

## Quality gates

- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic`
  — PASS (fixed `manual_let_else` in migrate.rs and `similar_names` in the acceptance test during the run).
- `cargo test --no-default-features --features cozo-backend,embeddings --all-targets` — all new tests PASS.
  Four unrelated binaries failed under maximal parallelism due to CozoDB SQLite `database is locked (code 5)`
  contention (aggravated by 3 stale Jul-10 `engram` daemons holding locks): `contract_retrieval_eval_status`,
  `integration_daemon_startup_order`, `integration_lang_ipc_indexing`, `integration_smoke`. After stopping the
  stale daemons, ALL FOUR PASS in isolation (`--test-threads=1`) — confirmed environmental, not a code regression.
- `cargo audit` — 10 pre-existing advisories in transitive deps (rand, scc, …). **Cargo.lock is UNCHANGED vs
  merge-base `eaa9086`** and Cargo.toml only added `[[test]]` entries → zero dependency delta, no regression.
  (Not a CI gate: `ci.yml` runs clippy + test only.)

## `--all-features` caveat

`cargo test --all-features` does NOT build: the optional `otlp-export` feature fails against pinned
`opentelemetry_sdk 0.26` (`SdkTracerProvider` / `SpanExporter::builder` gone). Pre-existing, unrelated to rec1.
CI and the correct local gate use `--no-default-features --features cozo-backend,embeddings`.

## Copilot review lifecycle (4 review passes; circuit breaker = 3 fix cycles)

PR **#241** (branch `feat/078-rec1-calledges` → `main`). Copilot posted findings across four
review passes; each pass drove one fix commit until the 3-cycle circuit breaker was reached.

| Cycle | Fix commit | Findings addressed |
|---|---|---|
| 1 | `252d625` | (1) method/receiver calls could resolve to an unrelated unique free function → added `is_method` marker, methods extracted but not promoted; (2) incremental-sync file-clear path did not retract the file's existing singletons → added `retract_resolved_calls_edges_for_file` there; (3) post-pass was additive-only → added targeted re-resolution retraction |
| 2 | `1487086` | (4) cycle-1 global retract-all was destructive after rehydration → replaced with TARGETED retraction scoped to currently-staged callers (preserves rehydrated singletons); (5) `count_call_sites` denominator still counted method calls → filter `is_method: false`; (6) post-pass singletons not added to `IndexResult.edges_created` |
| 3 | `abb23bf` | (7) path-qualified calls (`Type::parse`) reduced to bare segment could resolve to an unrelated free function → added `is_qualified` marker (scoped_identifier arm), qualified calls extracted but not promoted, denominator excludes them; (8) `reresolve_calls_edges` incremented `resolved` on every upsert → snapshot existing singletons up front and count only genuinely new caller-target pairs |
| 4 (post-breaker) | — accepted as backlog | (9) `migrate-down` lacks write exclusivity under shared `ENGRAM_DATA_DIR` multi-daemon; (10) resolution-column schema `:replace` bypasses `run_script_retrying` (SQLITE_BUSY under concurrent open). Both are shared-data-dir concurrency-hardening edge cases, not core-path regressions → recorded as stash items, threads replied + resolved, proceeded to merge per the 3-cycle circuit-breaker protocol. |

All review threads replied to and resolved (0 unresolved at merge). CI `build` green on final HEAD `abb23bf`
(3m39s). One earlier CI run hit the known flaky CozoDB `database is locked (code 5)` integration
contention (unrelated to rec1) and passed on re-run.

## Follow-ups / stash candidates (created via `backlogit stash add`)

- `2323C72A` (feature, high) — qualification-aware & method-aware call resolution: recover module-path,
  method-receiver, and `Type::assoc` recall deferred by findings 1 & 7, matching qualified index names
  without reintroducing false edges.
- `49561F22` (task, medium) — **D07F0919**: `false_edge_rate` is a lower bound (dangling callees only,
  not mis-resolution to an existing-but-wrong function); add sample-verification against ground truth.
- `E1A9ED33` (feature, medium) — persist/rehydrate `staged_call` rows through JSONL so the post-pass can
  fully revalidate after a restart (Copilot finding 4 fuller suggestion).
- `5C1EDA41` (task, medium) — Copilot finding 9: add write exclusivity to `migrate-down` for shared
  `ENGRAM_DATA_DIR` multi-daemon setups (or reject shared external data dirs).
- `8506BC68` (task, medium) — Copilot finding 10: route the resolution-column schema migration through
  `run_script_retrying` so a concurrent open retries on SQLITE_BUSY instead of failing startup.
- `2C949608` (feature, low) — optionally run the cross-file post-pass on the incremental-sync path
  (currently full/`--force` only).

## Post-merge closure (DONE)

- **Merged:** PR #241 as a **merge commit** (`gh pr merge 241 --merge --delete-branch`, no `--admin`).
  **Merge SHA `bf8d8a6`** (2 parents: base `eaa9086` + feature tip `abb23bf`); state `MERGED` @
  2026-07-12T02:54:11Z; verified an ancestor of `origin/main`.
- **Local main** fast-forwarded to `bf8d8a6` (`git pull --ff-only` → up to date).
- **Runtime verification:** `cargo build --release` (bin `engram`) succeeded (2m21s, no errors). Smoke of
  the 082.013 CLI entry point `engram migrate-down calls-resolution` against a throwaway git workspace +
  temp `ENGRAM_DATA_DIR`: exit 0, `resolution_column_dropped: true`, `retracted_singleton_edges: 0`, and
  idempotent (second run identical, exit 0). Non-git workspace correctly rejected with exit 2.
- **Backlog:** all 10 tasks `done`; feature **082-F** and shipment **078-S** moved `active → done`
  (archived to `.backlogit/archive/`). Six follow-up stash items created (above). `backlogit sync` OK.
- **Protected files** `.github/agents/auto-mergeinstall.agent.md` and `auto-tune.agent.md` remained
  uncommitted local deletions throughout — never staged, committed, restored, or pushed.
