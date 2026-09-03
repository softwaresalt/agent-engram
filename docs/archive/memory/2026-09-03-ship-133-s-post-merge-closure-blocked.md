---
title: "Ship session - PR #376 merge + 133-S post-merge closure (blocked at shipment archival)"
date: 2026-09-03
type: session-memory
doc_type: memory
agent: ship
shipment: 133-S
feature: 142-F
status: blocked
---

## Scope

Operator directed: merge PR #376 (approval scoped to this PR only — "Keep
working autonomously until the task is truly finished" — treated as a
one-time approval, not blanket future-PR authorization), re-verifying all
gates at exact HEAD `2005b3db94752dbe37946a98532c46dde1aad674`, then
complete the full Ship post-merge lifecycle for shipment `133-S` through
a closure PR brought to readiness but not merged, explicitly withholding
`134-S` claim until 133-S closure is complete.

## What was done

1. **Re-verified all merge gates** at HEAD `2005b3db94752dbe37946a98532c46dde1aad674`:
   local review readiness (`READY_WITH_FOLLOWUPS`, 0 P0/P1), P-018 Copilot
   review gate (`SATISFIED`, 0 unresolved threads across 5 review threads),
   CI green (`build`, `start-launcher-windows`), `mergeStateStatus: CLEAN`
   / `mergeable: MERGEABLE`, P-009 merge-commit-only confirmed at repo
   level (squash/rebase both disabled), pipeline-topology lifecycle gate
   passed (checked twice — before and immediately before merge, last-mile
   re-check).
2. **Merged PR #376** via `gh pr merge 376 --merge` → merge SHA
   `33a0a41e345cef8965b707346728d44fa5492daf`, merged at
   `2026-09-03T17:54:11Z`. Merge Confirmation Gate: `git fetch origin main`
   + `git merge-base --is-ancestor` → exit 0, `MERGE_CONFIRMED`.
3. **Updated main safely**: `git checkout main && git pull` fast-forwarded
   local `main` to the merge commit.
4. **Re-read fresh `_ship.agent.md` and `shipment-reconcile/SKILL.md`** on
   the newly-merged `main` per the mandatory pre-self-close reload rule.
5. **Pre/post shipment reconciliation** (manual, since this backlogit
   version's assumed direct-status-update safe-close path no longer
   exists — see blocker below): confirmed all 10 task-level manifest items
   (`142.001-T` + 5 subtasks, `142.002-T`, `142.004-T`, `142.006-T`,
   `142.007-T`) are `done` and archived in `.backlogit/archive/`; covering
   feature `142-F` correctly remains `active`/queued (verified by direct
   file enumeration under the `142.*` ID namespace: 59 direct task
   children + 28 nested subtask descendants = 87 total; the manifest
   contains 5 of each — 10 total — leaving 77 outside scope); orphan scan
   found no
   `shipment_id: 133-S` back-references (expected for this backlogit
   version). No `source_stash_id`/`source_deliberation_id` fields present
   on `133-S`/`142-F` — no source-artifact cleanup needed.
6. **Attempted shipment archival — BLOCKED.** `backlogit move 133-S
   --status shipped` unconditionally rejected
   (`ErrShipmentShippedRequiresEnvelope`, backlogit feature `144-F`
   hardening, no `--force` bypass). The only remaining path,
   `backlogit shipment ship 133-S`, would cascade-force-close covering
   feature `142-F` (only 5 of its 59 direct task children and 5 of its 28
   nested subtasks — 10 of 87 total descendants — in scope) and
   force-requeue-and-detach the other 77 —
   forbidden by this workspace's own P-015
   fully-covered-root test. Confirmed via direct CLI reproduction plus a
   deep-dive into the `backlogit` Go source
   (`internal/core/shipment_lifecycle.go`,
   `internal/core/gate_transition.go`, `internal/errors/errors.go`) and
   that repository's own design-decision record. **Recorded as high-priority
   follow-up stash `28C0E138`** with recommended remediation: Stage
   removes `142-F` from `133-S`'s `custom_fields.items`.
   **Round-5 correction (verified against `backlogit` source in this
   session)**: that recommended remediation is **not sufficient**.
   `featureScopeRoots` discovers a covering feature independent of
   explicit manifest membership, and `returnUnreleasedFeatureItems` is
   invoked unconditionally for every discovered feature — removing
   `142-F` from the manifest would stop it from being wrongly marked
   `done`, but would **not** stop the 77 non-manifest descendants from
   being force-requeued/detached. This is a workspace-wide risk affecting
   all ten shipments that jointly and exhaustively cover `142-F`'s 59
   direct children (verified this session: `133-S` 5, `134-S` 5, `135-S`
   4, `136-S` 5, `137-S` 6, `138-S` 8, `139-S` 6, `140-S` 7, `141-S` 7,
   `142-S` 6 = 59, no overlap/no gap). Recorded as new follow-up stash
   `F9767C12` (see closure doc's "Cascade mechanism correction" section
   for the full evidence chain and the verified shipment table).
