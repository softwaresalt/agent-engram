# Ship Session Memory — PR #188 verify hardening (PR #185 review follow-up) — Pre-Merge (operator-gated)

**Date**: 2026-07-02
**PR**: #188 — `064-verify-hardening` → `main` (OPEN, operator-gated; Ship did NOT merge)
**Follows**: PR #185 (MERGED `f3f7f2f`, branch pruned) — 5 leftover Copilot threads
**Base**: `main` @ `ad0b632` → branch HEAD `0a3b545`
**Task**: 064.004-T (Phase 1b hardening family, feature 064-F)
**Status**: FIXES SHIPPED TO BRANCH; PR OPEN awaiting operator approval/merge.

---

## Scope

The 5 unresolved Copilot review threads on the already-merged PR #185 could not
be fixed on #185 (branch pruned). Landed fixes/verifications on `main` via new
PR #188, then replied to + resolved each original #185 thread referencing the
new commits. Strict one-PR-at-a-time honored (opened exactly one PR).

## Thread outcomes (#185 → 0 unresolved)

- **[1] docs frontmatter** (`…NqWDp`) — FIXED (`5603865`): added YAML frontmatter
  to `2026-06-30-stage-B87680AB-session.md` mirroring sibling `…0E042A84…`.
- **[2] emit_summary "won't compile"** (`…NrAc0`) — FALSE POSITIVE: `json!`
  serializes by reference (`to_value(&expr)`); compiles on `main`, CI green.
- **[3] body.empty untested** (`…NrAdB`) — FIXED (`3908cb2`): added
  `empty_body_after_frontmatter_is_non_conformant` (coverage gap, GREEN).
- **[4] Windows verbatim containment** (`…NrQsy`) — FIXED (`dadf2da`, refined
  `0a3b545`): `contain_path` normalizes both sides for the containment
  comparison. RED→GREEN via `absolute_missing_path_inside_workspace_passes_containment`.
- **[5] CRLF frontmatter** (`…NrdMY`) — FALSE POSITIVE (empirical): CRLF +
  malformed frontmatter returns non-conformant because `str::lines()` strips the
  trailing `\r`. Kept `crlf_malformed_frontmatter_is_non_conformant` as a guard.

## RED→GREEN evidence ([4])

Before fix (Windows): `contain_path` on an absolute in-workspace **missing** path
returned `path '…\missing.md' is outside the workspace root` (misclassified).
After fix: passes containment; missing file correctly caught as exit-2 read error.

## PR #188 Copilot re-review (cycle 1)

- `verify.rs:189` — FIXED (`0a3b545`): keep verbatim `read` path for I/O
  (extended-length support), normalize only for the containment comparison.
- `docs:14` (H1 + frontmatter title) — DECLINED by convention (siblings pair
  `title:` with an H1; removing the H1 would alter body + break consistency).

Both #188 threads resolved → 0 unresolved.

## Gates / CI

fmt ✓ · clippy pedantic ✓ (0 warnings) · verify tests ✓ (unit 6 / contract 4 /
integration 6, pinned exit-code contract intact) · CI `build` green on Linux.
Local Windows full-suite flakes (backlog perf, daemon TTL, installer os-error-740
requires-elevation) are pre-existing + environment-induced, pass in isolation /
on CI, unrelated to this diff.

## Hygiene

Committed ONLY hardening changes: `src/cli/commands/verify.rs`,
`tests/unit/verify_core_test.rs`, `docs/memory/2026-06-30-stage-B87680AB-session.md`,
plus this closure/memory. Pre-existing `.backlogit/*` debt, untracked 067/telemetry
planning artifacts, `.cursor/mcp.json`, `.github/copilot-instructions.md`,
`.claude/` drift left untouched (explicit pathspecs; verified `git diff --cached
--stat` before each commit). No `backlogit` mutation (see closure note rationale).

## Reply mechanics

Used `gh api .../pulls/<n>/comments/<dbid>/replies -f "body=$var"` (raw field from
a real file read via `Get-Content -Raw`). Never `gh -b @file`. Re-read each posted
body to confirm rendering (no `@file` artifact). Resolved via
`gh api graphql … resolveReviewThread`, confirming `isResolved:true`.

## Merge gate

STOPPED. PR #188 open, 0 unresolved threads, green CI. Awaiting operator merge
approval. Ship did NOT merge.
