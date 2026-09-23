# Ship 141-S — Copilot review triage complete, blocked on CI runner infra

**Session**: continuation of `2026-09-20-ship-141-s-implementation-and-remediation.md`.
**Shipment**: `141-S`. **PR**: #407. **Branch**: `feat/141-s-error-transport-response-provenance-lifecycle-policy-and-generation-observability`.
**Current HEAD**: `d306e989473420f47d901a236ea6287acb92c1f6`.

## What happened this session

1. Completed all 4 Copilot review thread dispositions:
   - Thread `PRRT_kwDORJEduc6kM96E` (`lifecycle_policy.rs:187`, generation-activator wiring) — replied citing deferred stash `9B7EC1E4`, resolved. No code change (duplicate of pre-existing deferred gap).
   - Thread `PRRT_kwDORJEduc6kM96Y` (`tools/mod.rs:377`, read-context dispatch wiring) — replied citing deferred stash `6C5DF765`, resolved. No code change (duplicate of pre-existing deferred gap).
   - Thread `PRRT_kwDORJEduc6kM96O` (`activation.rs:1158`, `publish_active` torn-observability window) — **genuinely new, in-scope finding**. Fixed in commit `743064fa`: the `active` write-lock is now held across both the pointer swap and `note_active_generation`, closing the torn-read/torn-write window. Added regression test `publish_active_keeps_active_pointer_and_observability_state_under_one_boundary` with a mid-publication pause hook. Replied citing the fix commit, resolved.
   - Thread `PRRT_kwDORJEduc6kM96T` (`activation.rs:1407`, `resolve_and_open` disk-usage over-report) — **genuinely new, in-scope finding**. Fixed in commit `f6c49b49` (+ fmt fixup `d306e989`): `disk_usage_bytes.retained_runtime_copies` now reports the actual runtime-copied database file size (via `std::fs::metadata` on `opened.runtime_copy().path()`) instead of `cumulative_bytes` (the whole sealed-inventory validation total, unchanged for its original cap-checking purpose). Added integration test proving the reported value diverges correctly from the inventory total when extra sealed artifacts exist. Replied citing the fix commit, resolved.

2. All 4 review threads are now resolved via GraphQL `resolveReviewThread`.

3. Independently re-verified after the two fixes at HEAD `d306e989`:
   - `cargo check --all-targets` — clean
   - `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — clean
   - `cargo fmt --all -- --check` — clean
   - `cargo dev-test` (full suite, default parallelism) — **0 failures** (this run was fully clean, unlike the prior session's two isolated-pass/parallel-fail flaky tests — did not reproduce this time)

4. Dispatched a scoped Correctness Reviewer subagent against `git diff 8c7b758f..d306e989` (the two Copilot-driven fixes only). Verdict: **READY**. Confirmed no deadlock risk from the lock restructuring, cumulative-cap logic unchanged, both new tests genuinely discriminate old vs new behavior. One non-blocking advisory noted (test-only `OnceLock` hook scope, no action needed).

5. Re-requested Copilot review (new code pushed) and re-polled the P-018 gate:
   ```
   autoharness gate copilot-review 407 --repo softwaresalt/agent-engram --enforcement auto --max-wait 900 --json
   ```
   Result: **`SATISFIED`** at `head_ref_oid: d306e989473420f47d901a236ea6287acb92c1f6`, `rounds: 4`, `unresolved_thread_ids: []`. **P-018 gate is now fully PASSED.**

## Current blocker: CI runner infra, NOT a code issue

The `CI` workflow (run id `35547379638`, triggered by the `8c7b758f..d306e989` push) has been **stuck/hung repeatedly** across 5 consecutive attempts (cancel + rerun cycles), each time hanging at a different, progressively earlier step:

| Attempt | Hung at | Wait before cancel |
|---|---|---|
| 1 | `start-launcher-windows` completed with a **flaky timing failure** (`launcher_fails_open_to_copilot_within_one_prewarm_budget`, elapsed 8.9645s vs its own 8s hosted-runner budget assertion — margin < 1s, pre-existing test unrelated to any file this shipment touches). `build` job **passed cleanly** in this same attempt (6m29s, fmt/clippy/test all green). |
| 2 (rerun `--failed`) | `start-launcher-windows` → "Test PowerShell launcher contract" step hung >30 min | cancelled |
| 3 (full rerun) | both jobs hung at their "test"/"clippy" steps >30 min | cancelled |
| 4 (full rerun) | both jobs hung at "Cache cargo"/toolchain-adjacent steps >15 min | cancelled |
| 5 (full rerun, in flight) | both jobs hung at **"Install toolchain"** (the very first non-checkout step, normally 8-13s) for 9+ min | cancelled |

`https://www.githubstatus.com` reports "All Systems Operational" for Actions, but hosted Windows runner pool slowness/hangs frequently do not surface as a formal status-page incident. The fact that hangs have occurred at **every stage of the pipeline including the earliest steps that run before any of this shipment's code is even touched** is strong evidence this is **infrastructure-level GitHub Actions Windows-runner degradation**, not a regression introduced by 141-S.

