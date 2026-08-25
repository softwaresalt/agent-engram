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
3. Parent-directory fsync is reachable safely on Unix through `Dir::reopen_dir`, `into_std_file`, and `sync_all`; no ambient path/unsafe required. Current-HEAD PR review preserved Windows as an explicit residual because cap-std's read-only directory handle cannot satisfy `FlushFileBuffers` write access. Deliberation `025-D` is blocked at adversarial review.
4. Cloud placeholders require a disposable real OneDrive/Dropbox repository with dehydrated `.git` evidence. `023-D` and the spike remain blocked; 124-S is only an ordinary Windows control.
5. OTLP uses newer API names against the direct 0.26 stack, while `tracing-opentelemetry` 0.26 introduces an incompatible 0.25 type family. Current-HEAD PR review corrected the direction to an exact bridge alignment (`tracing-opentelemetry` 0.27) plus code-to-0.26 adaptation, not a broad telemetry upgrade.

## Created Artifacts

| Stash | Backlog IDs | Durable planning artifacts |
|---|---|---|
| 1CB366DB | 021-D accepted | authority decision; bind proof plan (standard PASS, adversarial BLOCKED) |
| 7B15B447 | 022-D accepted | authority decision; daemon-key CapRoot plan (standard PASS, adversarial BLOCKED) |
| 49000348 | 023-D blocked | cloud-placeholder spike (environment gate) |
| 1C2A3CB3 | 024-D blocked | safe-API spike; Windows identity plan (standard PASS, adversarial BLOCKED) |
| 5DF94427 | 025-D blocked | fsync decision; parent-fsync plan (standard PASS, adversarial BLOCKED) |
| 44E573BC | 131-F; 131.001-T through 131.007-T; 125-S | OTLP decision and reviewed, width-isolated PASS plan |

## OTLP Harvest and Shipment

Current hierarchy after PR #363 width remediation: `131-F` -> `131.001-T` RED -> `131.002-T` dependency GREEN -> `131.003-T` provider GREEN -> `131.004-T` attachment GREEN -> `131.005-T` shutdown GREEN -> `131.006-T` runtime VERIFY -> `131.007-T` quality VERIFY. Every task has acceptance criteria, parent ID, a <=105-minute estimate, fewer than 3 files, fewer than 5 functions, fewer than 4 scenarios, one skill domain, and an explicit prerequisite edge.

Step 5.5 scope guard PASS after re-harvest: shipment `125-S` contains exactly `[131-F, 131.001-T, 131.002-T, 131.003-T, 131.004-T, 131.005-T, 131.006-T, 131.007-T]`, parent first and dependency ordered. It remains the only queued shipment and is unclaimed.

## Stash Disposition

`44E573BC` archived after reviewed plan, harvested hierarchy, and queued shipment existed. `1CB366DB`, `7B15B447`, `49000348`, `1C2A3CB3`, and `5DF94427` remain active for traceable blockers. No stash was removed destructively.

## Review Outcomes and Blockers

- 44E573BC: PR review required lifecycle hardening after the initial plan review; hardening and the repeated standard plan review now PASS with retained provider ownership, deterministic emitted-span proof, and bounded shutdown/failure contracts.
- 1CB366DB, 7B15B447, 1C2A3CB3, 5DF94427: hardening present; standard persona review PASS; adversarial multi-model gate BLOCKED because no independent cross-model dispatch surface exists. No harvest or shipment created.
- 49000348: investigation disposition BLOCKED on provider environment; no implementation plan/shipment.

## Publication and Next Step

Planning commit is pending at memory-write time and will be associated with `131-F` and its tasks before push. Branch publication is required; Stage must not create a PR. After PR #362 is merged separately and this Stage commit is integrated into main, the exact next Ship claim is `125-S`. Ship starts with `131.001-T` RED and must not select identity work from the remaining stashes.

## Files Modified

Only `.backlogit/` artifacts/logs/stash state and planning/memory Markdown under `docs/` were modified. No application source, tests, Cargo files, config, protected shipment manifest, or PR state changed.

## Publication Result

Planning commit: `4cab736e05ebd832e048a0338949ef62c0089b2d`. It is associated with shipment `125-S`, feature `131-F`, and tasks `131.001-T` through `131.003-T`. A follow-up commit records these associations before branch push.

## Superseding PR #363 correction

A later configuration-only adversarial rerun did not provide authoritative
execution-model binding. The original stash IDs `7B15B447`, `1CB366DB`,
`1C2A3CB3`, and `5DF94427` are retained as archived harvest provenance, while
active replacements `172AE8CE`, `8C7733CE`, `721A42F0`, and `BD5DD62A` carry
the failed/unverified blocker. Features `132-F`–`135-F`, all child tasks, and
shipments `126-S`–`129-S` are blocked/non-executable.

Shipment `125-S` remains the only queued shipment from this cycle. Ship may not
claim it after review alone. Claim requires PR #362 and the exact final reviewed
PR #363 planning head on `origin/main`, clean review evidence for that exact
head, no active competing shipment, and the exact roster/dependencies. See
`docs/memory/2026-08-24/dark-factory-cycle5-adversarial-harvest-memory.md`.
