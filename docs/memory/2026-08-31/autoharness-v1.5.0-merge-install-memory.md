---
type: session-memory
date: 2026-08-31
branch: chore/autoharness-merge-install-20260831
commit: e2b35180
agent: auto-mergeinstall
---

# autoharness v1.5.0 merge-install

Merge-installed autoharness **1.5.0** over the workspace's existing **1.4.0** harness.
autoharness home: `C:\Python\Python314\Lib\site-packages\autoharness\data`.

## Outcome

`autoharness verify-workspace --workspace .` final state:

| Metric | Result |
|---|---|
| Strict schema blockers | 0 |
| Blockers | 0 |
| Warnings | 0 |
| Migration proposals | 0 |
| New artifacts (uninstalled templates) | 0 |
| Targeted guardrail checks | 71 / 71 pass |
| Portability findings | 0 |
| Unresolved placeholders | 12 (benign — see below) |

The CLI still exits `1` solely because of the 12 unresolved placeholders. Those are
by-design literals, not gaps:

* `.github/agents/_ship.agent.md:345` — a literal `` `{{VARIABLE}}` `` used as prose inside backticks.
* `.github/policies/policy-proposal.md` — a fill-in-the-blank proposal *form*; its
  `{{POLICY_ID}}` / `{{PROPOSED_AT}}` style tokens are meant to survive rendering.

## Pre-existing defect discovered and fixed

The installed `stage`, `ship`, and `orchestrator` agent definitions were verbatim
copies of *autoharness' own* pipeline agents — they described "the autoharness
repository", not agent-engram. They had never been rendered for this workspace.
All three were regenerated from the v1.5.0 templates and renamed to the canonical
`_stage` / `_ship` / `_orchestrator` identities.

## What changed

### New artifacts

* Instructions: `capability-pack-enforcement`, `coding-discipline`, `output-timestamps`,
  `escalation-protocol`, `copilot-code-review`
* Review personas (`.github/agents/subagents/`): `correctness-reviewer`,
  `maintainability-reviewer`, `template-integrity-reviewer`,
  `schema-cli-docs-coupling-reviewer`
* Skills: `brainstorm`, `doc-review`
* Prompts: `feature-flow`, `feature-flow-parallel`, `feature-flow-dark`
* Scripts: `pre-push-quality-gates.{ps1,sh}`, `pre-commit-pipeline-topology.{ps1,sh}`,
  `ci-topology-check.sh`

### Re-rendered from v1.5.0 templates

`backlogit` / `release-observability` / `agent-intercom` instructions,
`operational-closure` and `pr-lifecycle` SKILLs, `workflow-policies.md`,
`review/SKILL.md`, `AGENTS.md`, `start.ps1`, `start.sh`.

### Surgically merged (hand-customizations preserved)

* `github-pr-automation.instructions.md` — spliced in v1.5.0 §1.9 (pre-merge review
  readiness) and §1.10 (post-merge closure surveillance) **without** disturbing the
  hand-written Copilot merge-gate sections that encode the `commit_id == HEAD`
  invariant and the `--paginate` lesson from PRs #239/#240.
* `review/SKILL.md` — kept the engram-specific security-reviewer file globs
  (`src/daemon/ipc_server.rs`, `src/tools/*.rs`, `src/db/*.rs`, `src/config/*.rs`,
  `src/shim/transport.rs`, plus the token/secret keyword list) as sub-bullets under
  the template's generic `SECURITY_REVIEW_PATTERNS` bullet.
* `copilot-instructions.md` — took only the `cargo clippy --all-targets` correction;
  the installed file's curated stack rows, project structure, and engram MCP block
  are richer than the template render.

### State files rewritten (all schema-VALID)

