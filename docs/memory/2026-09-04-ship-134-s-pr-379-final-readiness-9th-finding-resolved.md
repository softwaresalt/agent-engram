# Ship 134-S — PR #379 Final Readiness (9th Copilot finding resolved)

**Session**: Ship agent execution for shipment `134-S` ("IPC seam extraction, mode
constructor migration, error envelope, descriptor schema"), feature `142-F`.

**Status at time of writing**: PR #379 is at full current-HEAD readiness. Awaiting
separate, explicit operator approval to merge. Post-merge closure has not been run
and must not be run until after merge is approved and confirmed.

## What changed since the prior checkpoint

The prior checkpoint (`2026-09-04-ship-134-s-pr-379-ready-for-operator-merge-decision.md`)
described the state after 8 Copilot findings across 3 review rounds. A 9th finding
(`PRRT_kwDORJEduc6fXWL2`, on `src/tools/capabilities.rs:527`, the `DOCTOR_SMOKE`
descriptor's capability classification) appeared afterward, at the same HEAD
`b6a5f860` (Copilot re-reviews on reply/resolve thread activity, not only on pushes).

Investigated and classified per P-021:
- Copilot argued `DOCTOR_SMOKE` is declared `Read`/`read_server_available: true`
  but its implementation (`run_smoke_test` in `src/tools/doctor.rs`) calls
  `set_workspace` and `_shutdown`, both independently declared `Control`.
- Confirmed the classification is not an oversight: it is the explicit,
  plan-mandated acceptance criteria (`docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`
  lines 1430-1431/1708) and is already pinned by the existing contract test
  `doctor_smoke_is_a_read_server_readiness_workflow`.
- Confirmed via grep that zero code outside `capabilities.rs` consumes
  `read_server_available` at all — there is no functional dispatch-gating
  defect today.
- Concluded: the real gap (F20's future mode-gated dispatch/replacement wiring
  for `doctor --smoke`, not yet implemented) is out of 134-S's manifest scope
  (F19/`142.005-T` is descriptor-schema only). Fixing this here would require
  either contradicting the plan-mandated classification or building F20's
  logic — neither is completing already-authorized scope.
- Captured as stash `F95653D1` (P-021 C2), after discovery lookup found a
  related-but-not-positively-confirmed-identical candidate (`1918AFD2`, the F20
  read-server-refusal wiring gap surfaced earlier this session from the
  mode-observability angle). Cross-referenced both ways; no code change made.
- Replied to and resolved thread `PRRT_kwDORJEduc6fXWL2` via GraphQL, citing the
  stash entry and rationale.
- Updated PR #379 body: added the finding-9 narrative, updated the out-of-scope
  findings list (now 5 stash entries), and refreshed the Local Review Readiness
  block (10 threads total, all resolved; follow-ups list now includes `F95653D1`).

## Historical readiness snapshot — HEAD `b6a5f860da7cae684953cb713a5c37acec1f88d4`

This snapshot describes the HEAD that existed **immediately before** the commit
that introduces this checkpoint file. It is historical evidence only, not a
claim about the PR's current readiness — committing this file necessarily
advances the PR to a new HEAD that this text cannot describe (see "Whatever
reads this file next" below for the authoritative current-state source). At
the snapshot HEAD:

- CI: `build` PASS (6m24s), `start-launcher-windows` PASS (2m25s)
- Copilot gate (P-018): `SATISFIED`, `unresolved_thread_ids: []`, `blocked: false`
- Review threads: 10 total, all `isResolved: true`
- PR mergeability: `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`
- Merge strategy (P-009): `allow_merge_commit: true`, `allow_squash_merge: false`,
  `allow_rebase_merge: false` — merge-commit-only confirmed
- Full local build/clippy(pedantic, default + git-graph)/fmt/test suite: clean,
  except the single known pre-existing Windows-only flake
  (`archive_verifier_runs_the_unpacked_native_binary`, stash `4EE241DC`)

**Known limitation of this snapshot**: the commit that adds this file (and the
`.backlogit/stash.jsonl` entry `F95653D1` alongside it) itself produces a new
HEAD. That new HEAD requires its own fresh CI run and its own fresh Copilot
review pass — the readiness evidence above does **not** carry forward to it.
Re-run the required gates for the actual current HEAD; do not treat this
snapshot as current-HEAD evidence at any point after its own commit lands.

## Whatever reads this file next

The PR body (`https://github.com/softwaresalt/agent-engram/pull/379`) is the
authoritative, always-current readiness record — re-verify `headRefOid` against
its "Reviewed HEAD" before trusting anything in this file as still valid, since
this file describes a point-in-time state as of its own commit and cannot
describe the HEAD its own commit produces.

**Do not merge without separate, explicit operator approval.**
**Do not run post-merge closure, knowledge graduation, or
`backlogit shipment ship`/safe-close until after that approval and a confirmed
merge** — `142-F` is a shared parent across 9 shipments (`134-S`..`142-S`) and the
cascade-close hazard documented in the PR body remains real.

## Stashed follow-ups (all P-021 C2 deferred scope, none block this PR)

- `4EE241DC` — pre-existing Windows-only flake, unrelated to 134-S (ablation-confirmed)
- `E12542FF` — pre-existing `--all-features`/otlp-export build break, unrelated to 134-S
- `1918AFD2` — no IPC surface exposes daemon mode yet; likely F20 scope
- `F95653D1` — `DOCTOR_SMOKE` descriptor/implementation mismatch; likely F20 scope
  (related to `1918AFD2` but not positively confirmed identical)
- `AA5698E3` — stale ship-owned checkpoints + legacy-schema anomalies found at
  session start; operational hygiene, not a code-scope item
