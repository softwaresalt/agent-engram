---
title: GSD v1.6.0 agents and skills merge-install
date: 2026-11-19T23:33:00Z
status: complete
---

## Outcome

Merge-installed the GSD v1.6.0 agents, subagents, skills, companion prompts,
core instructions, and carry-forward configuration into the workspace using
`.bin\gsd.exe` against `.gsd\gsd-assets-v1.6.0.zip`, while preserving all
workspace-authored custom artifacts and operator working-tree edits.

## Artifacts Added

* `.github/agents/subagents/evidence-consistency-reviewer.agent.md`
* `.github/skills/browser-automation/SKILL.md`
* `.github/skills/grill-me/SKILL.md`
* `.github/skills/gsd-dark-mode/SKILL.md`
* `.github/skills/gsd-show-dag-queue/SKILL.md`
* `.github/prompts/gsd-dark-mode.prompt.md`
* `.github/prompts/gsd-show-dag-queue.prompt.md`
* `.github/instructions/browser-verification.instructions.md`
* `.github/instructions/shipment-documentation-audit.instructions.md`
* `.gsd/carry-forward.yaml`

## Artifacts Updated and Preserved

* Re-rendered all 20 existing GSD-managed pipeline agents and subagents, 28
  existing GSD-managed skills, 4 prompts, and 31 workflow/coding instructions to
  match GSD v1.6.0 templates and the operator's `.gsd/config.yaml` model routing
  (`tier2`/`ship`: `mai-code-2`, `tier3`/`orchestrator`/`stage`:
  `gemini-4-carbon`, `stage`/`ship` escalation: `gpt-6.2-sol`).
* Registered global Copilot CLI agents and skills (`Auto-MergeInstall`,
  `Auto-Tune`, `install-harness`, `tune-harness`, `verify-harness`,
  `workspace-discovery`) in gitignored `.copilot/` via
  `gsd.exe setup-copilot-cli`.
* Backed up pre-update files to gitignored `.gsd/backups/2026-11-19/`.
* Preserved all 10 `preserved_artifacts` (`start.ps1` and `start.sh` restored
  from `main` to keep the 142-F PA-2 `engram preflight` launcher contracts, plus
  `rust-mcp-expert.agent.md`, `mcp-protocol-reviewer.agent.md`,
  `rust-safety-reviewer.agent.md`, `rust.instructions.md`,
  `rust-mcp-server.instructions.md`, `file-first-content.instructions.md`,
  `cp-review-fix.prompt.md`, and `policy-proposal.md`), `AGENTS.md`,
  `.github/copilot-instructions.md`, normalized `.gsd/carry-forward.yaml`
  (`files: [.gsd/config.yaml]`), and operator edits in `.gsd/config.yaml`.

## Verification

* `gsd.exe verify-workspace --workspace . --gsd-home .gsd/gsd-assets-v1.6.0.zip --json`
  passed with 0 blockers, 0 strict-schema blockers, 0 unresolved variables, 0
  portability findings, and 12/12 `targeted_checks` passing.
* `gsd.exe routing resolve` returned exit code 0 with zero degraded routes for
  `orchestrator`, `stage`, `ship`, and `ship` escalation.
