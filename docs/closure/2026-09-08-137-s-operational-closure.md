---
title: 137-S Candidate Indexing, Direct-Sync Boundary and Supervisor Crate Separation — Operational Closure
description: Post-merge release-readiness, monitoring, rollback, and follow-up record for shipment 137-S.
---

## Releasability

**Status: READY WITH CONDITIONS**

PR #388 merged via merge commit `ef0135bf05aba5d7329a7306eb78bb9b18e02f69`
(merge method: `gh pr merge 388 --merge`, operator-approved verbatim: *"PR
388: Merge approved"*, for the exact reviewed HEAD
`653e973e201883790a9862e6ff54ea84efbb902d`). Immediately pre-merge: PR open,
clean, mergeable; CI build + `start-launcher-windows` SUCCESS; P-018
copilot-review gate SATISFIED for the exact HEAD; repository settings allow
merge-commit only (squash/rebase disabled, P-009 satisfied).

The condition below mirrors the identical, already-established precedent
from 135-S's post-merge closure (`docs/closure/2026-09-05-135-s-runtime-verification.md`
"Post-merge re-run addendum") and is assessed non-blocking for the same
reason: a live `cli-daemon-status` probe against this brand-new
`post-merge/137-s-...` branch namespace did not return within a bounded
30-second budget, consistent with the documented per-branch Cozo
first-index cold-start cost, not a code defect. None of the daemon
lifecycle/IPC/indexing code this probe exercises is touched by 137-S (137-S
touches `src/services/code_graph.rs`, `src/cli/direct.rs`,
`src/installer/mod.rs`, `crates/engram-indexer/*`, and
`.github/workflows/release.yml` — none of which implement daemon
startup/IPC binding), and is extensively covered by the 688/689 passing
automated tests below, including 15/15 tests directly exercising this
shipment's own six manifest tasks.

**Condition**: the `cli-daemon-status` probe against a live-bound workspace
on this branch should be re-run in a quiet environment without competing
parallel builds before treating this shipment's runtime posture as
unconditionally verified. This condition does not block the closure PR — it
blocks upgrading releasability from `READY WITH CONDITIONS` to an
unconditional `READY`.

| Requirement | Evidence |
|---|---|
| Healthy signal | `cargo check --all-targets` GREEN; `cargo build` GREEN; `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` clean; `cargo dev-test` 688/689 GREEN (1 confirmed pre-existing, confirmed-unrelated, confirmed-reproducible-in-isolation flake — see below); 15/15 targeted tests across this shipment's own six manifest tasks (`integration_candidate_indexing_service`, `integration_direct_sync_mode`, `contract_supervisor_workspace_boundary`, `contract_supervisor_release_artifact`, `contract_supervisor_install_exclusion`, plus `cargo test -p engram-indexer`'s own boundary test) GREEN. |
| Review | Pre-merge: 6 per-task `code-review` (report-only) passes (one after a fix cycle), plus 1 final consolidated full-branch review at the pre-stash-capture HEAD: **READY_WITH_FOLLOWUPS** (single P2 finding — no cross-process write coordination between the new `engram-indexer` supervisor and existing direct-sync/daemon writers — correctly out of scope per P-021 C1, deferred to plan units F16-F18, captured to stash `AF5CE07E`; not implemented). §1.9 local review readiness + P-018 Copilot-review gate both passed for the merged HEAD per the pre-merge session record. |
| Runtime verification | `docs/closure/2026-09-08-137-s-runtime-verification.md` — verdict `PASS WITH FOLLOW-UP`. CLI-version, MCP-protocol (initialize + tools-catalog), and all six manifest tasks' own harnesses GREEN via substitute/direct real commands; `cli-daemon-status` probe **BLOCKED** (named condition above). |

## Invariants to preserve

- The indexing entry point accepts only a sealed `IndexTarget` minted by the
  F07 generation store; no public path accepts an arbitrary caller-supplied
  path for indexing. `IndexTarget::LegacyDirect` preserves current
  managed-mode indexing behavior; `IndexTarget::Candidate` writes only into
  an exclusive candidate directory; indexing never writes into the active
  generation (asserted by `integration_candidate_indexing_service`).