7. **Runtime verification** (foundations-only, no user-facing runtime
   change): `cargo build --release` (5m15s, clean), `engram --version`,
   `engram manifest` (full catalog unchanged), MCP contract suites
   (`contract_mcp_catalog_oracle` 9/9,
   `contract_mcp_tool_catalog_parity`/`contract_mcp_envelope`/
   `contract_read_server_cli_mcp_parity` F00 placeholders 1/1 each),
   `contract_shim_stdio_initialize` 18/19 (one pre-existing, confirmed
   unrelated failure via temporary diagnostic worktree at pre-merge `main`
   tip `c66d320e`, worktree removed after comparison). Daemon-lifecycle
   probes (`daemon-status`/`workspace-status`) judged not applicable (no
   daemon-lifecycle behavior touched by this shipment). Full report:
   `docs/closure/133-S-2026-09-03-runtime-verification.md`.
8. **Operational/release-observability closure** written:
   `docs/closure/133-S-2026-09-03-post-merge-closure.md`, `closure_status:
   BLOCKED`, `releasability: READY_WITH_CONDITIONS`, documenting the
   Windows directory-durability residual risk (stash `F2E84E15`, already
   present, not re-created) alongside the shipment-archival blocker.
9. **`docs/architecture.md`** updated (Module boundaries section):
   documented the new `crates/engram-indexer` stub crate and the
   `DaemonMode`/`AppState::mode` foundational plumbing (mode values
   `managed`/`read_server`, corrected after Copilot review flagged an
   initial-draft `strict` typo), proportionate to
   the actual (empty-stub, non-behavior-changing) scope shipped.
   **Round-5 correction**: the doc's description of `DaemonMode::resolve`
   wiring was itself overstated (implied it was reachable from production
   config loading). Corrected after verifying `PluginConfig` does carry a
   permissive `mode: Option<String>` field
   (`deserialize_permissive_mode`), but **no production call site**
   currently routes it through `DaemonMode::resolve` — confirmed via a
   full-repo grep of `src/`; every call site is a unit-test assertion in
   `tests/unit/plugin_config_test.rs` (21 tests) or
   `tests/unit/app_state_mode_test.rs` (6 tests), both run this session
   and passing 27/27 after killing a stale `engram.exe` process that was
   holding a file lock on `target/release/engram.exe`.
10. **compound-refresh evaluation**: scanned `docs/compound/` for entries
    referencing storage/generation/read-only-open topics potentially
    superseded by this shipment's F01 spike decision doc; none found
    requiring update, consolidation, or staleness marking — no action
    taken (correct outcome, not a skip).
