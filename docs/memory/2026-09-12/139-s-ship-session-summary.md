# 139-S Ship Session Summary

**Date**: 2026-09-12
**Agent**: Ship (P-017 dark-factory invocation, routed by Orchestrator)
**Shipment**: 139-S — Migrate read and lifecycle handlers to pinned generation context
**Branch**: `feat/139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context`
**Scope**: strictly 139-S only (142.034-T–142.039-T); 140-S/141-S/stash entries explicitly excluded

## Outcome

All 6 manifest tasks implemented, verified, and moved to `done`. Shipment remains `active`
(not yet closed — closure happens post-merge, out of scope for this dark-mode run since
`merge_approval_pre_authorized: false`). PR prepared for creation; session halts at the
merge-approval gate per the DARK_MODE_ACTIVE contract.

## Items completed

| Task | Title | Commit |
|---|---|---|
| 142.034-T | migrate core read handlers to pinned generation context | `4a3cade0` |
| 142.035-T | migrate report handlers to pinned generation context | `7604b56f` |
| 142.036-T | migrate lifecycle handlers to pinned generation context | `aa777c9a` + `8bfb790e` fixup |
| 142.037-T | migrate eval handler to pinned generation context | `a578baa7` |
| 142.038-T | migrate lint handler to pinned generation context | `b674ce15` |
| 142.039-T | migrate doctor handler to pinned generation context | `10c39b8a` |
| (quality) | satisfy clippy::pedantic and rustfmt in read-pin test harnesses | `12d56319` |

Owned files touched: `src/tools/{read,lifecycle,eval,lint,doctor}.rs` +
`tests/integration/{core_read_generation_pin_test,report_read_generation_pin_test,
lifecycle_read_generation_pin_test,eval_read_pin_test,lint_read_pin_test,
doctor_read_pin_test}.rs`.