- `Managed` mode preserves legacy direct index/sync under `DaemonLock`
  unchanged; `ReadServer` mode refuses direct sync with the stable,
  non-retryable F38 refusal naming `engram-indexer` as the supported path
  (asserted by `integration_direct_sync_mode`).
- The agent `engram` package declares no supervisor binary target; the
  supervisor crate (`crates/engram-indexer`) is a workspace member but not
  an agent dependency, and consumes only the minimal public facade from
  F06-F10 (asserted by `contract_supervisor_workspace_boundary`).
- The supervisor appears in neither the agent CLI nor the MCP tool catalog
  (asserted by `contract_tools_catalog` — unchanged tool count/schemas — and
  `contract_supervisor_workspace_boundary`).
- The release workflow publishes `engram-indexer` as a distinct artifact,
  and the agent release archive excludes it (asserted by
  `contract_supervisor_release_artifact`).
- Agent installation installs no supervisor binary or command (asserted by
  `contract_supervisor_install_exclusion`).
- `crates/engram-indexer` declares `#![forbid(unsafe_code)]`, matching the
  workspace-wide safety convention.
- `cargo dev-test` (default features) remains the canonical local merge
  gate and must stay GREEN modulo the pre-existing documented
  `archive_verifier_runs_the_unpacked_native_binary` flake.

## Pre-deploy audits

