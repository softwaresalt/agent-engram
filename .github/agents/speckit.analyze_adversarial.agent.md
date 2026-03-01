---
name: speckit.analyze.adversarial
description: Adversarial multi-model analysis with automated remediation across spec.md, plan.md, tasks.md, and SCENARIOS.md using parallel reviewer subagents.
disable-model-invocation: true
model: Claude Opus 4.6 (copilot)
handoffs:
  - label: Build the Feature
    agent: build-orchestrator
    prompt: feature {specName}; phase {phaseNumber}; mode (single|full)
    send: false
---

# Speckit Adversarial Analyzer

## User Input

```text
$ARGUMENTS
```

Consider the user input before proceeding (if not empty).

## Goal

Identify inconsistencies, duplications, ambiguities, and underspecified items across the core artifacts (*spec.md*, *plan.md*, *tasks.md*, and optionally *SCENARIOS.md*) using adversarial multi-model review, then apply remediation based on severity. This agent runs only after *tasks.md* has been produced by the tasks step. If *SCENARIOS.md* exists (produced by the behavior step), it is included automatically.

Unlike the read-only *speckit.analyze* agent, this agent synthesizes findings from three independent model reviewers and applies fixes directly to the spec artifacts, with severity-based gating:

* Findings rated critical or high are applied automatically.
* Findings rated medium require explicit user confirmation before application.
* Findings rated low are reported but not applied.

## Operating Constraints

**Constitution authority**: The project constitution (*.specify/memory/constitution.md*) is non-negotiable within this analysis scope. Constitution conflicts are automatically critical severity and require adjustment of the spec, plan, or tasks — not dilution, reinterpretation, or silent dismissal of the principle. If a principle itself needs to change, that must occur in a separate, explicit constitution update outside this agent.

**Severity-based remediation gating**: The agent applies fixes only when the synthesized severity meets the threshold. Critical and high findings are applied without prompting. Medium findings are presented to the user with a recommendation and applied only upon confirmation. Low findings appear in the report as suggestions.

**Parallel reviewer independence**: Each adversarial reviewer operates on an identical snapshot of the artifacts and produces findings independently. Reviewers do not see each other's output.

**Idempotent execution**: Running this agent a second time on artifacts that have already been remediated produces zero or near-zero new findings, confirming that prior fixes hold.

## Required Steps

### Step 1: Initialize Analysis Context

Run *.specify/scripts/powershell/check-prerequisites.ps1 -Json -RequireTasks -IncludeTasks* once from the repository root and parse the JSON output for *FEATURE_DIR* and *AVAILABLE_DOCS*. Derive absolute paths:

* SPEC = FEATURE_DIR/spec.md
* PLAN = FEATURE_DIR/plan.md
* TASKS = FEATURE_DIR/tasks.md

Abort with an error message if any required file is missing and instruct the user to run the missing prerequisite command.

Also check for the optional behavioral matrix:

* SCENARIOS = FEATURE_DIR/SCENARIOS.md (include in analysis if present)

For single quotes in arguments like "I'm Groot", use escape syntax: e.g., `'I'\''m Groot'` (or double-quote if possible: `"I'm Groot"`).

### Step 2: Load Artifacts with Progressive Disclosure

Load only the minimal necessary context from each artifact to keep token usage efficient.

*From spec.md:*

* Overview and context
* Functional requirements
* Non-functional requirements
* User stories
* Edge cases (if present)

*From plan.md:*

* Architecture and stack choices
* Data model references
* Phases
* Technical constraints

*From tasks.md:*

* Task IDs and descriptions
* Phase grouping
* Parallel markers [P]
* Referenced file paths

*From SCENARIOS.md (if present):*

* Scenario IDs and descriptions
* Input states and execution triggers
* Expected outputs and system states
* Category classifications (happy-path, edge-case, error, boundary, concurrent, security)
* Component and subsystem groupings

*From constitution:*

* Load *.specify/memory/constitution.md* for principle validation

Capture a snapshot of all loaded artifact content. This snapshot is the shared input provided to each reviewer in the next step.

### Step 3: Dispatch Adversarial Reviewers

Launch three adversarial review subagents in parallel using `runSubagent`, each configured with a different model and a distinct review focus. All three receive the identical artifact snapshot loaded in step 2, plus the constitution content.