**Correction (Copilot round-4 review, PR #393)**: an earlier version of this note claimed "no
files outside this owned set were modified." That was overbroad and imprecise — it is true for
*source and test files*, but this PR also adds normal Ship-workflow bookkeeping artifacts
outside the owned-files set: `.backlogit/stash.jsonl` (P-021 deferred-scope-expansion
entries), `.backlogit/queue/*.md` (task-status transitions), `docs/memory/2026-09-12/*.md`
(this file and prior checkpoints), and `docs/compound/test-failures/*.md` (learnings capture).
None of these are source or test changes and none expand the shipment's functional scope; the
original claim should have said "no *source or test* files outside this owned set were
modified" rather than an unqualified "no files."

## Quality gates (all independently verified by Ship)

- `cargo check --all-targets`: PASS (re-verified final: 1m18s clean)
- `cargo fmt --all -- --check`: PASS
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` (default features,
  the Step 4.3-mandated command): PASS
- 6 shipment harness integration tests (14 assertions): PASS, verified repeatedly
- Code review (code-review agent, full diff `origin/main..HEAD`): **READY**, 0 P0/P1/P2/P3 findings
- Full `cargo dev-test --no-fail-fast` (complete run, no early stop): 4 targets failed,
  ALL confirmed pre-existing/environmental and unrelated to the 5 owned production files:
  - `archive_verifier_runs_the_unpacked_native_binary` — confirmed reproduces identically on
    clean `origin/main` baseline clone (commit `47eb9e1e...`)
  - `backlog_index_100_items_under_5_seconds` — timing/perf test, passes in isolation,
    already tracked in stash `1346BC60`
  - `manifest_tool_count_matches_catalog` — passes in isolation, newly captured (`F1E5A255`)
  - `copilot_probe_then_handshake_completes_catalog_and_tool_call` — passes in isolation,
    newly captured (`F1E5A255`)
- `cargo lint`/`cargo ci` (`--all-features`): confirmed pre-existing broken repo-wide
  (OpenTelemetry API drift in `src/server/observability.rs`), reproduces on clean baseline
  clone; unrelated to 139-S; tracked in stash `74AAE80F`

## P-021 deferred-scope captures (stash entries, NOT implemented/triaged this session)

- `39049DEE` — pre-existing archive smoke test failure (see also long-standing `EC3BAF22`)
- `74AAE80F` — pre-existing OTEL/observability.rs `--all-features` build break
- `EFE9190A` — deferred scope candidate: full `ReadRequestContext` threading into shared
  dispatch infra (`src/tools/mod.rs` / `src/daemon/request_entry.rs`) — out of 139-S's
  owned-files set per P-021 C1
- `DE62B123` — pre-existing flaky metrics test (single occurrence)
- `9088F47D` — pre-existing test-suite flakiness pattern under full parallel `cargo dev-test`
  (2 timing-sensitive tests)
- `F1E5A255` — 2 additional flaky tests newly identified in the final complete
  `--no-fail-fast` run (`manifest_tool_count_matches_catalog`,
  `copilot_probe_then_handshake_completes_catalog_and_tool_call`)
- `F0A2A478` — deferred scope candidate: `set_workspace_with_probe` cannot distinguish a
  trusted-startup initial bind (`run_startup_driver`) from a public-handler bind
  (`dispatch()`) without a trust signal threaded from un-owned
  `src/daemon/startup_activation.rs` / `src/tools/mod.rs` — see the round-3 near-miss below

None of these were fixed, re-triaged, or expanded into. All require Stage deliberation.

## Branch state (final)

- Branch: `feat/139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context`
- HEAD: `3086969b19f40f8cba3b20396a7a826292c972bd`
- 13 commits ahead of `origin/main` (`47eb9e1e440790768f2813a1cc549c6d2c17f394`)
- PR: #393 — https://github.com/softwaresalt/agent-engram/pull/393
- CI: `build` SUCCESS, `start-launcher-windows` SUCCESS (re-verify at final pushed HEAD)
- P-018 copilot-review gate: re-run pending at HEAD `3086969b` (was SATISFIED at
  `679500ce`; round 3 introduced then reverted a regression, see below — expect at least one
  more review round on this push)
- P-009 merge-strategy guardrail: repo allows merge-commit only (squash/rebase disabled) — compliant
- P-014 local readiness: PR body to be rewritten again to reflect HEAD `3086969b`

## Post-implementation review-fix round (discovered via Copilot PR review, addressed before halt)

A GitHub-hosted Copilot review on PR #393 surfaced 7 findings at HEAD `b45a51dc`, all
triaged and closed:

- **3 in-scope, fixed directly** (commit `679500ce`):
  - `eval.rs` `run_retrieval_eval` opened the DB before checking the disabled-config early
    return — reordered so the check happens first; kept `connect_db` behind a dedicated
    `open_queries` helper so the existing structural test (asserting the pub handler body
    text doesn't contain `connect_db(`/`snapshot_dispatch_context(` directly) still passes.
  - `doctor.rs` `run_smoke_test` rustdoc was stale (claimed unconditional bind+shutdown;
    the new ReadServer branch does neither) — corrected to describe both modes.
  - `.backlogit/reconcile/139-S-pre-20260911-232025.md` was missing machine-readable
    `recommendation`/`artifact_type` frontmatter — added to match sibling reports.
- **4 out-of-scope, deferred per P-021 C1** (no code change; replied citing pre-existing
  stash `EFE9190A`, resolved): `doctor.rs:82`, `eval.rs:107`, `lifecycle.rs:1041`,
  `lint.rs:90`, `read.rs:77` all describe the same root architectural gap — handlers pin a
  per-invocation `DispatchSnapshot` rather than consuming the F20-admitted
  `Arc<ReadRequestContext>`, because `dispatch()` in un-owned `src/tools/mod.rs` does not
  thread that context to handlers at all (pre-existing F21 gap, stash `1918AFD2`). Closing
  this fully requires modifying `src/tools/mod.rs`, outside 139-S's owned-files set.
- A second Copilot review pass at the fix-commit HEAD (`679500ce`) flagged that the PR's
  readiness block was now stale relative to the new HEAD — corrected by rewriting the PR
  body and re-running a scoped code-review pass over the `12d56319..679500ce` diff (READY,
  0 findings).
- Re-verified after all fixes: `cargo check --all-targets`, `cargo clippy --all-targets -D
  warnings -D clippy::pedantic`, `cargo fmt --all -- --check`, and all 6 harness test files
  (15 assertions) — all PASS at final HEAD `679500ce`.

## Copilot review round 3 (HEAD `a01cedcd` → introduced-then-reverted regression)

A third Copilot review pass (triggered by the docs-only session-memory push, which still
re-arms review) flagged `src/tools/lifecycle.rs:398`: when ReadServer mode has no dispatch
context admitted yet (`snapshot_dispatch_context()` returns `None`), the code falls through
to the full write-capable bind path, and recommended refusing that case.

I implemented the refusal (commit `f54ed718`), added a regression test, and it passed
`cargo check`, `clippy --pedantic`, `fmt --check`, and the full `cargo dev-test` suite (702
passed). **This was not sufficient verification.** An independent scoped code-review agent
run against the `679500ce..f54ed718` diff caught that the fix was a critical regression:
`run_startup_driver` (`src/daemon/startup_activation.rs`) is the production entry point for
every daemon mode including `ReadServer`, and calls this exact function as the very first
bind — at a point where `snapshot_dispatch_context()` is *guaranteed* to be `None`. The
refusal fix made every `read_server`-mode daemon fail to start immediately. This was
confirmed directly by running `read_server_mode_survives_auto_spawn_and_bounded_restart`
(an end-to-end daemon-spawn integration test that **is** part of `cargo dev-test`'s target
set — see the round-4 correction below): it failed at `f54ed718` and passed at `679500ce`. This is exactly why an
earlier commit in this same PR (`8bfb790e`) had deliberately made the `None` case fall
through instead of refusing it — Copilot's round-3 suggestion reintroduced a previously-
fixed bug.

**Remediation**: reverted the fix and its test (`git revert f54ed718` → commit
`3086969b`), re-verified `cargo check`, `clippy --pedantic`, `fmt --check`, the lifecycle
pin test file (back to 3/3 passing), and `read_server_mode_survives_auto_spawn_and_bounded_restart`
(passing again). Captured stash `F0A2A478` documenting the real underlying gap (production
`ReadServer` mode has no trust-boundary separation between the trusted startup bind and the
public `set_workspace` handler) as a distinct, deferred, P-021 C1 out-of-scope finding —
correctly fixing it requires threading a trust signal from un-owned
`src/daemon/startup_activation.rs`/`src/tools/mod.rs::dispatch()` into `lifecycle.rs`.
Unresolved and re-resolved the Copilot thread with a corrected explanation citing the
revert and the new stash entry.

**Lesson for future sessions**: `cargo dev-test` DID include the regression-detecting test
(`.cargo/config.toml` defines `dev-test = "test --all-targets"`, no exclusion) — this was a
review-process failure, not a test-selection gap (see the round-4 correction below for the
full retraction). When a fix changes control flow in a function called from multiple
production entry points (here, both daemon startup and per-request dispatch), grep for all
call sites of the changed function before considering the fix verified, and always inspect
**complete** test output (or check the process exit code) rather than a truncated tail —
especially for a long multi-binary `--all-targets` run where a single expensive test's
`FAILED` marker can be buried mid-stream.

## Copilot review round 4 (HEAD `ff35e4d0` → 4 new findings, 2 genuine bugs + 2 doc corrections)

A fourth Copilot review pass (re-armed by the round-3 docs push) surfaced 4 new threads:

1. **Genuine bug (in scope)**: `tests/integration/report_read_generation_pin_test.rs` relies
   on a process-global test hook in `src/tools/read.rs`
   (`Mutex<Option<GenerationPinTestHook>>`) that two concurrently-running `#[tokio::test]`
   functions installing hooks for *different* methods could clobber, hanging one test forever
   on `reached_rx`. **Fixed** by refactoring the hook store to a per-method-keyed
   `Mutex<HashMap<String, GenerationPinTestHook>>` in `src/tools/read.rs` (owned file).
2. **Genuine bug (in scope)**: `src/tools/doctor.rs::run_smoke_test` re-read the daemon mode
   from on-disk config (`resolve_daemon_mode`) *after* `ensure_daemon_running` may have reused
   an already-live daemon — if config had drifted, this could send the Managed-mode smoke
   sequence (`set_workspace` + `_shutdown`) to a live `ReadServer` daemon, violating its
   non-destructive contract. **Fixed** by adding a `pub mode: String` field to `DaemonStatus`
   (`src/tools/lifecycle.rs`, populated via the pre-existing `DaemonMode::as_str()`) and
   refactoring `run_smoke_test` to always probe `get_daemon_status` first and derive the
   remaining smoke sequence from the daemon's own *observed* live mode rather than on-disk
   config. Both fixes stayed entirely within owned files.
3. **& 4. Documentation-accuracy findings (on Ship's own round-3 artifacts)**: Copilot
   correctly flagged that the compound doc's central claim ("cargo dev-test's default target
   set did not include this test") was factually wrong, and that the memory file's blanket
   "no files outside this owned set were modified" claim was contradicted by this PR's own
   docs/bookkeeping additions. **Independently re-verified both**: `cargo test --all-targets
   --no-run` confirmed the `integration_read_server_restart` binary is built and included;
   `git diff --stat` confirmed `.backlogit/`, `docs/memory/`, and `docs/compound/` files were
   indeed touched alongside the owned `src/tools/*.rs` set. Corrected both documents in place
   (this file, and the compound doc's frontmatter + Root Cause + Prevention sections) rather
   than leaving inaccurate institutional knowledge in the repository.

Also discovered incidentally during round-4 verification (not from Copilot, from Ship's own
full-suite re-verification) and captured as new P-021 deferred-scope stash entries per C1
(neither touches any 139-S owned file):

- `069B5F74` — `tests/contract/lint_dax_contract_test.rs::tool_count_is_twenty_one_and_matches_catalog`
  hardcodes the literal `21` and fails under `--features git-graph` (where
  `tools_catalog::TOOL_COUNT` is itself feature-gated to 23) — pre-existing, and the standard
  `cargo dev-test` alias does not enable `git-graph` so this does not surface in the canonical
  gate.
- `7A596F8C` — `contract_shim_stdio_initialize::t3_missing_result_is_terminal` failed once
  during a full-suite run but passed reliably in 3 isolated re-runs (clean HEAD and with
  round-4 changes present); consistent with the same environment-level parallel-load
  flakiness already tracked in stash `9088F47D`/`F1E5A255`.

Re-verified after both round-4 fixes: `cargo check --all-targets` (PASS), `cargo clippy
--all-targets -- -D warnings -D clippy::pedantic` (PASS), `cargo fmt --all -- --check` (PASS
after one auto-fix), all 6 owned pin-test files including with `--features git-graph` (PASS,
no hang — confirms the hook-registry fix), the end-to-end
`read_server_mode_survives_auto_spawn_and_bounded_restart` test (PASS), and a full `cargo
dev-test` run with **complete output captured to a file and grepped for every `test result:`
line** rather than a truncated tail (703 passed, 1 ignored, 0 failed at the canonical
no-git-graph invocation — the only failures observed were the two flaky/pre-existing items
above, seen only under the separate `--features git-graph` all-targets pass, and reproduced
as non-reproducible in isolation).

## Next steps

Round 4 fixes and doc corrections are being committed next, then pushed, then the 4 new
Copilot threads will be replied-to and resolved, the PR body rewritten to reflect the new
HEAD, and the P-018 gate re-run. **HALT at merge-approval gate** —
`merge_approval_pre_authorized: false` per the DARK_MODE_ACTIVE contract — remains the plan
once all gates are green at a stable final HEAD. Do not merge without a new explicit operator
approval signal. Shipment 139-S remains `active` (not yet `shipped`/closed — that happens
post-merge, out of scope for this run).
