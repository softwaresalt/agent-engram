# Ship Session Checkpoint — 135-S PR #383 Ready, Halted at Merge Approval Gate

**Date**: 2026-09-06 (session continuation of the 2026-09-05 preflight session)
**Shipment**: 135-S — "Retire HTTP and SSE transport surfaces"
**Branch**: `feat/135-s-retire-http-and-sse-transport-surfaces`
**PR**: [#383](https://github.com/softwaresalt/agent-engram/pull/383)
**Final HEAD**: `2bb97bd37b260f5fa33d98fc8e46d6a5bb2d61bb`
**Mode**: P-017 dark-factory, `merge_approval_pre_authorized=false`,
`admin_fallback_pre_authorized=false`. Run scope: 135-S → 142-S (this
invocation owns only 135-S).

> **SUPERSEDED (Copilot review comment 3943313719 on PR #383, correctly
> flagged)**: `2bb97bd3` was **not** the live HEAD when this file's later
> "gates green" claims were recorded. A subsequent continuity commit
> (`a11296f7`) and two further Copilot-remediation rounds (`c22303fe` for
> round 5, plus additional round-6 fixes) landed after this checkpoint's
> "Final HEAD" was written, so the gate/CI claims below do **not** cover
> the commit that would actually be merged. Every push resets the P-018
> review clock, so this file's HEAD and gate status are a point-in-time
> snapshot, not a live invariant. Do not rely on this file's "Final HEAD" or
> gate table for merge decisions — see the newer checkpoint written at the
> end of the review-remediation session that produced `c22303fe` and later
> commits (`docs/memory/` entries dated on or after 2026-09-06, filenamed
> for the copilot-review-remediation round) for the actual final HEAD and
> re-verified gate state.

## Operator approval (recorded, applied)

Verbatim: *"Approve the destructive deletion scope for 142.023-T and resume
Dark Factory mode."* Matched exactly against the halted `ProposedAction`
from the prior session's checkpoint
(`docs/memory/2026-09-05-ship-135-s-preflight-and-safety-gate.md`).
`ActionResult`: **approved** → **applied** (all 4 tasks committed, gates
green, PR open, all CI/review gates satisfied).

## Completed this session

1. Reconstructed prior-session state (branch, backlog, shipment status,
   memory checkpoint) without re-claiming the shipment or creating a
   parallel branch/worktree.
2. Implemented all four manifest tasks in dependency order:
   - **142.023-T** (`3d9b9976`): deleted `src/server/{mcp,router,sse}.rs`,
     updated `mod.rs`, deleted `connection_test.rs` + Cargo.toml
     registration, removed two orphaned `legacy-sse` test functions
     (required completion, not scope creep).
   - **142.024-T** (`c5f85ddd`): removed `legacy-sse` feature + 4 deps
     (kept `sysinfo`), wrote 4 real assertions in
     `supported_transport_surface_test.rs`.
   - **142.025-T** (`e7a53729`): fixed installer/templates HTTP claims,
     `docs/architecture.md`, updated `installer_test.rs`.
   - **142.026-T** (`28a55ca3`): marked ADR-0016/ADR-0003 superseded.
   - Archived all 4 tasks to `done` (`13317b81`).
3. Adversarial Review (3 reviewers, report-only) → `READY_WITH_FOLLOWUPS`
   (`4e22a892`), later extended via addendum to cover the full `main..HEAD`
   diff including the 3 prior-session commits.
4. Runtime verification (`82ab5c87`): CLI-version + MCP-protocol probes
   GREEN via substitute real commands; `cli-daemon-status` probe BLOCKED
   under session resource contention (not a regression).
5. Operational closure (`82ab5c87`, later revised): releasability
   `READY WITH CONDITIONS`.
6. Pushed branch, opened PR #383.
7. **Four rounds of GitHub Copilot automated review** (9, 2, 1, 1 = 13
   comments total). Every comment addressed: fixed in-scope gaps directly
   (rustdoc correction, test-assertion additions for both Copilot and
   Claude hooks, test rename `s068_custom_port_in_hook_urls` →
   `s068_custom_port_no_longer_rendered_in_hook_urls`, doc-content sync,
   **regenerated this repo's own checked-in
   `.github/copilot-instructions.md`/`.claude/instructions.md` via the real
   `engram install --hooks-only`** since 142.025-T's template fix only
   affects *future* installs), deferred out-of-scope findings to stash with
   thread replies. All 13 threads replied-to and resolved via GraphQL
   `resolveReviewThread`. Commits: `796717e2`, `a7ca96b8`, `afa1aff3`,
   `2bb97bd3`.
8. CI: `start-launcher-windows` failed once on a pre-existing, unrelated,
   hosted-runner timing flake (`launcher_fails_open_to_copilot_within_one_prewarm_budget`,
   14.48s vs its own 8s budget; confirmed zero diff to `start.ps1`/launcher
   files in this PR). Captured to stash (`F58ECAA8`), re-ran the CI job via
   `gh run rerun --failed` → GREEN. `build` check GREEN throughout.
9. **P-018 copilot-review gate: `SATISFIED: PASS`.**
10. **Pipeline-topology gate (`--phase lifecycle`): exit 0, pass.**
11. PR state: `OPEN`, `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`.

## Stash entries captured this session (all `requires_deliberation: true`)

| ID | Priority | Summary |
|---|---|---|
| `9A7C9F8F` | medium | Pre-existing `otlp-export` compile break (ambiguous vs. 7B270F79/E12542FF). |
| `0443D844` | medium | Pre-existing `archive_verifier` stdout flake (ambiguous vs. 58B33C45/4EE241DC/3067BC32). |
| `3A9CBD36` | medium | Stale HTTP/legacy-sse doc/config refs in 6 out-of-scope files. |
| `39B44E19` | medium | Supplemental: `docs/log-observation-guide.md` also stale (found via Copilot). |
| `E007DF00` | low | Dead `RateLimiter`/`check_rate_limit()` in `src/server/state.rs`. |
| `DA0AF326` | low | `.autoharness/workspace-profile.yaml` validator-manifest command drift. |
| `F58ECAA8` | low | Pre-existing hosted-runner `start-launcher-windows` timing flake. |

None of these seven are fixes for 135-S itself — every one is either
pre-existing (confirmed via baseline comparison) or a deliberate,
documented P-021 C1 scope boundary.

## HALTED — Merge Approval Gate (NON-NEGOTIABLE, P-014/P-017)

Everything required for merge readiness is green:

| Gate | Status |
|---|---|
| CI `build` | ✅ SUCCESS |
| CI `start-launcher-windows` | ✅ SUCCESS (after 1 re-run of a confirmed pre-existing flake) |
| P-018 copilot-review gate | ✅ `SATISFIED: PASS` |
| Pipeline-topology gate (lifecycle) | ✅ exit 0, pass |
| Local review readiness (§1.9-equivalent) | ✅ `READY_WITH_FOLLOWUPS`, recorded in PR body |
| `mergeStateStatus` | `CLEAN` |
| `mergeable` | `MERGEABLE` |

`merge_approval_pre_authorized=false` and `admin_fallback_pre_authorized=false`
for this dark-mode run — **no auto-merge may be attempted.** I am halting
here and requesting explicit operator approval to merge PR #383.

**Resumable handoff**: once operator approval is received, the next Ship
session (or this one, if resumed) should:
1. Re-verify `headRefOid` is still `2bb97bd37b260f5fa33d98fc8e46d6a5bb2d61bb`
   (re-run §1.9 + P-018 gates if HEAD advanced — both are last-mile
   re-checks per Step 5 items 15).
2. Confirm merge-commit strategy is configured (P-009) before merging.
3. Merge via `gh pr merge 383 --merge` (merge commit, not squash/rebase).
4. Proceed to Step 6 post-merge closure: confirm merge via
   `merge-base --is-ancestor`, create `post-merge/135-s-...` branch, close
   the shipment via `shipment-reconcile` safe-close, run
   `operational-closure` in `mode=post-merge`, stash any new follow-ups,
   archive source stash/deliberation artifacts, resync the backlog index,
   and **mandatorily invoke `compact-context`** (P-020).
5. Only after 135-S's full post-merge closure (including compaction) is
   the next shipment (136-S) eligible to start, per P-001 + P-020
   closure-gated routing.

I am halting the session here, awaiting explicit operator approval before
merging PR #383.
