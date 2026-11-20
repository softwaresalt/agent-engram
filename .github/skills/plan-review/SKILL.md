---
name: plan-review
description: "Multi-persona review gate for implementation plans. Validates architectural soundness, scope boundaries, and coding standards compliance before the harvest skill decomposes a plan into work items."
argument-hint: "[path to plan file in docs/exec-plans/]"
---

# Plan Review Gate

Validates implementation plans through multi-persona review before the harvest skill decomposes them into work items. This gate prevents flawed plans from generating flawed work hierarchies.

## Subagent Depth Constraint

This skill spawns reviewer subagents. Those subagents are leaf executors and MUST NOT spawn their own subagents. Maximum depth: plan-review skill → persona subagent (1 hop).

## Severity Scale

| Level | Meaning | Gate action |
|---|---|---|
| **P0** | Plan will produce unshippable or unsafe code (missing security, broken contracts, impossible scope) | Block harvest |
| **P1** | Plan has a high-impact gap that will cause significant rework (missing requirements, wrong decomposition, absent verification) | Block harvest |
| **P2** | Plan has a moderate gap (edge case coverage, missing test scenario, suboptimal decomposition) | Record as backlog follow-up |
| **P3** | Plan has a minor improvement opportunity (wording, optional optimization) | Advisory |

Use the same severity conventions as the code review skill adapted for plan
artifacts rather than code diffs.

## Inputs

* `plan_path`: (Required) Path to the plan file (`docs/exec-plans/{YYYY-MM-DD}-{slug}-plan.md`).

If no path is provided, search `docs/exec-plans/` for the most recent `*-plan.md` file and confirm with the operator.

## Output

Review results are **recorded in the plan's verdict manifest** — the single review
authority — through the atomic review-result recorder contract (`RecordReviewResult`,
reference implementation in `internal/ops`). One recording writes the immutable
per-attempt review artifact, the manifest roster entry together with the top-level
selection fields (`latest_attempt`, `latest_artifact`, `verdict`, `plan_revision`),
and the plan's `review_manifest` and `source_history` reference, so no interruption
leaves a manifest pointing at a missing artifact or an artifact without a roster
entry; an interrupted recording is resolved with `RecoverReviewRecording`. The
recorder refuses to overwrite an existing artifact, requires the next attempt number,
and rejects paths outside the workspace root. Pass `dispatch_mode` as a roster extra.
The plan-review skill produces a gate decision (`PASS`, `ADVISORY`, or `FAIL`) that is
the recorded `verdict`.

A human-readable `## Plan Review` section is still **appended to the plan file** as a
summary of the recorded attempt. It MUST include literal `dispatch_mode:` and
`decision:` marker lines matching the manifest, but it is not the authority: `harvest`
reads the manifest (`HARVEST_ADMITTED` / `HARVEST_REFUSED` / `MANIFEST_QUARANTINED`),
never this section. When the compact-context skill later consolidates the plan, it
merges the plan and appended reviews into a decided-plan.

## Dispatch Capability and Declared Degradation

Before spawning personas, treat reviewer subagent dispatch and model-specific
reviewer routing as required workflow capabilities under P-012:

1. Probe reviewer subagent dispatch if the environment exposes a probe. If it is
   available, record `TOOL_OK: reviewer-subagent-dispatch` and set
   `dispatch_mode: multi-agent`.
2. If reviewer subagent dispatch is unavailable but the plan can be reviewed inline
   by applying every selected persona's focus from the Persona Rubric Adapter, record
   `TOOL_DEGRADED: reviewer-subagent-dispatch — declared fallback: single-agent persona pass`
   and set `dispatch_mode: single-agent-declared-degradation`.
3. If reviewer subagent dispatch is available but model-specific dispatch is
   unavailable, record `TOOL_DEGRADED: model-specific-review-routing — declared fallback: same-model rubric pass`
   and set `dispatch_mode: same-model-declared-degradation`. Multi-model critique
   is preferred, but the same persona rubrics still apply through the available
   subagent dispatch surface. Preserve `dispatch_mode: single-agent-declared-degradation`
   only for the inline persona-coverage fallback in Step 2.
4. If neither subagent dispatch nor inline persona coverage can cover every selected
   persona, set `decision: FAIL` and do not present the plan as harvest-ready.

The degraded path is acceptable only when it reviews every selected persona and
normalizes findings to the P0-P3 severity scale. It is not acceptable to skip a
persona because dispatch failed.

## Relationship to P-012

