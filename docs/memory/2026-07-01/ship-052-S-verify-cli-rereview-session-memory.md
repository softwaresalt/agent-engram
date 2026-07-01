---
title: "Ship session — 052-S verify CLI re-review remediation (nit-2/nit-3)"
type: session-memory
role: ship
date: 2026-07-01
feature: 064-F
shipment: 052-S
pr: 185
branch: 064-engram-verify-cli
---

## What this session did

Resumed ownership of PR #185 to fix two operator-authorized Copilot re-review
nits on `src/cli/commands/verify.rs`, keep CI green, resolve the two threads,
and STOP before merge (operator-gated). Started at HEAD `3e4b234`, ended at
HEAD `9b4a342`.

### nit-2 — missing/unreadable non-markdown target must exit 2 (behavior)

The non-markdown short-circuit in `run_verify` returned `EXIT_CONFORMANT (0)`
before any existence check, so `engram verify does-not-exist.txt` exited `0`
instead of the contractual `2`. Fixed **test-first**:

* RED test `nonmarkdown_missing_file_exits_error` (I-VF-06) in
  `tests/integration/cli_verify_test.rs` — asserted exit `2`, observed exit `0`
  against pre-fix code (`faeb31e`).
* Fix (`00b0788`): `run_verify` now runs `tokio::fs::metadata(&target.read)`
  up front and maps NotFound/other/non-file to `EXIT_ERROR (2)` with the
  existing `cannot read '{display}'`-style message, **before** branching on
  markdown vs non-markdown. Existing non-markdown files still exit `0` (pinned
  contract preserved). Test went GREEN.

### nit-3 — document the `--quiet` stdout-summary exception (docs)

`OutputFormatter::success()` returns early (suppressing stdout) when the global
`--quiet` flag is set. The `verify` module docstring claimed the summary
envelope is always written to stdout. Documented the exception rather than
special-casing `verify` (keeps global `--quiet` semantics uniform). Commit
`1e61b4b`.

## Hard-won lessons (compound knowledge)

### 1. Copilot re-reviews cascade — bound them with the review-fix breaker

Each substantive push re-triggered a Copilot review that generated NEW inline
findings on unchanged/adjacent lines. Two behavior/doc fixes turned into five
downstream re-review findings across two rounds. Disposition rule that worked:
fix only findings that are **clearly trivial AND in-scope** (i.e. staleness the
current change itself introduced — the test module header and the clap help
text), and **reply/dispose** everything else (false positives, out-of-scope
coverage gaps, robustness nits) without unilaterally resolving. Stop pushing
code at the 3-cycle review-fix breaker; route the rest to backlog.

### 2. A behavior change owns its user-facing docs

The nit-2 exit-code change silently invalidated two descriptions: the
integration-test module header (fixed in `c7373f5`) and the `verify` clap help
in `src/bin/engram.rs` (fixed in `9b4a342`). When you change a pinned contract,
grep for every place that *describes* the old behavior (module docs, clap
`///` help, closure docs) and update them in the same PR.

### 3. Copilot "will not compile" can be a hallucination — verify empirically

Copilot claimed `emit_summary`'s `serde_json::json!({...})` moves `String`
fields out of a `&VerifyFinding` and "will not compile". It compiles fine
(`json!` serializes by reference); clippy-pedantic and the full suite were
green. Disprove compile-error claims with the actual green build rather than
pre-emptively "fixing" correct code.

### 4. Windows `canonicalize()` verbatim prefix vs `starts_with` containment

`Path::canonicalize()` returns `\\?\C:\...` on Windows. `contain_path`
canonicalizes the workspace root but keeps a *missing* absolute target lexically
un-canonicalized, so `read.starts_with(workspace_root)` can mismatch on the
verbatim prefix. This does NOT break the exit-code contract (existing files
canonicalize on both sides; a missing path exits `2` regardless), so it was left
as a backlog robustness item (`normalize_canonical`), not a blocker.

### 5. CI flake triage: prove it is a flake before re-running

`c018_07_denied_metrics_event_carries_agent_role` failed once on CI
(`got 0 event(s)` — a metrics-recorder timing race, unrelated to `verify`). The
prior HEAD's CI was green and the daemon-TTL flake passed in isolation, so
`gh run rerun --failed` was the correct remedy; it went green, as did the next
push's fresh CI run.

## Gotchas / mechanics

* Post threaded review replies with
  `gh api --method POST repos/{owner}/{repo}/pulls/{n}/comments/{comment_db_id}/replies -f "body=..."`
  and resolve with the GraphQL `resolveReviewThread` mutation. Never use
  `gh -b @file` (it posted a literal `@file`). Use single-quoted here-strings so
  backticks survive, and re-read the thread to confirm the body rendered.
* Keep commit messages/docs free of machine-specific absolute paths (a prior
  memory leaked a real username path and drew a review comment).

## State at handoff (STOP — operator merge gate)

* HEAD `9b4a342`; CI green on `c7373f5`, running on `9b4a342`.
* Resolved threads: nit-2 `PRRT_kwDORJEduc6NqWC8`, nit-3 `PRRT_kwDORJEduc6NqWDV`,
  plus re-review doc fixes `PRRT_kwDORJEduc6NrAdT` and `PRRT_kwDORJEduc6NrQtH`.
* Left open (non-blocking, backlog/triage): `PRRT_kwDORJEduc6NrAc0`
  (false positive), `PRRT_kwDORJEduc6NrAdB` (`body.empty` coverage),
  `PRRT_kwDORJEduc6NrQsy` (verbatim-prefix robustness).
* Left untouched per operator: thread [4] `PRRT_kwDORJEduc6NqWDp`
  (`docs/memory/2026-06-30-stage-B87680AB-session.md` frontmatter) — deferred.
* Did NOT merge; awaiting operator approval.
