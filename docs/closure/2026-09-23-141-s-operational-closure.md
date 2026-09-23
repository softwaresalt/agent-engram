---
title: 141-S Error Transport, Response Provenance, Lifecycle Policy, Generation Observability — Operational Closure
description: Releasability evidence per .autoharness/workspace-profile.yaml runtime_validation.releasability for PR #407 (141-S), post-merge.
---

## Mode

`post-merge`. PR #407 merged via merge commit
`f115835e260c089bf094d82fa565371077f5bac8` at 2026-09-23T16:09:51Z, final
reviewed/merged HEAD `0bcabd0a` (the merge commit itself is
`f115835e`, its sole parent tip on the feature branch was `0bcabd0a`).
Operator gave explicit chat approval ("PR 407: Merge approved",
2026-09-23T09:08:26.508-07:00) after a full independent re-verification of
every last-mile gate at that HEAD (no dark-mode pre-authorization was in
effect for this shipment; approval was solicited and given explicitly).

## Summary of change

141-S implements 7 tasks (`142.047-T`–`142.053-T`) migrating IPC/MCP/CLI
transport to carry the full domain error envelope instead of a lossy
string conversion, adds MCP and CLI structured success/error transport
parity, decorates responses with captured-context provenance, enforces a
read-server (Generation/ReadServer mode) lifecycle policy forbidding
hydration/scan/watcher/sync work, and reports generation observability
without requiring deletion-based invalidation. All 7 tasks were built via
TDD and passed the full quality-gate sequence (`cargo fmt`, `cargo clippy
--all-targets -D warnings -D clippy::pedantic`, `cargo dev-test`), **with
one known pre-existing, unrelated exception**:
`integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
was independently reconfirmed still failing during this shipment's own
review round and is not caused by, or related to, any of the 7 manifest
tasks (see `docs/closure/2026-09-23-141-s-runtime-verification.md` for
detail). The 7 tasks were reviewed by a 7-persona local adversarial
review (mixed BLOCKED/READY_WITH_FOLLOWUPS across personas; all P0/P1
findings fixed, 3 out-of-scope findings correctly deferred as P-021
stash entries).

## CI status and unresolved review items


**Final, as merged 2026-09-23**, HEAD `0bcabd0a`:

- Hosted CI (PR #407): `build` **PASS**; `start-launcher-windows`
  **PASS** on an operator-authorized single rerun (job `106480592120`,
  completed 2m11s — the extended `in_progress` API display during
  monitoring was step-status reporting lag, not a real hang; confirmed via
  a no-op `gh run cancel` attempt returning "Cannot cancel a workflow run
  that is completed").
- Local adversarial review (7 personas): mixed initial verdicts; all P0/P1
  findings fixed in-scope; 3 findings deferred as P-021 stash entries
  (`6C5DF765`, `9B7EC1E4`, `4628001C` — see runtime-verification report
  follow-ups). Final local readiness for HEAD `0bcabd0a`:
  `READY_WITH_FOLLOWUPS`, `P0=0, P1=0` blocking.
- GitHub-hosted Copilot review: engaged; 4 review threads raised across
  the review cycle, 2 resolved by citing the pre-existing deferred stash
  entries above (no code change needed — those findings were
  already captured), 2 resolved via new fixes (commits `743064fa`,
  `f6c49b49`, `d306e989`). P-018 gate
  (`autoharness gate copilot-review 407 ...`) verdict: **`SATISFIED`** for
  HEAD `0bcabd0a`, 0 unresolved threads — re-verified fresh immediately
  before merge per the operator's last-mile gate re-run instruction.
- No unresolved review items remain open on this PR as of merge. All
  threads are resolved.

## Runtime verification report

`docs/closure/2026-09-23-141-s-runtime-verification.md` — Verdict:
**`PASS_WITH_FOLLOW_UP`**.

## Validator evidence (structured handoff)

- **Surfaces exercised**: `cli` (version probe green on a freshly built,
  non-dirty binary at this closure branch's HEAD; live daemon-status
  probe against the real dev workspace timed out on wall-clock only, not
  functionally — see runtime-verification report), `api`/MCP (full
  contract-test suite green as part of the pre-merge `cargo dev-test`
  gate, with one known pre-existing, unrelated failure — see
  runtime-verification report), `background-job` (the required
  `workspace-sync` probe was attempted this pass and blocked by an
  already-running daemon holding the workspace lock; not silently
  substituted — see runtime-verification report for the explicit blocked
  disposition).
- **Manual checkpoints**: none declared for these surfaces beyond the
  automated probes.
- **Blocked prerequisites**: `workspace-sync` (required background-job
  probe) — see runtime-verification report. Does not change this
  shipment's own verdict.
- **Verdict**: `PASS_WITH_FOLLOW_UP`.

## Invariants to preserve

- Domain error envelopes must survive IPC/MCP/CLI transport without lossy
  string conversion (142.047-T/142.048-T/142.049-T/142.050-T).
- MCP and CLI transport must remain parity-matched per
  `docs/cli-mcp-parity.md`.
- Read-server (Generation/ReadServer) mode must never perform
  hydration/scan/watcher/sync work (142.052-T) — **not yet fully wired to
  real production dispatch**, per deferred stash `6C5DF765`; the enforced
  policy exists and is tested, but the production call path
  (`request_entry.rs::process_request`) does not yet route through the
  admission check that would trigger it.
- Generation observability reporting must degrade gracefully to `None`
  when no `GenerationActivator` is installed (142.053-T) — also not yet
  wired to a real production activator, per deferred stash `9B7EC1E4`.

## Pre-deploy audits

- No schema migrations, feature flags, or config changes are introduced by
  this shipment.
- No new environment variables or rollout prerequisites.
- Merge-only release path (workspace-local Rust binary plugin); the next
  tagged release build picks up this change via the existing
  `cargo-release` process.

## Deployment / rollout path

Merge-only. Workspace-local Rust binary; no live deploy step. End users
receive the change via the next tagged GitHub Release.

## Post-deploy checks

- `engram --version` reports the new release tag's embedded SHA.
- `engram daemon-status` reports `overall: green` after a normal bind.
- IPC error responses for a deliberately-triggered `EngramError` (e.g., an
  invalid workspace path) retain the structured error envelope (not a
  generic stringified message) across both MCP and CLI transports.
- A ReadServer-mode daemon does not emit hydration/scan/watcher/sync log
  lines during normal operation (policy enforcement present, though full
  production wiring remains a known follow-up per stash `6C5DF765`).

## Risky action record

No `ProposedAction`/`ActionRisk` entries — this shipment made no
destructive, high-blast-radius, or irreversible changes. All 7 tasks were
additive/structural transport and policy-enforcement changes with test
coverage.

## Healthy signals

- `cargo test --all-targets --no-fail-fast` introduces no *new* failures
  beyond the known, pre-existing, unrelated exception
  `integration_release_archive_smoke_workflow::archive_verifier_runs_the_unpacked_native_binary`
  (see Summary of change and the linked runtime-verification report). This
  named exception is a pre-existing condition, not a 141-S regression;
  defining "healthy" as unconditionally 100% green would make every future
  observation window report unhealthy even when 141-S has not regressed.
- `engram daemon-status` and `engram health` report green/healthy.
- No increase in generic/unstructured error responses observed in IPC/MCP
  transport logs.

## Failure signals

- A daemon or CLI error response reverts to a lossy, unstructured string
  instead of the structured domain error envelope (signals a regression in
  142.047-T/142.048-T's transport migration).
- `contract_read_path_pinning_enforcement` or any of the 7 shipped
  contract tests fail on a future PR (signals a regression in the shipped
  transport/policy/observability behavior).
- Generation observability reporting begins panicking or erroring instead
  of gracefully returning `None` when no activator is installed (would
  indicate a regression against 142.053-T's designed graceful-degradation
  contract).

## Monitoring plan

Standard daemon log observation per `docs/log-observation-guide.md`; no
additional monitoring infrastructure required for this shipment.

**Observable signal (corrected 2026-09-23, PR #408 review)**: the
prior version of this section named a
`shim::transport::translate_ipc_response` log line with an
`envelope_kind=` field. That field and log line do not exist —
`translate_ipc_response`/`structured_ipc_error_payload`
(`src/shim/transport.rs:431-471`) build the F38 error envelope
(`jsonrpc_code`, `message`, and, when present in `error.data`, the
daemon's `engram_code`/`engram_name`/`engram_details` fields) directly
into the MCP `structured_content`/CLI response payload; they do not
write it to any log stream. There is currently no log-based signal for
this behavior. The actual, verifiable check is a manual functional
probe: invoke a CLI or MCP call known to trigger an `EngramError` (for
example, an invalid workspace path) and inspect the returned
`structured_content` (MCP) or structured stdout (CLI) for the
`jsonrpc_code`/`message` envelope shape — not a bare, lossy string.
**Baseline**: every deliberately-triggered error response returns the
structured envelope shape; the shipped contract tests enforce this at
build time, so a manual probe should never observe a bare string
message in place of the envelope.
**Alert threshold**: any single manual probe (or user report) that
observes a bare/lossy string response instead of the structured
envelope triggers investigation (zero-tolerance threshold, consistent
with the baseline above). No automated log-based alert exists for this
signal until a future shipment adds structured logging at the
translation boundary; that gap is tracked as a follow-up (stash
`D77BCBBC`, captured with this correction).

**Follow-up (stash `6C5DF765`, `9B7EC1E4`)**: the read-server lifecycle
policy and generation observability reporting shipped by this PR are not
yet reachable from real production IPC dispatch — `process_request`
still dispatches context-free. This is a known, deliberate,
deliberation-flagged gap (composition-root wiring is a separate design
decision from the transport/policy/observability primitives themselves),
not a regression. Monitor for a follow-up shipment that completes this
wiring; until then, these two invariants are enforced in test but dormant
in production.

## Rollback trigger

Any healthy-signal regression above, or a user report of malformed/lossy
IPC or CLI error output under normal operation.

## Rollback procedure

Standard workspace-local rollback: reinstall the prior GitHub Release
asset/tag and flush regenerated `.engram/` state. No database migration
accompanies this shipment, so no down-migration is needed.

## Validation window

Standard: through the next tagged release plus 48 hours of ordinary
workspace usage exercising IPC/MCP/CLI error paths.

## Owner

Repository maintainer (release owner) accountable during the observation
window; no shipment-specific owner override.

## Compaction status (P-020)

`done` — mandatory `compact-context` invocation (`target: all`) completed
during this post-merge closure pass (Step 6, item 8). The 141-S release
unit's 5 memory files (~24 KB total, all eligible under the
"completed feature or chore" candidate rule) were consolidated into
`docs/memory/compacted/2026-09-23-141-s-error-transport-response-provenance-lifecycle-policy-and-generation-observability-compacted.md`;
verbose originals moved to `docs/archive/memory/2026-09-23/` (traceable,
not deleted). No plans qualified as compaction candidates this pass. This
closure artifact and the accompanying runtime-verification report were
authored directly (not compacted), consistent with the `140-S` closure
precedent of leaving the just-produced closure record itself
uncompacted.

## Releasability evidence

| Required evidence | Status |
|---|---|
| healthy-signal | **Satisfied** — CLI version probe green (freshly built, non-dirty binary); full test suite green with one known pre-existing, unrelated exception (see runtime-verification report); hosted CI green (both `build` and `start-launcher-windows`, the latter via one operator-authorized rerun). |
| failure-signal | **Satisfied** — named above; see Monitoring plan for the concrete log-query signal, baseline, and threshold. |
| monitoring-plan | **Satisfied with a follow-up** — a manual functional probe (invoke a known error-triggering call, confirm the returned response preserves the structured envelope rather than a bare string) with an explicit baseline and zero-tolerance alert threshold (see Monitoring plan above, corrected 2026-09-23 per PR #408 Copilot review — the earlier log-grep description named a field/log line that does not exist); automating this as a log-based signal is tracked as follow-up `D77BCBBC`. Separately, two invariants (read-server policy, generation observability) are enforced in test but not yet reachable from production dispatch, tracked as unresolved follow-ups `6C5DF765`/`9B7EC1E4`. |
| rollback-trigger | **Satisfied** — named above. |
| rollback-procedure | **Satisfied** — standard GitHub Release reinstall + `.engram/` flush; no migration to reverse. |
| owner | **Satisfied** — repository maintainer / release owner. |
| validation-window | **Satisfied** — through next tagged release + 48h. |
| follow-up (optional) | **Satisfied (tracked)** — 3 stash entries captured during 141-S's own local review (`6C5DF765`, `9B7EC1E4`, `4628001C`), plus 4 additional entries captured during closure PR #408's two Copilot review rounds: `5684685C` (validator-manifest command drift, advisory), `3A963D34` (`archived_status: done` vs. documented Shipment Sequencing Protocol prose reconciliation, requires deliberation), `21D0F63C` (correct malformed `feature`/`requires_deliberation` fields on `3A963D34`/`5684685C` at next Stage triage — Ship cannot edit stash entries post-capture per P-021 C5), and `D77BCBBC` (add automated log-based monitoring for envelope-fidelity signal). None block this PR's own scope. |

**Overall status: `READY_WITH_CONDITIONS`** — merge completed via merge
commit; `closure_status` for this shipment's own execution is `READY`
(all of 141-S's authorized manifest work — 7/7 tasks — is done, verified,
and archived), but `releasability` for the shipped change is
`READY_WITH_CONDITIONS` rather than unconditionally `READY` because two
of the shipped behaviors (read-server lifecycle policy enforcement,
generation observability reporting) are not yet wired into real
production IPC dispatch, per deferred stash `6C5DF765`/`9B7EC1E4`. This
distinction — completion status vs. release evidence — is deliberate:
completing 141-S's own scope does not retroactively resolve the two
still-open, deferred production-wiring follow-ups.

### Last-mile gate re-verification (this session, immediately before merge)

Per operator instruction, all last-mile gates were re-run fresh against
current HEAD `0bcabd0a` immediately before merge, independent of the
readiness report presented in the prior session:

- P-014 local readiness: current-HEAD evidence confirmed present in PR
  body, outcome `READY_WITH_FOLLOWUPS`, `P0=0/P1=0` blocking.
- P-018 Copilot review: `SATISFIED`, 0 unresolved threads, at HEAD
  `0bcabd0a`.
- Required CI: `build` PASS, `start-launcher-windows` PASS (post-rerun).
- GitHub mergeability: `MERGEABLE`/`CLEAN`.
- P-009 merge-commit-only: confirmed via repo settings
  (`allow_squash_merge=false`, `allow_rebase_merge=false`,
  `allow_merge_commit=true`).
- P-016 topology: `autoharness gate pipeline-topology --phase lifecycle
  --shipment 141-S` → exit 0 (single active shipment, correct branch,
  single worktree, no prohibited parallel worktree).
- Scope/secrets: `git diff main...HEAD` reviewed — only in-scope files
  changed, no secret patterns detected.

All gates passed; merge proceeded via `gh pr merge 407 --merge` (explicit
operator approval, not a dark-mode pre-authorization carve-out — this
session solicited and received a literal approval message).

## Source artifact cleanup

Performed during post-merge closure (Step 6, item 7). This shipment's
manifest is task-only (7 tasks: `142.047-T`–`142.053-T`); no feature or
chore is a top-level shipped item in this shipment's scope (the shared
covering feature `142-F` remains `active` with substantial roster scope
outside this shipment, and is explicitly not touched by this closure —
see the AUDIT RATIONALE in `.backlogit/archive/141-S.md`). Since no
top-level feature/chore exists in this shipment's manifest, there are
zero source-artifact cleanup candidates:

- Archived stash (`source_stash_id`): 0 candidates.
- Archived deliberations (`source_deliberation_id`): 0 candidates.
- Skipped (already archived or not found): none applicable.

This is consistent with the identical task-only-manifest precedent
established at the `137-S` through `140-S` closures against the same
`142-F` covering feature.

## Uncommitted-artifact disposition

The rerun-outcome memory checkpoint
(`docs/memory/2026-09-21-ship-141-s-pr407-authorized-rerun-passed.md`),
deliberately left uncommitted through the merge to avoid advancing the
already-gated PR #407 HEAD, was committed on this post-merge closure
branch (`post-merge/141-s-...`) rather than on `main` or the (already
merged, no-longer-mutable) feature branch, per explicit operator
instruction.
