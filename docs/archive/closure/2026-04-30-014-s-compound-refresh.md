---
title: 'Compound Refresh — Shipment 014-S: CozoDB Migration Phase 3-4'
shipment_id: 014-S
mode: apply
scope: all
context: Phase 3-4 (edge CRUD, BFS traversal, HNSW vector parity) shipped — reviewing compound library for superseded or stale entries
date: 2026-04-30
---

## Scope

All 27 entries in `docs/compound/` reviewed against the changes shipped in Shipment 014-S.

---

## Entries Reviewed and Classifications

| File | Classification | Rationale |
|------|---------------|-----------|
| `best-practices/pub-visibility-for-external-test-harness-2026-04-20.md` | **keep** | Unrelated to Phase 3-4 |
| `best-practices/sql-quoted-identifier-resolution-candidates-list-2026-04-29.md` | **keep** | SQL parser guidance, unrelated |
| `best-practices/surrealkv-wal-corruption-recovery-sleep-2026-04-23.md` | **keep** | SurrealDB only, unrelated |
| `bugs/stale-engram-citation-2026-04-29.md` | **keep** | Protocol still valid; not affected by Phase 3-4 |
| `build-errors/clippy-derivable-impls-enum-default-2026-03-30.md` | **keep** | Unrelated to Phase 3-4 |
| `build-errors/cozo-backend-api-parity-stub-required-2026-04-29.md` | **update** | Phase 3-4 complete: `cozo_queries.rs` is now a full implementation. Updated title/description to reflect that new methods should be fully implemented, not just stubbed. |
| `build-errors/dirbuilder-mode-no-effect-on-existing-dirs-2026-04-23.md` | **keep** | IPC server, unrelated |
| `build-errors/string-add-string-ref-type-error-2026-04-20.md` | **keep** | Rust type error, unrelated |
| `build-errors/tree-sitter-grammar-abi-tsx-dispatch-2026-04-15.md` | **keep** | Parser grammar, unrelated |
| `build-errors/tree-sitter-sequel-join-grammar-2026-04-29.md` | **keep** | SQL parser, unrelated |
| `build-errors/tree-sitter-sequel-node-kind-debugging-2026-04-27.md` | **keep** | SQL parser, unrelated |
| `concurrency-issues/rwlock-toctou-temporary-guard-lifetime-2026-04-23.md` | **keep** | AppState concurrency, unrelated |
| `database-issues/surrealdb-lowercase-where-clause-broken-2026-04-29.md` | **keep** | SurrealDB only, unrelated |
| `database-issues/surrealdb-select-star-serde-json-2026-04-29.md` | **keep** | SurrealDB only, unrelated |
| `git/git-revert-merge-commit-requires-m1-2026-04-27.md` | **keep** | Git workflow, unrelated |
| `test-failures/daemon-key-requires-git-dir-in-unit-tests-2026-04-23.md` | **keep** | Test infrastructure, unrelated |
| `test-failures/global-metrics-store-concurrent-test-isolation-2026-04-23.md` | **keep** | Test isolation, unrelated |
| `test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md` | **keep** | Contract test patterns, unrelated |
| `workflow-issues/backlogit-ship-blocked-child-expansion-2026-04-26.md` | **keep** | Backlogit tool quirk, unrelated |
| `workflow-issues/backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md` | **keep** | Backlogit workflow, unrelated |
| `workflow-issues/ci-rust-version-gap-clippy-lints-2026-04-20.md` | **keep** | CI/toolchain, unrelated |
| `workflow-issues/clippy-all-targets-test-file-lints-2026-04-29.md` | **keep** | Confirmed again: Round 1 CI failures in 014-S also triggered by missing `--all-targets` on test files |
| `workflow-issues/mutually-exclusive-features-no-default-features-2026-04-20.md` | **keep** | Feature flag pattern, confirmed valid again in 014-S |
| `workflow-issues/rust-1-95-clippy-lint-ci-mismatch-2026-04-29.md` | **keep** | CI toolchain, unrelated |
| `workflow-issues/ship-shipment-no-item-archive-files-2026-04-23.md` | **keep** | Stale note already present. In 014-S, all items were pre-archived before `backlogit_ship_shipment` ran (items were moved in the PR), so this session cannot confirm or deny whether MCP creates individual archive files for items NOT already in archive. Core guidance remains: treat `backlogit_ship_shipment` as unreliable for creating item archive files and rely on shipment-reconcile post-mode. |
| `workflow-issues/ship-shipment-overscoped-manifest-2026-04-20.md` | **keep** | Shipment manifest guidance, unrelated |
| `workflow-issues/ship-step-6-7-use-append-comment-not-stash-remove-2026-04-27.md` | **keep** | Confirmed valid: used `backlogit_append_comment` pattern in Step 6.7 (no stash IDs to record, but pattern is correct) |

---

## Files Updated

### `build-errors/cozo-backend-api-parity-stub-required-2026-04-29.md`

**Change**: Updated title, description, and resolution section to reflect that `cozo_queries.rs`
now has full Phase 3-4 implementations. New guidance: new methods added to `queries.rs` should
be **fully implemented** in `cozo_queries.rs` — not just stubbed — now that parity is
established. Stubs with `Ok(None)` are only acceptable for methods that have no meaningful
cozo implementation.

Citations added: `https://github.com/softwaresalt/agent-engram/pull/53`, `014-S`

---

## New Learnings Surfaced by 014-S

These patterns appeared during 014-S build and review work. They are candidates for new
compound entries (not written here — use the `compound` skill for each):

1. **`concerns_edge` composite key is `(task_id, symbol_id)` not `(from, to)`** — differs from all
   other edge tables; causes silent `:rm` no-ops if `from`/`to` names are used.
2. **`imports_edge` and `references_edge` require 3-field `:rm`** — providing only `(from, to)`
   silently no-ops because the composite key includes `import_path`/`qualified_name`.
3. **CozoDB `:rm` response is not a reliable delete count** — use SELECT count first, then `:rm`,
   then accumulate the SELECT count (not the `:rm` response) for return values.
4. **BFS edge filtering must happen during traversal not post-hoc** — post-hoc filtering can
   include nodes reachable only via excluded edge types, producing semantically incorrect results.

---

## Recommendation

Library is in good shape. One entry updated (`cozo-backend-api-parity-stub-required`).
4 new learnings identified for future `compound` skill invocations.