**Evidence the code itself is sound, independent of CI**:
- Local `cargo check`/`clippy -D warnings -D clippy::pedantic`/`fmt --check`/`cargo dev-test` (full suite, default parallelism) all pass cleanly at HEAD `d306e989`.
- CI's own `build` job (fmt+clippy+full test suite) **already passed once, cleanly**, on this exact HEAD (attempt 1, `build` job id `106175625591`/attempt2 `106176598584`, conclusion `success`, 6m29s) before the full-workflow rerun (my own operational mistake — should have used `--failed` scoping consistently) discarded that clean state by re-triggering `build` unnecessarily.
- P-018 Copilot-review gate: **SATISFIED**.
- Local review readiness re-confirmed: **READY** (scoped Correctness re-review of the delta).

A 6th rerun (full workflow) is in flight as of this checkpoint (triggered ~17:36 local). This is a monitoring/waiting posture, not a further code action.

## Remaining steps (unchanged from before, plus this blocker)

1. **Wait for CI to actually complete a clean, non-hung run** at HEAD `d306e989` (both `build` and `start-launcher-windows` green). If `start-launcher-windows` fails again on its own **legitimate** 8s-budget timing assertion (not a hang) once runner load normalizes, that specific failure is pre-existing/unrelated flakiness (git-blame confirmed: `start.ps1`, `tests/contract/start_launcher_test.rs` are untouched by any of 141-S's 7 tasks or the 6 remediation commits) and should be retried via `--failed` rerun rather than treated as a regression.
2. Update the PR body's `## Local Review Readiness` block: reviewed HEAD → `d306e989473420f47d901a236ea6287acb92c1f6`, outcome → `READY` (upgraded from `READY_WITH_FOLLOWUPS`; the only two prior follow-up-worthy findings from the Copilot round are now fixed, not follow-ups — the original 3 pre-existing-gap stash entries `6C5DF765`/`9B7EC1E4`/`4628001C` remain as legitimate deferred follow-ups from the first review round).
3. Re-run the full §1.9 local-readiness gate checklist once the body is updated.
4. Verify P-009 merge-commit-only strategy is configured for this repo (not yet checked this session).
5. Present the PR to the operator and wait for explicit merge approval (P-014) — do not auto-merge, do not use `--admin` to bypass missing/pending CI checks (explicitly forbidden by P-018's admin-fallback exclusion list).
6. After merge: Merge Confirmation Gate → Step 6 post-merge closure (`post-merge/141-s-*` branch, operational-closure, mandatory compact-context, backlog index resync, source-artifact cleanup, standard safe-close of shipment `141-S` — task-only manifest, NOT the P-015 cascade `ship` path).

## Constraints preserved

- Never touched `142-S` or the `0011C2DC` stash/spike branch (`docs/stage-0011c2dc-content-record-schema`).
- Never merged without explicit operator approval.
- Never used `--admin` to bypass required checks.
- All P-021 deferred findings remain single-write (no edits to stash entries `6C5DF765`, `9B7EC1E4`, `4628001C` — only cited in replies/residual-risk records).
