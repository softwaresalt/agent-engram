---
title: Auto-Tune PR #39 — Post-Merge Closure Memory
date: 2026-04-28
session: c0c7e503-873e-4654-a5ab-5914f4585e57
pr: 39
merge_sha: b7a34b5
branch: chore/autoharness-tune-2026-04-28
---

## What Was Accomplished

Executed a full Auto-Tune pass (TUNE-026..031) on the agent-engram workspace,
resolving 21 targeted harness-drift checks that had accumulated since the last
tune run.

### Commits Merged

- `052d744` — chore(agents): autoharness tune — TUNE-026..031
- `f0af80b` — fix(agents): address copilot review on PR #39

### Files Modified

| File | Change |
|---|---|
| `.github/agents/auto-tune.agent.md` | TUNE-026: added `learning_signals{}` field read to Step 5 |
| `.github/agents/ship.agent.md` | TUNE-027: added `source_deliberation_id` field read; clarified stash retirement is manual-only |
| `.github/skills/operational-closure/SKILL.md` | TUNE-028: added Step 5 Source artifact cleanup section; clarified manual-only stash retirement |
| `AGENTS.md` | TUNE-029: added `backlogit_get_metadata_catalog`, `backlogit_export_command_map` to backlogit overlay |
| `.autoharness/harness-manifest.yaml` | TUNE-030: added `autoharness_version`, `profile_hash`, `primitives_installed`; bumped `tuned_at` to 2026-04-28; added TUNE-026..031 to `last_tune_applied` |
| `.autoharness/workspace-profile.yaml` | TUNE-031: fixed enum violations (mcp_transport, recommended_reviewer, tool_type, features structure, harness_recommendations); added missing capability packs (agent-intercom, agent-engram, backlogit) |
| `.gitignore` | Added `.autoharness/tuning-reports/*.json` to prevent machine-specific path leakage |
| `.autoharness/tuning-reports/verify-*.json` | Removed from git tracking (machine-specific paths); verify-latest.json renamed to verify-pre-tune.json |

## Decisions Made

1. **Stash retirement clarification**: Changed all references to stash retirement
   from implying an automated flow to explicitly noting it is manual-only because
   `backlogit_stash_remove` is not in the installed registry.

2. **Tuning report JSON files are gitignored**: These files contain machine-specific
   Windows paths including username (`derek.williams` in `autoharness_home`).
   Added `.autoharness/tuning-reports/*.json` to `.gitignore`.

3. **Capability packs list was incomplete**: `workspace-profile.yaml`'s
   `harness_recommendations.capability_packs` was missing `agent-intercom`,
   `agent-engram`, and `backlogit` — the three primary active capability packs.
   All three were added.

4. **15 deferred schema blockers**: Structural type mismatches in workspace-profile.yaml
   (languages as objects vs strings, frameworks as array vs object, etc.) require
   workspace-discovery regeneration, not manual fixing. Deferred to next Auto-MergeInstall
   run.

## Verify Results (Post-Tune)

- 21/21 targeted checks: PASS
- 15 structural blockers: deferred (schema-level, no agent behavior impact)
- CI: both cozo-backend (56s) and surreal-backend (8m9s) green

## Copilot Review Comments (5 total)

All 5 addressed, replied to, and threads resolved before merge:

| Comment ID | File | Fix |
|---|---|---|
| 3156693267 | operational-closure/SKILL.md:101 | Clarified manual-only stash retirement |
| 3156693305 | workspace-profile.yaml:372 | Added 3 missing capability packs |
| 3156693317 | verify-post-tune.json:9 | Gitignored all tuning report JSON files |
| 3156693331 | verify-latest.json:17 | Renamed to verify-pre-tune.json + gitignored |
| 3156693356 | ship.agent.md:359 | Clarified manual-only stash retirement |

## Failed Approaches

- `Rename-Item` failed on the JSON file — had to use `Move-Item` instead on Windows
- `gh pr edit --add-reviewer "copilot"` does not work; must use
  `gh api repos/.../pulls/39/requested_reviewers -X POST -f "reviewers[]=copilot"`

## Open Items / Follow-Up

1. **15 deferred schema blockers** in workspace-profile.yaml need workspace-discovery
   regeneration via Auto-MergeInstall (not manual fixing)
2. **Next tune**: Recommend after next major feature merge or in ~30 days
3. **Backlog**: No backlogit items were associated with this tuning run (ad-hoc maintenance)

## Next Session Handoff

- `main` is clean and up-to-date at `b7a34b5`
- All feature branches for this session are merged
- No open PRs pending
- Queue items: check backlogit for any pending work items
