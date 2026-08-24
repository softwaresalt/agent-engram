---
title: "Dark-factory cycle 2 — six active bug Stage memory"
type: session-memory
date: 2026-08-24
agent: stage
branch: stage/dark-factory-cycle2-20260824-1540
base: 44a4324abbac5fefcb51b1362f37d48442e58a85
closure_pr: 362
---

# Dark-factory cycle 2 — six active bug Stage memory

## Scope and Boundaries

Staged exactly the six operator-selected active bug stashes. No source/test/config file was modified; no build, test, or linter ran; no shipment was claimed; no PR was created or modified. Closure PR #362 remained OPEN/CLEAN and separate. Shipment manifests 119-S, 122-S, and 123-S were read-only and byte-unmodified.

## Tool State

- Backlogit MCP was reachable, but bound to the default worktree. All semantic mutations used the registry CLI with `--cwd` against the dedicated Stage worktree.
- Initial default-worktree sync exposed the known 19-file stale-main parse failure. The dedicated origin/main worktree synced successfully with 0 parse failures.
- Engram MCP was unavailable; CLI daemon startup timed out and direct mode found the stuck daemon lock. After declared degradation, discovery used targeted backlogit queries, git history, dependency source reads, and narrow repository reads.
- Backlogit doctor found no new duplicate/orphan/partial-mutation blocker. Historical `archived_from` and shipped-event advisories remain; 119-S/122-S/123-S closure is owned by PR #362 and was not touched.

## Prioritization

| Rank | Stash | Priority | Rationale |
|---:|---|---|---|
| 1 | 1CB366DB | high | Main bind can compose canonical path/UUID/branch from different objects; highest correctness/security impact. |
| 2 | 7B15B447 | high | Same exploit family inside daemon-key selection and prerequisite for a sound combined bind API. |
| 3 | 49000348 | high | Potential legitimate-workspace rejection, but LOW confidence and unverifiable without real cloud provider state. |
| 4 | 44E573BC | medium | Deterministic optional-feature compile break and the only fully reviewable/dispatchable unit now. |
| 5 | 1C2A3CB3 | medium | Residual naming inconsistency, not false accept; spike found a safe API route. |
| 6 | 5DF94427 | medium | Narrow crash-durability risk that can remint identity; no direct attacker accept. |

Risk rank differs from execution order: 7B15B447 must precede 1CB366DB by dependency; 44E573BC is the only claimable shipment because all higher-risk identity plans are review- or environment-blocked.

## Decisions and Findings

1. `7B15B447` and `1CB366DB` are separate ordered release units, not one broad shipment. Deliberations `022-D` and `021-D` are accepted.
2. The safe Windows identity blocker was overturned: public `cap_fs_ext::MetadataExt` in pinned 4.0.2 exposes handle-derived `dev()/ino()` on Windows. ReFS 128-bit IDs remain an explicit caveat. Deliberation `024-D` is blocked only at adversarial review.
3. Parent-directory fsync is reachable safely through `Dir::reopen_dir`, `into_std_file`, and `sync_all`; no ambient path/unsafe required. Deliberation `025-D` is blocked at adversarial review.
4. Cloud placeholders require a disposable real OneDrive/Dropbox repository with dehydrated `.git` evidence. `023-D` and the spike remain blocked; 124-S is only an ordinary Windows control.
5. OTLP uses newer API names against pinned 0.26. The reviewed direction is code-to-pin adaptation, not a dependency upgrade.

## Created Artifacts

| Stash | Backlog IDs | Durable planning artifacts |
|---|---|---|
| 1CB366DB | 021-D accepted | authority decision; bind proof plan (standard PASS, adversarial BLOCKED) |
| 7B15B447 | 022-D accepted | authority decision; daemon-key CapRoot plan (standard PASS, adversarial BLOCKED) |
| 49000348 | 023-D blocked | cloud-placeholder spike (environment gate) |
| 1C2A3CB3 | 024-D blocked | safe-API spike; Windows identity plan (standard PASS, adversarial BLOCKED) |
| 5DF94427 | 025-D blocked | fsync decision; parent-fsync plan (standard PASS, adversarial BLOCKED) |
| 44E573BC | 131-F; 131.001-T; 131.002-T; 131.003-T; 125-S | OTLP decision and reviewed PASS plan |

## OTLP Harvest and Shipment

Hierarchy: `131-F` -> `131.001-T` RED -> `131.002-T` GREEN -> `131.003-T` verification. Every task has acceptance criteria, parent ID, exact width/time cap, and explicit RED/GREEN edge.

Step 5.5 scope guard PASS: harvest IDs were exactly `[131-F, 131.001-T, 131.002-T, 131.003-T]`; queued shipment `125-S` contains exactly those four IDs, parent first, with no pre-existing queue item. Only one independent shipment was produced, so operator batch/order/predecessor metadata is intentionally absent rather than hand-authored.

## Stash Disposition

`44E573BC` archived after reviewed plan, harvested hierarchy, and queued shipment existed. `1CB366DB`, `7B15B447`, `49000348`, `1C2A3CB3`, and `5DF94427` remain active for traceable blockers. No stash was removed destructively.

## Review Outcomes and Blockers

- 44E573BC: plan-review PASS; hardening correctly not required.
- 1CB366DB, 7B15B447, 1C2A3CB3, 5DF94427: hardening present; standard persona review PASS; adversarial multi-model gate BLOCKED because no independent cross-model dispatch surface exists. No harvest or shipment created.
- 49000348: investigation disposition BLOCKED on provider environment; no implementation plan/shipment.

## Publication and Next Step

Planning commit is pending at memory-write time and will be associated with `131-F` and its tasks before push. Branch publication is required; Stage must not create a PR. After PR #362 is merged separately and this Stage commit is integrated into main, the exact next Ship claim is `125-S`. Ship starts with `131.001-T` RED and must not select identity work from the remaining stashes.

## Files Modified

Only `.backlogit/` artifacts/logs/stash state and planning/memory Markdown under `docs/` were modified. No application source, tests, Cargo files, config, protected shipment manifest, or PR state changed.