P-012 applies to both registry-backed tools and non-registry workflow capabilities.
For this skill, reviewer subagent dispatch, model-specific reviewer routing, indexed
knowledge retrieval are capabilities that must be surfaced
as `TOOL_OK`, `TOOL_DEGRADED`, or `TOOL_UNAVAILABLE` before the review relies on
them. A degraded review remains valid only when the appended review records:

```text
dispatch_mode: multi-agent | single-agent-declared-degradation | same-model-declared-degradation
decision: PASS | ADVISORY | FAIL
```

Silent fallback from subagents to inline review is a P-012 violation. Silent same-model
review when an anchor or cross-model route was required is also a P-012 violation.

## Persona Rubric Adapter

Use this table as the authoritative mapping in both dispatched and inline modes.
The identity paths are installed artifact paths, not template source paths.

| Persona | Installed identity file | Focus used in all modes | Trigger / routing |
|---|---|---|---|
| Constitution Reviewer | `.github/agents/subagents/constitution-reviewer.agent.md` | Map plan units against constitutional principles and flag violations. | Always-on |
| Rust Reviewer | `.github/agents/subagents/rust-reviewer.agent.md` | Evaluate proposed Rust type signatures, error handling, package boundaries, and verification steps. | Always-on |
| Scope Boundary Auditor | `.github/agents/subagents/scope-boundary-auditor.agent.md` | Verify units stay within declared scope and detect scope creep, YAGNI, and unnecessary complexity. | Always-on |
| Learnings Researcher | `.github/agents/subagents/learnings-researcher.agent.md` | Search `docs/compound/` for prior solutions relevant to the plan's scope. | Always-on |
| Architecture Strategist | `.github/agents/subagents/architecture-strategist.agent.md` | Review cohesion, coupling, module boundaries, and dependency chains. | Cross-model; prefer `model_routing.anchor_review` when available |
| Agent-Native Parity Reviewer | `.github/agents/subagents/agent-native-parity-reviewer.agent.md` | Review MCP tools, agent-facing actions, context surfaces, and user/agent parity-sensitive workflows. | Conditional cross-model; may use anchor route |
| Security Lens Reviewer | `.github/agents/subagents/security-lens-reviewer.agent.md` | Review auth/authz, API surfaces, sensitive data, external trust boundaries, and secrets handling. | Conditional cross-model; may use anchor route |

## Reviewer Personas

Spawn all always-on personas and any triggered cross-model personas. Use different
models when available to force genuine diversity of critique. When
`model_routing.anchor_review` can be dispatched, assign it to one eligible
cross-model persona that is already triggered by the plan (Architecture Strategist
by default). If the anchor reviewer route is unavailable, declare degradation and
apply the same rubric inline or with the caller's model rather than skipping that
persona.

### Always-On Personas (same model as caller)

| Persona Subagent | Focus |
|---|---|
| **Constitution Reviewer** | Map plan units against constitutional principles. Flag violations. Never request volatile self-referential counts, own-commit SHAs, or live worktree hashes in plans. |
| **Rust Reviewer** | Evaluate proposed type signatures, error handling patterns, package boundaries, and verification steps. |
| **Scope Boundary Auditor** | Verify units stay within declared scope. Detect scope creep, YAGNI, unnecessary complexity. Never request self-referential file-count denominators, own-commit SHAs, or live worktree hashes in plans. |
| **Learnings Researcher** | Search `docs/compound/` for prior solutions relevant to the plan's scope. Report P0 if the plan contradicts a known past resolution; P1 if it ignores a highly relevant prior solution. |

### Cross-Model Personas (different model when available)

| Persona Subagent | Focus | Suggested Model |
|---|---|---|
| **Architecture Strategist** | Cohesion, coupling, module boundaries, dependency chains. | Anchor reviewer route when available; otherwise different from caller |
| **Agent-Native Parity Reviewer** | Plans that expose MCP tools, agent-facing actions, or user/agent parity-sensitive workflows. | Anchor reviewer route or different from caller |
| **Security Lens Reviewer** (`security-lens-reviewer.agent.md`) | Plans that touch auth/authz systems, API surfaces, sensitive data stores, external integrations, or secrets management. | Anchor reviewer route or different from caller |

If cross-model invocation is not available, run all personas with the caller's model and record the declared degradation. Multi-model is preferred but not blocking when the same rubric is fully applied.

## Workflow

### Step 1: Load and Parse Plan

