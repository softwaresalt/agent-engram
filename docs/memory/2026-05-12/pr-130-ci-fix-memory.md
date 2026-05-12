---
title: PR 130 CI fix memory
type: session-memory
date: 2026-05-12
pr: 130
branch: chore/autoharness-mergeinstall-v1.4.0
commit: 32c9a5c
status: completed
---

# PR 130 CI fix memory

## Outcome

PR #130 is green after commit `32c9a5c`.
The failing CI root cause from the GitHub Actions log was startup hydration
lock contention in Cozo query paths: `count_code_files()` and offline
file-hash reads/writes could hit `database is locked` during
`background_db_hydration`, which left indexing incomplete and caused
`t030_001_swift_function_indexed_via_ipc` to time out with no symbols.

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

## Validation

* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`
* `cargo dev-test`
* `gh pr checks 130 --watch --fail-fast`

Notes:

* `cargo audit` still reports pre-existing dependency advisories in transitive crates
  (`cozo`, `fastembed`, and related trees); no dependency changes were made in this fix
* local Windows runs showed a targeted C++ IPC test can fail in isolation, but the full
  `integration_lang_ipc_indexing` test binary passed and the GitHub CI check cleared on the fix commit

## Review thread handling

* Replied to all 3 Copilot threads after pushing `32c9a5c`
* Resolved threads via GraphQL:
  * `PRRT_kwDORJEduc6BQLUY` — declined with rationale on stable lock retention
  * `PRRT_kwDORJEduc6BQLUm` — fixed deterministic test
  * `PRRT_kwDORJEduc6BQLU5` — fixed harvested stash metadata

## Next state

* Branch `chore/autoharness-mergeinstall-v1.4.0` contains the fix
* PR #130 is updated, reviewed, and green
