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
doctor_read_pin_test}.rs`. No files outside this owned set were modified.

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

None of these were fixed, re-triaged, or expanded into. All require Stage deliberation.

## Branch state (final)

- Branch: `feat/139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context`
- HEAD: `679500ce73a592214fef3928717fb3bd02294d79`
- 11 commits ahead of `origin/main` (`47eb9e1e440790768f2813a1cc549c6d2c17f394`)
- PR: #393 — https://github.com/softwaresalt/agent-engram/pull/393
- CI: `build` SUCCESS, `start-launcher-windows` SUCCESS; `mergeStateStatus: CLEAN`
- P-018 copilot-review gate: **SATISFIED** (all 8 Copilot-authored threads across 2 review
  rounds replied-to and resolved; 4 fixed in-scope, 4 deferred out-of-scope citing stash
  `EFE9190A`, 1 PR-description-staleness finding addressed by rewriting the readiness block)
- P-009 merge-strategy guardrail: repo allows merge-commit only (squash/rebase disabled) — compliant
- P-014 local readiness: recorded in PR body, reviewed HEAD matches current HEAD

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

## Next steps

**HALT at merge-approval gate** — `merge_approval_pre_authorized: false` per the
DARK_MODE_ACTIVE contract. All automated gates (CI, P-018, P-009, P-014) pass. Awaiting
explicit new operator approval before any merge. Shipment 139-S remains `active` (not yet
`shipped`/closed — that happens post-merge, out of scope for this run).
