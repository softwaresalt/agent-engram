# Session Memory — Harness Tune autoharness v1.3.0

**Date**: 2026-04-25
**Branch**: `chore/autoharness-tune-v1.3.0`
**PR**: [#25](https://github.com/softwaresalt/agent-engram/pull/25)
**Status**: PR open, awaiting review and merge

---

## What Was Done

Full harness tune pass against autoharness v1.3.0 templates. 35 files changed (+4,552 / -65).

### Files Modified

| File | Change |
|------|--------|
| `.autoharness/backlog-registry.yaml` | +5 new ops (list/get/resolve_checkpoint, poll/ack_hook_events) |
| `.github/agents/stage.agent.md` | +session recovery, hook events, expanded checkpoints |
| `.github/agents/ship.agent.md` | Same 3 sections as stage |
| `.github/instructions/backlogit-sql-schema.instructions.md` | NEW — SQLite schema reference |
| `.github/instructions/backlogit-yaml-header-tooling.instructions.md` | NEW — YAML field→tool mapping |
| `.github/instructions/backlog-integration.instructions.md` | +5 rows in Extended Operations table |
| `.autoharness/harness-manifest.yaml` | Updated tuned_at, +2 new entries, verification metadata |

### Skills Regenerated (9)
deliberate, spike, shipment-reconcile, harvest, review, plan-review, compact-context, harness-architect, build-feature

### Instructions Regenerated (4)
architecture-doc, continuous-learning, strict-safety, circuit-breaker

### Backups
18 files backed up to `.autoharness/backups/2026-04-25-v1.3.0/`

### Preserved (Not Regenerated) — 11 files
backlogit.instructions, backlog-integration.instructions (updated separately), rust-reviewer.agent, technology-rust.instructions, mcp-server.instructions, concurrency-reviewer.agent, copilot-instructions, AGENTS.md, constitution.instructions, learn SKILL, observe SKILL

---

## Key Decisions

1. **Surgical patch of stage/ship agents** instead of full regeneration — both files have extensive local customization. The 3 new sections were inserted at the correct location (session context blocks) without disturbing existing content.
2. **Variable substitution map maintained** — all install-time variables were resolved; runtime variables (NAME, TITLE, etc.) intentionally left as `{{VAR}}` in skill templates.
3. **`autoharness verify-workspace --json` is broken in v1.3.0** — crashes with `AttributeError: 'str' object has no attribute 'get'` at `verify_workspace.py:917` when the manifest `scripts` section contains string values. Workaround: manual drift analysis.

---

## New v1.3.0 Features Now Active

- **Session-start recovery**: Stage/Ship agents poll `backlogit_list_checkpoints` on startup and offer to resume from a prior checkpoint
- **Hook event consumption**: Agents poll `backlogit_poll_hook_events` before triage to handle signals like `feature_review_ready` and `blocked_stale`
- **SQL schema reference**: Agents can now write targeted `backlogit_query_sql` queries using the schema reference
- **YAML header tooling**: The `title:` field issue (items invisible to backlogit index) is now formally documented in instructions

---

## Outstanding Operator Actions

1. **`.mcp.json` workspace paths** — still shows `D:\GitHub\` instead of `D:\Source\GitHub\`. Must be fixed manually (contains API key, excluded from commits).
2. **Tavily API key in `.mcp.json`** — should be externalized to env var.
3. **Copilot Review on PR #25** — `gh pr edit --add-reviewer copilot` not supported in this gh version. Add via browser: https://github.com/softwaresalt/agent-engram/pull/25

---

## Failed Approaches

- `autoharness verify-workspace --json` — crashes in v1.3.0, bug in scripts-section handling
- `gh pr edit --add-reviewer "copilot"` — not supported, returns `'' not found`
- `gh pr request-review 25 --reviewer "copilot"` — unknown flag `--reviewer`

---

## Next Steps for Next Session

1. Merge PR #25 after review
2. On merge: invoke `compound-refresh` (15 compound library entries may need consolidation/tagging)
3. Fix `.mcp.json` paths and API key handling
4. Consider creating a recurring backlog task for monthly harness tuning
