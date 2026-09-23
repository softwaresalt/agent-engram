---
title: "141-S error transport, response provenance, lifecycle policy, generation observability — compacted memory"
release_unit: "141-S"
compacted_date: "2026-09-23"
source_files:
  - "docs/memory/2026-09-20-ship-141-s-implementation-and-remediation.md"
  - "docs/memory/2026-09-20-ship-141-s-copilot-review-and-ci-infra.md"
  - "docs/memory/2026-09-20/141-s-review-followups-memory.md"
  - "docs/memory/2026-09-20/142.052-T-read-server-lifecycle-policy-memory.md"
  - "docs/memory/2026-09-21-ship-141-s-pr407-authorized-rerun-passed.md"
archived_originals: "docs/archive/memory/2026-09-23/"
---

## Outcome

Shipment `141-S` shipped and closed. PR #407 merged via merge commit
`f115835e260c089bf094d82fa565371077f5bac8` at 2026-09-23T16:09:51Z.
Shipment archived (`status: archived`/`archived_status: done`) via the
established manual safe-close workaround (P-015 partial-feature, 8th
consecutive shipment to reuse this pattern under covering feature `142-F`).

## Tasks completed (all 7, TDD, individually committed and archived)

| Task | Summary | Commit |
|---|---|---|
| `142.047-T` | Carry the domain error envelope over IPC | `fd30af4e` |
| `142.048-T` | Remove lossy IPC error conversion | `2ebf70cb` |
| `142.049-T` | Add MCP structured success and error transport | `be2d8e94` |
| `142.050-T` | Add CLI JSON success and error transport | `207cc8db` |
| `142.051-T` | Decorate responses with captured-context provenance | `dcc192b5` |
| `142.052-T` | Enforce read-server lifecycle policy | `047a81a8` |
| `142.053-T` | Report generation observability without deletion | `6da09890` |

`142.052-T` implementation notes: read-server startup skips managed
hydration/offline-scan/watcher-registration/implicit-sync/shutdown-flush
via new lifecycle activity counters in `lifecycle_policy.rs`; watcher
policy decision delegated from `watcher.rs` rather than added to
`ipc_server.rs`. `src/daemon/ipc_server.rs` deliberately untouched.

## Review gate and remediation

7-persona local adversarial review, first round: 4/7 `BLOCKED`, 3/7
`READY_WITH_FOLLOWUPS`, converging on one shared theme — production IPC
dispatch never wires captured-context/generation-activator plumbing into
real traffic. Investigation (git-blame vs. baseline `655d19e7`) confirmed
this wiring gap, plus a read-server `connect_db` tension, are **all
pre-existing**, not introduced by 141-S, and fixing them requires files
none of the 7 tasks own or new infrastructure design decisions. Captured
as P-021 C2 deferred stash entries (all still active/unharvested as of
closure):

- `6C5DF765` (high, bug) — `process_request` never calls
  `admit_read`/`ReadAdmission`; provenance decoration (142.051-T) and
  read-admission enforcement (142.052-T) unreachable for real traffic.
- `9B7EC1E4` (high, feature) — `run_read_server_startup` never installs a
  real `GenerationActivator`; generation observability (142.053-T) always
  `null` in a live ReadServer daemon.
- `4628001C` (medium, bug) — `get_workspace_status`'s code_graph stats
  block unconditionally calls `connect_db`, in tension with 142.052-T's
  no-hydration policy; pre-existing accepted tradeoff.

4 genuinely in-scope findings fixed directly:
1. Watcher mode threaded through resolved `DaemonMode` instead of
   re-reading disk (`1f42268a`).
2. `ObservabilityState` made atomic + `runtime_copies` pruning
   (`fd397962`, `bf061adf`).
3. CLI's duplicate IPC-error-translation logic consolidated
   (`f26f9f7e`).
4. Transport-vs-domain error envelope doc comments clarified
   (`bc1073d1`).

Scoped Correctness-Reviewer re-review at HEAD `4b023489`: `READY_WITH_FOLLOWUPS`,
confirming fixes correct, deferrals sound. One P3 advisory (MCP
`translate_ipc_response` → `isError: true` `CallToolResult` pattern,
legitimate/test-covered but worth a release-note callout) carried as PR
follow-up handling, not a new stash capture.