1. Read the plan file from `docs/exec-plans/`.
2. Extract implementation units, dependency graph, decisions, risks, hardening signals, and whether a `## Plan Hardening` section is present.
3. When `strict-safety` is enabled and the plan contains a `## Plan Hardening` section, also extract any `ProposedAction` / `ActionRisk` entries.
4. If the plan references an origin document, read that too for context.
5. Verify the plan frontmatter carries a positive integer `revision`; if it is missing, non-integer, or not positive, return the plan to `impl-plan` before reviewing.

### Step 2: Spawn or Apply Reviewer Personas

Spawn all always-on personas plus the cross-model personas whose trigger
conditions are met when dispatch is available. In declared-degradation mode, apply
the same persona focuses inline, keeping one finding list per persona so coverage
is auditable. Each dispatched or inline persona pass receives:

- The full plan content
- The origin requirements doc (if any)
- The project's coding standards and conventions (reference `.github/instructions/constitution.instructions.md`)
- The applicable Focus text from the Persona Rubric Adapter
- Instructions to return structured findings

Trigger conditions for cross-model personas:
* **Architecture Strategist**: always triggered
* **Agent-Native Parity Reviewer**: triggered when the plan exposes MCP tools, agent-facing actions, or user/agent parity-sensitive workflows
* **Security Lens Reviewer**: triggered when the plan touches authentication or authorization systems, API surfaces, sensitive data stores, external integrations crossing trust boundaries, or secrets and credentials management

When `model_routing.anchor_review` is dispatchable, route one triggered cross-model
persona through the anchor reviewer. If it is not dispatchable, record the declared
degradation and run that same persona rubric inline or same-model.

### Step 3: Collect and Merge Findings

As each persona returns:

1. Collect all findings into a unified list
2. Deduplicate: merge findings that identify the same issue from different perspectives
3. Assign final severity (use the more conservative severity when personas disagree)

### Step 4: Gate Decision

**Time-Boxed 2-Pass Cap (`MaxPlanReviewPasses = 2`)**: A plan undergoes **at most 1 full multi-persona review pass (Pass 1) + at most 1 targeted delta pass on P0/P1 fixes (Pass 2)** before freezing planning and transitioning to `harvest` / TDD (`ValidatePlanReviewPassBudget`, `ErrPlanReviewPassCapReached` on a 3rd pass). When `P0 == 0 && P1 == 0`, record `verdict: "PASS"` with any P2/P3 observations recorded in-place as advisory notes or deferred to code review so `EvaluateHarvestAdmission` (`verdict == "PASS"`) admits the plan to harvest, never triggering a 3rd plan-review pass.

| Condition | Decision | Action |
|---|---|---|
| Required reviewer dispatch or inline persona coverage is unavailable | **FAIL** | Record `TOOL_UNAVAILABLE` or degraded mode and return before `harvest`. |
| Plan shows hardening signals but lacks plan hardening or equivalent high-risk detail | **FAIL** | Return the plan to `plan-harden` or manual revision before `harvest`. |
| Strict-safety enabled, hardening present, but risky actions lack `ProposedAction` / `ActionRisk` classification | **FAIL** | Plans with hardening signals must classify risky actions explicitly when strict-safety is active. |
| Any P0 or P1 findings | **FAIL** | Present findings to user. Revise in place and run at most 1 targeted delta pass (Pass 2, `MaxPlanReviewPasses = 2`) before proceeding to `harvest`. |
| P2 findings only (`P0 == 0 && P1 == 0`) | **PASS** (advisory follow-ups recorded in place) | Record `verdict: "PASS"` with P2/P3 items noted in place as deferred advisory follow-ups so `EvaluateHarvestAdmission` admits the plan to `harvest`. |
| P3 findings only or none | **PASS** | Log findings as advisory. Proceed to `harvest`. |

Report the gate decision in the session output.

### Step 5: Record the Review

Record the review in place on the plan and review manifest (or via legacy distinct artifact path when explicitly requested), using Git commit history on the branch as the immutable audit trail rather than generating append-only `review-history/*-attempt-NN.md` errata chains for prose or typo edits. Commit the updated plan and review manifest to the branch after each completed review pass before revising the plan for Pass 2, and increment the plan's frontmatter integer `revision` whenever the plan is edited between review passes (up to `MaxPlanReviewPasses = 2`) so each pass binds to the exact revision reviewed and Git commit history preserves Pass 1's full findings and reviewed revision.