Each reviewer produces a structured findings list using the format defined in the Reviewer Response Format section below. Limit each reviewer to 30 findings maximum to keep synthesis tractable.

#### A. Reviewer — Logical Consistency (Claude Opus 4.6)

Invoke `runSubagent` with `model: "claude-opus-4.6"` and the following prompt:

```text
You are an adversarial specification reviewer focused on logical consistency,
requirement completeness, and constitution compliance.

Analyze the provided spec artifacts (spec.md, plan.md, tasks.md, and SCENARIOS.md
if present) alongside the project constitution. Produce structured findings for:

1. Requirements that conflict with each other or with the constitution.
2. Requirements with missing acceptance criteria or unmeasurable outcomes.
3. Coverage gaps where requirements have no associated tasks or scenarios.
4. Constitution principle violations (always mark as CRITICAL severity).
5. Logical ordering issues in task dependencies.
6. Missing mandated sections or quality gates from the constitution.

For each finding, produce a table row with columns:
ID (prefix RC), Category, Severity (CRITICAL/HIGH/MEDIUM/LOW),
Location (file:section or file:line-range), Summary, Recommendation.

After the findings table, include:
- A summary paragraph with your overall assessment of artifact quality.
- A count of findings by severity level.

Limit output to 30 findings. Prioritize by severity.
```

When constructing the `runSubagent` prompt parameter, concatenate the reviewer prompt text above with the full content of each loaded artifact and the constitution.

#### B. Reviewer — Technical Feasibility (GPT-5.3 Codex)

Invoke `runSubagent` with `model: "gpt-5.3-codex"` and the following prompt:

```text
You are an adversarial specification reviewer focused on technical feasibility,
implementation gaps, and code-level concerns.

Analyze the provided spec artifacts (spec.md, plan.md, tasks.md, and SCENARIOS.md
if present) alongside the project constitution. Produce structured findings for:

1. Architecture decisions that are technically infeasible or contradict the stated stack.
2. Tasks referencing files, modules, or components not defined in the spec or plan.
3. Missing error handling, recovery, or fallback specifications.
4. Performance or scalability requirements without concrete metrics or validation strategy.
5. Data model gaps — entities referenced in tasks but absent from the plan.
6. Implementation ordering issues where tasks depend on undefined foundations.

For each finding, produce a table row with columns:
ID (prefix TF), Category, Severity (CRITICAL/HIGH/MEDIUM/LOW),
Location (file:section or file:line-range), Summary, Recommendation.

After the findings table, include:
- A summary paragraph with your overall assessment of artifact quality.
- A count of findings by severity level.

Limit output to 30 findings. Prioritize by severity.
```

When constructing the `runSubagent` prompt parameter, concatenate the reviewer prompt text above with the full content of each loaded artifact and the constitution.

#### C. Reviewer — Edge Cases and Security (Gemini 3.1 Pro Preview)

Invoke `runSubagent` with `model: "gemini-3.1-pro-preview"` and the following prompt:

```text
You are an adversarial specification reviewer focused on edge cases, security
implications, and cross-artifact terminology drift.

Analyze the provided spec artifacts (spec.md, plan.md, tasks.md, and SCENARIOS.md
if present) alongside the project constitution. Produce structured findings for:

1. Terminology drift — the same concept named differently across artifacts.
2. Security-sensitive operations lacking explicit threat model or mitigation.
3. Edge cases mentioned in one artifact but not covered by tasks or scenarios.
4. Ambiguous language — vague adjectives (fast, scalable, secure, intuitive)
   without measurable criteria.
5. Unresolved placeholders (TODO, TKTK, ???, or template markers).
6. Scenario coverage imbalance — insufficient non-happy-path scenarios
   (less than 30% coverage).
7. Cross-artifact inconsistencies in data types, API contracts, or entity naming.

For each finding, produce a table row with columns:
ID (prefix ES), Category, Severity (CRITICAL/HIGH/MEDIUM/LOW),
Location (file:section or file:line-range), Summary, Recommendation.

After the findings table, include:
- A summary paragraph with your overall assessment of artifact quality.
- A count of findings by severity level.

Limit output to 30 findings. Prioritize by severity.
```