## Copilot review + CI cycle (PR #407)

4 Copilot review threads: 2 resolved by reference to the pre-existing
deferred stash entries above (no code change), 2 genuinely new in-scope
findings fixed:
- `activation.rs::publish_active` torn-observability window closed by
  holding the write-lock across pointer swap + `note_active_generation`
  (`743064fa` + regression test).
- `activation.rs::resolve_and_open` disk-usage over-report fixed —
  `retained_runtime_copies` now reports actual runtime-copy file size via
  `fs::metadata` instead of the whole sealed-inventory total (`f6c49b49`,
  fmt fixup `d306e989`).

Second scoped Correctness-Reviewer pass (delta `8c7b758f..d306e989`):
`READY`. P-018 gate: `SATISFIED` at HEAD `d306e989`, 0 unresolved threads.

**CI runner infrastructure degradation**: `start-launcher-windows` hung
repeatedly across 8 cancel/rerun cycles (progressively earlier hang
points each time, including pre-checkout steps — strong evidence of
GitHub Actions Windows-runner pool degradation, not a code regression).
Operator explicitly authorized exactly one additional rerun
(`gh run rerun 35548357399 --failed` → job `106480592120`). Result:
**PASS** (2m11s) — the extended `in_progress` API observation during
polling was severe step-status display lag (confirmed via a no-op
`gh run cancel` returning "Cannot cancel a workflow run that is
completed"), not a real hang. `build` unaffected (unchanged `success`,
6m18s). PR body's stale Local Review Readiness block (still referencing
HEAD `4b023489`) updated to reflect final HEAD `0bcabd0a`, both Copilot
fixes, and the final green CI outcome — a metadata-only edit, did not
advance HEAD.

## Merge and closure

Operator explicit approval ("PR 407: Merge approved",
2026-09-23T09:08:26.508-07:00). All last-mile gates re-verified fresh at
HEAD `0bcabd0a` immediately before merge: P-014 (readiness block present,
current HEAD, `READY_WITH_FOLLOWUPS`/P0=0,P1=0), P-018 (`SATISFIED`, 0
unresolved), CI green, `MERGEABLE`/`CLEAN`, P-009 (repo confirmed
merge-commit-only: `allow_squash_merge=false`,
`allow_rebase_merge=false`, `allow_merge_commit=true`), P-016 (topology
gate exit 0), scope/secrets scan clean. Merged via `gh pr merge 407
--merge`. Merge SHA `f115835e260c089bf094d82fa565371077f5bac8`, confirmed
reachable from `origin/main` via `merge-base --is-ancestor`.

Post-merge: `post-merge/141-s-...` closure branch created (the sole
correct location for closure commits, per Step 6.0 NON-NEGOTIABLE — an
initial mistaken direct-to-`main` archival commit was caught before push
and relocated onto this branch via `git branch` + `git reset --hard
origin/main` on `main`, no data lost). Shipment `141-S` safe-closed
(manual archive-file workaround, 8th consecutive reuse under covering
feature `142-F`; `backlogit shipment ship` known-hanging, generic
`backlogit move --status shipped` fallback confirmed CLI-rejected exit 9).
Covering feature `142-F` verified untouched (SHA-256
`59263E8FFB779485E135A7AA41D9DAAC89B4A996B767D128D76A1AD2E70404C3`, 802
bytes, unchanged since `138-S`). Pre/post reconciliation both `PROCEED`.
Runtime verification: `PASS_WITH_FOLLOW_UP`. Operational closure:
`READY_WITH_CONDITIONS` (conditioned on the 2 deferred production-wiring
follow-ups `6C5DF765`/`9B7EC1E4`). Compound addendum added recording
141-S as the 8th shipment reusing the manual safe-close workaround.

## Constraints preserved throughout

Shipment `142-S` and stash `0011C2DC` / branch
`docs/stage-0011c2dc-content-record-schema` never touched. No merge
without explicit operator approval; no `--admin` bypass ever used. All
P-021 deferred stash entries remain single-write (cited only, never
edited).
