---
title: 140-S Migrate Services to Pinned Context and Enforce Read-Path Pinning — Operational Closure
description: Releasability evidence per .autoharness/workspace-profile.yaml runtime_validation.releasability for PR #404 (140-S), pre-merge.
---

## Mode

`pre-merge` — awaiting explicit operator merge approval (P-014). Merge
approval and admin fallback are **not pre-authorized** for this dark-mode run
(`merge_approval_pre_authorized=false`, `admin_fallback_pre_authorized=false`).

## Summary of change

140-S migrates 6 read services (`search`, `registry`, `retrieval_eval`,
`metrics`, `dax_lint`, `git_graph`) to consume a caller-pinned
`ReadRequestContext` instead of resolving storage/workspace state
themselves, and adds a static guard test (`142.046-T`,
`contract_read_path_pinning_enforcement`) enforcing that migration against
regression. Two cross-task regressions surfaced during implementation were
fixed in-scope: (1) `contract_lint_dax`/registry root-path derivation
(`31e4d3a6`), (2) a stale `report_read_generation_pin` metrics fixture
(`eda83c03`). One P-021-eligible review-thread fix (a missing
`recommendation:` frontmatter field on this shipment's own reconciliation
report) was applied directly (`adf2d274`).

## CI status and unresolved review items

**Corrected 2026-09-18 (readiness audit)**: this section previously cited
stale evidence pinned to HEAD `adf2d274` (Copilot pass 1 only) after two
further content commits and two further Copilot passes had already landed.
The status below reflects the last HEAD examined by this audit,
`afb705ab`; this record does not track HEAD advances that occur after the
commit containing it — the PR's mutable `## Local Review Readiness` block
is the authoritative current-HEAD source.

- Hosted CI (PR #404, HEAD `afb705ab`): `build` **PASS**,
  `start-launcher-windows` **PASS** (required reruns across several pushed
  HEADs; pre-existing hosted-runner timing flake, stash `F58ECAA8`).
- Local adversarial review: `READY_WITH_FOLLOWUPS`, `P0=0, P1=0` blocking.
- GitHub-hosted Copilot review: engaged automatically across 3 review
  passes (re-arming per push), 9 threads total, all replied-to and
  resolved as of `afb705ab`. P-018 gate
  (`autoharness gate copilot-review 404 ...`) verdict: **`SATISFIED`** for
  HEAD `afb705ab`, 0 unresolved threads at that HEAD.
- A 3rd Copilot pass, submitted after `afb705ab` was pushed, flagged that
  this closure record and the shipment execution memory checkpoint had not
  been refreshed after that push, and that the follow-up stash list below
  omitted `DB0661A6`. Both are corrected in this same audit pass; see the
  follow-up row below for the corrected list.
- No unresolved review items remain open on this PR as of this audit.

## Runtime verification report

`docs/closure/2026-09-18-140-s-runtime-verification.md` — Verdict:
**`PASS_WITH_FOLLOW_UP`**.

## Validator evidence (structured handoff)

- **Surfaces exercised**: `cli` (version + daemon-status probes, both
  green/healthy), `api`/MCP (full contract-test suite green in canonical
  debug-mode gate; a release-profile-only timing sensitivity in 2/19
  pre-existing, unmodified shim tests was observed and confirmed unrelated
  to 140-S), `background-job` (all 6 service-migration harnesses plus the
  static guard, all green).
- **Manual checkpoints**: none declared for these surfaces beyond the
  automated probes.
- **Blocked prerequisites**: none.
- **Verdict**: `PASS_WITH_FOLLOW_UP`.

## Invariants to preserve

- Managed-mode read-service behavior is unchanged (explicit acceptance
  criterion on every one of the 6 migrated services).
- No migrated service calls `connect_db` or derives a database/workspace
  path independently of its caller's pinned context (enforced by
  `142.046-T`'s static guard, within its documented detection scope — see
  deferred stash `A3E0E607` for known coverage gaps in that guard itself).
- `data_dir` and workspace root (`path`) remain independently configurable
  (`ENGRAM_DATA_DIR` override) — this is a pre-existing, intentional design
  property, not something 140-S may quietly collapse.

## Pre-deploy audits

- No schema migrations, feature flags, or config changes are introduced by
  this shipment.
- No new environment variables or rollout prerequisites.
- Merge-only release path (no separate deploy step for this workspace-local
  plugin architecture); the next tagged release build will pick up this
  change per the existing `cargo-release` process.

## Deployment / rollout path

Merge-only. This is a workspace-local Rust binary; end users receive the
change via the next tagged release (`cargo-release` + `git-cliff`), not a
live deployment.

## Post-deploy checks

- `engram --version` reports the new release tag's embedded SHA.
- `engram daemon-status` reports `overall: green` (or `yellow` only for
  expected fresh-daemon transient states) after a normal bind.
