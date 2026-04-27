---
title: "Compacted: Autoharness v1.3.2 Tune — PRs #30, #31 (TUNE-001..017)"
compacted_from:
  - docs/memory/2026-04-26/autoharness-tune-pr-memory.md
  - docs/memory/2026-04-26/autoharness-tune-post-merge-memory.md
  - docs/memory/2026-04-27/pr-31-merge-memory.md
date: 2026-04-27
status: complete
---

## Summary

First autoharness v1.3.2 tune pass (TUNE-001..017) across two PRs:
- **PR #30** (`chore/autoharness-tune-2026-04-26`) — merge commit `3a0def0`
- **PR #31** (`post-merge/autoharness-tune-2026-04-26`) — merge commit `2b8dc68`

## Files Changed

- `.github/agents/stage.agent.md`, `ship.agent.md`, `pr-lifecycle.md` — v1.3.2 alignment
- `.github/instructions/architecture-doc.instructions.md` — clarified `docs/research/` purpose
- `.github/agents/ship.agent.md` step 6.7 — rewrote to use only supported backlog operations
- `.autoharness/harness-manifest.yaml` — TUNE-001..017 recorded
- `.autoharness/tuning-reports/2026-04-26-tuning-report.md` — new file (machine paths redacted)
- `agent-engram.code-workspace`, `start.ps1` — workspace startup and agent injection fixes

## Key Decisions

- `--admin` merge used for both PRs (Copilot review submits `COMMENTED`, never `APPROVED`)
- Unrelated integration test failure (`daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted`) excluded from scope — PR modifies only agent/harness files, no Rust source
- `start.ps1` defaulted `COPILOT_HOME` to workspace-local `.copilot/` directory
- `git revert <merge-sha>` → must use `git revert --no-edit -m 1 <merge-sha>` for merge commits

## Copilot Review

- PR #30: 7 comments fixed (commit `bc9df10`), 4 follow-up comments fixed — all threads resolved
- PR #31: 2 comments fixed (commit `4116215`), no new findings on re-review — all threads resolved

## Compound Entries

No new compound entries from this pass — all findings were instruction-alignment corrections.

## Recurring Pattern

`git revert --no-edit -m 1 <merge-sha>` pattern appeared in 4 closure artifacts across this session. Compound entry recommended if it recurs.