* `.autoharness/workspace-profile.yaml` — corrected `workspace_path` (was a stale
  `D:\` path), fixed 10 shape violations, added `runtime_validation` for
  `cli` / `api` / `background-job`.
* `.autoharness/config.yaml` — migrated to schema 1.1.0 with per-role `stage`/`ship`
  routes and nested escalation, `anchor_review`, `ai_tools.copilot_cli.args`, `sidecars`.
* `.autoharness/harness-manifest.yaml` — regenerated: 107 artifacts with SHA-256
  checksums, `variables_used`, `capability_pack_overlays`, `upgrade_notes`.

## Upstream template gaps patched locally

Four v1.5.0 guardrail checks cannot be satisfied by the v1.5.0 templates themselves.
Each was patched with semantically identical wording. Re-check these after the next
autoharness upgrade — the local patch may become redundant or conflict.

| Check | Template gap | Local fix |
|---|---|---|
| `ship_release_closure_sequence` | Template says "do not allow another top-level release unit to begin yet"; check requires "another top-level release unit may not begin yet" | Reworded in `_ship.agent.md` |
| `escalation_directive_present` | Stage's `ESCALATION_DEGRADED` fallback bullet put a coordinating `or` between "never" and "another execution attempt", defeating the checker's fail-closed negation guard | Split into two sentences in `_stage.agent.md` |
| `reload_propagation_directive` | Template lacks the literal "no independent model binding", and `ROUTING_DEGRADED` fell outside the checker's 600-char window | Tightened the propagation bullet in `_orchestrator.agent.md` |
| `closure_source_artifact_cleanup` | `_ship.agent.md.tmpl` tells Ship to write a `Source artifact cleanup` section into the closure artifact, but `operational-closure/SKILL.md.tmpl` never defines it | Added the section (with `source_stash_id` / `source_deliberation_id`) to `operational-closure/SKILL.md` |

Two portability P1s (`~/.autoharness` hardcoded in `_orchestrator.agent.md`) also
originate upstream; replaced with "the default global installation directory
reported by `autoharness home`".

## Gotchas worth remembering

* `verify-workspace-report.json` → `targeted_checks` is a **dict keyed by check name**
  and each entry's pass flag is **`ok`**, not `status`. Filtering on `status` yields a
  false "everything failing" reading.
* The manifest's `variables_used` map is what verify re-renders with
  (`verify_workspace.py:2758`). Populating it dropped unresolved placeholders 104 → 12.
  All values must be strings.
* v1.5.0 pipeline agent templates are `templates/agents/_stage.agent.md.tmpl`
  (underscore-prefixed). `AGENTS.md.tmpl` and `copilot-instructions.md.tmpl` live under
  `templates/foundation/`.
* Startup-script contract: having both the current marker (`$enabledSidecars = @(`)
  and a legacy marker (`Invoke-EngramCommandWithProgress`, or the `COPILOT_HOME
  redirects the Copilot CLI database` comment) makes the contract `ambiguous`. The old
  `ENGRAM_PREWARM_TIMEOUT_MS` pre-warm guard must not be re-added. `Push-Location` /
  `cd` are not legacy markers, so workspace cwd anchoring was safely restored.
* Escalation routing: an escalation route that resolves identically to the acting
  role's own route is `ESCALATION_DEGRADED` (H3). Stage's role route is
  `claude-opus-5`, so its escalation is set to `gpt-5.6-sol` / openai / xhigh.
  Nested `<role>.escalation` and the legacy flat `escalation` must not both be
  non-empty (H2) — the flat block is left empty.
* `{{FEATURE_SHIPMENTS}}` is used in `_ship.agent.md.tmpl` as if it were a conditional,
  rendering the nonsense line "When `true` is `true`". Reworded locally.

## Dark factory mode (P-017) — installed and verified

All 8 dark-factory contract checks pass: `dark_factory_policy_contract`,
`dark_factory_orchestrator_contract`, `dark_factory_ship_contract`,
`dark_factory_pr_lifecycle_contract`, `dark_factory_intercom_contract`,
`dark_factory_prompt_contract`, `dark_factory_github_pr_automation_contract`,
`dark_factory_foundation_contract`.

Surfaces carrying the contract:

| Surface | Content |
|---|---|
| `.github/policies/workflow-policies.md` | `## P-017: Dark Factory Autonomy Contract` — trigger phrases, activation contract, scope rule, `DARK_MODE_SCOPE` resume/audit evidence, violation action |
| `.github/agents/_orchestrator.agent.md` | `### Dark Factory Mode Trigger Semantics (P-017)` — activation record fields (`scope`, `merge_approval_pre_authorized`, `admin_fallback_pre_authorized`, `stop_conditions`, `visibility_mode`), `DARK_MODE_START` / `DARK_MODE_SCOPE` events, multi-shipment ordered sequence + restart cursor |
| `.github/agents/_ship.agent.md` | dark-mode-aware execution and closure |
| `.github/skills/pr-lifecycle/SKILL.md` | `DARK_MODE_ACTIVE` handling in the merge path |
| `.github/instructions/github-pr-automation.instructions.md` | §1.9.6 Dark-Mode Merge Authorization and Admin Fallback — merge-state classification table, `COPILOT_REVIEW_BLOCK` non-bypass |
| `.github/instructions/agent-intercom.instructions.md` | dark-mode visibility events |
| `AGENTS.md` | Development Workflow item 4 (P-017) |
| `.github/prompts/feature-flow-dark.prompt.md` | `/feature-flow-dark` shim for the exact trigger |

Activation is **only** via the exact phrases `Run pipeline in dark mode` or
`Run pipeline in dark factory mode`, or the `/feature-flow-dark` prompt. Vague
autonomy language ("go autonomous", "run everything") must not be inferred as dark mode.

### Open question — prompt/agent name mismatch (upstream)

The three feature-flow prompt shims declare `agent: Orchestrator` in frontmatter,
but the installed agent declares `name: _Orchestrator`. Likewise
`stage-grouping-analysis.prompt.md` says `agent: Stage` against `name: _Stage`.

This is **not** a local mistake — autoharness v1.5.0's own dogfooded workspace
(`{autoharness_home}/.github/`) has the identical pairing, and the
`dark_factory_prompt_contract` check *requires* the literal string
`agent: Orchestrator`. Changing it locally to `_Orchestrator` would fail that
guardrail and diverge from upstream.

Left as-is deliberately. **Verify at runtime**: if `/feature-flow-dark` fails to
bind to the `_Orchestrator` agent in your client, the leading `_` is not being
stripped during resolution and this needs an upstream fix in autoharness rather
than a local edit. Invoking `_Orchestrator` directly with the trigger phrase is
the workaround and is fully equivalent — the prompt is only a shim.

## Not done / next steps

* **Not pushed and no PR opened.** Branch `chore/autoharness-merge-install-20260831`
  holds commit `e2b35180`. Open a PR rather than merging to `main` directly.
* The new `scripts/pre-commit-pipeline-topology.*` and `scripts/pre-push-quality-gates.*`
  are installed but **not wired into git hooks**. Wiring them is an operator decision.
* `graphtor-docs` and `browser-verification` packs remain disabled; their overlay
  sections were stripped from `AGENTS.md` and `capability-pack-enforcement.instructions.md`.
* Render toolchain kept under `.autoharness/staging/` (gitignored): `vars.json` (204
  variables), `render.py`, `validate.py`, `gen_manifest.py`, `emit_manifest.py`.
  Re-run `gen_manifest.py` then `emit_manifest.py` after any harness artifact edit so
  manifest checksums stay accurate.
* Backups of every overwritten file are in `.autoharness/backups/2026-08-31-merge-install/`
  (gitignored; path separators encoded as `__`).