- The 6 migrated MCP tools (`unified_search`, tools backed by `registry`,
  `run_retrieval_eval`/`get_retrieval_eval_report`, metrics-backed report
  tools, `lint_dax`, `query_changes`/git-graph tools) continue to return
  correct results in both Managed mode (primary, verified) and
  Generation/ReadServer mode where already supported pre-shipment (DAX lint
  Generation-mode support was never present; see stash `9BB01D31`).

## Risky action record

No `ProposedAction`/`ActionRisk` entries — this shipment made no destructive,
high-blast-radius, or irreversible changes. The one investigated-and-reverted
production fix attempt (`metrics.rs::resolve_usage_path`) was fully reverted
via `git checkout --` before commit, leaving zero net risk in the shipped
diff.

## Healthy signals

- `cargo test --all-targets --no-fail-fast` stays green (538 binaries).
- `engram daemon-status` and `engram health` report green/healthy.
- No increase in `WorkspaceError::NotSet` or path-resolution error rates in
  daemon logs for the 6 migrated services under Managed-mode usage.

## Failure signals

- A migrated service (`search`, `registry`, `retrieval_eval`, `metrics`,
  `dax_lint`, `git_graph`) begins returning `WorkspaceError::NotSet` or
  stale/empty results under normal Managed-mode operation.
- `contract_read_path_pinning_enforcement` fails on a future PR (signals a
  new, unclassified live-root/`.engram`/`connect_db` read crept back onto
  the read path).
- A configured `ENGRAM_DATA_DIR` workspace silently loses visibility into
  previously-written metrics or retrieval-eval reports (the exact deferred
  `E6CA4ED1`/`7C23A682` failure mode) — this is a KNOWN, documented,
  deferred edge case, not a new regression, but should be watched for
  production reports if `ENGRAM_DATA_DIR` usage is common.

## Monitoring plan

Standard daemon log observation per `docs/log-observation-guide.md`; no
additional monitoring infrastructure is required for this shipment beyond
what already exists. Watch for `WorkspaceError::NotSet` spikes attributable
to the 6 migrated services specifically.

## Rollback trigger

Any healthy-signal regression above, or a user report of missing/incorrect
search, registry, retrieval-eval, metrics, DAX-lint, or git-graph results
under normal (non-`ENGRAM_DATA_DIR`-configured) Managed-mode operation.

## Rollback procedure

Standard workspace-local rollback: reinstall the prior GitHub Release
asset/tag and flush regenerated `.engram/` state, per the workspace
profile's documented `rollback-procedure` releasability requirement. No
database migration accompanies this shipment, so no down-migration is
needed.

## Validation window

Standard: through the next tagged release plus 48 hours of ordinary
workspace usage across the migrated services (search, registry,
retrieval-eval, metrics, DAX lint, git graph).

## Owner

Repository maintainer (release owner) accountable during the observation
window; no shipment-specific owner override.

## Compaction status (P-020)

`pending` — Ship's mandatory `compact-context` invocation happens at
post-merge closure (Step 6), which has not yet run. This shipment is still
`pre-merge`, awaiting explicit operator approval.

## Releasability evidence

| Required evidence | Status |
|---|---|
| healthy-signal | **Satisfied** — CLI version/daemon-status probes green; full test suite green. |
| failure-signal | **Satisfied** — named above. |
| monitoring-plan | **Satisfied** — standard daemon log observation, no shipment-specific additions needed. |
| rollback-trigger | **Satisfied** — named above. |
| rollback-procedure | **Satisfied** — standard GitHub Release reinstall + `.engram/` flush; no migration to reverse. |
| owner | **Satisfied** — repository maintainer / release owner. |
| validation-window | **Satisfied** — through next tagged release + 48h. |
| follow-up (optional) | **Satisfied** — **corrected 2026-09-18 (readiness audit)**: 6 stash entries newly captured during 140-S (`E6CA4ED1`, `10EE5E43`, `9BB01D31`, `A3E0E607`, `7C23A682`, `DB0661A6` — the last of these, a path-traversal-shaped finding on `branch_name`/`compare_to`, was previously omitted from this row) plus 2 entries reused from a prior shipment (`F58ECAA8`, `DA0AF326`); none block this PR's own scope. |

**Overall status: `READY_WITH_CONDITIONS`**

Condition: explicit operator merge approval is required before this PR may
merge (P-014; `merge_approval_pre_authorized=false` for this dark-mode run).
All other releasability evidence is fully satisfied — this is a process
gate, not an unmet technical condition.

## Source artifact cleanup

Not yet performed — this section is filled in by Ship's post-merge closure
(Step 6, item 7), after operator-approved merge. Placeholder per the
`operational-closure` skill contract:

- Archived stash (`source_stash_id`): pending post-merge closure.
- Archived deliberations (`source_deliberation_id`): pending post-merge closure.
- Skipped (already archived or not found): pending post-merge closure.
