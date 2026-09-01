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
| Targeted guardrail checks | 68 / 68 pass |
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

Surfaces carrying the contract (snapshot taken **before** the `agent-intercom`
capability pack was removed later in this session — see "agent-intercom
capability pack removed" below; `.github/instructions/agent-intercom.instructions.md`
no longer exists in the workspace):

| Surface | Content |
|---|---|
| `.github/policies/workflow-policies.md` | `## P-017: Dark Factory Autonomy Contract` — trigger phrases, activation contract, scope rule, `DARK_MODE_SCOPE` resume/audit evidence, violation action |
| `.github/agents/_orchestrator.agent.md` | `### Dark Factory Mode Trigger Semantics (P-017)` — activation record fields (`scope`, `merge_approval_pre_authorized`, `admin_fallback_pre_authorized`, `stop_conditions`, `visibility_mode`), `DARK_MODE_START` / `DARK_MODE_SCOPE` events, multi-shipment ordered sequence + restart cursor |
| `.github/agents/_ship.agent.md` | dark-mode-aware execution and closure |
| `.github/skills/pr-lifecycle/SKILL.md` | `DARK_MODE_ACTIVE` handling in the merge path |
| `.github/instructions/github-pr-automation.instructions.md` | §1.9.6 Dark-Mode Merge Authorization and Admin Fallback — merge-state classification table, `COPILOT_REVIEW_BLOCK` non-bypass |
| `.github/instructions/agent-intercom.instructions.md` | dark-mode visibility events — **removed later in this session with the `agent-intercom` pack** |
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

## agent-intercom capability pack removed

Follow-up operator request: drop `agent-intercom` from this workspace's enabled
capability packs. The pack is an opt-in extra in the v1.5.0 registry
(`default_in_preset: []`, `mcp_requirements: []`) and ships no MCP server, so removal
has no runtime dependency impact.

### What was removed

