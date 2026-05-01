---
title: "Operational Closure: Shipment 017-S — SurrealDB Removal (Phase 7 CozoDB Migration)"
date: 2026-05-01
shipment: 017-S
feature: 001.008-C
pr: "https://github.com/softwaresalt/agent-engram/pull/63"
merge_sha: 8cd565b
mode: post-merge
status: READY
---

# Operational Closure: 017-S — SurrealDB Removal

## Summary

Phase 7 of the CozoDB migration: complete removal of the `SurrealDB` dependency
(`surrealdb = "2"`), the `surreal-backend` Cargo feature, and all associated source
modules (`src/db/queries.rs`, `src/db/schema.rs`), test files, and test helpers.
`CozoDB` is now the sole embedded database backend. Net: ~5,930 lines deleted
across 22 files, ~2,400 Cargo.lock lines removed (transitive deps gone).

**PR #63** merged via commit `8cd565b` on branch
`chore/001.008-C-surreal-removal → main`.

---

## Invariants to Preserve

- `cargo build` succeeds with default features (no `--no-default-features` needed)
- `cargo clippy -- -D warnings -D clippy::pedantic` passes clean
- All non-flaky tests pass under `--features cozo-backend,embeddings`
- `--features surreal-backend` produces a clear `compile_error!` diagnostic
- `CozoDB` SQLite path resolves to `{data_dir}/cozo/{branch_safe}/engram.db`
- All 20 CozoDB schema relations bootstrap idempotently at connection time

---

## Risky Action Record

| Action | Risk | Approval | Result |
|---|---|---|---|
| Delete `src/db/queries.rs` (3,400 lines) | `destructive` | Operator: "prioritize SurrealDB removal first" | `applied` |
| Delete `src/db/schema.rs` (200 lines) | `destructive` | Operator approval above | `applied` |
| Remove `surrealdb` dep from `Cargo.toml` | `high` | Operator approval above | `applied` — ~200 transitive deps removed |
| Delete 9 test/helper files | `destructive` | Operator approval above | `applied` |
| Rewrite CI matrix (remove `surreal-backend` leg) | `moderate` | Implicit in execution scope | `applied` |

---

## Pre-Deploy Audits

- [x] `cargo fmt --all -- --check` passes
- [x] `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic` passes
- [x] `cargo test --no-default-features --features cozo-backend,embeddings` passes (pre-existing flaky `s_cs4` excluded via `continue-on-error: true`)
- [x] CI green on PR #63 (commit `de2e8d6`): `CI/build` — 1m 58s, 0 failures
- [x] All 4 Copilot review comments addressed and replied to
- [x] Shipment reconciliation pre-mode: PROCEED
- [x] Shipment reconciliation post-mode: PROCEED (P-007 clean — no archive deletions)
- [x] `docs/architecture.md` updated — SurrealDB section removed, dual-backend replaced with CozoDB-only section

---

## Deployment Path

**Merge-only** (no deployment step — local daemon binary, no hosted service).
Change is absorbed on next `cargo build` by any consumer of this codebase.
No data migration required — `CozoDB` SQLite format unchanged.

---

## Post-Deploy Checks

1. `cargo build` succeeds from a clean checkout on the `main` branch
2. `cargo test --no-default-features --features cozo-backend,embeddings` passes
3. `cargo clippy --no-default-features --features cozo-backend,embeddings --all-targets -- -D warnings -D clippy::pedantic` passes clean
4. `cargo build --no-default-features` produces the expected `compile_error!` diagnostic (not a cryptic missing-crate error)
5. Confirm `Cargo.lock` no longer contains `surrealdb` or `surrealkv` entries

---

## Healthy Signals

- Build time improvement: first `cargo check` after removal ~90 s → subsequent ~7 s (confirmed during execution)
- No `surrealdb`/`surrealkv` entries in `Cargo.lock`
- CI runs in <3 minutes (single job, no matrix overhead)
- All integration tests pass in isolation; pre-existing flaky test (`s_cs4`) unaffected

## Failure Signals

- `cargo build` fails: likely a `cozo-backend` feature or `cozo` dep misconfiguration
- Clippy reports `doc_markdown` or `items_after_statements` in test files: means a test file doc comment was missed during the cleanup
- Tests fail on a symbol not found: check `src/db/mod.rs` re-exports — `queries`, `cozo_backend`, and `workspace` must all be accessible

---

## Monitoring Plan

This is a local-first developer tool with no hosted runtime. No dashboards or alert
thresholds are applicable. Monitoring is CI-based:

| Signal | Check | Owner |
|---|---|---|
| Build correctness | `CI/build` GitHub Actions on every PR | Repo maintainer |
| Clippy cleanliness | `cargo clippy` step in CI | Repo maintainer |
| Test stability | `cargo test` step (`continue-on-error: true` for `s_cs4` flaky) | Repo maintainer |

---

## Rollback Trigger

**Trigger**: `cargo build` fails on `main` after merge AND the root cause traces to
a missing `cozo` dep or broken `cozo-backend` feature wiring.

**Note**: SurrealDB removal is irreversible from this point — the `surrealdb` crate
and source modules are deleted. There is no rollback; the fix path is a forward patch.

## Rollback Procedure

There is no rollback for this change — operator waived the rollback requirement
("don't worry about end users; we don't really have any end users yet").

**Forward fix path**: If a regression is found, create a new bug-fix branch from
`main`, fix the specific failure, and ship via standard PR process.

---

## Validation Window

**Duration**: 48 hours post-merge (light monitoring — next CI run or developer
build).
**Owner**: Repo maintainer (`softwaresalt`).

---

## Source Artifact Cleanup

| Field | Value |
|---|---|
| `source_stash_id` | Not present on `001.008-C` — chore originated from exec plan, not stash |
| `source_deliberation_id` | Not present on `001.008-C` |
| Deliberation references | `docs/decisions/2026-05-01-cozodb-phase5-7-deliberation.md` (linked from exec plan) |
| Exec plan | `docs/exec-plans/2026-05-01-cozodb-phase5-7-decided-plan.md` |

---

## Follow-Up Items (Stashed)

| Summary | Priority | Source |
|---|---|---|
| Fix stale doc comments in `cozo_queries.rs`, `gate.rs`, `tools_catalog.rs` (internal SurrealDB/SurrealQL references) | low | Discovered during PR review |
| Fix `s_cs4` flaky test: `CozoDB` SQLite unwrap() panic on concurrent open (U015-FLK1) | high | Pre-existing tracked bug |
| Remove remaining `#[cfg(feature = "cozo-backend")]` guards (cozo is now unconditional) | medium | Backlog task `001.008.002-T` acceptance criterion (partially deferred) |

---

## Readiness Status

**READY** — PR #63 merged, CI green, all review comments addressed, backlog archived,
`docs/architecture.md` updated. No open blockers.
