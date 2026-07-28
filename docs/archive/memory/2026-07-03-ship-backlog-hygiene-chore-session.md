# Ship session — 2026-07-03 — backlog-hygiene chore PR

## Scope
Land the accumulated post-merge backlog/doc bookkeeping debt as ONE
`chore(backlog)` PR against `main`. STOP at the merge gate (open PR, drive CI
green, do NOT merge). Strict one-PR-at-a-time: 0 open PRs at start; open exactly
one. No source-code changes.

## Preconditions (verified)
- Start on `main` at `c38b855`; `origin/main` identical; `git pull --ff-only`
  was a no-op ("Already up to date").
- Open PRs at start: `[]` (zero).
- PR #187 merged: SHA `ad0b63297e104054261abeb28aa9790a2b67dbd7`,
  mergedAt `2026-07-02T18:42:19Z`, mergedBy `softwaresalt`.
- PR #188 merged: SHA `c38b85519fae1585f8e7a4e399b3b0fad709894c`,
  mergedAt `2026-07-03T05:57:07Z` (== HEAD `c38b855`).

## Landmine honored
- Did **NOT** run `backlogit sync`. The working-tree `.backlogit/*.md` markdown
  is the source of truth (reflects all prior state transitions); `backlogit sync`
  would union the stale SQLite cache back over the correct markdown and revert
  the reconciliation. Staged markdown as-is. Never staged `.backlogit/backlogit.db*`
  (gitignored).

## Commit groups (branch `chore/backlog-hygiene` from `main`)
1. `chore(backlog)`: reconcile post-merge backlog state (053-S queue->archive;
   065-F archive->queue; 065.00X-T archived w/ merge SHA; 064-F 067 pivot note;
   064.004-T hardening-subset shipped note; stash + memories housekeeping).
   Note: `.backlogit/stash.jsonl` (non-archive) had EOL-only drift, no committable
   content diff — correctly a no-op.
2. `chore(backlog)`: add 067-F usage-telemetry-emit plan + items (012-D, 067-F,
   067-S, 067.001-004-T, exec-plan, decision-017, stage session memory).
3. `docs`: session memories + backlogit-sync compound learning (+ this note).
4. `docs(closure)`: reconcile merge_sha for 053-S (ad0b632) and PR #188 (c38b855)
   closures; 052-S already carried `merge_sha: f3f7f2f…` (no edit needed).
5. `chore(agents)`: carry forward operator install-aligned harness defs
   (orchestrator.agent.md, ship.agent.md, copilot-instructions.md).

## Left UNSTAGED (true local/env drift — never committed here)
`.cursor/mcp.json`, `.claude/`, `.backlogit/telemetry.jsonl`,
`docs/design-docs/.gitkeep` (and gitignored `.backlogit/backlogit.db*`).

## Merge gate
STOPPED at merge gate. PR opened against `main`; Copilot review requested; CI
driven green. Ship did NOT merge — awaiting operator approval.
