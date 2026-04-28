---
type: compacted-memory
source_files:
  - docs/memory/2026-04-28/auto-tune-pr39-closure-memory.md
date: 2026-04-28
release_unit: auto-tune-pr39
pr: 39
merge_sha: b7a34b5
---

# Compacted Memory — Auto-Tune PR #39 (TUNE-026..031)

## What Was Done

Full Auto-Tune pass on the agent-engram workspace. Resolved 21/21 targeted harness-drift
checks via TUNE-026..031. CI green, Copilot review (5 comments) addressed, PR #39 merged.

## Files Modified

| File | Change |
|---|---|
| `.github/agents/auto-tune.agent.md` | TUNE-026: `learning_signals{}` field read added to Step 5 |
| `.github/agents/ship.agent.md` | TUNE-027: `source_deliberation_id` read; stash retirement is manual-only (no `backlogit_stash_remove` in registry) |
| `.github/skills/operational-closure/SKILL.md` | TUNE-028: Step 5 Source artifact cleanup section; manual-only stash retirement clarified |
| `AGENTS.md` | TUNE-029: `backlogit_get_metadata_catalog`, `backlogit_export_command_map` added to backlogit overlay |
| `.autoharness/harness-manifest.yaml` | TUNE-030: added `autoharness_version`, `profile_hash`, `primitives_installed`; bumped `tuned_at`; logged TUNE-026..031 |
| `.autoharness/workspace-profile.yaml` | TUNE-031: fixed enum violations; added `agent-intercom`, `agent-engram`, `backlogit` to `capability_packs` |
| `.gitignore` | Added `.autoharness/tuning-reports/*.json` (machine-specific paths) |
| `verify-*.json` | Removed from git tracking; renamed `verify-latest.json` → `verify-pre-tune.json` |

## Key Decisions

1. **Stash retirement is manual-only** — `backlogit_stash_remove` is not in the installed
   registry. Both `ship.agent.md` and `operational-closure/SKILL.md` now say "for manual
   retirement; no automated retire operation exists in the installed registry."
2. **15 deferred schema blockers** — structural type mismatches in `workspace-profile.yaml`
   require workspace-discovery regeneration via Auto-MergeInstall, not manual fixing. No
   agent behavior impact. Deferred.
3. **Tuning report JSON files gitignored** — these embed a machine-specific user path
   in `autoharness_home` (e.g., `C:\Users\<user>\...`); `.autoharness/tuning-reports/*.json`
   added to `.gitignore`.

## Verify Results

- 21/21 targeted checks: PASS
- 15 structural blockers: deferred
- CI: cozo-backend (56s) ✅, surreal-backend (8m9s) ✅

## Copilot Review Comments Resolved

All 5 threads replied to and resolved programmatically via `gh api graphql`.

## Failed Approaches

- `Rename-Item` failed on Windows for JSON file rename → use `Move-Item` instead
- `gh pr edit --add-reviewer "copilot"` fails → use `gh api repos/.../requested_reviewers -X POST`

## Follow-Up

- 15 deferred schema blockers need Auto-MergeInstall regeneration
- Next tune: recommend ~30 days or after next major feature merge
