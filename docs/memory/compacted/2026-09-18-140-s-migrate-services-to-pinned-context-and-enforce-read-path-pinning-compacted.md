---
title: "140-S migrate services to pinned context and enforce read-path pinning — compacted memory"
description: "Consolidated summary of implementation, review, readiness-audit correction, and closure for shipment 140-S. Verbose originals archived under docs/archive/memory/2026-09-18/."
---

# 140-S — compacted memory

**Shipment**: 140-S (feature 142-F) — migrate services to pinned context and enforce
read-path pinning. **Branch**:
`feat/140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning`.
**PR**: #404, merged as merge commit `eb1416cdb007c2f389f5856569f4c2ee7a982ddf` on
2026-09-18T23:52:07Z. **Final status**: shipment manually safe-closed
(`archived_status: done`); `142-F` remains active, byte-for-byte unchanged
(SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`,
unchanged since the `138-S`/`139-S` closures).

## Manifest and files modified

Tasks: `142.040-T`–`142.046-T` (search/embedding, registry/evaluation, retrieval-eval,
metrics/query-stat, DAX lint, git graph services, plus a static read-path-pinning
enforcement guard). Owned files:
`src/services/{search,registry,retrieval_eval,metrics,dax_lint,git_graph}.rs`,
`src/tools/{eval,lint,write}.rs`, `src/server/state.rs`,
`tests/contract/read_path_pinning_enforcement_test.rs`, and 6 corresponding
integration test files under `tests/integration/`.

## Key decisions and fixes

* Each migrated service now consumes a caller-pinned `ReadRequestContext` instead of
  resolving storage/workspace state itself. `142.046-T`'s static guard test enforces
  the migration against regression.
* **In-scope cross-task regression wave 1** (fixed, `3227a68b`): the closing guard
  went RED (2/4 subtests) against the fully-landed manifest, revealing `dax_lint.rs`
  and `eval.rs` had left `workspace_root`/`.engram` construction outside the approved
  pinning seam. Fixed within the same shipment (P-021 C1: same contract surface as the
  guard's own acceptance criterion).
* **In-scope cross-task regression wave 2** (fixed, `31e4d3a6` + `eda83c03`):
  full-suite run surfaced `contract_lint_dax` and `report_read_generation_pin`
  failures. Root cause 1: `dax_lint.rs`/`registry.rs` derived "workspace root" by
  guessing from `data_dir` (wrong — `data_dir` and workspace `path` are independently
  configurable). Fixed via new `ReadRequestContext::root_path()` accessor + shared
  `registry::registry_path_for()` helper. Root cause 2: a pre-existing test-fixture
  bug (`MetricsFixture.data_dir` mismatched the seeded path) — fixed the fixture, not
  production code (an initial attempt to "fix" `metrics.rs` itself was correctly
  reverted after confirming the migrated read contract mandated `data_dir()`
  resolution).
* **Deferred, not fixed** (materially larger than single-owned-file scope; all
  captured via P-021 defer-capture):
  * `E6CA4ED1` (high) — metrics writer (`workspace_path`) vs. migrated reader
    (`context.data_dir()`) divergence under a configured `ENGRAM_DATA_DIR`. A direct
    fix was attempted and reverted after it reproduced 2 regressions in
    `usage_telemetry_emit` tests (ambient `ENGRAM_DATA_DIR` in this dev/CI shell) —
    the safe fix requires threading a single pinned `data_dir` through the entire
    background-writer subsystem, out of scope for this task.
  * `7C23A682` (high) — identical divergence pattern in `eval.rs`/retrieval-eval
    (writer: `workspace_root/.engram/eval`; reader: `context.data_dir()`).
  * `9BB01D31` (medium) — DAX lint has no Generation-mode `root_path()` support;
    confirmed genuinely pre-existing (pre-migration code was Managed-mode-only too).
  * `A3E0E607` (medium) — the static guard's substring-matching methodology misses
    aliased variable names and omits `registry.rs` from its scope.
  * `DB0661A6` (high) — `metrics.rs` joins caller-supplied `branch_name`/`compare_to`
    into a filesystem path with no sanitization; confirmed genuinely pre-existing via
    `git show main:src/tools/read.rs` (140-S only relocated the read call, did not
    introduce the missing sanitization).
  * `10EE5E43` (medium) — `archive_verifier_runs_the_unpacked_native_binary`
    intermittent/environmental MCP-stdio flake; disjoint from 140-S's diff.
  * `95D6C74C` (low) — a 4th-pass Copilot finding (post corrective-commit HEAD) that
    the operational-closure doc's Monitoring plan section lacked explicit
    baseline/threshold values; deferred per the session's one-commit
    anti-tail-chasing invariant rather than triggering a second commit.

## Readiness-audit correction (post-initial-merge-approval, pre-merge, single corrective commit)

A resumed session identified that the PR's mutable Local Review Readiness evidence
had gone stale relative to HEAD, and that several committed docs misstated path/
follow-up contracts. Each disputed claim was independently validated against
`origin/main` (not merely trusted from prior Copilot classification):

* `E6CA4ED1`/`7C23A682` (writer/reader `data_dir` divergence): **confirmed real**,
  correctly deferred (fix would be subsystem-wide, out of single-task scope).
* `DB0661A6` (unsanitized `branch_name`/`compare_to`): **confirmed genuinely
  pre-existing**, correctly deferred.
* `9BB01D31` (dax_lint Generation-mode gap): **confirmed genuinely pre-existing**,
  correctly deferred.
* Genuine **documentation misstatements** found and corrected in the single
  corrective commit (`18944fb1809926d3648fbbde7cc35be92f8deca5`): `registry.rs`'s
  docstring wrongly claimed a pinned-data-directory read (actually
  `context.root_path()`); the retrieval-eval memory doc wrongly claimed the reader
  preserved the workspace-path contract; the operational-closure doc had a stale HEAD/
  thread-count and omitted `DB0661A6` from its follow-up list; the shipment execution
  memory doc called an earlier commit "final" despite the commit containing that claim
  being itself later (corrected to commit-relative/historical language, not
  recursively chased).
* Applied the operator's anti-tail-chasing invariant: exactly one corrective content
  commit; a NEW Copilot finding raised against that corrective HEAD (`95D6C74C`,
  monitoring-plan completeness) was handled via defer-capture (reply + resolve), never
  a second commit. Further HEAD-tracking lived only in the PR's mutable body.

## Verification (final, re-confirmed at post-merge closure)

* `cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo clippy
  --all-targets -- -D warnings -D clippy::pedantic`: all PASS at final HEAD
  `18944fb1`.
* All 7 manifest tasks' own harnesses plus the static guard: all green.
* Full `cargo test --all-targets --no-fail-fast`: 180/181 test binaries PASS at final
  HEAD; the 1 exception (`archive_verifier_runs_the_unpacked_native_binary`) is the
  already-deferred, disjoint `10EE5E43` flake (confirmed consistent, not new).
* CI (hosted): `build` and `start-launcher-windows` both SUCCESS at final HEAD
  `18944fb1` on first run (no reruns needed).
* 4 rounds of Copilot review (10 threads total), all replied-to and resolved; P-018
  gate `SATISFIED` at final HEAD.
* P-009 (merge-commit-only), P-016/pipeline-topology (lifecycle phase), and P-014/
  §1.9 readiness all independently re-verified PASS at final HEAD immediately before
  merge.

## Merge / approval basis

Operator approval ("PR 404: Merge approved", 2026-09-18T23:09:00Z) predated the
corrective commit. Ship independently re-verified, at the new HEAD, all conditions of
the dark-mode carve-out (`merge_approval_pre_authorized=true`, scope match, §1.9 PASS,
CI green, P-009/P-016 PASS) before proceeding, recording an explicit
`DARK_MODE_MERGE_AUTHORIZED` audit line rather than treating the pre-correction chat
approval as automatically transferable without re-verification. Admin fallback was
never used or needed.

## Closure references

* Operational closure: `docs/closure/2026-09-18-140-s-operational-closure.md`
  (final status: `READY` post-merge; releasability evidence fully satisfied).
* Runtime verification: `docs/closure/2026-09-18-140-s-runtime-verification.md`
  (verdict: `PASS_WITH_FOLLOW_UP`).
* Reconciliation: `.backlogit/reconcile/140-S-pre-20260918T235400Z.md`,
  `.backlogit/reconcile/140-S-post-20260918T235900Z.md` (both `PROCEED`).
* Manual safe-close archive: `.backlogit/archive/140-S.md` (full AUDIT RATIONALE,
  7th consecutive shipment to reuse the `backlogit shipment ship` non-termination
  workaround against covering feature `142-F`).
* Compound addendum: `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`
  (2026-09-18 addendum).

## Follow-ups (all stashed, none blocking, all await Stage triage)

`E6CA4ED1`, `7C23A682`, `9BB01D31`, `A3E0E607`, `DB0661A6`, `10EE5E43` (reused from
prior sessions, confirmed disjoint), `95D6C74C` (new, this session's readiness-audit
segment). No top-level feature/chore in this shipment's scope, so source-artifact
cleanup (Step 6 item 7) produced zero candidates.

## Archived originals

Verbose originals preserved at
`docs/archive/memory/2026-09-18/140-s-shipment-execution-memory.md` (24 KB, full
implementation + PR + readiness-audit narrative),
`docs/archive/memory/2026-09-18/retrieval-eval-service-pin-memory.md`, and
`docs/archive/memory/2026-09-18/142-045-t-git-graph-pinned-context-memory.md`.
