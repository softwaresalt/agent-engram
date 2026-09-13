---
title: 139-S Migrate Read and Lifecycle Handlers to Pinned Generation Context — Operational Closure
description: Post-merge release-readiness, monitoring, rollback, and follow-up record for shipment 139-S.
---

## Releasability

**Status: READY WITH CONDITIONS**

PR #393 merged via merge commit `08e816394cfa1945fdf234bd77048ac867a7ea1f`
(merge method: `gh pr merge 393 --merge`, operator-approved explicitly, PR-scoped
only: *"PR 393: Merge approved"*, for the exact reviewed HEAD
`c34247667e14d000614e24c17a2bd815e03d601f`). Immediately pre-merge: full
last-mile gate re-run — headRefOid matched `c3424766`; CI green (`build`,
`start-launcher-windows`); 0 unresolved Copilot review threads across 12
review rounds; P-018 copilot-review gate independently re-run twice and
returned `SATISFIED` both times at the exact HEAD; `mergeStateStatus: CLEAN`,
`mergeable: MERGEABLE`; repository settings allow merge-commit only
(squash/rebase disabled, P-009 satisfied); P-016 topology confirmed (single
worktree, correct branch, `pipeline-topology` gate PASS at `lifecycle` phase).
The PR body's `## Local Review Readiness` block was found to cite a
one-commit-stale reviewed HEAD (`f843b4d5`) relative to the actual current
HEAD (`c3424766`, a docs-only session-memory commit with zero source/test
changes) — the Ship agent independently reviewed that delta (benign,
no findings, no code risk) and updated the readiness block to the current
HEAD before merge, satisfying the P-014 HEAD-advance re-check requirement.

The condition below mirrors the identical, already-established precedent
from 137-S/138-S's post-merge closures and is assessed non-blocking for the
same reason: the live `cli-daemon-status` probe (manifest literal `engram
status`, which does not exist as a subcommand — pre-existing drift, stash
`DA0AF326`) was not attempted against any daemon in this closure session.
Substitute harness evidence is provided in
`docs/closure/2026-09-13-139-s-runtime-verification.md`.

