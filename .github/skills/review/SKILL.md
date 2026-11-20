---
name: review
description: "Structured code review using tiered persona subagents, confidence-gated findings, and a merge/dedup pipeline. Use when reviewing code changes before creating a PR, as a build gate, or for standalone review."
argument-hint: "[mode:autofix|mode:report-only] [branch name or file paths]"
---

# Code Review

Reviews code changes using dynamically selected reviewer personas. Spawns persona subagents that return structured findings, then merges and deduplicates into a unified report.

Follow
`.github/instructions/strict-safety.instructions.md`: for high-risk diffs, call
out the `ProposedAction`, `ActionRisk`, approval, and rollback gaps that should
be visible before merge or deployment.

## Subagent Depth Constraint

This skill spawns reviewer subagents. Those subagents are leaf executors and MUST NOT spawn their own subagents. Maximum depth: review skill → persona subagent (1 hop).

## Mode Detection

Check arguments for `mode:autofix` or `mode:report-only`. Strip the mode token before interpreting remaining arguments.

| Mode | When | Behavior |
|---|---|---|
| **Interactive** (default) | No mode token | Review, present findings, ask for decisions |
| **Autofix** | `mode:autofix` | No user interaction. Apply `safe_auto` fixes only, write artifact, emit residual work |
| **Report-only** | `mode:report-only` | Read-only. Report findings plus a readiness verdict, with no edits or follow-up item creation |

### Autofix mode rules

- Skip all user questions
- Apply only `safe_auto` findings
- Leave `gated_auto`, `manual`, and `advisory` findings unresolved
- Write a review artifact to `docs/closure/`
- Create backlog follow-up items for unresolved actionable findings
- Record a readiness outcome: `READY`, `READY_WITH_FOLLOWUPS`, or `BLOCKED`
- Never commit, push, or create a PR

### Report-only mode rules

- Skip all user questions
- Never edit files
- Return structured findings plus a readiness outcome to caller
- Do not write a review artifact
- Do not create backlog follow-up items
- Safe for the ship agent to invoke during the build loop

### Stage-run analysis posture (report-only)

When Stage invokes this skill in `mode: report-only` for planning or learning input (for example,
an adversarial analysis of a merged or in-flight PR), report-only MAY write a durable review
artifact under `docs/reviews/` and MAY capture P-021 findings to the stash as capture-only entries.
Every other report-only guarantee is retained: no edits to the reviewed files, no PR mutation, no
thread reply or resolve, and no commit or push. The ship agent's report-only invocation keeps the
default posture above (no artifact, no follow-up items).

## Severity Scale

| Level | Meaning | Build gate action |
|---|---|---|
| **P0** | Critical breakage, exploitable vulnerability, data corruption | Block commit |
| **P1** | High-impact defect in normal usage, breaking contract | Block commit |
| **P2** | Moderate issue (edge case, perf, maintainability) | Record as backlog follow-up item |
| **P3** | Low-impact, minor improvement | User's discretion |

## Action Routing

| Class | Default owner | Meaning |
|---|---|---|
| `safe_auto` | Review skill (autofix mode) | Deterministic local fix |
| `gated_auto` | operator approval | Fix exists but changes behavior/contracts |
| `manual` | Backlog follow-up item | Actionable work requiring human judgment |
| `advisory` | Informational | Learnings, rollout notes, residual risk |

Routing rules:

- Choose the more conservative route on disagreement between personas
- Only `safe_auto` findings enter the autofix queue
- `requires_verification: true` means a fix needs tests or re-review

## Readiness Outcome Contract

Every review run must produce one of these outcomes for the reviewed HEAD:

| Outcome | Meaning | Ship / PR action |
|---|---|---|
| `READY` | Zero unresolved P0/P1 findings and no required follow-up items | PR may be prepared |
| `READY_WITH_FOLLOWUPS` | Zero unresolved P0/P1 findings, but one or more P2/P3 findings need explicit follow-up tracking or residual-risk notes | PR may be prepared only with follow-up handling recorded |
| `BLOCKED` | One or more unresolved P0/P1 findings remain | Do not create or present a PR |

The readiness summary must include:

* reviewed HEAD SHA or equivalent diff identity
* counts for P0, P1, P2, and P3 findings
* follow-up item IDs or residual-risk notes when outcome is `READY_WITH_FOLLOWUPS`
* whether runtime verification follow-up is required

