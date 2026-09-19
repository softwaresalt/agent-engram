---
title: 140-S Migrate Services to Pinned Context and Enforce Read-Path Pinning — Operational Closure
description: Releasability evidence per .autoharness/workspace-profile.yaml runtime_validation.releasability for PR #404 (140-S), pre-merge.
---

## Mode

`post-merge` — **updated 2026-09-18** (post-merge closure pass). PR #404
merged via merge commit `eb1416cdb007c2f389f5856569f4c2ee7a982ddf` at
2026-09-18T23:52:07Z, final reviewed HEAD `18944fb1809926d3648fbbde7cc35be92f8deca5`.
For this session's re-resumption, `merge_approval_pre_authorized=true` and
`admin_fallback_pre_authorized=false` per the operator's dark-mode
activation record (scope: shipment 140-S / PR #404 only); admin fallback
was never used — the merge succeeded via the normal merge path.

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

**Final, as merged 2026-09-18**: this section is now historical —
describing state up through and including the final reviewed/merged HEAD
`18944fb1`. No further HEAD changes occurred after that commit; it was
the sole corrective commit made during this session's readiness-audit
cycle, per the operator's anti-tail-chasing directive (at most one
corrective content commit; further findings handled via defer-capture,
never a second commit).

- Hosted CI (PR #404, final HEAD `18944fb1`): `build` **PASS**
  (8m0s), `start-launcher-windows` **PASS** (4m25s) — green on first run
  at this HEAD, no reruns required.
- Local adversarial review: `READY_WITH_FOLLOWUPS`, `P0=0, P1=0` blocking,
  re-run and confirmed current for HEAD `18944fb1`.
- GitHub-hosted Copilot review: engaged automatically across 4 review
  passes total (re-arming per push — 3 passes against the pre-audit HEAD
  progression, plus 1 additional pass against the corrective HEAD
  `18944fb1` itself), 10 threads total, all replied-to and resolved as of
  `18944fb1`. P-018 gate (`autoharness gate copilot-review 404 ...`)
  verdict: **`SATISFIED`** for HEAD `18944fb1`, 0 unresolved threads.
- The 3rd Copilot pass (against the pre-audit HEAD progression) flagged
  that this closure record and the shipment execution memory checkpoint
  had gone stale, and that the follow-up stash list below omitted
  `DB0661A6`. Both were corrected in the single corrective commit
  (`18944fb1`).
- The 4th Copilot pass (against corrective HEAD `18944fb1` itself) flagged
  a monitoring-plan completeness gap (missing explicit baseline/threshold
  values in the Monitoring plan section below). Per the operator's
  one-commit anti-tail-chasing invariant, this was **not** fixed via a
  second commit — it was captured as P-021 deferred-scope-expansion stash
  entry `95D6C74C` (kind: task, priority: low), replied-to on the review
  thread citing that entry, and the thread resolved. See the follow-up
  row below.
- No unresolved review items remain open on this PR as of merge. All 10
  threads across all 4 passes are resolved.

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

**Follow-up (stash `95D6C74C`)**: a 4th-pass Copilot review flagged that
this Monitoring plan section lacks explicit baseline/threshold values
(e.g., a numeric rate or count that would trigger investigation). This is
a genuine documentation-completeness gap, deferred per P-021 (out of scope
for the single corrective commit already made in this session) rather
than fixed here. Deferred to Stage for triage/prioritization; not required
to block this shipment's own release.

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

`done` — mandatory `compact-context` invocation (`target: all`) completed
during this post-merge closure pass (Step 6, item 8). The 140-S release
unit's 3 memory files (24 KB + 2 smaller task memos, all eligible under
the "completed feature or chore" candidate rule) were consolidated into
`docs/memory/compacted/2026-09-18-140-s-migrate-services-to-pinned-context-and-enforce-read-path-pinning-compacted.md`;
verbose originals moved to `docs/archive/memory/2026-09-18/` (traceable,
not deleted). No plans or other closure artifacts qualified as compaction
candidates this pass (below file-count/age thresholds beyond the
just-closed release unit itself).

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
| follow-up (optional) | **Satisfied** — 7 stash entries total: 6 captured during the initial implementation/review cycle (`E6CA4ED1`, `10EE5E43`, `9BB01D31`, `A3E0E607`, `7C23A682`, `DB0661A6`) plus 1 captured during this session's readiness-audit/post-corrective-commit cycle (`95D6C74C`, monitoring-plan completeness gap), plus 2 entries reused from a prior shipment (`F58ECAA8`, `DA0AF326`); none block this PR's own scope. |

**Overall status: `READY_WITH_CONDITIONS` at time of merge; now `READY` post-merge** — merge
completed via merge commit, all conditions were satisfied at merge time
(P-014 approval reasoning below), and post-merge closure (this document)
is now complete.

### P-014 approval-basis note (this session's re-resumption)

The operator's chat approval ("PR 404: Merge approved",
2026-09-18T23:09:00Z) was given for the pre-correction HEAD `afb705ab`,
before this session's single corrective commit (`18944fb1`) changed HEAD.
Ship independently re-verified, at the new HEAD, all conditions of the
dark-mode activation-record carve-out for treating
`merge_approval_pre_authorized=true` as sufficient without soliciting a
literal re-approval message: scope match (140-S/PR #404 only) confirmed;
§1.9 readiness gate re-run and PASS at `18944fb1`; hosted CI green at
`18944fb1`; P-009 (merge-commit-only strategy) and P-016 (no prohibited
parallel worktree) both confirmed PASS. All conditions being independently
satisfied at the new HEAD, Ship proceeded to merge rather than halting to
solicit a second literal approval message, and recorded an explicit
`DARK_MODE_MERGE_AUTHORIZED` audit line before invoking `gh pr merge`.

## Source artifact cleanup

Performed during post-merge closure (Step 6, item 7). This shipment's
manifest is task-only (7 tasks: `142.040-T`–`142.046-T`); no feature or
chore is a top-level shipped item in this shipment's scope (the shared
covering feature `142-F` remains `active` with substantial roster scope
outside this shipment and is explicitly not touched by this closure — see
the AUDIT RATIONALE in `.backlogit/archive/140-S.md`). Per Step 6 item 7,
source-artifact cleanup applies to "each shipped top-level item in scope
(feature or chore)"; since none exists in this shipment's manifest, there
are zero candidates to process:

- Archived stash (`source_stash_id`): 0 candidates (no top-level
  feature/chore in scope).
- Archived deliberations (`source_deliberation_id`): 0 candidates (no
  top-level feature/chore in scope).
- Skipped (already archived or not found): none applicable.

This is consistent with the identical task-only-manifest precedent
established at the `137-S`, `138-S`, and `139-S` closures against the same
`142-F` covering feature.
