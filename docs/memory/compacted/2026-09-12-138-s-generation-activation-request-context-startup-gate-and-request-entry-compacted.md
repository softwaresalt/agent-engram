---
title: "Compacted memory — 138-S: Generation activation, request context, startup gate and request entry"
description: "Dense consolidated summary of the full 138-S session lifecycle (Stage re-plan → RS5 approval gate halt → Engram-unreachable recovery halt → RS5 approval + resume → 11-round review implementation → operator pause → merge approval → merge → post-merge closure), replacing 7 verbose checkpoints"
---

## Release unit

Shipment `138-S`, covering feature `142-F` (roster: 59 units). Branch
`feat/138-s-generation-activation-request-context-startup-gate-and-request-entry`.
PR [#391](https://github.com/softwaresalt/agent-engram/pull/391). Merge
commit `81b19b0d91c79c9c456ce703dca42a4688978dfa` (merge commit strategy,
P-009). Manifest: `142.018-T` + 4 subtasks (`142.018.001-ST`–`142.018.004-ST`),
`142.019-T`, `142.028-T`, `142.029-T`, `142.030-T`, `142.031-T`, `142.032-T`,
`142.033-T` + 2 subtasks (`142.033.001-ST`, `142.033.002-ST`) — 14 items
total, all `done`, all archived. Shipment record archived
(`archived_status: done`) via manual safe-close.

## Timeline and key decisions

1. **Stage re-plan (2026-08-27)**: revision-1 plan for `138-F` (originally
   re-parented from a `137-F` review follow-up via `backlogit adopt`) failed
   PR #365 Copilot review with 12+ feasibility/correctness defects after
   Ship exceeded the 3-cycle review-fix circuit breaker and routed back to
   Stage. Stage produced **plan revision 2** — a full re-grounded redesign
   (not an addendum): fixed a previously-unnoticed fail-open race (the
   late-readiness monitor could overwrite a request-path `Degraded` with
   `Ready` — closed via `watch::Sender::send_if_modified`), corrected false
   transport-framing and terminal-record-write claims, and re-decomposed
   7 tasks into 14 under a three-phase, mechanically-enforced TDD
   dependency DAG (19 `blocks` edges, acyclic). New gate `138.002-R`
   approved and superseded `138.001-R`. Shipment `131-S` (later re-manifested
   as `138-S`) carried all 15 members (`138-F` + 14 tasks), queued and
   unclaimed.
2. **RS5 approval gate halt (2026-09-10, dark-mode)**: Ship claimed the
   shipment, created the branch, and ran preflight (`cargo check` clean),
   then halted **before any harness scaffolding** because manifest item
   `142.018-T` (F17, generation activation) carries an explicit
   plan-authored governance annotation: `RS5 - ActionRisk high — requires
   operator approval before Ship implements it`, corroborated by
   `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`. The
   RS5 gate transitively blocked 12 of 14 manifest items (only
   `142.031-T`/`142.032-T` were independently executable); Ship declined to
   build a partial set (violates the "harness once, up front, for the full
   scope" protocol and would have been an unauthorized scope-narrowing
   decision reserved for Stage). No source touched; shipment left `active`
   as recoverable state.
3. **Engram-unreachable recovery halt (2026-09-10)**: on a later resume
   attempt, the required Checkpoint-Recovery / Prune-on-Restore Protocol
   engram read failed on both the 138-S branch and (per operator-authorized
   bounded excursion) the staging branch `chore/stage-critical-engram-readiness`
   — both timed out after ~30s. Per protocol, this is an unambiguous
   fail-closed condition: no bounded degraded prune was substituted, no
   resume was attempted, no checkpoint resolved. Diagnostics noted 13
   concurrent `engram` processes across sessions/workspaces (later
   clarified by the operator as normal per-workspace shims, not
   proliferation). PID 30528 was not stopped (no approval existed for it).
4. **RS5 approval + successful resume (2026-09-10/11)**: operator explicitly
   approved *"Approve RS5/F17 implementation for 142.018-T and resume
   checkpoint checkpoint-20260910-222318.json for all of shipment 138-S"*,
   and separately supplied direct named-pipe IPC evidence (bypassing the
   health-gated CLI) proving the installed Engram substrate was reachable
   and readable, satisfying the prune/gate precondition. Ship recorded the
   approval, resumed the checkpoint cursor, and resolved
   `checkpoint-20260910-222318.json` only after the resume was confirmed
   successful.
5. **Implementation (14/14 manifest items, 11 rounds of Copilot review)**:
   * `142.018-T` + 4 subtasks (F17) — `src/services/generations/activation.rs`:
     typed manifest parse/bounds enforcement, `activate_initial` with
     deadline/store resolution, single-flight `maybe_activate_newer`,
     immutable rejection cache with transient backoff. Subject to the most
     review iteration: rounds 6, 7, 9 fixed a TOCTOU digest gap, bounded
     manifest/digest reads, capped `ExistingDbLocation` construction and
     cumulative digest/size accounting, and closed a rejection-cache gap
     for repeated retries of an already-rejected initial revision (the
     latter suppressed as a non-blocking P2 follow-up, stash `5C873386`,
     rather than opening an 11th fix round).
   * `142.019-T` — mode-agnostic `ReadRequestContext` constructors in
     `src/server/state.rs`.
   * `142.028-T` — startup readiness gated on initial generation activation
     (`src/daemon/startup_activation.rs`); round-2 fix derived
     `ReadServerStartupGate` identity from its activator; round-4 fix
     rejected initial activation from an unbound socket phase.
   * `142.029-T` — request entry order and background activation
     (`src/daemon/request_entry.rs`); fixes resolved descriptor and
     authorized before triggering activation, and bounded concurrent
     background reconciliation spawns.
   * `142.030-T` — read-server dispatch capability gate
     (`src/tools/mod.rs`); round-10 fix closed a gap where a `Managed`-mode
     context with no generation could still pass the refusal check
     (refusal now requires `context.generation().is_some()`).
   * `142.031-T` — descriptor-derived stdio MCP tool catalog
     (`src/shim/tools_catalog.rs`); fixes advertised git-graph tools and
     scoped the MCP catalog parity allowlist to git-graph exclusions.
   * `142.032-T` — descriptor-derived CLI workflow surface
     (`src/cli/runner.rs`).
   * `142.033-T` + 2 subtasks — read-input ownership inventory with
     fail-on-unclassified guard; fixes reclassified `snapshot.workspace_uuid`
     as disallowed, scoped a wildcard match, enumerated missing read
     inputs, and corrected `connection_count` classification.
   * Round-11 also suppressed a P3 test-coverage gap (`errors/mod.rs:1011`,
     no `to_response()` coverage for 5 new activation-error branches) as
     stash `3FFEE99B`.
6. **Operator-directed pause (2026-09-11)**: mid-review-cycle, with PR #391
   already fully green/merge-ready and no active command running, the
   operator issued an urgent pause directive. Ship created checkpoint
   `checkpoint-20260912-020327.json` (status `active`, phase
   `merge-gate-halt-operator-pause`), wrote a memory doc, and committed
   locally (`1c3eb3e8`) **without pushing** — pushing would have re-armed
   CI/Copilot review, contradicting the "stop looping" instruction.
7. **Gates before merge**: 11 review rounds, 27/27 threads resolved at
   final HEAD `6b39ee94`; CI green (`build`, `start-launcher-windows`,
   including one hosted-runner timing-flake rerun unrelated to the diff);
   P-018 Copilot-review gate `SATISFIED` at the exact HEAD; repo
   merge-strategy settings confirm merge-commit-only (P-009).
   `merge_approval_pre_authorized=false` — halted for explicit operator
   approval.
8. **Operator merge approval (2026-09-12)**: explicit, PR-scoped approval
   ("OPERATOR MERGE APPROVAL: PR #391 is explicitly approved for merge").
   Ship independently re-verified the local checkpoint commit `1c3eb3e8`
   never leaked onto the PR (remote HEAD unchanged at `6b39ee94`), re-ran
   the full last-mile gate (headRefOid match, CI green, 27/27 threads
   resolved, Copilot review at exact HEAD, mergeable clean, P-009/P-016
   confirmed), then merged via `gh pr merge 391 --merge`. `MERGE_CONFIRMED`
   via `gh pr view` + `git merge-base --is-ancestor`.
9. **Post-merge closure (2026-09-12)**:
   * Post-merge closure branch
     `post-merge/138-s-generation-activation-request-context` created from
     fresh `main`; the paused local checkpoint commit was cherry-picked
     onto it as `ce3b2fba` (preserving it off the implementation PR, per
     explicit operator instruction).
   * Pre-mode reconciliation: all 14 manifest items `pre-archived`, no
     orphans → `PROCEED`.
   * **`backlogit shipment ship` not attempted at all** — the manual
     safe-close procedure was applied directly by reusing the existing
     `135-S`/`137-S` non-termination evidence against the shared `142-F`
     covering feature (`docs/compound/workflow-issues/backlogit-shipment-ship-non-terminating-large-covering-feature-2026-09-06.md`,
     now updated with a `138-S` evidence-reuse addendum). Hand-authored
     `.backlogit/archive/138-S.md`, removed the queue file, `backlogit
     sync` clean. Verified `142-F` byte-for-byte unchanged (SHA-256
     `59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`,
     802 bytes, identical to the 137-S snapshot).
   * Post-mode reconciliation: all archive files present, no deletions
     (P-007 clean) → `PROCEED`.
   * Runtime verification: build/fmt/clippy all clean; 87/87 targeted
     tests across all 14 manifest tasks GREEN; full
     `cargo test --all-targets --no-fail-fast` 2457/2460 GREEN — the 3
     exceptions all confirmed pre-existing and unrelated: a full-suite-only
     metrics timing flake (stashed `9D313653`), a full-suite-only HCL
     cold-start flake (stashed `58B33C45`, from `133-S`), and the
     long-documented `archive_verifier_runs_the_unpacked_native_binary`
     flake (newly captured `EC3BAF22`, ambiguous against 5 prior entries).
     A fourth transient (`contract_shim_stdio_initialize`'s terminal-
     classification test) observed only in an earlier fail-fast run did
     not reproduce in the `--no-fail-fast` run and needed no separate
     capture. `cli-daemon-status` live probe intentionally not attempted
     against the shared-environment daemon per explicit operator
     instruction (see readiness-latch defect below). Verdict: `PASS WITH
     FOLLOW-UP`; releasability `READY WITH CONDITIONS`.
   * **Readiness-latch defect root-caused this session** (via
     operator-authorized direct named-pipe IPC, not the health-gated CLI):
     branch publication clears `AppState.hydration_ready`
     (`src/server/state.rs::publish_workspace_generation_transition`); the
     watcher branch-refresh path
     (`src/daemon/lifecycle_policy.rs::run_watcher_driver`) completes
     successfully but never calls `set_hydration_ready_for_permit` when
     there is no transferred successor, so `_health` stays `starting`
     forever. Assessed against 138-S's own P-021 C1 same-contract-surface
     test — 138-S owns activation/context/startup-gate/request-entry, not
     the watcher-refresh readiness handoff — and correctly deferred as
     out-of-scope, captured to stash `265F99BE`, **not implemented**.
   * Source artifact cleanup: checked `source_stash_id`/
     `source_deliberation_id` on all 16 shipped-scope items (14 tasks +
     `142-F` + `138-S`) — **none present anywhere**; nothing retired.
   * `compound-refresh`: one entry updated (the non-terminating
     `shipment ship` doc, `138-S` evidence-reuse addendum).

## Stash follow-ups referenced or captured (all `requires_deliberation: true`, none blocking)

| ID | Priority | Summary |
|---|---|---|
| `265F99BE` | high | **Root-caused this session, newly captured**: workspace-generation readiness-latch defect (`_health` stays `starting` forever after a branch/workspace switch with no transferred successor). Out of scope per P-021 C1; awaits Stage triage/planning as its own release unit. |
| `5C873386` | medium | Round-11 suppressed finding: `activation.rs:821` — `activate_initial` never consults the rejection cache before repeating validation. Out of scope for this PR's already-completed fix cycles. |
| `3FFEE99B` | low | Round-11 suppressed finding: `errors/mod.rs:1011` — 5 new activation-error branches lack `to_response()` coverage. |
| `9D313653` | (cited, not re-captured) | Post-merge runtime-verification finding: `services::metrics` full-suite-only timing flake, confirmed unrelated, passes in isolation. |
| `58B33C45` | (cited, not re-captured) | Post-merge runtime-verification finding: `hcl_indexing_test` cold-start full-suite-only flake, confirmed unrelated; originally captured against `133-S`. |
| `EC3BAF22` | low | **Post-merge, newly captured**: `archive_verifier_runs_the_unpacked_native_binary` Windows stdout-truncation flake, confirmed unrelated, ambiguous against 5 prior entries (`58B33C45`, `4EE241DC`, `3067BC32`, `0443D844`, `7D47F30B`). |
| `DA0AF326` | (cited, not re-captured) | Validator-manifest command drift — re-confirmed identical this session. |

## Outcome

138-S fully closed: shipment archived, all 14 tasks archived, covering
feature untouched, PR #391 merged, closure evidence recorded in
`docs/closure/2026-09-12-138-s-{operational-closure,runtime-verification}.md`
and `docs/closure/138-S-2026-09-12-post-merge-closure.md`. Post-merge
closure PR still required for the backlog-archival branch
(`post-merge/138-s-...`) — awaits its own explicit operator approval,
separate from PR #391's approval. `compact-context` (this file) invoked as
the mandatory P-020 step.

## Superseded/compacted originals (see `docs/archive/memory/2026-09-12/`)

* `2026-08-27-stage-138-revision-2-replan-session.md`
* `2026-08-27-stage-138-terminal-vs-transient-health-session.md`
* `2026-09-10-138-s-rs5-approval-gate-halt.md`
* `2026-09-10/138-s-recovery-engram-unreachable-on-staging-branch-halt.md`
* `2026-09-10-138-s-rs5-approval-resume.md`
* `2026-09-12-138-s-merge-gate-ready.md`
* `2026-09-12-138-s-operator-pause.md`