### Local Review Readiness and Dark Mode

This readiness outcome is the local review record consumed by Ship and
pr-lifecycle before PR presentation. When `DARK_MODE_ACTIVE` is present under
P-017, this local review record is the authoritative merge-readiness signal:

* unresolved P0/P1 findings always produce `BLOCKED`
* `READY_WITH_FOLLOWUPS` must include concrete follow-up item IDs or explicit
  residual-risk notes
* hosted Copilot/GitHub review cannot replace this local review record
* advisory shadow-review comments are follow-ups by default unless the operator
  or policy explicitly elevates them to blocking status
* the reviewed HEAD SHA or equivalent diff identity must be current when the PR
  readiness block is written

## Reviewer Personas

### Always-On (every review)

| Persona Subagent | Focus |
|---|---|
| **Constitution Reviewer** | Constitutional compliance |
| **Rust Reviewer** | Language-specific safety and correctness |
| **Correctness Reviewer** | Logic errors, edge cases, and behavioral correctness |
| **Maintainability Reviewer** | Complexity, coupling, and premature abstraction |
| **Learnings Researcher** | Search compound library for related past issues |

### Conditional (based on changed files)

Use a different model from the caller when available to force genuine diversity of critique. Cross-model is preferred but not blocking. For high-risk template, policy, review-surface, or schema/skill diffs, one eligible conditional persona may use the `model_routing.anchor_review` anchor reviewer route when model-specific dispatch is available. If unavailable, record declared degradation and apply the same rubric with the caller's model rather than skipping the persona.

| Persona Subagent | Select when diff touches | Suggested Model |
|---|---|---|
| **Architecture Strategist** | Module boundaries, new abstractions, dependency changes | Different from caller |
| **Concurrency Reviewer** | Concurrent/async patterns | Different from caller |
| **Scope Boundary Auditor** | Changes spanning multiple domains or exceeding expected scope | Different from caller |
| **Agent-Native Parity Reviewer** | MCP SDKs, tool handlers, agent-exposed actions, or user/agent parity-critical flows | Different from caller |
| **Security Reviewer** | Auth middleware, public endpoints, input handling, permission checks, secret management | Different from caller |
| **Template Integrity Reviewer** | `.tmpl` files, Markdown workflow assets, generated artifact references, or policy/instruction surfaces | Different from caller |
| **Schema-CLI-Docs Coupling Reviewer** | Cross-domain diffs spanning schemas, CLI verification logic, install/tune skills, and operator docs | Different from caller |
| **Evidence Consistency Reviewer** | `docs/closure/**`, `docs/memory/**`, `.backlogit/**`, `docs/reviews/**`, or any file whose frontmatter carries a key read by the workspace's own gates (`closure_status`, `compaction_status`, `conditions[].satisfied`); also required when Step 1.5 retrieves a matching class trigger; the shipment-level audit obligation is unconditional (every shipment, fast-path with a recorded no-documentation-surface outcome, per `.github/instructions/shipment-documentation-audit.instructions.md`) | Different from caller |

## Workflow

### Step 1: Determine Review Scope

1. Identify changed files from git diff, explicit file list, or caller-provided scope
2. Categorize each file by type and domain
3. Identify which instruction files apply (via `applyTo` patterns)

### Step 1.5: Review-Pattern Retrieval

Runs between Step 1 and Step 2 so its output can influence persona routing. Hosted-review findings
are predictable, so retrieve the known classes before routing instead of rediscovering them one
hosted round at a time.

1. Derive a class query from the changed-file categories identified in Step 1 (for example closure
   or memory evidence, backlog data, review histories, gate-read frontmatter, skill or instruction text).
2. Search `docs/compound/` by `problem_type`, `category`, `root_cause`, and `tags`, prioritising
   `docs/compound/2026-09-07-copilot-review-finding-pattern-taxonomy.md` (the 12 root-cause classes
   RC-1 to RC-12 and the 10-point evidence-consistency checklist).
3. Emit `PATTERN_RETRIEVAL_OK {n} classes` when one or more classes match, or
   `PATTERN_RETRIEVAL_EMPTY` when none match. Empty is non-blocking.
4. When a retrieved class trigger matches the diff, mark the **Evidence Consistency Reviewer** as
   required for Step 2.
5. Degraded mode: when search tooling is unavailable, log
   `TOOL_DEGRADED: pattern-retrieval — falling back to reading the taxonomy file directly` and read
   the taxonomy file. Never silently skip this step.