1. Record via `RecordReviewResult`: in **in-place mode** (`--in-place` / `in_place: true`, the default for `MaxPlanReviewPasses = 2`), `RecordReviewResult` updates the plan (`docs/exec-plans/{slug}-plan.md`) and the review manifest (`{docs.root}/{docs.subdirectories.reviews}/{slug}-plan-review.md` from `.gsd/config.yaml`, defaulting to `docs/reviews/{slug}-plan-review.md` when `docs.subdirectories.reviews` is absent) in place via `CASBatchWriteWithBackoff` (up to 3 exponential-backoff retries: `100ms`, `400ms`, `1600ms`) with `--artifact-path` omitted, validating canonical LF formatting and rejecting volatile self-referential commit SHA placeholders. Legacy callers that supply a distinct `--artifact-path` remain supported for backward compatibility. Do not write the manifest, the artifact, or the plan reference by any other path. If recording fails, report the failure and do not present the plan as harvest-ready; resolve an interrupted recording with `RecoverReviewRecording` before retrying. Invoke the recorder through the registered operation: `gsd op review record-result --plan-path <plan_path>
   --manifest-path <manifest_path> [--in-place | --artifact-path <artifact_path>]
   (--artifact-body <text> | --artifact-body-file <path>) --attempt <n>
   --verdict <verdict> --plan-revision <rev> --dispatch-mode <mode> [--p0-open <n>] [--p1-open <n>] --json` (MCP tool
   `review_record-result`; exactly one of `artifact_body` or `artifact_body_file`;
   `dispatch_mode` is required and must be `multi-agent`,
   `single-agent-declared-degradation`, or `same-model-declared-degradation`). Recover with
   `gsd op review recover-recording --manifest-path <manifest_path> --json` (MCP tool
   `review_recover-recording`). A refusal returns `REVIEW_REFUSED` with a typed code
   (`PATH_REJECTED`, `ARTIFACT_EXISTS`, `ATTEMPT_OUT_OF_ORDER`, `RESULT_INVALID`,
   `PLAN_REVISION_MISMATCH`, `MANIFEST_NOT_RECORDABLE`, `RECORDING_PENDING`, `RECORDING_ACTIVE`,
   `ARTIFACT_BODY_INVALID`, `ARTIFACT_BODY_UNREADABLE`, `PLAN_REVIEW_PASS_CAP_REACHED`; an unsupported `dispatch_mode` returns `RESULT_INVALID`); `RECORDING_PENDING` means run
   `recover-recording` first; `RECORDING_ACTIVE` means another recorder or recovery call currently holds
   the per-manifest OS lock, so wait for the active call to finish and retry, and do not run
   `recover-recording` while a live holder is active.
2. Ensure the `## Plan Review` summary section on the plan file (written in place by `--in-place` or appended after legacy artifact recording) includes:

* `dispatch_mode: multi-agent`, `dispatch_mode: single-agent-declared-degradation`, or `dispatch_mode: same-model-declared-degradation`
* `decision: PASS` or `decision: FAIL` (when `P0 == 0 && P1 == 0`, record `decision: PASS` with P2/P3 advisory follow-ups noted in place)
* Gate decision and rationale
* Whether plan hardening was required and whether that requirement was satisfied
* Persona coverage table showing every selected persona and whether it ran as a subagent, inline pass, same-model fallback, or anchor reviewer pass
* All findings organized by severity
* Specific recommendations for addressing P0/P1 issues
* Acknowledgment of P2/P3 items for awareness
* Runtime verification and operational closure gaps called out explicitly when missing

The summary is appended so that the plan and its review travel together as a single
artifact; the manifest, not the summary, is what `harvest` reads. The compact-context
skill later consolidates this into a decided-plan.

## Quality Criteria

* Every implementation unit is reviewed by at least the always-on personas
* Every selected persona is covered in both multi-agent dispatch and single-agent declared-degradation modes
* The gate decision correctly reflects finding severities
* Findings include actionable recommendations
* The review is recorded in the manifest before the gate decision is communicated
* The manifest verdict, the appended summary's `decision:` marker, and the recorded artifact agree
* The appended summary includes literal `dispatch_mode:` and `decision:` markers
* Plans with hardening signals are failed when hardening is missing or materially incomplete
* Plans that touch runtime surfaces are checked for verification and closure readiness


## Model Routing

This skill operates at **Tier 2 (Standard)** — plan review coordination and finding assembly. One eligible cross-model persona may use `model_routing.anchor_review` when model-specific dispatch is available.

Generated by gsd | Template: plan-review/SKILL.md.tmpl