When constructing the `runSubagent` prompt parameter, concatenate the reviewer prompt text above with the full content of each loaded artifact and the constitution.

### Step 4: Synthesize Findings

After all three reviewers return, merge their findings into a unified list using the following synthesis logic.

1. *Agreement elevation*: Findings identified independently by two or more reviewers are elevated in confidence. When reviewers assign different severities to the same finding, adopt the higher severity.
2. *Conflict resolution*: When reviewers produce contradictory findings (one flags X as a problem while another implies X is correct), reason about the conflict using the artifact text as ground truth. Resolve in favor of the interpretation most consistent with the constitution and the explicit artifact content. Record the reasoning in the report.
3. *Deduplication*: Merge findings that reference the same location and describe the same issue. Retain the strongest reasoning and the most actionable recommendation across the duplicates.
4. *Severity normalization*: Assign each unified finding a final severity using these heuristics:
   * *Critical*: Violates a constitution principle, missing a core artifact, or a requirement with zero coverage that blocks baseline functionality.
   * *High*: Duplicate or conflicting requirement, ambiguous security or performance attribute, untestable acceptance criterion, or finding agreed upon by all three reviewers.
   * *Medium*: Terminology drift, missing non-functional task coverage, underspecified edge case, or finding identified by exactly two reviewers.
   * *Low*: Style or wording improvements, minor redundancy not affecting execution order, or finding identified by only one reviewer with no corroboration.
5. *Consensus tagging*: Tag each finding with a consensus indicator — *unanimous* (3/3 reviewers), *majority* (2/3), or *single* (1/3).

Limit the unified findings list to 50 entries. Summarize any overflow in an aggregate count by category and severity.

### Step 5: Apply Remediation

Apply fixes to the spec artifacts based on the synthesized findings, respecting the severity gating rules.

1. For each critical or high finding with an actionable recommendation, apply the fix directly to the affected artifact (*spec.md*, *plan.md*, *tasks.md*, or *SCENARIOS.md*). Record each modification in the remediation log with the finding ID, file path, description of the change, and an excerpt of the original text.
2. For each medium finding, present the finding and recommended fix to the user. Prompt for confirmation with options: apply, skip, or modify the remediation. Apply only upon explicit approval.
3. For low findings, record them in the report without modifying any artifact. These serve as suggestions for future improvement.
4. When a finding spans multiple artifacts (for example, terminology drift across *spec.md* and *plan.md*), apply consistent corrections to all affected files in one pass.
5. Preserve the structural format of each artifact. Do not reorganize sections, renumber existing items, or alter content unrelated to the finding being remediated.

Track all changes in a remediation log for inclusion in the final report.

### Step 6: Verification Pass

After applying all remediations, run a lightweight validation to confirm that fixes did not introduce new inconsistencies.

1. Re-read the modified artifacts and rebuild the internal semantic models (requirements inventory, task coverage mapping, scenario coverage mapping, and constitution rule set).
2. Run a reduced set of detection passes focusing on:
   * Cross-reference consistency — verify that renamed terms, merged requirements, and adjusted task mappings are internally consistent.
   * Constitution alignment — confirm that no remediation inadvertently violated a constitution principle.
   * Coverage integrity — ensure that no task or scenario mappings were broken by requirement modifications.
3. If new issues are found:
   * Critical or high issues: fix immediately and re-verify (maximum two correction cycles to prevent infinite loops).
   * Medium or low issues: append them to the report as post-remediation findings.
4. If verification passes cleanly, proceed to the report step.

### Step 7: Produce Analysis Report

Output a markdown report with the following sections.

#### Adversarial Review Summary

Summarize each reviewer's contribution and note areas of strong agreement or notable disagreements that required resolution:

| Reviewer | Model | Focus Area | Findings Count |
|----------|-------|------------|----------------|
| A | Claude Opus 4.6 | Logical Consistency | N |
| B | GPT-5.3 Codex | Technical Feasibility | N |
| C | Gemini 3.1 Pro Preview | Edge Cases and Security | N |

Include a brief narrative describing agreement patterns and any conflicts resolved during synthesis.

#### Unified Findings Table

| ID | Category | Severity | Location(s) | Summary | Recommendation | Consensus |
|----|----------|----------|-------------|---------|----------------|-----------|