**Condition**: two pre-existing, cross-cutting build/test issues discovered
during this shipment's implementation session — (1) `cargo lint`/`cargo ci
--all-features` fail to compile due to an OpenTelemetry dependency conflict
(stash `74AAE80F`, high priority), and (2) a deferred-scope architectural
gap around full `ReadRequestContext` threading into handler dispatch (stash
`EFE9190A`) — should be triaged, deliberated, and (if accepted) planned by
Stage as their own release units. Both are out of 139-S's scope per P-021
C1 (139-S owns migrating the six named handler families to the pinned
context; it does not own the workspace's `--all-features` build
configuration or a broader dispatch-layer context-threading redesign).
These conditions do not block the closure PR — they block upgrading
releasability from `READY WITH CONDITIONS` to an unconditional `READY`.

| Requirement | Evidence |
|---|---|
| Healthy signal | `cargo fmt --all -- --check` clean; `cargo check --all-targets` clean; `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` clean; `cargo test --all-targets --no-fail-fast` 2467/2469 GREEN (2 confirmed pre-existing, confirmed-unrelated flakes, both already stashed — see runtime verification); 17/17 targeted tests across all 6 manifest tasks' own harnesses GREEN. |
| Review | Pre-merge: 12 rounds of Copilot review findings recorded, fixed or reused/deferred, and re-verified across the PR lifetime; all threads resolved at the merged HEAD; genuine in-scope bugs found and fixed at rounds 4, 9, and 11 (pin-before-embed ordering, validate-before-open-storage ordering across 4 handlers); 10 out-of-scope findings deferred to stash per P-021 C2/C3. §1.9 local review readiness (re-confirmed at the true final HEAD by this closure session) + P-018 Copilot-review gate both passed for the merged HEAD, re-verified at the last-mile gate immediately before merge. |
| Runtime verification | `docs/closure/2026-09-13-139-s-runtime-verification.md` — verdict `PASS WITH FOLLOW-UP`. Build/check/fmt/clippy GREEN; all 6 manifest tasks' own harnesses GREEN (17/17); full-suite 2467/2469 with 2 confirmed-unrelated flakes cited (`9088F47D`, `39049DEE`, both already stashed, not re-captured); `cli-daemon-status` probe intentionally not attempted (named condition above, unrelated). |

## Invariants to preserve

- All migrated handlers (`map_code`, `impact_analysis`, `query_graph`,
  `query_changes`, `query_memory`, `list_symbols`, `unified_search`,
  `get_workspace_statistics` in `src/tools/read.rs`; the report handlers;
  the lifecycle handlers in `src/tools/lifecycle.rs`; the eval handler in
  `src/tools/eval.rs`; the lint handler in `src/tools/lint.rs`; the doctor
  handler in `src/tools/doctor.rs`) read exclusively through the pinned
  dispatch-snapshot/generation context — none resolve a database or
  workspace path independently of that pinned context.
- Handlers with params to validate deserialize and validate those params
  **before** opening or bootstrapping storage (the round-11 fix,
  `queries_from_context(&DispatchSnapshot)`), matching the pre-migration
  baseline behavior of surfacing `InvalidParams` rather than a
  storage/lock error for malformed requests (asserted by
  `integration_core_read_generation_pin`, `integration_doctor_read_pin`,
  `integration_eval_read_pin`, `integration_lifecycle_read_generation_pin`,
  `integration_lint_read_pin`, `integration_report_read_generation_pin`).
- `unified_search` pins the dispatch context via `pinned_queries` **before**
  calling `embedding::embed_text` (the round-9 fix), preventing a
  background generation from being observed mid-request.
- `#![forbid(unsafe_code)]` remains enforced workspace-wide; `cargo clippy
  --all-targets -- -D warnings -D clippy::pedantic` (which denies
  `unwrap_used`/`expect_used` workspace-wide) remains clean.
- `cargo dev-test` (default features) remains the canonical local merge
  gate and must stay GREEN modulo the two pre-existing documented
  failures: `t046_s050_daemon_exits_after_idle_timeout_and_restarts`
  (full-suite-contention-only — passes cleanly in isolation) and
  `archive_verifier_runs_the_unpacked_native_binary` (reproduces even in
  isolation and on the clean `origin/main` baseline — not
  contention-only; see `docs/closure/2026-09-13-139-s-runtime-verification.md`
  for the isolation repro evidence). Both are already stashed.

## Pre-deploy audits

- Confirmed the merged commit range is additive/corrective to
  `src/tools/{read,lifecycle,eval,lint,doctor}.rs` and their 6 corresponding
  integration test files only, plus normal Ship-workflow bookkeeping
  artifacts under `.backlogit/`, `docs/memory/`, and `docs/compound/` — no
  unrelated production files were touched.
- Confirmed no data-migration or schema change is introduced: handlers now
  read through an already-established pinned-context abstraction; no new
  persisted schema was added.
- Confirmed `Cargo.lock` was not modified by this shipment.
- Confirmed the two open follow-up conditions (`74AAE80F` OpenTelemetry
  `--all-features` build break, `EFE9190A` dispatch-context-threading gap)
  are pre-existing/architectural, not regressions introduced by the 6
  merged tasks.

## Post-deploy checks

- `target\debug\engram.exe --version` confirmed against the merged binary
  (via this closure branch's own build, source-identical to the merge
  commit): `engram 0.3.0-rc.1+g7dad29c6`, exit 0.
- All 6 manifest tasks' own harnesses re-run directly against the
  post-merge closure branch and confirmed green (see runtime verification).
- Recommended follow-up (non-blocking): re-run a live `cli-daemon-status`
  probe against a freshly-created workspace in a quiet environment once the
  validator-manifest drift (`DA0AF326`) is corrected, to confirm the
  migrated handlers do not introduce any startup-readiness regression.

## Monitoring

No new default-on runtime surface was introduced beyond what 139-S's own
manifest already covers (all 6 handler families are existing MCP tool
dispatch entry points, now reading through an already-established pinned
context instead of resolving storage independently — not a new standalone
service). The daemon's existing structured JSON logs, `get_health_report`,
and `get_daemon_status` remain the monitoring surface for handler
dispatch/param-validation/storage-open ordering outcomes.

**Observability correction (stash `3F1AEFE1`)**: the two SLIs below are
aspirational, not currently log-queryable through the named surfaces.
`HealthReport` exposes only the eight fixed checks documented in
`src/models/health.rs:51-62` (no per-handler error-class breakdown), and
`UsageEvent` (`src/models/metrics.rs`) records only a generic
success/error `outcome`, not an error class or the entry/read generation
IDs either threshold needs. Adding those structured fields is out of
scope per P-021 C1 for this shipment's handler-migration work; it is
captured as stash `3F1AEFE1` for Stage to deliberate as its own release
unit. Until that instrumentation exists, both rows below are an
**unresolved releasability condition**, not an active, executable
monitor. The Owner column is also corrected: the Ship session that
authored this closure cannot literally remain a responder for a 7-day
window, so the operator is the sole owner of record.

| SLI | Baseline | Alert / rollback threshold | Owner | Validation window |
|---|---|---|---|---|
| Rate of `InvalidParams` vs. storage/lock errors on malformed requests to a migrated handler (`map_code`, `impact_analysis`, `query_graph`, `query_changes`, report/lifecycle/eval/lint/doctor handlers) — **not currently log-queryable; requires the error-class instrumentation captured in stash `3F1AEFE1`** | 0 storage/lock errors for malformed requests (pre-migration and post-migration baseline are identical: `InvalidParams` only) | Any single observed storage/lock error (instead of `InvalidParams`) for a malformed request to a migrated handler triggers the rollback trigger below, once the operator can observe it (manual code review / ad hoc log inspection until stash `3F1AEFE1` lands) | Operator monitoring `get_health_report`/`get_daemon_status` output and structured JSON logs during normal use (not the Ship session, which ends at closure) | 7 days of normal developer usage following merge to `main` (matches the "Standard PR review + CI window" cited under Validation window below; this table makes that window's duration and owner explicit) |
| `unified_search` observing a database generation newer than the one current at handler entry (dispatch-context pin violation) — **not currently log-queryable; requires the generation-ID instrumentation captured in stash `3F1AEFE1`** | 0 occurrences (pinning is intended to make this structurally impossible) | Any single observed occurrence triggers the rollback trigger below, once the operator can observe it (manual code review / ad hoc log inspection until stash `3F1AEFE1` lands) | Operator monitoring `get_health_report`/`get_daemon_status` output and structured JSON logs during normal use (not the Ship session, which ends at closure) | 7 days of normal developer usage following merge to `main` |

## Failure signals

- `cargo dev-test` or `cargo ci` turning newly red on `main` after merge,
  beyond the two already-documented pre-existing flakes.
- Any operator report that a malformed request against `map_code`,
  `impact_analysis`, `query_graph`, `query_changes`, or any other migrated
  handler mutates storage or surfaces a database/lock error instead of
  `InvalidParams`.
- Any operator report that `unified_search` (or any other migrated handler)
  observes a newer database generation than the one current at handler
  entry.

## Rollback trigger

Any of the failure signals above, observed on `main` after merge and
attributable to this shipment's commits (not to the two pre-existing,
independently-documented flakes, and not to the two separately-tracked
follow-up conditions above, both of which pre-date this shipment).

## Rollback procedure

Revert the merge commit `08e816394cfa1945fdf234bd77048ac867a7ea1f` as a
single unit on `main` (`git revert -m 1 08e816394cfa1945fdf234bd77048ac867a7ea1f`),
not a sub-range. The merge commit encompasses the entire PR #393 commit
chain in one operation — all six manifest tasks' implementation commits
(`4a3cade0`, `7604b56f`, `aa777c9a`, `a578baa7`, `b674ce15`, `10c39b8a`)
through the final review-fix and docs/bookkeeping commits (`f843b4d5`,
`c3424766`). An earlier draft of this procedure cited a sub-range starting
at `679500ce` (an intermediate review-fix commit that lands *after* the six
implementation commits, not before them); reverting only that sub-range
would leave the core migration applied. Reverting the merge commit as a
unit restores the pre-shipment handler behavior (each handler resolving
storage independently rather than through the pinned context). No
production or runtime data is touched by this change (source-only
additions plus backlog-state bookkeeping) — rollback carries no
data-migration risk.

## Risky action record

| Field | Value |
|---|---|
| `ProposedAction` | (1) Merge PR #393 to `main`: migrate `map_code`, `impact_analysis`, `query_graph`, `query_changes`, report handlers, lifecycle handlers, the eval handler, the lint handler, and the doctor handler to the pinned dispatch-snapshot/generation context — purely additive/corrective to production source. (2) Shipment safe-close bookkeeping for `139-S`: delete `.backlogit/queue/139-S.md` and author `.backlogit/archive/139-S.md` (manual safe-close, P-015 partial-feature procedure, identical pattern to `133-S`/`134-S`/`135-S`/`137-S`/`138-S`). |
| `ActionRisk` | (1) `moderate` — additive/corrective production source change across 5 handler-owning files; extensively covered by 12 rounds of review and 17/17 targeted tests plus a 2467/2469 full-suite run. (2) `destructive` per the strict-safety schema's literal inclusion of "deletes" — `.backlogit/queue/139-S.md` is deleted from the working tree. This is Ship-role-permitted, non-discretionary bookkeeping (Role Boundary explicitly allows "close shipments, archive completed items"; post-merge closure Step 6 is a mandatory continuation of the same operator-approved merge action); the file's full content is preserved verbatim in `.backlogit/archive/139-S.md`; the change is fully git-reversible; it lands on a dedicated closure branch/PR requiring its own separate explicit operator approval before reaching `main`. |
| `ActionResult` | (1) `applied` — merge completed cleanly via `gh pr merge 393 --merge`; ancestry verified (`git merge-base --is-ancestor 08e81639... origin/main`, exit 0); approval: explicit, PR-scoped operator approval ("PR 393: Merge approved") for the exact reviewed HEAD, re-verified via the full last-mile gate immediately before merge. (2) `applied` — verified via live re-read (`status: active` before, `archived_status: done` / `status: archived` after via `backlogit get 139-S`), `142-F` verified byte-for-byte unchanged (SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802 bytes), zero orphans, no unrestored archive deletions (`git status -- ".backlogit/archive/"`). |

## Owner

Ship agent (this session). Action (1) — merging PR #393 — was performed on
behalf of the operator's explicit, PR-scoped approval ("PR 393: Merge
approved") for the exact reviewed HEAD; that approval does not extend to
any other action. Action (2) — shipment safe-close bookkeeping — was
authored under Ship's own Role-Boundary-permitted, non-discretionary
backlog mandate ("close shipments, archive completed items"), as already
grounded in the `ActionRisk` row above — it was not performed "on behalf
of" the PR #393 approval, and the PR #393 approval is not cited as, nor
does it constitute, authorization for this separate destructive action.
Per the `ActionRisk` row, action (2) still requires its own separate
explicit operator approval before landing on `main` via this closure PR
(#394); as of this writing that separate approval has not yet been
obtained — the action is authored and reviewable on this closure branch
only, pending that approval.

## Validation window

Standard PR review + CI window. The change migrates 6 handler families to
an already-established pinned-context abstraction, with 12 rounds of
Copilot review (including 2 genuine in-scope correctness fixes at rounds 9
and 11) and full local test coverage (17/17 targeted, 2467/2469 full-suite)
providing direct coverage of the change surface. The two open follow-up
conditions (`74AAE80F`, `EFE9190A`) are tracked above and do not gate this
validation window — both are pre-existing/architectural conditions
orthogonal to this shipment's own scope.

## Follow-up requirements (all previously captured to stash by the
implementation session, none blocking this closure)

| Stash ID | Priority | Summary |
|---|---|---|
| `284285B5` | high | Pre-existing finding predating this closure session's final review pass; not re-captured, cited for traceability. |
| `30174AD7` | medium | Pre-existing finding predating this closure session's final review pass; not re-captured, cited for traceability. |
| `F9767C12` | high | Pre-existing finding predating this closure session's final review pass; not re-captured, cited for traceability. |
| `B9CC92AC` | medium | Pre-existing finding predating this closure session's final review pass; not re-captured, cited for traceability. |
| `39049DEE` | high | **Confirmed pre-existing (reproduces on `origin/main` baseline before any 139-S change)**: `archive_verifier_runs_the_unpacked_native_binary` fails with `ARCHIVE_SMOKE=FAIL: non-JSON stdout from MCP stdio`. Long-documented recurring flake across `133-S` through `138-S`. Re-confirmed by this closure session's own runtime verification; not re-captured. |
| `74AAE80F` | high | **Pre-existing build break (unrelated to 139-S)**: `cargo lint`/`cargo ci --all-features` fail to compile due to an OpenTelemetry dependency conflict. Out of scope per P-021 C1 — 139-S does not own workspace `--all-features` build configuration. Blocks unconditional `READY` per the condition above. |
| `EFE9190A` | medium | **Deferred scope expansion candidate**: full direct threading of the pinned `ReadRequestContext` into handler dispatch beyond this shipment's 6 migrated handler families. Requires Stage deliberation as its own release unit. Blocks unconditional `READY` per the condition above. |
| `DE62B123` | medium | Pre-existing flaky test (unrelated to 139-S): `services::metrics::tests::full_channel_branch_switch_is_acknowledged_before_following_event` — full-suite-contention-only, passes in isolation. |
| `9088F47D` | medium | **Confirmed pre-existing, re-verified by this closure session**: `integration_daemon_lifecycle::t046_s050_daemon_exits_after_idle_timeout_and_restarts` full-suite-contention-only flake (and the same metrics-branch-switch flake as `DE62B123`) — passes cleanly in isolation. Not re-captured. |
| `F1E5A255` | medium | Pre-existing flaky tests (unrelated to 139-S) under full parallel `cargo dev-test`: `manifest_tool_count_matches_catalog` and a Copilot-PR-related test. |
| `F0A2A478` | high | **Deferred scope expansion**: `set_workspace_with_probe` in `src/tools/lifecycle.rs` cannot safely distinguish a trusted-startup initial bind from an untrusted runtime call. Out of scope per P-021 C1 for this shipment's migration work. |
| `069B5F74` | low | Deferred scope expansion: a contract test hardcodes a stale tool-count literal (`21`), now mismatched (`23`) as the catalog has grown — pre-existing test-maintenance debt, not introduced by 139-S. |
| `7A596F8C` | low | Pre-existing flaky test (unrelated to 139-S) under full parallel `cargo dev-test`: `contract_shim_stdio_initialize::t3_missing_result_is_terminal`. |
| `652C3104` | low | Deferred scope expansion: a Copilot round-4 finding on `src/tools/capabilities.rs` (`DOCTOR_SMOK...` truncated), reused/deferred per P-021 C2/C3 — out of scope for this shipment's manifest. |
| `DA0AF326` | (cited, not re-captured) | `.autoharness/workspace-profile.yaml` runtime-validation probe commands have drifted from the actual CLI/test surface — pre-existing, unrelated to 139-S; re-confirmed again this session (`engram status` still does not exist). |
| `76153F55` | high | **Deferred scope expansion**: closing 139-S via manual safe-close (`archived_status: done`, never a literal `status: shipped` transition) may leave queued successor shipment 140-S ineligible under the dependency-eligibility rule in `.github/instructions/backlogit.instructions.md:63-65`, which requires a `blocks`-type predecessor to have reached `shipped`. Third occurrence of the same recurring class already captured as `77A4E71C` (135-S/PR #384) and `F35EA0E6` (137-S/PR #389); resolving whether this literally blocks 140-S in practice requires either mutating 140-S or redesigning backlogit's shipment-lifecycle semantics — both out of scope per P-021 C1 for 139-S's own closure. Recommend Stage resolve this recurring class workspace-wide together with the two prior occurrences. |
| `3F1AEFE1` | medium | **Deferred scope expansion**: the Monitoring section's two SLIs (`InvalidParams` vs. storage/lock error rate; `unified_search` stale-generation occurrence) are not currently log-queryable — `HealthReport` exposes only 8 fixed checks (`src/models/health.rs:51-62`) and `UsageEvent` records only a generic success/error `outcome`, not error class or entry/read generation IDs. Adding that instrumentation is out of scope per P-021 C1 for this shipment's handler-migration work. Recorded as an unresolved releasability condition in the Monitoring section above pending Stage deliberation. |

All out-of-scope findings above are `requires deliberation` and await Stage
triage/harvest. None represent a regression introduced by, or a gap in,
139-S's own stated scope. This closure session also disclosed (and
preserves for traceability, per the operator's instruction) that the
implementation session exceeded the Ship agent's 3-cycle review-fix circuit
breaker (12 Copilot review rounds) and performed a lossless stash/pop
affecting `.backlogit/stash.jsonl` during round-9 diagnosis; no stash
content was discarded, overwritten, or silently absorbed by this closure.

## Source artifact cleanup

Checked `custom_fields.source_stash_id` and `custom_fields.source_deliberation_id`
on every item in the shipped scope (`142.034-T`, `142.035-T`, `142.036-T`,
`142.037-T`, `142.038-T`, `142.039-T` — 6 items total — plus covering
feature `142-F` and the shipment record `139-S` itself) via `backlogit get
{id}`.

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
or archival was performed.

## Compaction status

`done` — `compact-context --target all` invoked at Ship Step 8 (post-merge
closure, 2026-09-13). Candidate scope: the 2 memory files for the
just-closed 139-S release unit (`139-s-ship-session-summary.md` 35 KB,
`139-s-pinned-read-handler-migration-memory.md` 2.7 KB — the eligible
candidates per the completed-work rule; no other memory/plan/closure
artifacts in the workspace met the age/size compaction thresholds). Result:
2 verbose checkpoints consolidated into 1 compacted summary
(`docs/memory/compacted/2026-09-13-139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context-compacted.md`);
originals preserved (never deleted) under `docs/archive/memory/2026-09-12/`.
0 exec-plans and 0 additional closure records were compaction candidates
(no stale/threshold-exceeding artifacts found beyond this shipment's own
just-produced closure documents, which are current, not stale). No
degradation — this run completed cleanly.

## Post-merge closure record

| Field | Value |
|---|---|
| Merge SHA | `08e816394cfa1945fdf234bd77048ac867a7ea1f` |
| Merge method | merge commit (`gh pr merge 393 --merge`) |
| Merge confirmed | `MERGE_CONFIRMED` — `gh pr view 393` state `MERGED` at `2026-09-13T00:37:15Z`; `git merge-base --is-ancestor 08e81639... origin/main` exit 0 |
| Shipment closure | Manual safe-close (see `.backlogit/archive/139-S.md` AUDIT RATIONALE) — `backlogit move 139-S --status shipped` confirmed rejected by the CLI (exit 9); manual archive-file creation used, matching the `133-S`/`134-S`/`135-S`/`137-S`/`138-S` precedent for P-015-protected covering-feature scope |
| Task statuses | 142.034-T, 142.035-T, 142.036-T, 142.037-T, 142.038-T, 142.039-T (6 items total) — all `done`, all individually archived pre-closure (confirmed `pre-archived` in reconciliation) |
| Covering feature | `142-F` — verified `active`, byte-for-byte unchanged (SHA-256 `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802 bytes; P-015 protection confirmed) |
| Reconciliation | `.backlogit/reconcile/139-S-pre-20260913-003937.md` (PROCEED), `.backlogit/reconcile/139-S-post-20260913-004121.md` (PROCEED) |
| Post-merge closure branch | `post-merge/139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context` |
| Post-merge closure PR | PR #394 (`chore: post-merge closure for 139-S — Migrate read and lifecycle handlers to pinned generation context`), created from branch `post-merge/139-s-migrate-read-and-lifecycle-handlers-to-pinned-generation-context`; open, undergoing Copilot review-comment remediation as of this writing; merge requires its own separate explicit operator approval (see Owner section above) — not yet obtained |
| Canonical gate-evidence file | `docs/closure/139-S-2026-09-13-post-merge-closure.md` (machine-discoverable frontmatter for the `pipeline-topology` gate's `shipment_readiness` check on later `142-F`-covering shipments) |