- Confirmed `crates/engram-indexer/Cargo.toml` gained only the `engram = {
  path = "../.." }` path dependency plus test-harness dev-dependencies
  (dependency sections only, per 142.020-T's owned-file scope) — no root
  `Cargo.toml` changes beyond what F12a already established.
- Confirmed `.github/workflows/release.yml` builds and publishes a distinct
  `engram-indexer` artifact without altering existing agent release
  artifacts (asserted by `release_workflow_retains_the_cross_platform_packaging_contract`,
  which passed in this session's `cargo dev-test` run).
- Confirmed `Cargo.lock` was regenerated (not hand-edited) via `cargo check
  --all-targets`.

## Post-deploy checks

- `engram --version` confirmed against the merged binary:
  `engram 0.3.0-rc.1+gef0135bf-dirty`, exit 0 (the `-dirty` suffix reflects
  this closure session's own uncommitted backlog-bookkeeping files, not
  uncommitted source).
- Spot-check (post-first-release) that a scratch `engram-indexer` build
  succeeds standalone (`cargo build -p engram-indexer`) and that a fresh
  `engram install` in a scratch workspace never places or references an
  `engram-indexer` binary or command (covered by
  `contract_supervisor_install_exclusion`, re-run against the actual release
  archive once F14's workflow first executes on a tagged release).

## Monitoring

No new default-on runtime surface was introduced: `engram-indexer` is a
separately distributed, non-agent supervisor executable that is not invoked
by any agent CLI or MCP tool path, and no production caller of the new
`IndexTarget::Candidate` path exists yet (per the existing stash `AF5CE07E`
follow-up on cross-process write coordination, deferred to future plan
units F16-F18). The daemon's existing structured JSON logs,
`get_health_report`, and `get_daemon_status` remain unchanged and continue
to be the monitoring surface for the IPC/CLI/stdio-MCP transports.

## Failure signals

- `cargo dev-test` or `cargo ci` turning newly red on `main` after merge,
  beyond the already-documented pre-existing `archive_verifier_runs_the_unpacked_native_binary`
  flake.
- Any operator report that direct sync in `ReadServer` mode returns
  anything other than the stable, non-retryable F38 refusal, or that
  `Managed` mode direct sync/index behavior changed.
- Any operator report of a candidate index build writing into or mutating
  the active generation.
- Any operator report that the agent release archive or agent install path
  now includes an `engram-indexer` binary or command.

## Rollback trigger

Any of the failure signals above, observed on `main` after merge and
attributable to this shipment's commits (not to the pre-existing,
independently-documented `archive_verifier` flake).

## Rollback procedure

`git revert` of this shipment's commit range on `main` (the six manifest
tasks' implementation commits: `5fcfb31f`/`43397163` (142.015-T),
`61873a71` (142.016-T), `f5e94755` (142.020-T), `91bfbac7` (142.021-T,
test-only), `10a8da07` (142.022-T), `2741aa69` (142.027-T, test-only), plus
their done/archive bookkeeping commits). Reverting restores the
pre-shipment `code_graph.rs`/`direct.rs` behavior, removes the
`crates/engram-indexer` supervisor crate and its release-workflow entry,
and restores the pre-shipment installer exclusion state. No production or
runtime data is touched by this change (source-only additions, a new
workspace crate, a release-workflow edit, and backlog-state bookkeeping) —
rollback carries no data-migration risk.

## Risky action record

| Field | Value |
|---|---|
| `ProposedAction` | (1) Merge PR #388 to `main`: sealed `IndexTarget` type acceptance, a mode-boundary refusal path, a new non-agent supervisor crate, a release-workflow addition, and an installer exclusion assertion — purely additive to production source, no deletions. (2) Shipment safe-close bookkeeping for `137-S`: delete `.backlogit/queue/137-S.md` and author `.backlogit/archive/137-S.md` (manual safe-close, P-015 partial-feature procedure). |
| `ActionRisk` | (1) `moderate` — additive-only production source change, no existing behavior removed or changed, no wired production caller of the new sealed-target/supervisor surfaces yet. (2) `destructive` per the strict-safety schema's literal inclusion of "deletes" — `.backlogit/queue/137-S.md` is deleted from the working tree. This is Ship-role-permitted, not independently destructive-approval-gated, bookkeeping: the Role Boundary explicitly allows Ship to "close shipments, archive completed items," post-merge closure (Step 6) is a mandatory, non-discretionary continuation of the same operator-approved merge action (not a fresh, separately-proposed destructive action), the file's full content is preserved verbatim in `.backlogit/archive/137-S.md` (nothing is lost), the change is fully git-reversible, and it lands on a dedicated closure branch/PR (#389) requiring its own separate explicit operator approval before reaching `main`. |
| `ActionResult` | (1) `applied` — merge completed cleanly via `gh pr merge 388 --merge`; ancestry verified (`git merge-base --is-ancestor`); approval: explicit, PR-scoped operator approval (*"PR 388: Merge approved"*) for the exact reviewed HEAD `653e973e201883790a9862e6ff54ea84efbb902d`. (2) `applied` — verified via live re-read (`status: active` before, `archived_status: done` after), `142-F` verified byte-for-byte unchanged, zero orphans, no unrestored archive deletions (`git status -- ".backlogit/archive/"`). No approval beyond the Role-Boundary authorization above is claimed for this action; it is not retroactively described as separately operator-approved. |

## Owner

Ship agent (this session), on behalf of the operator who approved PR #388
verbatim (*"PR 388: Merge approved"*) for the exact reviewed HEAD.

## Validation window

Standard PR review + CI window. No extended bake/soak period is warranted:
the change adds a sealed-target boundary, a mode-refusal path, and a
separately-distributed, not-yet-wired supervisor crate with no production
caller. The full local test suite (688/689, one pre-existing unrelated
flake) plus 15/15 targeted tests directly covering this shipment's six
manifest tasks provide direct coverage of the change surface. The one open
condition (daemon-status probe re-run) is tracked above and does not gate
this validation window.

## Follow-up requirements (all captured to stash, none blocking this closure)

| Stash ID | Priority | Summary |
|---|---|---|
| `AF5CE07E` | (Stage to reprioritize) | **Pre-merge review follow-up**: no cross-process write coordination yet exists between the new `engram-indexer` supervisor and existing direct-sync/daemon writers — correctly out of scope per P-021 C1 (deferred to plan units F16-F18). Referenced here for traceability only; not triaged or implemented by Ship. |
| `7D47F30B` | medium | **Post-merge closure finding**: `archive_verifier_runs_the_unpacked_native_binary` fails reproducibly on this Windows build host with apparent MCP-stdio stdout truncation — long-documented, pre-existing, confirmed unrelated to 137-S (PR #388's hosted CI reported SUCCESS for this merge commit). Ambiguous against four prior occurrences (`58B33C45`, `4EE241DC`, `3067BC32` from 133-S/134-S; `0443D844` from 135-S); a new entry was captured per the discovery-reuse protocol rather than guessing which prior entry to reuse. |
| `DA0AF326` | low | (cited, not re-captured) `.autoharness/workspace-profile.yaml` runtime-validation probe commands have drifted from the actual CLI/test surface — pre-existing, unrelated to 137-S; re-confirmed again this session (`engram status` still does not exist; real subcommands are `daemon-status`, `workspace-status`, `stats`). |

All three are `requires_deliberation: true` (or already so recorded) and
await Stage triage/harvest. None represent a regression introduced by, or a
gap in, 137-S's own stated scope.

## Source artifact cleanup

Checked `custom_fields.source_stash_id` and `custom_fields.source_deliberation_id`
on every item in the shipped scope (142.015-T, 142.016-T, 142.020-T,
142.021-T, 142.022-T, 142.027-T, covering feature 142-F, and the shipment
record 137-S itself) via `backlogit get {id}`.

- Archived stash (`source_stash_id`): **none present on any shipped-scope
  item** — 0 stash entries archived.
- Archived deliberations (`source_deliberation_id`): **none present on any
  shipped-scope item** — 0 deliberation artifacts archived.
- Skipped (already archived or not found): n/a — no candidate fields were
  present to begin with.

No source-artifact retirement was performed. This is the correct, precise
outcome per the Ship Role Boundary: cleanup is scoped strictly to
manifest-derived `source_stash_id`/`source_deliberation_id` references, and
none exist for this shipment's scope. No discretionary stash edit, triage,
or archival was performed. The existing follow-up stash `AF5CE07E` was
referenced above for traceability only and was NOT triaged, edited, or
implemented, per the operator's explicit scope instruction.

## Compaction status

`done` — `compact-context --target all` invoked at Ship Step 8 (post-merge
closure, 2026-09-08). Candidate scope: the 9 memory checkpoints for the
just-closed 137-S release unit (the eligible candidate per the
completed-work rule; no other memory/plan/closure artifacts in the
workspace met the age/size compaction thresholds). Result: 9 verbose
checkpoints consolidated into 1 compacted summary
(`docs/memory/compacted/2026-09-08-137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation-compacted.md`);
originals preserved (never deleted) under
`docs/archive/memory/2026-09-08/`. 0 exec-plans and 0 closure records were
compaction candidates (no stale/threshold-exceeding artifacts found). No
degradation — this run completed cleanly.

## Post-merge closure record

| Field | Value |
|---|---|
| Merge SHA | `ef0135bf05aba5d7329a7306eb78bb9b18e02f69` |
| Merge method | merge commit (`gh pr merge 388 --merge`) |
| Merge confirmed | `MERGE_CONFIRMED` — `gh pr view 388` state `MERGED` at `2026-09-09T06:05:39Z`; `git merge-base --is-ancestor ef0135bf... origin/main` exit 0 |
| Shipment closure | Manual safe-close (see `.backlogit/archive/137-S.md` AUDIT RATIONALE) — `backlogit shipment ship` was never attempted (established non-terminating tool defect against the shared `142-F` covering feature, per `docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`); direct `move --status shipped` confirmed rejected by the CLI (exit 9); manual archive-file creation used, matching the 134-S/135-S precedent for P-015-protected covering-feature scope |
| Task statuses | 142.015-T, 142.016-T, 142.020-T, 142.021-T, 142.022-T, 142.027-T — all `done`, all individually archived pre-closure (confirmed `pre-archived` in reconciliation) |
| Covering feature | `142-F` — verified `active`, byte-for-byte unchanged (SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802 bytes; P-015 protection confirmed) |
| Reconciliation | `.backlogit/reconcile/137-S-pre-20260908T230954Z.md` (PROCEED), `.backlogit/reconcile/137-S-post-20260908T231240Z.md` (PROCEED) |
| Post-merge closure branch | `post-merge/137-s-candidate-indexing-direct-sync-boundary-and-supervisor-crate-separation` |
| Post-merge closure PR | #389, "chore: post-merge closure for 137-S — Candidate indexing, direct-sync boundary and supervisor crate separation" — open, awaiting its own separate explicit operator approval |
| Canonical gate-evidence file | `docs/closure/137-S-2026-09-08-post-merge-closure.md` (machine-discoverable frontmatter for the `pipeline-topology` gate's `shipment_readiness` check on `138-S` and later `142-F`-covering shipments) |