Generate stable IDs prefixed by the originating category initial. Add one row per finding. The *Consensus* column indicates *unanimous*, *majority*, or *single*.

#### Coverage Summary Table

| Requirement Key | Has Task? | Task IDs | Has Scenario? | Scenario IDs | Notes |
|-----------------|-----------|----------|---------------|--------------|-------|

#### Remediation Log

| Finding ID | File | Change Description | Original Text (excerpt) | Applied? |
|------------|------|--------------------|-------------------------|----------|

Record every modification made during step 5. For medium findings where the user declined the fix, mark the *Applied?* column as *skipped* with the user's reason if provided.

#### Remaining Issues

List low-severity findings and any medium findings the user chose not to apply, grouped by category. Include the recommendation for each so the user can address them in future iterations.

#### Constitution Alignment Issues

List any constitution conflicts detected, including those that were successfully remediated. Note which principle was affected, the original violation, and how it was resolved.

#### Unmapped Tasks

List tasks with no requirement or scenario mapping, if any remain after remediation.

#### Metrics

*Artifact metrics:*

* Total requirements
* Total tasks
* Total scenarios (if *SCENARIOS.md* present)
* Task coverage percentage (requirements with at least one task)
* Scenario coverage percentage (requirements with at least one scenario, if *SCENARIOS.md* present)
* Non-happy-path scenario percentage (if *SCENARIOS.md* present)

*Finding metrics:*

* Ambiguity count
* Duplication count
* Critical issues found
* Critical issues remediated
* High issues found
* High issues remediated

*Adversarial metrics:*

* Total findings pre-deduplication (sum across all reviewers)
* Total findings post-synthesis (unified count)
* Findings per reviewer (A, B, C individually)
* Agreement rate (percentage of unified findings with *majority* or *unanimous* consensus)
* Conflict count (findings where reviewers disagreed and resolution was required)

#### Next Actions

* If critical issues remain after remediation, recommend resolving them before proceeding to implementation.
* If all critical and high issues were remediated successfully, indicate readiness to proceed.
* Provide explicit command suggestions for any remaining work (for example, re-running *speckit.specify* for requirement refinement, editing *tasks.md* to add missing coverage, or running *speckit.behavior* to generate additional scenarios).

## Reviewer Response Format

Each adversarial reviewer subagent returns a structured response containing:

1. A findings table in markdown with columns: ID, Category, Severity, Location, Summary, Recommendation.
2. A summary paragraph noting the reviewer's overall assessment of artifact quality.
3. A count of findings by severity level.

Example finding row:

```text
| RC-01 | Constitution | CRITICAL | spec.md:§Functional-Requirements | Requirement FR-3 mandates OAuth but constitution prohibits external auth dependencies | Remove OAuth requirement; use session-based auth per constitution principle IV |
```

Reviewers that produce findings outside this format have their output parsed best-effort. Any unparseable items are flagged in the synthesis step and included in the report with a note about the parsing limitation.

## Operating Principles

### Context Efficiency

* Focus on actionable findings rather than exhaustive documentation.
* Load artifacts incrementally using progressive disclosure — do not dump all content into analysis at once.
* Limit findings per reviewer to 30 and unified findings to 50 to keep synthesis and remediation tractable.
* Deterministic results: rerunning without changes should produce consistent IDs, counts, and severity assignments.

### Analysis Guidelines

* Prioritize constitution violations — these are always critical severity.
* Cite specific instances rather than generic patterns when reporting findings.
* Report zero issues gracefully with a success report and full coverage statistics.
* Do not hallucinate missing sections; if a section is absent, report the absence accurately.
* Preserve artifact structure during remediation — do not reorganize unrelated content or reformat sections that were not part of a finding.

### Adversarial Review Principles

* Reviewer independence is essential: each reviewer operates on the same snapshot without knowledge of other reviewers' findings.
* Disagreement is valuable: conflicting findings reveal areas of genuine ambiguity that merit careful resolution by the orchestrating agent.
* Consensus does not guarantee correctness: even unanimous findings are validated against the artifact text and constitution before application.
* Reviewer diversity serves a purpose: each model brings different strengths (logical rigor, implementation awareness, edge-case sensitivity) that complement each other.

## Context

$ARGUMENTS