6. Taxonomy absent: when the taxonomy file is not present in this workspace's compound library, emit
   `PATTERN_RETRIEVAL_EMPTY` noting `taxonomy unavailable`, still route the Evidence Consistency Reviewer under the
   universal per-shipment audit obligation (file-path triggers select full invocation; a shipment that touches no
   documentation surface may fast-path and must record that outcome), and rely on the persona's embedded 10-point checklist.
7. Carry the result (`PATTERN_RETRIEVAL_OK`, `PATTERN_RETRIEVAL_EMPTY`, or the degraded record) into the
   readiness summary so the caller can record it in the PR readiness block.

### Step 2: Route Personas

1. Always-on: spawn Constitution Reviewer, Rust Reviewer, Correctness Reviewer, Maintainability Reviewer, Learnings Researcher
2. Conditional: analyze changed file paths, content patterns, and workspace agent-native signals to select additional personas:
   * Select **Security Reviewer** (`security-reviewer.agent.md`) when the diff touches: authentication or authorization code, public endpoint handlers, user input processing, permission or role checks, secret or credential management, or files matching `src/server/**, src/tools/**, src/daemon/**, src/db/**, src/config/**, src/shim/** (IPC transport, MCP tool dispatch, DB query construction, config/env parsing, transport security, and files referencing unsafe, deserialize, secret, token, key, password, or credential)`
   * Select **Template Integrity Reviewer** (`template-integrity-reviewer.agent.md`) when the diff touches template files, Markdown harness artifacts, review/policy/instruction assets, or generated-artifact reference tables
   * Select **Schema-CLI-Docs Coupling Reviewer** (`schema-cli-docs-coupling-reviewer.agent.md`) when the diff spans schema files, `src/` verification logic, install/tune skills, or operator-facing documentation in the same change set
   * Select **Evidence Consistency Reviewer** (`evidence-consistency-reviewer.agent.md`) when the diff touches `docs/closure/**`, `docs/memory/**`, `.backlogit/**`, `docs/reviews/**`, or any file whose frontmatter carries a key read by the workspace's own gates (`closure_status`, `compaction_status`, `conditions[].satisfied`), or when Step 1.5 marked it required, or when the shipment's universal documentation consistency audit (`.github/instructions/shipment-documentation-audit.instructions.md`) requires a non-fast-path review. The audit obligation itself applies to every shipment; a shipment that touches no documentation surface may fast-path, but must record that outcome. It reads the branch tree (`git show <rev>:<path>`, `git grep ... <rev>`), never the working tree. The review caller passes the candidate revision `<rev>`: `HEAD` for a committed branch, or, for a pre-commit review (such as the docs-route review that runs before the task commit while `HEAD` is still the baseline), a candidate tree built from a temporary index (`GIT_INDEX_FILE=<scratch path>`, `git read-tree HEAD`, `git add -A -- <owned paths>`, `git write-tree`), which includes the pending changes without touching the real index or advancing `HEAD`, so the persona sees the newly added evidence rather than the old documents
3. When `model_routing.anchor_review` is dispatchable and the diff touches high-risk template, policy, review-surface, or schema/skill contracts, assign the anchor reviewer to one triggered conditional persona (Template Integrity Reviewer by default, otherwise Schema-CLI-Docs Coupling Reviewer). If model-specific dispatch is unavailable, record `TOOL_DEGRADED: model-specific-review-routing — declared fallback: same-model rubric pass` and apply the same rubric.

### Step 3: Spawn Persona Subagents

Spawn all selected personas. Each receives:

- The list of changed files with line ranges
- The diff content relevant to their domain
- Instructions to return structured findings
- Codebase search directive: when the `agent-engram` capability pack is enabled, follow `.github/instructions/agent-engram.instructions.md` and route structural code questions (`list_symbols`, `map_code`, `impact_analysis`, `query_graph`) before grep/glob; use grep/glob for literal-text exemptions or fallback

### Step 4: Collect and Merge Findings

As each persona returns:

1. Collect all findings
2. Deduplicate: merge findings that identify the same issue
3. Assign final severity (more conservative on disagreement)
4. Assign final action routing
5. Derive the readiness outcome:
   * `BLOCKED` if any unresolved P0/P1 findings remain
   * `READY_WITH_FOLLOWUPS` if P0/P1 is clear but actionable P2/P3 findings require backlog follow-up or explicit residual-risk notes
   * `READY` otherwise
