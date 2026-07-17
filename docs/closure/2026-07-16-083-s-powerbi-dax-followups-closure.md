---
title: "Operational Closure - 083-S PowerBI/DAX PR#246 review-deferred followups"
doc_type: closure
source: "083-S shipment (feature 087-F; tasks 087.001-T..087.004-T)"
description: >-
  Post-merge closure for shipment 083-S. Records the PowerBI/DAX follow-up work
  deferred from PR #246, one pre-PR cross-model adversarial review, four Copilot
  review cycles, the collector-versus-sweep symlink-safety hardening, deferred
  follow-ups, backlog archival state, and rollback posture after PR #257 merged.
topic: "Harden DAX linting, PowerBI impact docs, and symlink-safe source traversal"
depth: "closure"
decision_status: "SHIPPED - merged to main as merge commit e0f8e440 via PR #257"
author: ship
date: 2026-07-16
verdict: SHIPPED
pr: 257
merge_commit: e0f8e440
target_commit: 4325273
branch: feat/083-powerbi-dax-followups
scope: "PowerBI/DAX follow-ups plus symlink-cycle-safe centralized source traversal and deletion sweeps"
reviewers:
  - gpt-5.6-sol
  - gemini-3.1-pro-preview
  - gpt-5.6-terra
  - gemini-3.5-flash
  - copilot
linked_artifacts:
  - "083-S"
  - "087-F"
  - "087.001-T"
  - "087.002-T"
  - "087.003-T"
  - "087.004-T"
  - "087.005-T"
  - "087.006-T"
  - "PR #257"
  - "PR #246"
---

## Summary

Shipment 083-S completes the follow-up work that PR #246 deferred for the PowerBI and DAX
indexing surfaces. It hardens DAX lint fingerprinting and comment handling, documents PowerBI
impact analysis, and — most substantially — makes source-tree traversal and the complementary
deletion sweeps symlink-cycle-safe and workspace-contained. 083-S merged as PR #257 with merge
commit `e0f8e440`.

## What shipped

Strict TDD governed the release unit: tests were added or extended before the implementation they
gated.

| Task | Delivered |
|---|---|
| `087.001-T` | A version namespace (`TMDL_DAX_INDEX_VERSION`) is folded into the persisted `.tmdl` content hash, so bumping it invalidates the incremental hash-skip and forces a one-time re-index of unchanged Power BI files after a DAX-capable upgrade (no `--force` or file edit needed) |
| `087.002-T` | DAX `--` line comments are tokenized so references and division inside comments no longer produce findings |
| `087.003-T` | `impact_analysis` PowerBI behavior documented |
| `087.004-T` | `collect_recursive` is symlink-cycle-safe: directory symlinks are followed only when their canonical target stays under the workspace root, and canonical directory visits are tracked to prevent cycles and alias duplication |

The traversal work was centralized into `src/services/source_traversal.rs` and shared by the
PowerBI, PBIP, notebook, and backlog indexers, replacing four independent per-indexer traversals.

## Adversarial and Copilot review

Per the operator directive to minimize Copilot iterations, a cross-model adversarial review ran
before the PR opened (rust `gpt-5.6-sol`, security `gemini-3.1-pro-preview`, scope `gpt-5.6-terra`,
follow-up `gemini-3.5-flash`). It fixed three P1 findings pre-PR and deferred one P2
(local concurrent-read / TOCTOU hardening).

Four Copilot review cycles then ran. Every surfaced finding was treated as genuine or
conservatively addressed. The symlink-safety fixes all fail closed: they can drop a stale record
or skip an untrusted path, but they never index or retain content outside the workspace root.

* **Cycle 1 (4 findings):** the collectors were made symlink-safe in 087.004-T, but the
  complementary deletion sweeps still followed symlinks (collector-versus-sweep asymmetry), so
  stale content records and graph nodes could survive. Fixed in commit `5154f39` with a shared
  `is_regular_file_in_workspace` helper (`symlink_metadata` no-follow, `is_file`, then canonicalize
  within root) applied to all four sweeps; backlog `compute_deleted_paths` was threaded with the
  workspace root.
* **Cycle 2 (6 findings):** the PowerBI and notebook `workspace_relative_path` validators accepted
  Windows root-relative paths (`\foo`) because `is_absolute()` needs a drive prefix, so the sweep
  could probe outside the workspace; the backlog `compute_deleted_paths` probed absolute and `..`
  paths with no pre-validation. Fixed in commit `f2fc3f0`: `has_root()` was added to the PowerBI and
  notebook validators, and backlog `compute_deleted_paths` was rewritten as a `filter_map` guarded
  by a new relative-only `workspace_relative_path` validator (backlog nodes store relative
  `file_path`, so relative-only is the correct production contract). A new `S-BI-10` regression test
  covers escape-path skipping, and three duplicate test-ID doc comments were deconflicted.
* **Cycle 3 (3 substantive findings):** the centralized traversal silently dropped unreadable
  directories, losing the warning the monitoring plan relies on — fixed in commit `71d5b5c`
  (`collect_recursive` now warns with the directory and error before returning on a `read_dir`
  failure). The other two are genuine but architectural and were deferred (see below).
* **Cycle 4:** clean pass. Copilot reviewed 18 of 18 changed files at HEAD `4325273` and generated
  no new comments.

## Verification

Quality gates were green for the merged work at each cycle: `cargo fmt --all -- --check`, clippy
with `-D warnings -D clippy::pedantic`, and the affected test binaries
(`unit_backlog_indexer`, `integration_powerbi_search_ingestion`, `integration_pbip_search_ingestion`,
and the notebook and DAX lib tests). The backlog, PowerBI, PBIP, and notebook sweep suites all
passed, including symlink assertions that execute on the local Windows environment where the test
process holds symlink-creation privilege.