11. **Follow-up stash entries**: confirmed all 5 pre-existing entries
    still present (`A7C0BA5F`, `5A7FBC37`, `58B33C45`, `7B270F79`,
    `F2E84E15` — no re-stashing, single-write invariant preserved). A
    discovery-lookup gap during this session's first pass created a new
    entry (`F9D1C495`, priority high) for the shipment-archival blocker
    before checking for an existing match — Copilot review round 2
    correctly flagged this as a duplicate of pre-existing entry
    `28C0E138` (`created_at: 2026-09-03T04:37:02Z`, originally captured
    during PR #372's review, well before this session started).
    **First remediation attempt (round 2, reverted)**: archived
    `F9D1C495` and edited `28C0E138` to consolidate the evidence — Copilot
    review round 4 correctly flagged this as a **P-010 role-boundary
    violation** (Ship's role boundary forbids discretionary stash
    archival/edit outside two narrow exceptions, neither of which applies
    here; the discovery-failure protocol reserves duplicate remediation
    for Stage's own triage). **Corrected**: both mutations were reverted —
    `F9D1C495` restored to active (unarchived, original text), `28C0E138`
    restored to its original pre-session text. **Final state**: both
    entries remain active and unedited, flagged in the closure doc's
    "Stash duplicate — flagged for Stage triage" section for Stage's own
    duplicate-detection/harvest disposition. Ship performs no further
    stash mutation on either entry.
12. **Mandatory P-020 `compact-context --target all`**: invoked
    (satisfying the per-merge mandate). First pass incorrectly
    consolidated this session's three 133-S-specific memory checkpoints
    (`2026-09-03-ship-pr-372-stage-133-s-merge-closure.md`,
    `2026-09-03-ship-133-s-mid-session-checkpoint.md`,
    `2026-09-03-ship-133-s-pr-ready-checkpoint.md`) into a compacted
    summary and archived the originals — Copilot review round 2 correctly
    flagged this as premature, since the compact-context skill's own
    eligibility rule excludes checkpoints for active work items, and both
    `133-S` and `142-F` remain `active` (closure explicitly `BLOCKED`,
    not complete). Remediated: the three checkpoint files were restored
    to `docs/memory/`, and the compacted summary plus its
    `docs/archive/memory/` copies were removed. Scanned `docs/exec-plans/`
    and `docs/closure/` for additional 133-S/142-F candidates: the one
    related exec-plan (`2026-09-02-separate-indexer-read-server-plan.md`)
    governs feature `142-F` as a whole (not yet complete across future
    shipments), so it correctly does not qualify for compaction. **Final,
    correct outcome: compact-context was invoked (P-020 mandate
    satisfied) and found zero eligible candidates this session** — a
    valid scan-only no-op, since neither `133-S` nor `142-F` has actually
    reached a completed/shipped state. Recorded `compaction_status: done`
    (successful invocation, correctly did nothing) in the closure artifact.
13. **Backlog index resync**: `backlogit sync` → `CLOSURE_INDEX_SYNC_OK`
    (`Indexed 1291 artifacts`), reflecting the task-level archival that
    already occurred as part of the merged PR's own build phase.

## Blocked at

Shipment `133-S` record archival (`status: shipped` / archived). See
`docs/closure/133-S-2026-09-03-post-merge-closure.md` for the full evidence
chain. This is a genuine backlogit-1.10.1-vs-P-015 tooling/policy conflict,
not a Ship execution error, and is not something Ship can resolve within
its own role boundary (editing `custom_fields.items` on a shipment is a
Stage-only planning-field operation). The `pipeline-topology` gate's
predecessor check independently and correctly blocks `134-S` claim
(`PREDECESSOR_NOT_SHIPPED`) regardless of this closure artifact's content.

## Next steps

1. Commit all closure artifacts on a dedicated `post-merge/133-s-...`
   branch (not `main`), push, open the closure PR, run local review +
   §1.9/P-018 gates to bring it to readiness — **do not merge** without
   separate explicit operator approval.
2. Present final report to operator: merge SHA, shipment status
   (`active`, blocked), closure PR link, and the exact remaining gate
   (Stage must resolve the `142-F` cascade-membership conflict for
   `133-S` — verified this session to require more than a manifest edit:
   removing `142-F` from `133-S`'s `custom_fields.items` alone does not
   prevent `backlogit shipment ship`'s unconditional
   `returnUnreleasedFeatureItems` cascade from force-requeuing/detaching
   `142-F`'s 77 non-manifest descendants. Manual safe-close remains
   required for `133-S` and, per the same mechanism, for all nine sibling
   shipments — `134-S` through `142-S` — until `142-F` becomes fully
   covered by whichever ships last, or `backlogit` changes this behavior).
3. Do **not** claim `134-S` until `133-S` reaches a genuinely
   shipped/archived terminal state and this closure PR (if any repository
   state change requires one) is merged.

## Round 5 Copilot review summary (PR #377, HEAD `a81ce1a8`)

Five findings, all substantive (not cosmetic): (1-3) three threads on the
same root cause — the "remove `142-F` from the manifest" remediation
recommended by stash `28C0E138`/`F9D1C495` is factually incorrect;
verified directly against `backlogit` source and corrected throughout this
memory file, the closure doc, and a new stash entry `F9767C12`. (4)
`docs/architecture.md` overstated `DaemonMode` production wiring; corrected
after confirming (full-repo grep) zero production call sites exist outside
unit tests. (5) The runtime-verification doc mislabeled F02/F03's real
mode-contract unit suites as inert placeholders and never actually ran
them; corrected by running `unit_plugin_config` (21/21 passed) and
`unit_app_state_mode` (6/6 passed) and updating the Probe Outcomes table.
