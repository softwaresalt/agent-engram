# Ship 134-S — PR #379 ready for operator merge decision

**Session outcome**: PR #379 brought to full current-HEAD readiness. Awaiting
explicit, separate operator approval to merge. Post-merge closure explicitly
NOT run this session per operator instruction.

**Authoritative HEAD note**: this checkpoint is a point-in-time session note,
not the system of record for "the current HEAD." Committing this file (and
the accompanying compound learnings entry) is itself a further commit on the
PR branch, which necessarily advances HEAD past whatever commit is described
below as "reviewed"/"final" at the time of writing — CI and the P-018
Copilot gate re-run on every push, including a docs-only one. **The PR #379
body is the always-current, authoritative readiness record**; if this file's
HEAD reference and the PR body ever disagree, the PR body wins. See the PR
for the true final HEAD and final gate results before acting on merge.

## Shipment scope

- Shipment: `134-S` (parent feature `142-F`, shared across 9 future
  shipments `134-S..142-S`)
- Depends on `133-S`, closed on `origin/main` at merge
  `0681c4984cb2849e1a318284b03cc19089726a91` (closure_status READY,
  compaction_status done, all 77 future members intact)
- Branch: `feat/134-s-ipc-seam-extraction-mode-constructor-migration-error-envelope-descriptor-schema`
- Manifest: 12 task items, all `done`
  - `142.003-T` (F38 error envelope), `142.005-T` + 3 subtasks (F19 tool
    descriptors), `142.008-T` + 4 subtasks (F04a IPC seam extraction),
    `142.009-T` (F04 AppState constructor migration), `142.010-T` (F05 shim
    restart mode propagation)

## PR state as of the code fix (HEAD `ed80b71d`, before this checkpoint's own commit)

- PR: #379 — `https://github.com/softwaresalt/agent-engram/pull/379`
- HEAD at that point: `ed80b71dcc3e9ae8bc398070f7a4bca15d2df564`
- State: OPEN, `mergeStateStatus: CLEAN`, `mergeable: MERGEABLE`
- CI: `build` PASS (6m9s), `start-launcher-windows` PASS (2m4s) — both green
- Copilot gate (P-018): `SATISFIED`, 0 unresolved threads, confirmed via
  `autoharness gate copilot-review` and an independent GraphQL query (all 6
  threads across 2 review rounds `isResolved: true`)
- Merge strategy (P-009): repo-level squash/rebase disabled, merge-commit
  only confirmed via `gh api repos/.../` (`allow_merge_commit: true`,
  `allow_squash_merge: false`, `allow_rebase_merge: false`)
- Local Review Readiness: `READY`, P0=0, P1=0, full build/clippy(pedantic
  default+git-graph)/fmt/dev-test evidence recorded in the PR body

This checkpoint's own commit (plus the compound learnings entry alongside
it) necessarily produces a new HEAD past `ed80b71d`. That new HEAD's CI and
Copilot gate results are tracked in the PR body, not repeated here — see
the "Authoritative HEAD note" above.

## Commit history this session (chronological)

1. `87174b38` — fail-closed mode resolution (Copilot findings 1+2, round 1)
2. `a4f0a46f` — multi-file `impl AppState` block scan fix (Copilot finding 3)
3. `35d8b082` — git-graph descriptor completeness guard (Copilot finding 4)
4. `a14c9836` — fail-closed on unreadable config.toml, not just malformed
   (Copilot finding 6, round 2)
5. `f406e8a6` — CI margin widened to +90s (Round 2 CI remediation —
   **later found to be a misdiagnosis**)
6. `ed80b71d` — **root-cause fix**: stopped the self-defeating idle-TTL
   probe livelock in `await_endpoint_released` (Round 3 CI remediation, the
   real fix)

## Root cause of the genuine CI-only hang (the key finding this session)

`read_server_mode_survives_auto_spawn_and_bounded_restart` hung to its full
timeout budget on Linux CI twice (50.26s/50s, then 110.088s/110s after a 3x
margin widening) while passing reliably (~42-46s) on Windows locally. Landing
almost exactly at the deadline both times — despite tripling the margin —
ruled out ordinary CI slowness.