The GitHub Actions `build` check was green at HEAD `4325273`.

The merge gate was satisfied before PR #257 merged:

* Copilot review `commit_id == HEAD` for `4325273`
* Copilot removed from `requested_reviewers`
* 0 unresolved review threads
* `mergeable_state == clean`

## Deferred follow-ups

Two Copilot cycle-3 findings were genuine but represent architectural changes to
deletion and persistence semantics. Landing either safely requires broader verification than a late
review cycle allows, and the change was judged unsafe to rush during an unattended (operator-AFK)
autonomous run. Both were filed as backlog follow-ups and remain queued; neither is a known
false-edge or data-exposure path.

| Item | Status | Follow-up |
|---|---|---|
| `087.005-T` | queued | Reconcile deletion sweeps against the set of paths actually collected each pass so a directory-symlink alias cannot leave a stale, alias-backed record |
| `087.006-T` | queued | Persist PowerBI TMDL content records atomically (or gate the hash-skip on a completion marker) so a partial write cannot make a file look unchanged and permanently skip missing summaries |

`087.006-T` is the same TOCTOU / atomicity concern the pre-PR adversarial review deferred as P2;
Copilot independently surfaced it, confirming the deferral rather than expanding scope.

## Operational notes and rollback

### Invariants to preserve

* Collectors and deletion sweeps agree on symlink semantics: a final-component file symlink is not
  live, and a directory symlink is followed only when its canonical target stays under the workspace
  root.
* Deletion sweeps never probe the filesystem outside the workspace root: absolute, root-relative,
  `..`, and drive-prefix paths are rejected before any probe.
* The traversal and sweep behavior is additive to indexing correctness: it removes stale records and
  contains traversal. It does not add or drop in-workspace content, though canonical-directory dedup
  can change which alias path is emitted for a symlink-aliased file (the known path-selection
  behavior tracked by deferred `087.005-T`).

### Pre-deploy audit

* Feature flags: none.
* Data migration: none.
* Schema note: no `SCHEMA_VERSION` change in this shipment.
* Review gate: one pre-PR cross-model adversarial review plus four Copilot cycles, with the final
  Copilot review bound to HEAD `4325273`.
* Backlog gate: 083-S, 087-F, and 087.001-T..087.004-T are closed and archived; 087.005-T and
  087.006-T remain queued follow-ups.

### Deployment or rollout path

The rollout path is merge-only for the repository: PR #257 landed on `main` through a merge commit
(`e0f8e440`). Runtime absorption occurs when users run a binary containing that merge and index a
workspace, which re-derives PowerBI, PBIP, notebook, and backlog records through the centralized,
symlink-safe traversal and sweeps.

### Post-deploy checks

* Index a workspace containing symlinked source directories and confirm no content is indexed from a
  target whose canonical path escapes the workspace root.
* Remove or retarget a symlinked source file and confirm the deletion sweep retires its record.
* Spot-check logs for the new "skipping unreadable directory during source traversal" warning when a
  directory is unreadable.

### Healthy signals

* PowerBI, PBIP, notebook, and backlog ingestion suites remain green in CI and locally.
* Deletion sweeps retire records for removed or symlink-retargeted files and retain regular
  in-workspace files.
* No content records reference paths outside the workspace root.

### Failure signals

* A record retained for a file whose symlink now resolves outside the workspace.
* A deletion sweep probing or acting on an absolute or `..` path.
* An unreadable directory silently producing an empty subtree with no warning.

### Monitoring plan

Manual and CI observation are the monitoring surfaces for this local-first daemon. The primary SLIs
are the PowerBI, PBIP, notebook, and backlog ingestion suite pass rates, deletion-sweep correctness
on symlink fixtures, and the presence of skipped-source warnings for unreadable directories.
Baseline is the green PR #257 suite. The threshold to investigate is any record referencing a path
outside the workspace root, any sweep probe outside the root, or a deterministic failure in the
ingestion suites.

### Rollback trigger and procedure

* Trigger: a record retained or created for content outside the workspace root, a deletion sweep
  acting on an untrusted path, or a deterministic ingestion-suite regression traced to traversal.
* Procedure: revert PR #257 with a merge-commit revert of `e0f8e440`, rebuild, and ship the reverted
  binary. No data migration or manual database rewrite is required because the change affects
  traversal and sweep behavior, which are re-derived on the next index.

### Risky action record

* **ProposedAction:** merge the centralized symlink-safe traversal and sweep hardening after
  adversarial and Copilot review, deferring two architectural findings.
* **ActionRisk:** moderate. The change affects shared traversal and deletion behavior across four
  indexers but fails closed and is re-derived on index.
* **ActionResult:** applied. PR #257 merged as `e0f8e440` after the four-point Copilot merge gate
  passed at HEAD `4325273`.
* **Approval path:** operator granted PR and merge authority for this unattended run; the
  orchestrator retained the final closure PR merge gate.

### Validation window and owner

* Owner: ship and repository maintainer.
* Window: next 7 days or next 3 CI runs touching PowerBI/DAX or source traversal, whichever gives
  earlier confidence, plus any first index run on a workspace containing symlinked sources.
* Closure status: **READY WITH FOLLOW-UP**. The feature is shipped and validated; the two deferred
  follow-ups are stale-record and durability improvements that do not open a false-edge or
  data-exposure path.
