# Ship Session Memory — PR #186 Copilot Fix (verify 064-F spurious link) — Pre-Merge (operator-gated)

**Date**: 2026-07-02
**PR**: #186 — `chore/backlog-064-reconcile` → `main` (OPEN, operator-gated; Ship did NOT merge)
**Base HEAD before**: `56905b0` → **New HEAD**: `940aceb6260b4c9cc78c324b4b4ad72b3a648646`
**Status**: FIX SHIPPED TO BRANCH; PR left OPEN awaiting operator approval/merge.

---

## Scope

Address ONE Copilot review finding on PR #186 (the only open PR — strict
one-PR-at-a-time policy honored; closed PR #187 untouched).

- Thread `PRRT_kwDORJEduc6NztmX` · comment `3511097151` ·
  author `copilot-pull-request-reviewer` · `.backlogit/queue/064-F.md:17`.
- Finding (VALID): the branch had ADDED a spurious `informs` link from verify
  feature `064-F` to `062.003-T` — a reconcile leftover from the old TMDL
  `064-*` lineage. Verify `064-F` (deterministic gates & telemetry; stash
  `B87680AB`, deliberation `011-D`) has no relationship to the PBIP/TMDL task
  `062.003-T`, whose `informs` relationship correctly lives on the re-IDed TMDL
  feature `066-F`.

## Fix (commit `940aceb`)

Removed exactly two lines under the `links:` block of `.backlogit/queue/064-F.md`
(`- target_id: 062.003-T` / `link_type: informs`), leaving only the correct
`- target_id: 011-D` / `related_to` entry. `git diff --cached --stat` = single
file, 1 insertion / 3 deletions (2 link lines + a benign `updated_at`
re-serialization bump from backlogit sync). No consistency fix was needed:
`062.003-T` has no `links:` back-ref to `064-F` (only a narrative "reconciled
from 064-F" mention), and `066-F.md` already carries the legit
`062.003-T informs` edge.

## Hard-Won Landmine: `backlogit sync` unions stale cache back INTO markdown

Editing the markdown to remove the link and running `backlogit sync` did NOT
stick — sync RE-ADDED the link to the file (and bumped `updated_at`). Root cause:

- The `links:` block is materialized in the SQLite cache `item_links`
  (`.backlogit/backlogit.db`, gitignored/disposable). `sync` reconciles as a
  UNION of markdown + existing cache rows and writes the result BACK to the
  markdown, so a stale cache row resurrects a link deleted from source.
- Worse: **6 orphaned `backlogit mcp` stdio servers** (from 6/30–7/1) held the
  DB open, so the disposable cache could not be deleted/rebuilt cleanly.

**Durable fix procedure** (matches sync's documented "force the disposable
cache to match the file-backed source of truth"):
1. Re-remove the link from markdown.
2. `Stop-Process -Id` each orphaned `backlogit mcp` PID (28496, 29212, 30712,
   27036, 18420, 14944) to release the DB lock. (CLI-only workflow — no MCP
   tools in use, so no impact; live IDE clients respawn on demand.)
3. Delete `.backlogit/backlogit.db{,-wal,-shm}` (gitignored cache).
4. `backlogit sync` → fresh rebuild from markdown-only → link stays removed;
   `item_links` now shows only the legit `066-F → informs → 062.003-T`.
5. `backlogit doctor` → identical 43 pre-existing `archived_from_self_ref`
   warnings; ZERO `duplicate_id` / `root_id_collision`; no new orphan/back-ref
   for `064-F` or `062.003-T`.

## Reply + Resolve

- Copilot auto-resolved thread `PRRT_kwDORJEduc6NztmX` (`resolvedBy: Copilot`,
  `isOutdated: true`) once commit `940aceb` changed that exact line.
- Posted a documenting reply (comment `3513707631`, endpoint
  `pulls/186/comments/3511097151/replies` via `gh api -f "body=..."` — NOT
  `@file`; re-read confirmed clean rendering, backticks intact, no literal-text
  artifact).
- Ran `resolveReviewThread` idempotently → `isResolved: true`.
- Final PR thread scan: **total 1, unresolved 0.**

## Fresh Copilot re-review (post-push)

Review `4618299619` (COMMENTED, 14:12:26Z): "reviewed 20 of 20 files, generated
no new comments." Two LOW-CONFIDENCE SUPPRESSED (not posted) observations, both
OUT OF SCOPE for this task and pre-existing from the earlier reconcile commits:

- `.backlogit/archive/066-F.md:3` — missing `archived_from` frontmatter.
- `.backlogit/archive/052-S.md:3` — missing `archived_from` frontmatter.

Left untouched (discipline: only `064-F.md` may change). **Flag for
Stage/operator**: consider a follow-on to add `archived_from` to
`archive/066-F.md` and `archive/052-S.md` for provenance consistency (contrast
`archive/062.003-T.md`, `archive/020-S.md`, `archive/066-S.md`).

## Final State

- New commit: `940aceb6260b4c9cc78c324b4b4ad72b3a648646` (pushed to
  `origin/chore/backlog-064-reconcile`).
- CI run `28596505459` on `940aceb`: **success** — fmt, clippy, test, audit all
  green (backlog-doc-only change did not affect Rust gates).
- PR #186: `state OPEN`, `mergeable MERGEABLE`,
  `mergeStateStatus BLOCKED` due solely to `reviewDecision REVIEW_REQUIRED`
  (the operator approval gate). **STOPPED before merge (operator-gated).**
- `headRefOid`: `940aceb6260b4c9cc78c324b4b4ad72b3a648646`.

## Working-Tree Drift (left untouched — 065 branch)

On session start the worktree was on `065-daemonless-direct-docs` with drift
(`.backlogit/memories.json`, `.cursor/mcp.json`, `.github/copilot-instructions.md`
modified; untracked `.backlogit/telemetry.jsonl`, `.claude/`,
`docs/design-docs/.gitkeep`). Stashed to get a clean 064 worktree, then restored
to the original branch afterward. This memory note itself is left UNTRACKED
(not committed — PR must contain only `064-F.md`).