Root cause: `await_endpoint_released`'s polling loop called
`endpoint_reachable()` → `probe()` every 100ms to detect the daemon releasing
its IPC endpoint. `probe()` performs a real connect to the Unix domain
socket / named pipe, and the daemon's `accept_loop` resets its own idle TTL
on **every accepted connection**, regardless of whether the client sends a
request before dropping the stream (`ipc_server.rs`, `ttl.reset()`, S046
contract). The test's own liveness polling was therefore perpetually
re-arming the very idle timer it was waiting to expire — a genuine
self-defeating livelock, not a flake. Linux CI's Unix domain socket
`accept()` succeeds essentially every time, so the daemon could never
accumulate a full 20s of true idle time. Windows only ever passed by chance,
when a scheduling gap between probes happened to exceed the timeout.

Fix (`ed80b71d`): `await_endpoint_released` now sleeps past the full idle
timeout **before** every probe (settle window = `idle_timeout + 10s`,
bounded to 3 attempts), so a probe can only ever observe genuine
non-termination, never induce it. Verified locally: the test now passes
deterministically in ~80s.

This is recorded as a reusable pattern for `docs/compound/` consideration:
*a reachability/liveness probe used as a release-detection polling
mechanism can itself prevent the condition being polled for, if the
observed system treats the act of checking as activity that resets its own
completion clock.*

## Quality gate evidence (code fix HEAD `ed80b71d`; unaffected by this
docs-only checkpoint commit)

- `cargo build --all-targets` — clean
- `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — clean
  (default features)
- `cargo clippy --all-targets --features git-graph -- -D warnings -D
  clippy::pedantic` — clean
- `cargo fmt --all -- --check` — clean
- `cargo dev-test --no-fail-fast` — all green except the single known
  pre-existing Windows-only flake `archive_verifier_runs_the_unpacked_native_binary`
  (stash `4EE241DC`, confirmed unrelated to this shipment's scope)
- `cargo test --test integration_read_server_restart` (targeted) — 3/3 pass,
  79.56s, deterministic

## Follow-ups stashed (P-021 C2, all pre-existing entries from earlier in
the session, no new entries created this segment)

- `4EE241DC` — pre-existing Windows-only flake
  `archive_verifier_runs_the_unpacked_native_binary`
- `E12542FF` — pre-existing `--all-features`/otlp-export build break in
  `src/server/observability.rs`
- `1918AFD2` — no IPC surface exposes daemon mode yet; Copilot finding 5
  deferred here (late-surfacing-thread reuse, no new entry)
- `AA5698E3` — stale `ship`-owned crash-recovery checkpoints + schema-legacy
  checkpoint validation anomalies found at session start

## Explicitly NOT done this session (per operator instruction)

- **No merge.** Awaiting separate, explicit operator approval.
- **No post-merge closure.** No `backlogit shipment ship` (cascade hazard:
  `142-F` is a shared parent across 9 shipments — safe-close via
  `shipment-reconcile` must be used instead when that stage is reached).
- No knowledge graduation, compound-refresh, or compact-context — all
  Step 6 activity deferred to a future post-merge session.

## Next steps (future session)

1. Operator reviews PR #379 and either approves merge or requests changes.
2. On approval: merge via merge-commit strategy (`gh pr merge 379 --merge`),
   re-running the last-mile P-018/P-009/§1.9 gates immediately beforehand
   per Ship Step 5 items 15-16 (re-check HEAD hasn't advanced, re-run
   Copilot gate unconditionally).
3. After merge is confirmed (`gh pr view 379 --json state,mergedAt,mergeCommit`
   showing `MERGED`, and the merge SHA verified as an ancestor of
   `origin/main`), run the full Step 6 post-merge closure protocol in a
   fresh session: post-merge closure branch, safe-close via
   `shipment-reconcile` (NOT `backlogit shipment ship`), operational-closure,
   knowledge graduation, compound-refresh, compact-context, backlog index
   resync.
