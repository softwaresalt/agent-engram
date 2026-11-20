---
title: GSD v1.6.0 tune-harness execution and adversarial verification
date: 2026-11-20T02:15:15Z
status: complete
---

## Outcome

Completed full `/tune-harness` execution (`workspace-discovery` -> drift/contract
scan -> learning-signal mining -> safe auto-application -> deterministic
`gsd.exe verify-workspace` -> 4-model `verify-harness` adversarial audit and
consensus auto-remediation) against `.gsd/gsd-assets-v1.6.0.zip`.

## Key Remediations Applied

* Synced `.gsd/backlog-registry.yaml`, `backlog-integration.instructions.md`,
  `.markdownlint.json`, `harness-doctor/SKILL.md`, and all 13 `scripts/*` to
  GSD v1.6.0 (restoring guarded `gsd checkpoint create`, `AGENT_TEMP_DIR`
  containment, P-019 required `markdownlint` gate, and PS 5.1 `Join-Path`
  compatibility).
* Replaced the broken `from gsd.backlog_root import resolve_backlog_root` Python
  import in `.github/agents/_ship.agent.md` Step 4.1 with a zero-dependency
  Python stdlib resolver honoring `BACKLOGIT_WORKSPACE_DIR` and fail-closed
  ambiguity checks.
* Added `'engram/*'` to `_stage.agent.md` and `_ship.agent.md` tool allowlists,
  aligned prompt `agent:` frontmatter (`_Orchestrator`, `_Stage`), fixed
  `adversarial-review.agent.md` (`max_subagent_tier: 3`, `## Model Routing`),
  cleared unconfigured `ALT_*`/`ANCHOR_REVIEW_*` fallback literals, normalized
  `SECURITY_*` pattern variables, and promoted `MD041` opening headings to H1.
* Updated `.gsd/workspace-profile.yaml` (`152` `src/` + `12` `crates/` `.rs`
  files, `284` test files, `kbdb` detection block, `.backlogit/config.yaml`
  path), cleaned `FOLDED_CORE_PACK` residue, wove all 5 Core Workflow Protocols
  across `AGENTS.md`, `.github/copilot-instructions.md`, and
  `constitution.instructions.md`, and removed the retired `agent-intercom`
  section from `.github/copilot-instructions.md`.

## Deferred / Operator-Review Items

* `TUNE-006`: `start.ps1` startup-script contract (`ambiguous`, `manual_review: true`; preserved for `140.001-T` contract tests)
* `TUNE-007`: Legacy `.backlogit/` root (`backlogit migrate --workspace-dir --dry-run` preview when outside active shipment `176-S`)
* `TUNE-008`: Optional `kbdb` capability-pack enablement in `.gsd/config.yaml`
* `TUNE-009`: Optional route/intercom refresh on 3 workspace-authored subagents (`preserved_artifacts`)
* `TUNE-010`: `.gsd/policy-proposals/P-DYN-001.md` operator review

## Verification

* Tuning report written to `.gsd/tuning-reports/2026-11-20-tuning-report.md`
* `gsd.exe verify-workspace`: EXIT 0 (0 blockers, 0 unresolved variables, 12/12 `targeted_checks` PASS)
* `cargo test --test integration_installer`: 2/2 PASS