6. Report the readiness outcome and the P0-P3 finding counts in the session output

### Step 5: Apply Actions (mode-dependent)

**Interactive mode:**

1. Present findings grouped by severity (P0 first)
2. For each P0/P1, ask the user to accept, modify, or reject the recommendation
3. Apply approved fixes

**Autofix mode:**

1. Apply all `safe_auto` findings automatically
2. Create backlog follow-up items for unresolved actionable findings
3. Write review artifact to `docs/closure/`
4. Include the readiness outcome, reviewed HEAD, and follow-up item IDs in the artifact

**Report-only mode:**

1. Return structured findings and the readiness outcome to caller
2. No side effects: no edits, no review artifact, no follow-up items (the Stage-run analysis posture above is the only exception)

When the diff changes runtime surfaces, include an explicit recommendation for whether follow-up runtime verification is required and which mode (`manual`, `api`, `browser`) is appropriate.

When the diff includes destructive potential, contract changes, migrations,
security-sensitive edits, or other high-blast-radius work, include an explicit
recommendation for whether strict-safety action classification or approval
follow-up is required before merge or deployment.

When this review surfaces 3 or more P0/P1 findings (or the diff touches security-sensitive or
compliance-critical surfaces), recommend escalation to the `adversarial-review` agent
(`.github/agents/subagents/adversarial-review.agent.md`, following
`.github/instructions/adversarial-review.instructions.md`) for multi-model consensus validation
before blocking the build.

### Post-Fix Delta Check

A bounded, mechanical check that runs after each review-fix commit, on the touched files only,
immediately before push. It is referenced from pr-lifecycle Step 4b. Fixes create the most recurrent
hosted-review defects (the fix's own new prose becomes a new evidence surface), so the delta check
verifies exactly four drift-prone classes:

| Check | Class | Rule |
|---|---|---|
| `head_sha_parity` | RC-9 | The PR body's current-HEAD readiness value equals the SHA being pushed: `git rev-parse HEAD` before the push, then the PR `headRefOid` once the push succeeds, exactly as pr-lifecycle Step 5c and §1.9.4 Check 1 require. A committed evidence artifact cannot contain its own SHA, so it records a separate historical `reviewed_subject_sha` (the commit it reviewed, which may be a parent) and says so explicitly; that historical value is never substituted for the PR-body current-HEAD value. |
| `identifier_propagation` | RC-10 | `git grep` each corrected identifier (stash ID, shipment ID, verdict token); every hit agrees. |
| `chronology_verbs` | RC-11 | Every before / prior-to / immediately-before mutation claim resolves to evidence whose commit precedes the mutation, OR the same paragraph contains the word `reconstructed`. This enforces disclosure, not prohibition. |
| `date_ordering` | RC-12 | Every disposition date is on or after the `created_at` of the entity it dispositions. |

Emit a 4-line pass/fail block (one line per check) into the PR Review History.

Binding scope rules:

* The delta check is **NOT a review-fix cycle** and does **not** consume circuit-breaker budget; it only
  verifies the fix just made. See `docs/compound/093-S-review-loop-convergence.md` and
  `docs/compound/2026-08-16-bounded-review-fix-cycle-scope-and-mechanical-consequence-judgment.md`.
* Timing: validate the local candidate SHA (`git rev-parse HEAD`) against the local readiness record before the
  push, and retain the 4-line result in the session output (this is the only check possible before a PR exists).
  After a successful push, verify that the PR body readiness value and the remote `headRefOid` equal that same
  SHA, and record the result in the PR Review History. The check never weakens the §1.9 merge gate.
* A failing check is corrected as part of the same fix (same contract surface, P-021 C1) and re-run once; it
  is never expanded into out-of-scope work.
* Owner: Ship, during its review-fix loop. Stage may run it in the report-only analysis posture. It is never
  a persona subagent.

## Quality Criteria

* Every changed file is reviewed by at least the always-on personas
* All P0 findings are addressed before the review is marked complete
* P1 findings require explicit acknowledgment (fix or defer with rationale)
* The review report accurately reflects all findings and their resolution status


## Model Routing

This skill operates at **Tier 2 (Standard)** — review coordination and finding assembly.

Generated by gsd | Template: review/SKILL.md.tmpl
