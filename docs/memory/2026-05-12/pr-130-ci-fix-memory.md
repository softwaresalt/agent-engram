---
title: PR 130 CI fix memory
type: session-memory
date: 2026-05-12
pr: 130
branch: chore/autoharness-mergeinstall-v1.4.0
commit: 87af02e
status: completed
---

# PR 130 CI fix memory

## Outcome

PR #130 is green after commits `32c9a5c` and `87af02e`.
The fix shipped in two steps:

* `32c9a5c` fixed the GitHub log's original startup hydration lock contention:
  `count_code_files()` and offline file-hash reads/writes could hit
  `database is locked` during `background_db_hydration`, leaving indexing
  incomplete and causing `t030_001_swift_function_indexed_via_ipc` to time
  out with no symbols
* `87af02e` fixed the follow-on startup sync race:
  startup auto-sync could exit early when hydration already held the indexing
  lock, leaving a fresh workspace "ready" with an empty graph until a later sync

## Files changed

* `src/db/cozo_queries.rs`
  * added shared `SQLITE_BUSY` helpers and immutable retry support
  * routed startup-sensitive hydration and file-tracker queries through retry paths
* `src/db/cozo_backend/mod.rs`
  * kept same-path DB-open serialization
  * documented why `DB_OPEN_LOCKS` intentionally retains stable per-path locks
  * replaced the timing-based lock test with deterministic signaling
* `.backlogit/archive/stash.jsonl`
  * fixed shipment 049 harvested stash records to use `removal_reason: "harvested"`
  * added `harvested_artifact_id` values for `049.001-T` through `049.006-T`
* `tests/integration/cli_direct_test.rs`
  * fixed clippy `doc_markdown` warnings
* `src/daemon/ipc_server.rs`
  * queued startup auto-sync through the existing `pending_sync` path when hydration already held the indexing lock
  * added a regression test covering the queued startup-sync path
* `src/daemon/watcher.rs`
  * made `.engram/` an unconditional watcher exclusion so daemon-managed writes cannot keep TTL alive
  * added a watcher unit test covering empty user exclusion lists

## Validation

* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
* `cargo test --no-default-features --features cozo-backend,embeddings --lib engram_dir_is_always_excluded_even_with_empty_patterns`
* `cargo test --no-default-features --features cozo-backend,embeddings --test integration_daemon_startup_order`
* `cargo test --no-default-features --features cozo-backend,embeddings --test integration_lang_ipc_indexing`
* `gh pr checks 130 --watch --fail-fast`

Notes:

* `cargo audit` still reports pre-existing dependency advisories in transitive crates
  (`cozo`, `fastembed`, and related trees); no dependency changes were made in this fix
* local Windows runs showed the earlier all-targets command was slow enough to be impractical for repeated iteration,
  so the final validation used the directly affected integration suites and then the authoritative PR CI run

## Review thread handling

* Replied to all 3 Copilot threads after pushing `32c9a5c`
* Resolved threads via GraphQL:
  * `PRRT_kwDORJEduc6BQLUY` — declined with rationale on stable lock retention
  * `PRRT_kwDORJEduc6BQLUm` — fixed deterministic test
  * `PRRT_kwDORJEduc6BQLU5` — fixed harvested stash metadata

## Next state

* Branch `chore/autoharness-mergeinstall-v1.4.0` contains both fix commits
* PR #130 is updated, reviewed, and green