| Surface | Change |
|---|---|
| `.autoharness/config.yaml` | dropped from `capability_packs` (authoritative "enabled" list) |
| `.autoharness/workspace-profile.yaml` | dropped from `capability_packs`, recommendation entry deleted, `agent_intercom` block flipped to `detected: false` / `recommended: false` |
| `.autoharness/harness-manifest.yaml` | dropped from `capability_packs`, `capability_pack_overlays` entry deleted, both artifact rows deleted (108 → 106 artifacts) |
| `.github/instructions/agent-intercom.instructions.md` | **deleted** (the pack's `overlay_instruction`) |
| `.github/prompts/ping-loop.prompt.md` | **deleted** (intercom heartbeat prompt; had no upstream template) |
| `AGENTS.md` | `### Capability Overlay — agent-intercom` section, the `agent-intercom + backlogit` interaction row, and the destructive-command approval bullet removed |
| `.github/instructions/constitution.instructions.md` | `### Capability Overlay — agent-intercom` section removed |
| `.github/copilot-instructions.md` | `### agent-intercom` under **Optional Capability Packs** removed |
| `.github/skills/review/SKILL.md`, `doc-review/SKILL.md` | unguarded `ping` directives guarded; `gated_auto` owner re-pointed from `agent-intercom approval` to `Operator approval` |

### Why ~110 `intercom` mentions were deliberately LEFT in place

Before the change, the intercom reference count in every workspace file matched the
pristine v1.5.0 template **exactly** (`_ship.agent.md` 30/30, `AGENTS.md` 8/8,
`copilot-instructions.md` 9/9, and so on). Autoharness templates are plain
`{{UPPER_SNAKE}}` substitution with **no conditionals**, so those clauses render
regardless of pack selection — a fresh install *without* agent-intercom produces
byte-identical files. Every remaining mention is self-guarding
("When the `agent-intercom` capability pack is installed, …") and is therefore inert.

Stripping them would create permanent template drift that `verify-workspace` would
re-flag on the next tune, for zero behavioral gain. Only genuinely **unguarded**
directives were fixed — an audit found exactly two (`review` and `doc-review` SKILLs,
both of which said "Call `ping` at session start." with no guard) plus two routing-table
rows naming `agent-intercom approval` as an owner.

### Guardrail interactions worth knowing

* Pack-scoped checks live in `PACK_ASSERTIONS` keyed by pack id, so
  `agent_intercom_instruction`, `review_intercom_workflow`, and
  `dark_factory_intercom_contract` (the last carries `requires_pack: "agent-intercom"`)
  all became **not-applicable**. That is why the total dropped 71 → 68 with **zero**
  failures — it is not a regression.
* `copilot_remote_operator_guidance` lives in `FOUNDATION_ASSERTIONS` and is
  **unconditional**: it hard-requires `## Remote Operator Integration` plus a literal
  `### agent-intercom` heading in `.github/copilot-instructions.md`. That section was
  therefore **kept**, with its dangling `ping-loop.prompt.md` sentence replaced by an
  explicit "this pack is **not enabled** in this workspace" note. Deleting the heading
  would fail the check.
* `capability_pack_enforcement` is driven by `RETRIEVAL_ENFORCED_PACKS`
  (`agent-engram`, `graphtor-docs`) only. agent-intercom is not retrieval-enforced, so
  `capability-pack-enforcement.instructions.md` needed no edit.
* `start.ps1` / `start.sh` never listed intercom as a sidecar (`("backlogit", "engram")`),
  consistent with the pack shipping no MCP server.

Post-removal verification: 0 blockers, 0 warnings, 0 migration proposals, 0 uninstalled
templates, 0 portability findings, all three schema contracts `current`, both startup
script contracts `current`, **68/68 targeted checks pass**.

## Not done / next steps

* **Upstream autoharness gaps — tracked as `140-F`.** The two judgment calls above are
  upstream defects, not engram problems. Full analysis with exact upstream file/line
  references and proposed fixes:
  `docs/compound/autoharness-optional-pack-content-not-gated-in-templates-2026-08-31.md`.
  They cannot be fixed from here — Constitution IV forbids writing outside the workspace
  tree, and the autoharness source lives at `C:\Source\GitHub\autoharness`.
  Once upstream ships the fix, drop the local guards in `review`/`doc-review` SKILLs and
  remove the retained `### agent-intercom` section from `copilot-instructions.md`.

* **Not pushed and no PR opened.** ~~Branch `chore/autoharness-merge-install-20260831`
  holds commit `e2b35180`.~~ **Superseded** — see "PR #371, CI, and review-fix cycle" below.
* The new `scripts/pre-commit-pipeline-topology.*` and `scripts/pre-push-quality-gates.*`
  are installed but **not wired into git hooks**. Wiring them is an operator decision.
* `graphtor-docs`, `browser-verification`, and now `agent-intercom` packs remain
  disabled; their overlay sections were stripped from `AGENTS.md` and
  `capability-pack-enforcement.instructions.md`.
* Render toolchain kept under `.autoharness/staging/` (gitignored): `vars.json` (204
  variables), `render.py`, `validate.py`, `gen_manifest.py`, `emit_manifest.py`.
  Re-run `gen_manifest.py` then `emit_manifest.py` after any harness artifact edit so
  manifest checksums stay accurate.
* Backups of every overwritten file are in `.autoharness/backups/2026-08-31-merge-install/`
  (gitignored; path separators encoded as `__`).

## PR #371, CI, and review-fix cycle

Branch `chore/autoharness-merge-install-20260831` → PR
[#371](https://github.com/softwaresalt/agent-engram/pull/371).

### Requesting a Copilot review (the identifier that actually works)

`gh pr edit --add-reviewer copilot` fails, and so do the REST identifiers
`copilot`, `copilot-pull-request-reviewer`, and `copilot-pull-request-reviewer[bot]`.
The literal login that works is **`Copilot`** (capital C, no bot suffix):

```powershell
'{"reviewers":["Copilot"]}' |
  gh api repos/softwaresalt/agent-engram/pulls/371/requested_reviewers -X POST --input -
```

`requested_reviewers` stays `[]` afterwards, but the timeline records
`review_requested → Copilot`. Reviews land authored as
`copilot-pull-request-reviewer[bot]`, so match with
`startswith("copilot-pull-request-reviewer")` and pass `--paginate` to
`/reviews` (default page size 30, oldest-first — the HEAD review is last).
Review latency is roughly 7–8 minutes.

### CI `paths-ignore` blind spot (root cause of two "surprise" failures)

`.github/workflows/ci.yml` uses `paths-ignore` covering `.backlogit/**`,
`docs/**`, `.autoharness/**`, `*.md`, `.github/**/*.md`, and `scripts/**/*.md`.
`build` is **not** a required status check on `main`. Doc-only PRs therefore skip
`build` entirely, so code-affecting regressions merge undetected until an
unrelated PR re-arms CI. This PR added non-markdown files under `scripts/` and
touched the launchers, which re-armed full CI and surfaced two latent failures:

1. **`start-launcher-windows` — caused by this install.** The autoharness v1.5.0
   template overwrote engram's hand-hardened, contract-tested `start.ps1`/`start.sh`
   with the generic v1.1.0 thin-shim, breaking `tests/contract/start_launcher_test.rs`
   (prewarm elapsed 34.4s against an 8s ceiling). Both launchers were reverted to
   `main` in `166baadc`. `autoharness verify-workspace` independently classifies
   `start.ps1` as `manual_review: true` / "Do not auto-apply", which corroborates
   the revert. Reconciliation deferred to `140.001-T`.
2. **`build` — pre-existing, from PR #370.** Commit `2ee9ceac` (doc-only, so CI was
   skipped) renamed the contract-required heading `## G3 post-publish verification`
   to `## Final post-publish and native verification` in
   `docs/closure/2026-08-29-v0.3.0-rc.1-verification.md`. That heading is asserted
   literally by `verification_record_separates_g1_evidence_from_g3_artifact_proof`.
   Restored the canonical heading in `865e5931` (one line; no fabricated evidence).

Also filed `141-F` for the Windows-only
`archive_verifier_runs_the_unpacked_native_binary` failure (passes on
`ubuntu-latest`; the verifier appears to assume a bounded/single-line read and
engram's full tool catalogue payload exceeds it) plus the `paths-ignore` blind
spot itself.

### Review-fix cycle 1

Copilot posted 23 threads at HEAD `865e5931`; 21 were unresolved. All were
answered and resolved across two commits:

| Commit | Scope |
|---|---|
| `2c13e65b` | Redacted the operator username from `.autoharness/harness-manifest.yaml` and `workspace-profile.yaml` (and from `emit_manifest.py` so the redaction survives regeneration); held the Ship role boundary by routing to Stage instead of assembling shipments; switched stash creation to `backlogit_stash`/`backlogit_stash_get`; added a CLI fallback note for the unregistered `archive_item` op; stripped rendered-boolean tautologies; repointed dead topology-gate doc refs; aligned `doc-review` scope and placeholder exclusions. |
| `5bb6f4ea` | Preserved P-018 fail-closed semantics: added the missing §1.9.4 **Check 5** to the `pr-lifecycle` pre-merge gate and clarified across `pr-lifecycle`, `_ship.agent.md`, `feature-flow-dark.prompt.md`, and P-014/P-017 in `workflow-policies.md` that **engagement** — not operator elevation — arms the gate. |

**Declined (contract-required):** the three `feature-flow*.prompt.md` threads
flagging `agent: Orchestrator` vs `name: _Orchestrator`. The verifier's
`dark_factory_prompt_contract` asserts the literal string `agent: Orchestrator`,
and the same pairing exists in autoharness' own dogfooded workspace. Workaround:
invoke `_Orchestrator` directly. Upstream fix tracked in `140-F`.

`autoharness verify-workspace --workspace .` holds at **0 strict schema blockers,
0 blockers, 0 warnings, 68-of-68 targeted checks** after every commit above. The
CLI still exits 1 solely because of 12 benign unresolved placeholders (documented
`{{VAR}}` literals inside code spans).
