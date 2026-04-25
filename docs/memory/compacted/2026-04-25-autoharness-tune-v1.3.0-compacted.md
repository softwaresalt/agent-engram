---
type: compacted-memory
source_files:
  - docs/memory/2026-04-25/tune-session-memory.md
  - docs/memory/2026-04-25/autoharness-tune-v1.3.0-memory.md
compacted_at: 2026-04-25
pr: 25
merge_commit: 36ce9d2b83f2f0f6b1fe3a898c67b72d82ad34ff
status: shipped
---

# Compacted: autoharness v1.3.0 Tune Session (2026-04-25)

## Outcome

PR #25 merged to `main` as `36ce9d2b`. All CI checks green. 11 Copilot review
comments resolved across 3 rounds.

## What Was Built

Full harness tune pass aligning workspace to autoharness v1.3.0 templates.
44 files changed (+4,719 / -156). Additionally fixed two pre-existing
`collapsible_match` CI failures in `src/services/parsing/markdown.rs`
(Rust 1.95 / `--all-targets` surfaced them; Rust 1.85 local toolchain did not).

### New capabilities activated

- **Session-start checkpoint recovery**: Stage/Ship poll `backlogit_list_checkpoints`
  on startup and offer to resume from a prior checkpoint.
- **Hook event consumption**: Agents poll `backlogit_poll_hook_events` before
  triage to handle `feature_review_ready` and `post_merge_closure` signals.
- **SQL schema reference**: `backlogit-sql-schema.instructions.md` teaches agents
  the table structure for targeted `backlogit_query_sql` queries.
- **YAML header tooling**: `backlogit-yaml-header-tooling.instructions.md`
  documents the `title:` field requirement (items invisible to index without it).

### Key files changed

| Area | Change |
|---|---|
| `.autoharness/backlog-registry.yaml` | +5 ops: list/get/resolve_checkpoint, poll/ack_hook_events |
| `.github/agents/stage.agent.md` `.github/agents/ship.agent.md` | Surgically patched with 3 new v1.3.0 sections |
| 9 skills regenerated | deliberate, spike, shipment-reconcile, harvest, review, plan-review, compact-context, harness-architect, build-feature |
| 4 instructions regenerated | architecture-doc, continuous-learning, strict-safety, circuit-breaker |
| `src/services/parsing/markdown.rs` | Fixed `collapsible_match` on heading/code-block end arms |
| `start.ps1` | Fixed `$local_copilot`, `@args`, exe resolution, removed PS7-only `??` |
| `.gitignore` | Wildcard `.github/local-agents/` |

### Preserved (not regenerated) — 11 files
backlogit.instructions, backlog-integration.instructions (updated separately),
rust-reviewer.agent, technology-rust.instructions, mcp-server.instructions,
concurrency-reviewer.agent, copilot-instructions, AGENTS.md,
constitution.instructions, learn SKILL, observe SKILL

## Key Decisions

1. **Surgical patch** of stage/ship agents (not full regeneration) — both had
   extensive local customization; 3 new sections inserted at correct location.
2. **Runtime variables left as `{{VAR}}`** — install-time variables resolved;
   runtime slots like `{{NAME}}`, `{{TITLE}}` intentionally preserved in skill templates.
3. **`autoharness verify-workspace --json` is broken in v1.3.0** — crashes with
   `AttributeError: 'str' object has no attribute 'get'` at `verify_workspace.py:917`.
   Workaround: manual drift analysis. Not yet filed upstream.

## Failed Approaches

- `autoharness verify-workspace --json` — crashes in v1.3.0 (scripts-section bug)
- `gh pr edit --add-reviewer "copilot"` — not supported; Copilot review triggered
  automatically by push instead
- `??` (null-coalescing) in `start.ps1` — PS 7+ only; replaced with `if/else`

## CI Fix Context

Rust version mismatch: local 1.85 vs CI 1.95 (`cozo-backend` target).
- `collapsible_match` fires in 1.95 on `if bool_var { }` inside a match arm
  even when the bool is not a pattern. Fix: use match guard (`Event::End(X) if flag =>`).
- Always run `cargo clippy --all-targets` locally — without `--all-targets`, test
  files are skipped and CI-only lint errors are missed.

## Follow-Up Items (stashed)

1. Disable `allow_rebase_merge` on repo to fully satisfy P-009
2. Fix `.mcp.json` workspace paths (`D:\GitHub\` → `D:\Source\GitHub\`)
3. Remove or externalize Tavily API key from `.mcp.json`

## Closure Artifact

`docs/closure/2026-04-25-autoharness-tune-v1.3.0-closure.md`
